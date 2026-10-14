use hwp_engine::{
    layout::Engine,
    model::{Document, Node},
    pdf::Op,
    qa,
};
fn fonts() -> &'static std::path::Path {
    std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn input() -> Document {
    Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/overflow/nested-mixed.hwpx"),
    )
    .unwrap()
}
fn clear_cache(n: &mut Node) {
    n.children.retain(|c| c.name != "linesegarray");
    for c in &mut n.children {
        clear_cache(c)
    }
}
#[test]
fn cacheless_mixed_reflow_preserves_objects_rows_and_notes() {
    let mut d = input();
    clear_cache(&mut d.sections[0]);
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let pages: Vec<_> = e
        .pages
        .iter()
        .map(|p| {
            p.ops
                .iter()
                .filter_map(|o| {
                    if let Op::Text { text, .. } = o {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>()
        })
        .collect();
    let all = pages.concat();
    for i in 1..=60 {
        assert_eq!(all.matches(&format!("행 R {i:03}")).count(), 1);
    }
    for i in 1..=12 {
        assert_eq!(all.matches(&format!("각주 N{i:03}:")).count(), 1);
    }
    assert_eq!(all.matches("회전 개체").count(), 1);
    assert_eq!(e.report.pictures, 1);
    assert!(
        pages
            .iter()
            .find(|p| p.contains("행 R 031"))
            .unwrap()
            .contains("각주 N001:")
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
    assert!(
        q.foreground_image_overlaps.is_empty(),
        "{:?}",
        q.foreground_image_overlaps
    );
}
#[test]
fn force_reflow_matches_missing_cache_for_mixed_content() {
    let a = input();
    let mut b = input();
    clear_cache(&mut b.sections[0]);
    let mut x = Engine::new(&a, fonts());
    let mut y = Engine::new(&b, fonts());
    x.render_reflow().unwrap();
    y.render().unwrap();
    assert_eq!(
        serde_json::to_value(&x.pages).unwrap(),
        serde_json::to_value(&y.pages).unwrap()
    );
}
#[test]
fn document_flow_rewraps_cells_body_and_multiple_notes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (height, width) in [(84188, 59528), (72000, 59528), (84188, 42000)] {
        let mut d = Document::open(&root.join("samples/reflow/mixed-no-cache.hwpx")).unwrap();
        fn resize(n: &mut Node, h: usize, w: usize) {
            if n.name == "pagePr" {
                n.attrs.insert("height".into(), h.to_string());
                n.attrs.insert("width".into(), w.to_string());
            }
            for c in &mut n.children {
                resize(c, h, w)
            }
        }
        resize(&mut d.sections[0], height, width);
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let pages: Vec<_> = e
            .pages
            .iter()
            .map(|p| {
                p.ops
                    .iter()
                    .filter_map(|o| {
                        if let Op::Text { text, .. } = o {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<String>()
            })
            .collect();
        let all = pages.concat();
        let counts = |s: &str| {
            let mut m = std::collections::BTreeMap::new();
            for c in s.chars().filter(|c| !c.is_whitespace()) {
                *m.entry(c).or_insert(0usize) += 1;
            }
            m
        };
        let source = counts(&hwp_engine::model::text(&d.sections[0]));
        let drawn = counts(&all);
        for (ch, n) in source {
            assert!(
                drawn.get(&ch).copied().unwrap_or(0) >= n,
                "lost character {ch}"
            );
        }
        for marker in [
            "INTRO-START",
            "INLINE-END",
            "CELL-START",
            "CELL-END",
            "BODY-END",
        ] {
            assert_eq!(all.matches(marker).count(), 1, "{height}: {marker}");
        }
        for i in 1..=4 {
            assert_eq!(all.matches(&format!("본문 각주 B{i:03}:")).count(), 1);
        }
        for i in 1..=12 {
            assert_eq!(all.matches(&format!("각주 N{i:03}:")).count(), 1);
        }
        assert_eq!(e.report.pictures, 2);
        assert!(
            pages
                .iter()
                .find(|p| p.contains("행 R 031"))
                .unwrap()
                .contains("각주 N001:")
        );
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
        assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
        assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
        assert!(
            q.foreground_image_overlaps.is_empty(),
            "{:?}",
            q.foreground_image_overlaps
        );
    }
}
