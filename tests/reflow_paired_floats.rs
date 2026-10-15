use hwp_engine::{
    layout::Engine,
    model::{Document, Node, text},
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
fn open(name: &str) -> Document {
    Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("samples/reflow/{name}.hwpx")),
    )
    .unwrap()
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
#[test]
fn opposing_objects_restore_each_side_independently_across_paragraphs() {
    for name in ["paired-float-single", "paired-float-columns"] {
        for height in [60000, 44000] {
            let mut d = open(name);
            fn resize(n: &mut Node, h: usize) {
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
            let left = e.report.bounds.iter().find(|b| b.id == "object-0").unwrap();
            let right = e
                .report
                .bounds
                .iter()
                .find(|b| b.id == "paired-right")
                .unwrap();
            assert_eq!(left.page, right.page);
            let lines: Vec<_> = e
                .report
                .bounds
                .iter()
                .filter(|b| {
                    b.kind == "reflow-line" && (b.id == "float-0" || b.id.starts_with("follow-"))
                })
                .collect();
            assert!(lines[0].x >= left.x + left.w && lines[0].x + lines[0].w <= right.x);
            assert!(
                lines.iter().any(|l| l.page == right.page
                    && l.y >= left.y + left.h + 5.
                    && l.y < right.y + right.h
                    && l.x < left.x + left.w
                    && l.x + l.w <= right.x),
                "no one-side restoration"
            );
            assert!(
                lines.iter().any(|l| l.w > lines[0].w + 100.),
                "no full-width restoration"
            );
            let note = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "line" && b.id == "follow-note-body")
                .unwrap();
            assert_eq!(note.page, left.page);
            let all = e
                .pages
                .iter()
                .flat_map(|p| &p.ops)
                .filter_map(|o| match o {
                    Op::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>();
            let counts = |s: &str| {
                let mut m = std::collections::BTreeMap::new();
                for c in s.chars().filter(|c| !c.is_whitespace()) {
                    *m.entry(c).or_insert(0usize) += 1;
                }
                m
            };
            let actual = counts(&all);
            for (c, n) in counts(&text(&d.sections[0])) {
                assert!(actual.get(&c).copied().unwrap_or(0) >= n, "lost {c}");
            }
            assert_eq!(all.matches("오른쪽 개체").count(), 1);
            assert_eq!(e.report.pictures, 2);
            check(&e);
        }
    }
}
#[test]
fn insufficient_middle_space_falls_back_without_object_overlap() {
    let mut d = open("paired-float-single");
    fn widen(n: &mut Node) {
        if n.a("id") == "paired-right" {
            n.children
                .iter_mut()
                .find(|n| n.name == "sz")
                .unwrap()
                .attrs
                .insert("width".into(), "41000".into());
        }
        for c in &mut n.children {
            widen(c)
        }
    }
    widen(&mut d.sections[0]);
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let left = e.report.bounds.iter().find(|b| b.id == "object-0").unwrap();
    let right = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "paired-right")
        .unwrap();
    assert!(right.page > left.page || right.y >= left.y + left.h);
    check(&e);
}
#[test]
fn following_paragraph_with_large_margin_clears_both_objects() {
    let mut d = open("paired-float-single");
    d.paras
        .get_mut("flow-1")
        .unwrap()
        .children
        .iter_mut()
        .find(|n| n.name == "margin")
        .unwrap()
        .children
        .iter_mut()
        .find(|n| n.name == "left")
        .unwrap()
        .attrs
        .insert("value".into(), "40000".into());
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let right = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "paired-right")
        .unwrap();
    let next = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "line" && b.id == "follow-1")
        .unwrap();
    assert!(next.page > right.page || next.y >= right.y + right.h);
    check(&e);
}
