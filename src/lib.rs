//! 본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.
pub mod chart;
mod decoration;
pub mod distribution;
pub mod hwp;
pub mod layout;
pub mod math;
mod math_extra;
pub mod model;
pub mod pdf;
pub mod qa;

use anyhow::Result;
use std::path::Path;
pub fn convert(input: &Path, output: &Path, font_dir: &Path) -> Result<layout::Report> {
    anyhow::ensure!(input != output, "Input and output must differ");
    let document = model::Document::open(input)?;
    let mut engine = layout::Engine::new(&document, font_dir);
    engine.render()?;
    if let Some(p) = output.parent() {
        std::fs::create_dir_all(p)?;
    }
    pdf::save(output, &engine.pages, &mut engine.fonts, &document.binaries)?;
    let audit = qa::inspect(&engine.pages, &engine.fonts, &engine.report.bounds);
    std::fs::write(
        output.with_extension("qa.json"),
        serde_json::to_vec_pretty(&audit)?,
    )?;
    std::fs::write(
        output.with_extension("report.json"),
        serde_json::to_vec_pretty(&engine.report)?,
    )?;
    Ok(engine.report)
}
