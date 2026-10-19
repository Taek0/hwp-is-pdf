use hwp_engine::{layout::Engine, model::Document};
use std::path::Path;
#[test]
fn exported_paragraph_rules_do_not_cross_realized_text_or_the_next_paragraph() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("hwp-editor/.tools/law-probe.hwpx");
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !path.exists() || !fonts.exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    let q = hwp_engine::qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(
        q.border_crossings.is_empty(),
        "{} border crossings",
        q.border_crossings.len()
    );
    assert!(q.text_overlaps.is_empty() && q.outside_page.is_empty());
}
