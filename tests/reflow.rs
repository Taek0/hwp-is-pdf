use hwp_engine::{
    layout::Engine,
    model::{Document, text},
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
#[test]
fn public_hwp_paragraphs_use_decoded_flow_properties() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/web/paragraphs.hwp"),
    )
    .unwrap();
    assert!(d.paras.values().all(|p| p.child("margin").is_some()
        && p.child("lineSpacing").is_some()
        && p.child("breakSetting").is_some()));
    let mut e = Engine::new(&d, fonts());
    e.render_reflow_text().unwrap();
    let source = d.sections.iter().map(text).collect::<String>();
    let output = e
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
    assert_eq!(compact(&source), compact(&output));
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
fn open(name: &str) -> Document {
    Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("samples/reflow/{name}.hwpx")),
    )
    .unwrap()
}
fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}
#[test]
fn cacheless_reflow_preserves_text_and_avoids_collisions() {
    for name in ["plain-no-cache", "two-column-no-cache"] {
        let d = open(name);
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        assert!(e.pages.len() > 1);
        let source = d.sections.iter().map(text).collect::<String>();
        let output = e
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
        assert_eq!(compact(&source), compact(&output));
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{name}: {:?}", q.outside_page);
        assert!(q.text_overlaps.is_empty(), "{name}: {:?}", q.text_overlaps);
        assert!(
            q.border_crossings.is_empty(),
            "{name}: {:?}",
            q.border_crossings
        );
        for id in ["0", "6", "12"] {
            let head = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "reflow-line" && b.id == id)
                .unwrap();
            let next = (id.parse::<usize>().unwrap() + 1).to_string();
            let body = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "reflow-line" && b.id == next)
                .unwrap();
            assert_eq!(head.page, body.page);
        }
        let kept: std::collections::BTreeSet<_> = e
            .report
            .bounds
            .iter()
            .filter(|b| b.kind == "reflow-line" && b.id == "8")
            .map(|b| (b.page, b.x.round() as i64))
            .collect();
        assert_eq!(kept.len(), 1);
    }
}
#[test]
fn explicit_reflow_ignores_stale_cached_lines() {
    let a = open("plain-no-cache");
    let b = open("plain-cached");
    let mut x = Engine::new(&a, fonts());
    let mut y = Engine::new(&b, fonts());
    x.render().unwrap();
    y.render_reflow_text().unwrap();
    assert_eq!(
        serde_json::to_value(&x.pages).unwrap(),
        serde_json::to_value(&y.pages).unwrap()
    );
}
#[test]
fn complex_reflow_is_rejected_instead_of_dropping_objects() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/overflow/nested-mixed.hwpx"),
    )
    .unwrap();
    let mut e = Engine::new(&d, fonts());
    let error = e.render_reflow_text().unwrap_err().to_string();
    assert!(error.contains("objects/notes/control"), "{error}");
}
#[test]
fn changing_font_size_recalculates_line_and_page_counts() {
    let normal = open("plain-cached");
    let mut larger = open("plain-cached");
    larger
        .chars
        .get_mut("0")
        .unwrap()
        .attrs
        .insert("height".into(), "2000".into());
    let mut a = Engine::new(&normal, fonts());
    let mut b = Engine::new(&larger, fonts());
    a.render_reflow_text().unwrap();
    b.render_reflow_text().unwrap();
    assert!(b.pages.len() > a.pages.len());
    assert!(
        b.report
            .bounds
            .iter()
            .filter(|b| b.kind == "reflow-line")
            .count()
            > a.report
                .bounds
                .iter()
                .filter(|b| b.kind == "reflow-line")
                .count()
    );
    let q = qa::inspect(&b.pages, &b.fonts, &b.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
#[test]
fn headings_and_widows_stay_together_at_different_page_heights() {
    for height in [30000, 35000, 40000, 45000, 55000] {
        let mut d = open("plain-no-cache");
        fn resize(n: &mut hwp_engine::model::Node, h: usize) {
            if n.name == "pagePr" {
                n.attrs.insert("height".into(), h.to_string());
            }
            for c in &mut n.children {
                resize(c, h)
            }
        }
        resize(&mut d.sections[0], height);
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        for id in ["0", "6", "12"] {
            let h = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "reflow-line" && b.id == id)
                .unwrap();
            let next = (id.parse::<usize>().unwrap() + 1).to_string();
            let b = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "reflow-line" && b.id == next)
                .unwrap();
            assert_eq!(h.page, b.page, "height {height}, heading {id}");
        }
    }
}
