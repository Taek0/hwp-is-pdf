//! Conservative alpha-cell spans, normalized to the displayed crop.
use crate::model::Node;
pub(crate) fn alpha_rects(bytes: &[u8], node: &Node) -> anyhow::Result<Option<Vec<[f64; 4]>>> {
    if crate::metafile::is_wmf(bytes) {
        return Ok(None);
    }
    let decoded = image::load_from_memory(bytes)?;
    if !decoded.color().has_alpha() {
        return Ok(None);
    }
    let image = decoded.to_rgba8();
    if image.pixels().all(|p| p[3] == 255) {
        return Ok(None);
    }
    let (w, h) = (image.width(), image.height());
    let mut crop = [0., 0., 1., 1.];
    if let Some(c) = node
        .child("imgClip")
        .filter(|c| c.n("right") > c.n("left") && c.n("bottom") > c.n("top"))
    {
        let dim = node.child("imgDim");
        let explicit = dim.is_some_and(|d| d.n("dimwidth") > 0. && d.n("dimheight") > 0.);
        let (dw, dh) = if explicit {
            (dim.unwrap().n("dimwidth"), dim.unwrap().n("dimheight"))
        } else {
            (w as f64 * 75., h as f64 * 75.)
        };
        crop = [
            c.n("left") / dw,
            c.n("top") / dh,
            c.n("right") / dw,
            c.n("bottom") / dh,
        ];
        if !explicit {
            for bound in &mut crop[2..] {
                if *bound > 1. && *bound < 1.01 {
                    *bound = 1.;
                }
            }
        }
    }
    let (sx, sy) = (w.div_ceil(128).max(1), h.div_ceil(128).max(1));
    let mut out = Vec::new();
    for y in (0..h).step_by(sy as usize) {
        let mut run = None;
        for x in (0..=w).step_by(sx as usize).chain(std::iter::once(w)) {
            let opaque = x < w
                && (y..(y + sy).min(h))
                    .any(|yy| (x..(x + sx).min(w)).any(|xx| image.get_pixel(xx, yy)[3] > 0));
            if opaque && run.is_none() {
                run = Some(x);
            }
            if !opaque && let Some(start) = run.take() {
                let a = (start as f64 / w as f64).max(crop[0]);
                let b = (x as f64 / w as f64).min(crop[2]);
                let top = (y as f64 / h as f64).max(crop[1]);
                let bottom = ((y + sy).min(h) as f64 / h as f64).min(crop[3]);
                if b > a && bottom > top {
                    out.push([
                        (a - crop[0]) / (crop[2] - crop[0]),
                        (top - crop[1]) / (crop[3] - crop[1]),
                        (b - crop[0]) / (crop[2] - crop[0]),
                        (bottom - crop[1]) / (crop[3] - crop[1]),
                    ]);
                }
            }
        }
    }
    Ok(Some(out))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cropped_alpha_spans_keep_holes_and_exclude_transparent_pixels() {
        let image = image::RgbaImage::from_fn(8, 4, |x, _| {
            if x == 2 || x == 5 {
                image::Rgba([200, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 0, 0])
            }
        });
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let mut n = Node::default();
        let mut clip = Node {
            name: "imgClip".into(),
            ..Default::default()
        };
        for (k, v) in [("left", 75), ("right", 525), ("top", 0), ("bottom", 300)] {
            clip.attrs.insert(k.into(), v.to_string());
        }
        n.children.push(clip);
        let spans = alpha_rects(bytes.get_ref(), &n).unwrap().unwrap();
        assert_eq!(spans.len(), 8);
        assert!(
            spans
                .iter()
                .all(|r| r[2] <= 1. / 3. + 1e-9 || r[0] >= 2. / 3. - 1e-9)
        );
    }
}
