use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
fn fonts() -> &'static std::path::Path {
    std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
#[test]
fn recovered_furniture_respects_parity_hiding_and_body_space() {
    for name in ["parity", "hidden-first"] {
        let d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/furniture-{name}.hwpx")),
        )
        .unwrap();
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        assert!(e.pages.len() > 2);
        for (index, page) in e.pages.iter().enumerate() {
            let all = page
                .ops
                .iter()
                .filter_map(|o| {
                    if let Op::Text { text, .. } = o {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>();
            if index == 0 && name == "hidden-first" {
                assert!(!all.contains("HEADER:") && !all.contains("FOOTER-PAGE:"));
            } else {
                assert_eq!(all.contains("ODD-HEADER:"), index % 2 == 0);
                assert_eq!(all.contains("EVEN-HEADER:"), index % 2 == 1);
                assert!(all.contains(&format!("FOOTER-PAGE: {}", index + 1)));
            }
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty());
        assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    }
}
#[test]
fn one_missing_cell_cache_triggers_complete_content_recovery() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/furniture-partial-nested.hwpx"),
    )
    .unwrap();
    assert!(
        d.sections[0]
            .children_named("p")
            .all(|p| p.child("linesegarray").is_some())
    );
    let mut auto = Engine::new(&d, fonts());
    auto.render().unwrap();
    let mut force = Engine::new(&d, fonts());
    force.render_reflow().unwrap();
    assert_eq!(
        serde_json::to_value(&auto.pages).unwrap(),
        serde_json::to_value(&force.pages).unwrap()
    );
    assert!(
        auto.report
            .warnings
            .iter()
            .any(|w| w.contains("Partial nested"))
    );
    let all = auto
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|o| {
            if let Op::Text { text, .. } = o {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<String>();
    assert_eq!(all.matches("INNER-BOX:").count(), 1);
    for i in 0..42 {
        assert_eq!(all.matches(&format!("ROW-{i:02}:")).count(), 1);
    }
    let q = qa::inspect(&auto.pages, &auto.fonts, &auto.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
