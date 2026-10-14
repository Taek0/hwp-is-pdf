use hwp_engine::{layout::Engine, model::Document, pdf::Op};
fn main() -> anyhow::Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    let doc = Document::open(std::path::Path::new(&a[0]))?;
    let mut e = Engine::new(
        &doc,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render()?;
    let p = a[1].parse::<usize>()? - 1;
    let x0 = a[2].parse::<f64>()?;
    let y0 = a[3].parse::<f64>()?;
    for op in &e.pages[p].ops {
        if let Op::Text { x, y, .. } = op
            && *x >= x0
            && *x < x0 + 130.
            && *y >= y0
            && *y < y0 + 40.
        {
            println!("{op:?}");
        }
    }
    Ok(())
}
