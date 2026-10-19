//! Preserve source bytes, but make missing BMP pixel payloads explicit to layout.
use super::Node;
use anyhow::Result;
use std::{collections::BTreeMap, io::Cursor};

pub(super) fn mark(nodes: &mut [Node], binaries: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
    for node in nodes {
        if node.name == "img" {
            let id = node.a("binaryItemIDRef").to_string();
            if let Some(bytes) = binaries.get(&id)
                && truncated_rgb_bmp(bytes)
            {
                let replacement = format!("{id}-unavailable-pixels");
                if !binaries.contains_key(&replacement) {
                    let mut png = Cursor::new(Vec::new());
                    image::DynamicImage::ImageRgba8(image::RgbaImage::new(1, 1))
                        .write_to(&mut png, image::ImageFormat::Png)?;
                    binaries.insert(replacement.clone(), png.into_inner());
                }
                node.attrs.insert("unavailableSourceImage".into(), id);
                node.attrs.insert("binaryItemIDRef".into(), replacement);
            }
        }
        mark(&mut node.children, binaries)?;
    }
    Ok(())
}
fn truncated_rgb_bmp(b: &[u8]) -> bool {
    if b.len() < 54 || !b.starts_with(b"BM") {
        return false;
    }
    let word = |at| u16::from_le_bytes(b[at..at + 2].try_into().unwrap());
    let dword = |at| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    let (w, h) = (dword(18) as i32, dword(22) as i32);
    if dword(14) != 40
        || dword(30) != 0
        || word(26) != 1
        || !matches!(word(28), 24 | 32)
        || w <= 0
        || h == 0
        || w as u64 * h.unsigned_abs() as u64 > 16_000_000
    {
        return false;
    }
    let offset = dword(10) as u64;
    let required = offset + (w as u64 * word(28) as u64).div_ceil(32) * 4 * h.unsigned_abs() as u64;
    offset >= 54
        && offset <= b.len() as u64
        && dword(2) as u64 >= required
        && required > b.len() as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_provably_missing_uncompressed_pixels_are_marked() {
        let mut b = vec![0; 54];
        b[..2].copy_from_slice(b"BM");
        for (at, v) in [(2, 70u32), (10, 54), (14, 40), (18, 2), (22, 2)] {
            b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        b[26] = 1;
        b[28] = 24;
        assert!(truncated_rgb_bmp(&b));
        b.resize(70, 0);
        assert!(!truncated_rgb_bmp(&b));
        b.truncate(54);
        b[30] = 1;
        assert!(!truncated_rgb_bmp(&b));
    }
}
