//! HWP binary adapter: images.
use super::*;

pub(super) fn mark_pcx(nodes: &mut [Node], ids: &BTreeSet<String>) {
    for node in nodes {
        if node.name == "img" && ids.contains(node.a("binaryItemIDRef")) {
            node.attrs.insert("sourceFormat".into(), "PCX".into());
        }
        mark_pcx(&mut node.children, ids);
    }
}

pub(super) fn decode_pcx(d: &[u8]) -> Result<Vec<u8>> {
    ensure!(d.len() >= 128 && d[0] == 10, "Invalid PCX");
    let bits = d[3] as usize;
    let planes = d[65] as usize;
    let stride = u16(d, 66)? as usize;
    let w = (u16(d, 8)? as usize)
        .checked_sub(u16(d, 4)? as usize)
        .context("PCX width")?
        + 1;
    let h = (u16(d, 10)? as usize)
        .checked_sub(u16(d, 6)? as usize)
        .context("PCX height")?
        + 1;
    ensure!(
        w * h <= 16_000_000 && stride * planes * h <= LIMIT,
        "PCX too large"
    );
    ensure!(
        matches!((bits, planes), (1, 1..=4) | (8, 1) | (8, 3)),
        "Unsupported PCX format {bits}/{planes}"
    );
    ensure!(stride * 8 >= w * bits, "PCX stride");
    let required = stride * planes * h;
    let mut raw = Vec::with_capacity(required);
    let mut p = 128;
    while raw.len() < required {
        let v = *d.get(p).context("Truncated PCX")?;
        p += 1;
        let (count, value) = if v & 0xc0 == 0xc0 {
            let value = *d.get(p).context("PCX RLE")?;
            p += 1;
            ((v & 63) as usize, value)
        } else {
            (1, v)
        };
        ensure!(
            count > 0 && raw.len() + count <= required,
            "Invalid PCX RLE"
        );
        raw.resize(raw.len() + count, value);
    }
    let mut im = image::RgbImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let rgb = if bits == 8 && planes == 3 {
                [
                    raw[y * stride * 3 + x],
                    raw[y * stride * 3 + stride + x],
                    raw[y * stride * 3 + 2 * stride + x],
                ]
            } else {
                let index = if bits == 1 {
                    let mut v = 0;
                    for plane in 0..planes {
                        v |= (((raw[y * stride * planes + plane * stride + x / 8] >> (7 - x % 8))
                            & 1) as usize)
                            << plane;
                    }
                    v
                } else {
                    raw[y * stride + x] as usize
                };
                let palette = if bits == 8 {
                    ensure!(
                        d.len() >= 769 && d[d.len() - 769] == 12,
                        "Missing PCX palette"
                    );
                    d.len() - 768
                } else {
                    16
                };
                bytes(d, palette + index * 3, 3)?.try_into()?
            };
            im.put_pixel(x as u32, y as u32, image::Rgb(rgb));
        }
    }
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(im).write_to(&mut output, image::ImageFormat::Png)?;
    Ok(output.into_inner())
}
