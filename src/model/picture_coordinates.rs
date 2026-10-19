//! Recover retained crop coordinates when a writer replaces a source bitmap.
use super::Node;
use std::{collections::BTreeMap, io::Cursor};
pub(super) fn normalize(nodes: &mut [Node], binaries: &BTreeMap<String, Vec<u8>>) {
    for n in nodes {
        if n.name == "pic"
            && n.a("binaryHwp") != "1"
            && n.child("imgDim").is_none()
            && let (Some(clip), Some(rect), Some(image)) =
                (n.child("imgClip"), n.child("imgRect"), n.child("img"))
            && let Some(bytes) = binaries.get(image.a("binaryItemIDRef"))
            && let Ok(reader) = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()
            && let Ok((pw, ph)) = reader.into_dimensions()
        {
            let points: Vec<_> = rect
                .children
                .iter()
                .filter(|p| p.name.starts_with("pt"))
                .collect();
            let left = points
                .iter()
                .map(|p| p.signed("x"))
                .fold(f64::INFINITY, f64::min);
            let top = points
                .iter()
                .map(|p| p.signed("y"))
                .fold(f64::INFINITY, f64::min);
            let right = points.iter().map(|p| p.signed("x")).fold(0., f64::max);
            let bottom = points.iter().map(|p| p.signed("y")).fold(0., f64::max);
            if points.len() == 4
                && left == 0.
                && top == 0.
                && right > 0.
                && bottom > 0.
                && clip.n("right") > clip.n("left")
                && clip.n("bottom") > clip.n("top")
                && clip.signed("left") >= 0.
                && clip.signed("top") >= 0.
                && clip.n("right") <= right
                && clip.n("bottom") <= bottom
                && (clip.n("right") > pw as f64 * 75. * 1.01
                    || clip.n("bottom") > ph as f64 * 75. * 1.01)
            {
                n.children.push(Node {
                    name: "imgDim".into(),
                    attrs: BTreeMap::from([
                        ("dimwidth".into(), right.to_string()),
                        ("dimheight".into(), bottom.to_string()),
                    ]),
                    ..Default::default()
                });
                n.attrs
                    .insert("cropBasis".into(), "retainedImageRect".into());
            }
        }
        normalize(&mut n.children, binaries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_pixels_do_not_erase_a_crop_inside_the_retained_rectangle() {
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            8,
            8,
            image::Rgb([200, 200, 200]),
        ))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
        let binaries = BTreeMap::from([("image".into(), png.into_inner())]);
        let source=crate::model::xml(br#"<pic><img binaryItemIDRef="image"/><imgRect><pt0 x="0" y="0"/><pt1 x="60000" y="0"/><pt2 x="60000" y="12000"/><pt3 x="0" y="12000"/></imgRect><imgClip left="400" top="0" right="13000" bottom="2600"/></pic>"#).unwrap();
        let mut n = source.clone();
        normalize(std::slice::from_mut(&mut n), &binaries);
        assert_eq!(n.child("imgDim").unwrap().n("dimwidth"), 60000.);
        let normalized = n.clone();
        normalize(std::slice::from_mut(&mut n), &binaries);
        assert_eq!(n, normalized);
        let mut native = source;
        native.attrs.insert("binaryHwp".into(), "1".into());
        normalize(std::slice::from_mut(&mut native), &binaries);
        assert!(native.child("imgDim").is_none());
    }
}
