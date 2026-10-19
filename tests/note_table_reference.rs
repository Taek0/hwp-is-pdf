use hwp_engine::{
    layout::Engine,
    model::{Document, text},
};
use std::path::Path;
#[test]
fn repeated_two_row_header_with_body_notes_does_not_carry_empty_row_tails() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp");
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !path.exists() || !fonts.exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let mut ps = Vec::new();
    d.sections[0].all("p", &mut ps);
    let label = ps.into_iter().find(|p| text(p) == "[개발부담금]").unwrap();
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    assert_eq!(e.pages.len(), 50);
    let line = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "line" && b.id == label.a("id"))
        .unwrap();
    assert_eq!(line.page, 33);
    let baseline = line.y + label.find("lineseg").unwrap().n("baseline") / 100.;
    assert!((baseline - 163.5625).abs() < 0.2, "{baseline}");
    let q = hwp_engine::qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(
        q.border_crossings.is_empty() && q.text_overlaps.is_empty() && q.outside_page.is_empty()
    );
}
