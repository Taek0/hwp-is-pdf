fn main() -> anyhow::Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    let x = image::open(&a[0])?.to_rgb8();
    let y = image::open(&a[1])?.to_rgb8();
    anyhow::ensure!(
        x.dimensions() == y.dimensions(),
        "Different raster dimensions"
    );
    let mut b = [x.width(), x.height(), 0, 0];
    let mut n = 0u64;
    for (px, py, p) in x.enumerate_pixels() {
        if p != y.get_pixel(px, py) {
            n += 1;
            b[0] = b[0].min(px);
            b[1] = b[1].min(py);
            b[2] = b[2].max(px);
            b[3] = b[3].max(py);
        }
    }
    println!("changed_pixels={n}, bounds={b:?}");
    Ok(())
}
