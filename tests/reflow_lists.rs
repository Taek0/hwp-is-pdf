use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
fn fonts() -> &'static std::path::Path {
    std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn open(path: &str) -> Document {
    Document::open(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}
fn check(e: &Engine) {
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
fn preserves_text(d: &Document, e: &Engine) {
    let source = d
        .sections
        .iter()
        .map(hwp_engine::model::text)
        .collect::<String>();
    let output = e
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|o| match o {
            Op::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    let counts = |s: &str| {
        let mut counts = std::collections::BTreeMap::new();
        for c in s.chars().filter(|c| !c.is_whitespace()) {
            *counts.entry(c).or_insert(0usize) += 1;
        }
        counts
    };
    let actual = counts(&output);
    for (c, n) in counts(&source) {
        assert!(
            actual.get(&c).copied().unwrap_or(0) >= n,
            "Missing {c}: source {n}, output {:?}",
            actual.get(&c)
        );
    }
}
#[test]
fn hierarchical_lists_keep_labels_and_hanging_indents_across_pages() {
    let d = open("samples/reflow/lists-no-cache.hwpx");
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert!(e.pages.len() > 2);
    let expected = [
        "98.", "98.1)", "98.2)", "99.", "99.1)", "100.", "●", "●", "101.",
    ];
    let labels: Vec<_> = e
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|o| match o {
            Op::Text { text, .. } if expected.contains(&text.as_str()) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, expected);
    for i in 0..9 {
        let id = format!("list-{i}");
        let lines: Vec<_> = e
            .report
            .bounds
            .iter()
            .filter(|b| b.kind == "line" && b.id == id)
            .collect();
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|b| (b.x - lines[0].x).abs() < 0.01));
    }
    let mut forced = Engine::new(&d, fonts());
    forced.render_reflow().unwrap();
    assert_eq!(
        serde_json::to_value(&e.pages).unwrap(),
        serde_json::to_value(&forced.pages).unwrap()
    );
    check(&e);
    preserves_text(&d, &e);
}
#[test]
fn public_hwp_notes_reflow_with_numbered_paragraphs() {
    let d = open("samples/tables-notes/footnote-01.hwp");
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    assert!(e.pages.len() > 1);
    assert!(e.report.tables > 0);
    check(&e);
    preserves_text(&d, &e);
}
#[test]
fn hwpx_bullet_parahead_and_large_label_metrics_are_used() {
    let mut d = open("samples/reflow/lists-no-cache.hwpx");
    let bullet = d
        .header
        .children
        .iter_mut()
        .find(|n| n.name == "bullet" && n.a("id") == "21")
        .unwrap();
    let mut head = hwp_engine::model::Node {
        name: "paraHead".into(),
        ..Default::default()
    };
    for key in ["charPrIDRef", "textOffsetType", "textOffset"] {
        head.attrs
            .insert(key.into(), bullet.attrs.remove(key).unwrap());
    }
    head.attrs.insert("textOffset".into(), "1800".into());
    bullet.children.push(head);
    d.chars
        .get_mut("2")
        .unwrap()
        .attrs
        .insert("height".into(), "2600".into());
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    check(&e);
    preserves_text(&d, &e);
    for p in &e.pages {
        for op in &p.ops {
            if let Op::Text { text, size, .. } = op
                && text == "●"
            {
                assert_eq!(*size, 26.);
            }
        }
    }
}
#[test]
fn missing_list_definition_is_an_explicit_error() {
    let mut d = open("samples/reflow/lists-no-cache.hwpx");
    d.header.children.retain(|n| n.name != "numbering");
    let mut e = Engine::new(&d, fonts());
    assert!(
        e.render()
            .unwrap_err()
            .to_string()
            .contains("Missing numbering/bullet definition")
    );
}
