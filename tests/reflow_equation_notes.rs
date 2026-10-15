use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};

#[test]
fn public_hwp_endnotes_keep_source_characters_after_forced_reflow() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/tables-notes/endnote-01.hwp"),
    )
    .unwrap();
    let mut e = Engine::new(
        &d,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render_reflow().unwrap();
    let all = e
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
    let counts = |s: &str| {
        let mut result = std::collections::BTreeMap::new();
        for c in s.chars().filter(|c| !c.is_whitespace()) {
            *result.entry(c).or_insert(0usize) += 1;
        }
        result
    };
    let drawn = counts(&all);
    let original = d
        .sections
        .iter()
        .map(hwp_engine::model::text)
        .collect::<String>();
    for (c, n) in counts(&original) {
        assert!(drawn.get(&c).copied().unwrap_or(0) >= n, "missing {c}");
    }
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
#[test]
fn equations_and_all_endnotes_survive_column_and_page_continuations() {
    for name in ["single", "columns", "unequal"] {
        let d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/equation-notes-{name}.hwpx")),
        )
        .unwrap();
        let mut e = Engine::new(
            &d,
            std::path::Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        e.render_reflow().unwrap();
        assert!(e.pages.len() > 2);
        assert_eq!(e.report.equations, 12);
        let all = e
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
        for i in 0..9 {
            assert_eq!(all.matches(&format!("BODY-{i}:")).count(), 1, "{name}");
        }
        for i in 1..=3 {
            for j in 0..18 {
                assert_eq!(
                    all.matches(&format!("NOTE-{i}-{j:02}:")).count(),
                    1,
                    "{name}"
                );
            }
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{name} {:?}", q.outside_page);
        assert!(q.text_overlaps.is_empty(), "{name} {:?}", q.text_overlaps);
    }
}
