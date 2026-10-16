//! Shared HWP/HWPX brightness, contrast and grayscale/monochrome raster effects.
use crate::model::Node;
use anyhow::{Context, Result};
use std::{collections::BTreeMap, io::Cursor};
pub(crate) fn apply(nodes: &mut [Node], binaries: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
    for node in nodes {
        if node.name == "img" && node.a("effectsApplied") != "1" {
            let brightness = node.n("bright").clamp(-100., 100.);
            let contrast = node.n("contrast").clamp(-100., 100.);
            let gray = matches!(node.a("effect"), "1" | "GRAY_SCALE" | "2" | "BLACK_WHITE");
            let monochrome = matches!(node.a("effect"), "2" | "BLACK_WHITE");
            if brightness != 0. || contrast != 0. || gray {
                let id = node.a("binaryItemIDRef").to_owned();
                let derived = format!("{id}-effect-{brightness}-{contrast}-{gray}-{monochrome}");
                if !binaries.contains_key(&derived) {
                    let image = image::load_from_memory(
                        binaries.get(&id).context("Missing effect image")?,
                    )?;
                    let mut rgba = image.to_rgba8();
                    for px in rgba.pixels_mut() {
                        // Hancom's grayscale uses the encoded RGB luma weights,
                        // not the image crate's linear-light luminance weights.
                        let luma =
                            0.299 * px[0] as f64 + 0.587 * px[1] as f64 + 0.114 * px[2] as f64;
                        for v in &mut px.0[..3] {
                            let adjusted =
                                tone(if gray { luma } else { *v as f64 }, brightness, contrast);
                            *v = if monochrome {
                                if adjusted >= 128. { 255 } else { 0 }
                            } else {
                                adjusted.clamp(0., 255.) as u8
                            };
                        }
                    }
                    let mut out = Cursor::new(Vec::new());
                    image::DynamicImage::ImageRgba8(rgba)
                        .write_to(&mut out, image::ImageFormat::Png)?;
                    binaries.insert(derived.clone(), out.into_inner());
                }
                node.attrs.insert("binaryItemIDRef".into(), derived);
                node.attrs.insert("effectsApplied".into(), "1".into());
            }
        }
        apply(&mut node.children, binaries)?;
    }
    Ok(())
}

fn tone(value: f64, brightness: f64, contrast: f64) -> f64 {
    let gain = 1. + contrast / 100.;
    let black = 127.5 * (1. - gain);
    // Brightness adds a constant offset, preserving the contrast slope until
    // clipping. Calibrated against independent and combined Hancom exports;
    // see reference/IMAGE_EFFECTS.md for the tested parameter range.
    value * gain + black + brightness / 100. * (255. - black)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tone_matches_independent_hancom_pixel_samples() {
        // Median decoded pixels at black, red (238,6,45), and white source
        // regions in the four user exports. Allow one level for JPEG rounding.
        for (brightness, contrast, black, middle, white) in [
            (0., 0., 0., 79., 255.),
            (70., 0., 178., 255., 255.),
            (0., -50., 64., 103., 191.),
            (70., -50., 197., 237., 255.),
        ] {
            for (input, expected) in [(0., black), (79.814, middle), (255., white)] {
                let actual = tone(input, brightness, contrast).clamp(0., 255.).floor();
                assert!((actual - expected).abs() <= 1.);
            }
        }
    }
    #[test]
    fn grayscale_uses_encoded_rgb_luma_and_keeps_alpha() {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([238, 6, 45, 67]));
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let mut binaries = BTreeMap::from([("x".into(), png.into_inner())]);
        let mut nodes =
            vec![crate::model::xml(br#"<img binaryItemIDRef="x" effect="GRAY_SCALE"/>"#).unwrap()];
        apply(&mut nodes, &mut binaries).unwrap();
        let result = image::load_from_memory(&binaries[nodes[0].a("binaryItemIDRef")])
            .unwrap()
            .to_rgba8();
        assert_eq!(*result.get_pixel(0, 0), image::Rgba([79, 79, 79, 67]));
    }
    #[test]
    fn effects_preserve_alpha_and_source_pixels_and_are_idempotent() {
        let mut image = image::RgbaImage::new(2, 1);
        image.put_pixel(0, 0, image::Rgba([240, 50, 20, 64]));
        image.put_pixel(1, 0, image::Rgba([255, 255, 255, 0]));
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let source = png.into_inner();
        let mut binaries = BTreeMap::from([("x".into(), source.clone())]);
        let mut nodes = vec![
            crate::model::xml(br#"<img binaryItemIDRef="x" effect="BLACK_WHITE" bright="10"/>"#)
                .unwrap(),
        ];
        apply(&mut nodes, &mut binaries).unwrap();
        let id = nodes[0].a("binaryItemIDRef").to_owned();
        let result = image::load_from_memory(&binaries[&id]).unwrap().to_rgba8();
        assert_eq!(result.get_pixel(0, 0)[3], 64);
        assert_eq!(result.get_pixel(1, 0)[3], 0);
        assert!(
            result
                .pixels()
                .all(|p| p[0] == p[1] && p[1] == p[2] && matches!(p[0], 0 | 255))
        );
        apply(&mut nodes, &mut binaries).unwrap();
        assert_eq!(nodes[0].a("binaryItemIDRef"), id);
        assert_eq!(binaries["x"], source);
        assert_eq!(binaries.len(), 2);
    }
}
