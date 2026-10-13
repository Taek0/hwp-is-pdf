use hwp_engine::{layout::Engine, model::Document};
use std::path::PathBuf;

#[test]
fn supplied_math_document_has_complete_object_coverage_and_repaired_box() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root
        .parent()
        .unwrap()
        .join("2단_수학시험지_변환검증_7826.hwpx");
    let fonts = if cfg!(windows) {
        PathBuf::from("C:/Windows/Fonts")
    } else {
        PathBuf::from("/mnt/c/Windows/Fonts")
    };
    // This fixture is user-owned and deliberately not bundled in the crate.
    if !input.exists() || !fonts.exists() {
        eprintln!("SKIP: local user fixture/fonts unavailable");
        return;
    }
    let doc = Document::open(&input).unwrap();
    let mut engine = Engine::new(&doc, &fonts);
    engine.render().unwrap();
    assert_eq!(engine.report.pages, 9);
    assert_eq!(engine.report.equations, 249);
    assert_eq!(engine.report.pictures, 22);
    assert!(
        engine
            .report
            .warnings
            .iter()
            .any(|s| s.contains("+30.47pt"))
    );
    let b = engine
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "rect" && b.id == "1653018157")
        .unwrap();
    assert_eq!(b.page, 6);
    assert!((b.h - 68.66).abs() < 0.001);
    // All four lines within this box fit inside its corrected geometry.
    let internal: Vec<_> = engine
        .report
        .bounds
        .iter()
        .filter(|l| {
            l.page == b.page
                && l.kind == "line"
                && l.x > b.x + 1.
                && l.x < b.x + 10.
                && l.y >= b.y
                && l.y < b.y + b.h
        })
        .collect();
    assert!(internal.len() >= 4);
    assert!(internal.iter().all(|l| l.y + l.h <= b.y + b.h + 0.01));
    // Following answer line must start below the box, not through it.
    let answer = engine
        .report
        .bounds
        .iter()
        .filter(|l| l.page == 6 && l.kind == "line" && l.x > b.x + 10. && l.y > b.y + 40.)
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap();
    assert!(answer.y >= b.y + b.h);
}
