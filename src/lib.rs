//! 본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.
pub mod chart;
mod decoration;
pub mod distribution;
pub mod graphics;
pub mod hwp;
mod image_layout;
pub mod layout;
pub mod math;
mod math_extra;
pub mod model;
mod number_format;
mod pagination;
pub mod pdf;
pub mod qa;
mod reflow;
mod tabs;
mod wrap_region;

use anyhow::Result;
use std::path::Path;
pub fn convert(input: &Path, output: &Path, font_dir: &Path) -> Result<layout::Report> {
    convert_with_reflow(input, output, font_dir, false)
}
/// Reflow supported mixed-content sections, or recover sections without line caches.
pub fn convert_with_reflow(
    input: &Path,
    output: &Path,
    font_dir: &Path,
    force: bool,
) -> Result<layout::Report> {
    convert_with_options(input, output, font_dir, force, false)
}
/// Contour wrapping is an opt-in geometric approximation, independent of source SQUARE semantics.
pub fn convert_with_options(
    input: &Path,
    output: &Path,
    font_dir: &Path,
    force: bool,
    contours: bool,
) -> Result<layout::Report> {
    anyhow::ensure!(input != output, "Input and output must differ");
    let document = model::Document::open(input)?;
    let mut engine = layout::Engine::new(&document, font_dir);
    if contours {
        engine.render_contours()?;
    } else if force {
        engine.render_reflow()?;
    } else {
        engine.render()?;
    }
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
