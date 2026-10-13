//! Crop an existing PDF raster for a legible QA preview; does not modify the PDF.
fn main() -> anyhow::Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(a.len() == 6, "INPUT.png OUTPUT.png X Y WIDTH HEIGHT");
    let im = image::open(&a[0])?;
    let p: Vec<u32> = a[2..].iter().map(|s| s.parse()).collect::<Result<_, _>>()?;
    anyhow::ensure!(
        p[0] + p[2] <= im.width() && p[1] + p[3] <= im.height(),
        "Crop outside image"
    );
    im.crop_imm(p[0], p[1], p[2], p[3]).save(&a[1])?;
    Ok(())
}
