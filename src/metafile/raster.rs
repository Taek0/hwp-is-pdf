use super::*;
pub(super) fn bitmap(d: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        u32(d, 0)? == 0x00cc0020 && u16(d, 4)? == 0,
        "Unsupported WMF raster operation/palette"
    );
    let dib = d.get(22..).context("Missing WMF DIB")?;
    let header = u32(dib, 0)? as usize;
    ensure!(
        (40..=124).contains(&header) && dib.len() >= header,
        "Unsupported WMF DIB header"
    );
    let width = u32(dib, 4)? as i32;
    let height = u32(dib, 8)? as i32;
    ensure!(
        width > 0 && height != 0 && (width as i64) * (height as i64).abs() <= 16_000_000,
        "Oversized WMF DIB"
    );
    let bits = u16(dib, 14)?;
    ensure!(
        matches!(bits, 1 | 4 | 8 | 16 | 24 | 32),
        "Unsupported WMF bitmap depth"
    );
    let used = u32(dib, 32)? as usize;
    let palette = if used > 0 {
        used
    } else if bits <= 8 {
        1usize << bits
    } else {
        0
    };
    ensure!(palette <= 256, "Oversized WMF palette");
    let masks = if header == 40 && u32(dib, 16)? == 3 {
        12
    } else {
        0
    };
    let offset = 14 + header + palette * 4 + masks;
    ensure!(offset <= 14 + dib.len(), "Truncated WMF palette");
    let mut bmp = b"BM".to_vec();
    bmp.extend(((14 + dib.len()) as u32).to_le_bytes());
    bmp.extend([0; 4]);
    bmp.extend((offset as u32).to_le_bytes());
    bmp.extend(dib);
    // Decode here to reject invalid image dimensions/data before PDF writing.
    image::load_from_memory(&bmp).context("Invalid WMF bitmap")?;
    Ok(bmp)
}
