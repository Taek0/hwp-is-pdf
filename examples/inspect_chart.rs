//! Inspect chart data and retained native presentation without rendering.
fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("INPUT.hwp|hwpx"))?;
    let doc = hwp_engine::model::Document::open(std::path::Path::new(&path))?;
    for section in &doc.sections {
        let mut charts = Vec::new();
        section.all("chart", &mut charts);
        for chart in charts {
            println!("{chart:#?}");
        }
    }
    Ok(())
}
