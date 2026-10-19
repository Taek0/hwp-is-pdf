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
            .any(|s| s.contains("Rewrapped overflowing plain-text cells"))
    );
    let mut title_rows = std::collections::BTreeMap::<i64, String>::new();
    for op in &engine.pages[0].ops {
        if let hwp_engine::pdf::Op::Text { text, y, size, .. } = op
            && *y < 150.
            && (*size - 14.).abs() < 0.01
        {
            title_rows
                .entry((y * 100.).round() as i64)
                .or_default()
                .push_str(text);
        }
    }
    assert!(
        title_rows
            .values()
            .any(|t| t.contains("내신 도깨비 (기말고사"))
    );
    assert!(title_rows.values().any(|t| t.contains("시험대비)")));
    let audit = hwp_engine::qa::inspect(&engine.pages, &engine.fonts, &engine.report.bounds);
    assert!(
        audit.border_crossings.is_empty(),
        "{:?}",
        audit.border_crossings
    );
    assert!(audit.text_overlaps.is_empty());
    assert!(audit.outside_page.is_empty());
    assert!(audit.foreground_image_overlaps.is_empty());
    // Hancom's print is text-relative despite the HWPX wire enum PAPER.
    // Reference PDF top/left are 48.24/28.32 pt (printer quantization).
    assert!(engine.pages[0].ops.iter().any(|op| matches!(op,
        hwp_engine::pdf::Op::Line{x,y,x2,y2,..}
        if (*x-28.35).abs()<0.02 && (*y-48.20).abs()<0.02 && *x2>*x && (*y2-*y).abs()<0.01)));
    // The saved line already reserves 68.66pt. Do not add the 30.47pt
    // difference from the obsolete declared shape size a second time.
    assert!(
        !engine
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
                && l.x < b.x + b.w - 1.
                && l.y >= b.y
                && l.y < b.y + b.h
        })
        .collect();
    assert_eq!(internal.len(), 4);
    // The reference has hanging continuation lines; requiring every line at
    // the left inset would reject the corrected HWPX paragraph margins.
    assert!(internal[1].x > internal[0].x + 20.);
    assert!(internal[3].x > internal[2].x + 20.);
    assert!(internal.iter().all(|l| l.y + l.h <= b.y + b.h + 0.01));
    // Following answer line must start below the box, not through it.
    let answer = engine.pages[5]
        .ops
        .iter()
        .filter_map(|op| match op {
            hwp_engine::pdf::Op::Text { x, y, text, .. }
                if text.contains('①') && *x >= b.x - 1. && *y > b.y =>
            {
                Some(*y)
            }
            _ => None,
        })
        .min_by(f64::total_cmp)
        .unwrap();
    assert!(answer >= b.y + b.h);
}
