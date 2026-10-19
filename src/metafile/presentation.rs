//! Recover a complete static OLE mask/color bitmap pair without executing WMF.
use super::*;
use std::io::Cursor;
pub(crate) fn picture(data: &[u8]) -> Option<Vec<u8>> {
    decode(data).ok()
}
fn decode(data: &[u8]) -> Result<Vec<u8>> {
    ensure!(data.len() <= 64 * 1024 * 1024, "Oversized OLE presentation");
    // Standard CF_METAFILEPICT, empty target device, DVASPECT_CONTENT.
    ensure!(
        u32(data, 0)? == u32::MAX
            && u32(data, 4)? == 3
            && u32(data, 8)? == 4
            && u32(data, 12)? == 1,
        "Unsupported presentation header"
    );
    let size = u32(data, 36)? as usize;
    ensure!(
        size == data.len().saturating_sub(40),
        "Presentation size mismatch"
    );
    let rs = records(&data[40..])?;
    ensure!(
        rs.iter().all(|r| matches!(
            r.kind,
            0 | 0x626 | 0x103 | 0x107 | 0x20b | 0x20c | 0x1e | 0x127 | 0xb41
        )),
        "Non-bitmap presentation operations"
    );
    let draws: Vec<_> = rs.iter().filter(|r| r.kind == 0xb41).collect();
    ensure!(
        draws.len() == 2
            && u32(draws[0].data, 0)? == 0x00ee0086
            && u32(draws[1].data, 0)? == 0x008800c6,
        "Expected complete mask/color pair"
    );
    let mut decoded = Vec::new();
    for r in draws {
        let d = r.data;
        ensure!(
            u16(d, 4)? == u16(d, 12)?
                && u16(d, 6)? == u16(d, 14)?
                && [8, 10, 16, 18]
                    .into_iter()
                    .all(|i| u16(d, i).ok() == Some(0)),
            "Partial bitmap presentation"
        );
        let mut dib = 0x00cc0020u32.to_le_bytes().to_vec();
        dib.extend([0, 0]);
        dib.extend_from_slice(&d[4..]);
        let image = image::load_from_memory(&raster::bitmap(&dib)?)?.to_rgba8();
        ensure!(
            image.width() == u16(d, 6)? as u32 && image.height() == u16(d, 4)? as u32,
            "Cropped presentation bitmap"
        );
        decoded.push(image);
    }
    let color = decoded.pop().unwrap();
    let mask = decoded.pop().unwrap();
    ensure!(
        mask.dimensions() == color.dimensions(),
        "Mismatched presentation bitmaps"
    );
    ensure!(
        u32(data, 28)? == mask.width() && u32(data, 32)? == mask.height(),
        "Presentation does not cover declared extent"
    );
    for r in &rs {
        if r.kind == 0x20b {
            ensure!(
                u16(r.data, 0)? == 0 && u16(r.data, 2)? == 0,
                "Translated presentation window"
            );
        }
        if r.kind == 0x20c {
            ensure!(
                u16(r.data, 0)? as u32 == mask.height() && u16(r.data, 2)? as u32 == mask.width(),
                "Scaled presentation window"
            );
        }
    }
    let mut result = color;
    for (p, m) in result.pixels_mut().zip(mask.pixels()) {
        ensure!(
            m[0] == m[1] && m[1] == m[2] && matches!(m[0], 0 | 255),
            "Non-binary presentation mask"
        );
        p[3] = m[0];
    }
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(result).write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn truncated_or_nonstandard_presentations_are_not_used() {
        assert!(picture(&[]).is_none());
        assert!(picture(&[0; 40]).is_none());
        let mut b = vec![0; 40];
        b[..4].fill(255);
        b[4] = 3;
        b[8] = 4;
        b[12] = 1;
        assert!(picture(&b).is_none());
    }
}
