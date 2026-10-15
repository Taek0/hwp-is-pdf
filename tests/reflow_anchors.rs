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
fn negative_absolute_and_oversized_anchors_preserve_content() {
    for name in [
        "anchors-negative",
        "anchors-absolute",
        "anchors-absolute-columns",
        "anchors-large",
        "anchors-large-group",
        "anchors-chain",
    ] {
        let d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/{name}.hwpx")),
        )
        .unwrap();
        let mut e = Engine::new(&d, fonts());
        e.render_contours().unwrap();
        let all = e
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        let compact = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        assert_eq!(compact(&all), compact(&text(&d.sections[0])));
        let find = |id: &str| e.report.bounds.iter().find(|b| b.id == id).unwrap();
        match name {
            "anchors-negative" => {
                let p = find("negative-picture");
                let before = e
                    .report
                    .bounds
                    .iter()
                    .find(|b| b.kind == "line" && b.id == "intro")
                    .unwrap();
                assert!(p.page > before.page || p.y >= before.y + before.h - 0.01);
            }
            "anchors-absolute" => {
                let p = find("paper-picture");
                assert!((p.x - 80.).abs() < 0.01 && (p.y - 100.).abs() < 0.01);
                let q = find("page-picture");
                assert!(
                    (q.x - (595.28 - 50. - 80. - 10.)).abs() < 0.01 && (q.y - 230.).abs() < 0.01
                );
            }
            "anchors-absolute-columns" => {
                let p = find("paper-picture");
                assert!((p.x - 650.).abs() < 0.01 && (p.y - 120.).abs() < 0.01);
            }
            "anchors-large" => {
                let p = find("large-picture");
                assert!(p.h < 700. && p.h > 300.);
                assert!(e.report.warnings.iter().any(|w| w.contains("fitted")));
            }
            "anchors-large-group" => {
                let p = find("large-group");
                assert!(p.h < 700. && p.h > 300.);
                let child = find("large-child");
                assert!(
                    (child.h / p.h - 100000. / 120000.).abs() < 0.001,
                    "group member was scaled twice"
                );
                assert!(all.contains("BIG-GROUP"));
            }
            _ => {
                let a = find("chain-first");
                let b = find("chain-second");
                assert!(b.page > a.page);
                assert_eq!(e.report.pictures, 2);
            }
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{name} {:?}", q.outside_page);
        assert!(q.text_overlaps.is_empty(), "{name} {:?}", q.text_overlaps);
        assert!(
            q.border_crossings.is_empty(),
            "{name} {:?}",
            q.border_crossings
        );
        assert!(
            q.foreground_image_overlaps.is_empty(),
            "{name} {:?}",
            q.foreground_image_overlaps
        );
    }
}
#[test]
fn square_reflow_also_resolves_absolute_anchors() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/anchors-absolute-columns.hwpx"),
    )
    .unwrap();
    let mut a = Engine::new(&d, fonts());
    a.render_reflow().unwrap();
    let mut b = Engine::new(&d, fonts());
    b.render_contours().unwrap();
    assert_eq!(
        serde_json::to_value(&a.pages).unwrap(),
        serde_json::to_value(&b.pages).unwrap()
    );
}
#[test]
fn absolute_top_bottom_preserves_position_and_clears_the_entire_band() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/anchors-absolute.hwpx"),
    )
    .unwrap();
    fn edit(n: &mut hwp_engine::model::Node) {
        if n.a("id") == "paper-picture" {
            n.attrs.insert("textWrap".into(), "TOP_AND_BOTTOM".into());
        }
        for c in &mut n.children {
            edit(c);
        }
    }
    edit(&mut d.sections[0]);
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    let p = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "paper-picture")
        .unwrap();
    assert!((p.y - 100.).abs() < 0.01);
    assert!(
        !e.pages[p.page - 1]
            .ops
            .iter()
            .any(|o| matches!(o,Op::Text{y,..} if *y>p.y+2. && *y<p.y+p.h-2.))
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.foreground_image_overlaps.is_empty());
}
#[test]
fn oversized_objects_in_one_paragraph_use_safe_sequential_blocks() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/anchors-chain.hwpx"),
    )
    .unwrap();
    let run = d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "body-2")
        .unwrap()
        .children
        .iter_mut()
        .find(|r| r.name == "run")
        .unwrap();
    let i = run.children.iter().position(|n| n.name == "pic").unwrap();
    let second = run.children.remove(i);
    d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "anchor")
        .unwrap()
        .children
        .iter_mut()
        .find(|r| r.name == "run")
        .unwrap()
        .children
        .push(second);
    let mut e = Engine::new(&d, fonts());
    e.render_contours().unwrap();
    assert_eq!(e.report.pictures, 2);
    assert!(
        e.report
            .warnings
            .iter()
            .any(|w| w.contains("sequential blocks"))
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
    assert!(q.foreground_image_overlaps.is_empty());
}
