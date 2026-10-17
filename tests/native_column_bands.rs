use hwp_engine::{layout::Engine, model::Document, qa};
use std::path::Path;

#[test]
fn supplied_column_document_keeps_body_and_nested_columns() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-Minimal-Print/01_print/03_column_layout.hwp");
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !input.exists() || !fonts.exists() {
        return;
    }
    let d = Document::open(&input).unwrap();
    let source = d.sections.clone();
    let p = d.sections[0]
        .children_named("p")
        .find(|p| {
            p.children_named("run")
                .flat_map(|r| r.children_named("ctrl"))
                .any(|c| c.child("colPr").is_some_and(|c| c.u("colCount") == 3))
        })
        .unwrap();
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    assert_eq!(e.pages.len(), 4);
    let lines: Vec<_> = e
        .report
        .bounds
        .iter()
        .filter(|b| b.kind == "line" && b.id == p.a("id"))
        .collect();
    assert_eq!(lines.len(), 11);
    for (i, x) in [85.08, 230.64, 376.08].into_iter().enumerate() {
        assert_eq!(lines[i].page, 2);
        assert!((lines[i].x - x).abs() < 0.15);
        assert!((lines[i].y - 614.61).abs() < 0.15, "{}", lines[i].y);
    }
    assert_eq!(lines[3].page, 3);
    assert!((lines[3].y - 99.21).abs() < 0.01);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
    assert!(q.foreground_image_overlaps.is_empty());
    assert_eq!(source, d.sections);
}
