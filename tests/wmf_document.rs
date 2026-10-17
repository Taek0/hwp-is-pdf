use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
use std::path::Path;
#[test]
fn supplied_wmf_document_keeps_rasters_vectors_text_and_caption_clearance() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-Minimal-Print/02_reference_ready/06_wmf_document.hwp");
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !input.exists() || !fonts.exists() {
        return;
    }
    let d = Document::open(&input).unwrap();
    assert_eq!(
        d.binaries.keys().filter(|k| k.contains("-wmf-")).count(),
        21
    );
    let original = d.sections.clone();
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    assert_eq!(e.pages.len(), 11);
    assert_eq!(e.report.pictures, 13);
    assert_eq!(
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter(|o| matches!(o,Op::Image{id,..} if id.contains("-wmf-")))
            .count(),
        21
    );
    assert!(
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .any(|o| matches!(o,Op::Path{points,..} if points.len()>3))
    );
    let first = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "metafile" && b.page == 1)
        .unwrap();
    assert!((first.y - 468.716).abs() < 0.01);
    let wide = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "metafile" && b.page == 3 && b.w > 400.)
        .unwrap();
    assert!(
        (wide.x - 93.55).abs() < 0.1 && (wide.w - 426.97).abs() < 0.3,
        "Native picture must preserve its paper-contained margin overlap: x={}, w={}",
        wide.x,
        wide.w
    );
    assert!(
        !e.report
            .warnings
            .iter()
            .any(|w| w.starts_with("Oversized pic"))
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
    assert!(q.foreground_image_overlaps.is_empty());
    assert_eq!(original, d.sections);
}
