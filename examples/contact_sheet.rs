//! File-only visual QA helper: assemble Poppler PNGs, no Python dependency.
use anyhow::{Context, Result};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let output = args
        .first()
        .context("Usage: contact_sheet OUTPUT.png PAGE1.png ...")?;
    let files = &args[1..];
    let count = files.len() as u32;
    anyhow::ensure!(count > 0, "No images");
    let mut sheet = image::RgbImage::from_pixel(
        320 * count.min(4),
        480 * count.div_ceil(4),
        image::Rgb([210, 210, 210]),
    );
    for (i, path) in files.iter().enumerate() {
        let im = image::open(path)?.thumbnail(310, 450).to_rgb8();
        image::imageops::overlay(
            &mut sheet,
            &im,
            (i % 4 * 320 + 5) as i64,
            (i / 4 * 480 + 5) as i64,
        );
    }
    sheet.save(output)?;
    Ok(())
}
