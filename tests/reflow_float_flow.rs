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
fn resize(n: &mut Node, height: usize) {
    if n.name == "pagePr" {
        n.attrs.insert("height".into(), height.to_string());
    }
    for c in &mut n.children {
        resize(c, height)
    }
}
#[test]
fn following_paragraphs_wrap_with_spacing_notes_and_page_transitions() {
    for name in ["float-flow-single", "float-flow-columns"] {
        for height in [60000, 42000] {
            let mut d = Document::open(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("samples/reflow/{name}.hwpx")),
            )
            .unwrap();
            resize(&mut d.sections[0], height);
            let mut e = Engine::new(&d, fonts());
            e.render().unwrap();
            let object = e.report.bounds.iter().find(|b| b.id == "object-0").unwrap();
            let mut widths = Vec::new();
            for j in 1..=5 {
                let lines: Vec<_> = e
                    .report
                    .bounds
                    .iter()
                    .filter(|b| b.kind == "reflow-line" && b.id == format!("follow-{j}"))
                    .collect();
                assert!(!lines.is_empty());
                if j <= 2 {
                    assert_eq!(lines[0].page, object.page);
                    assert!(lines[0].y < object.y + object.h);
                }
                for line in lines {
                    widths.push(line.w);
                    if line.page == object.page && line.y < object.y + object.h {
                        assert!(line.x >= object.x + object.w || line.x + line.w <= object.x);
                    }
                }
            }
            assert!(widths.iter().any(|w| *w > widths[0] + 50.));
            let note = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "line" && b.id == "follow-note-body")
                .unwrap();
            assert_eq!(note.page, object.page);
            let all = e
                .pages
                .iter()
                .flat_map(|p| &p.ops)
                .filter_map(|o| match o {
                    Op::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>();
            assert_eq!(all.matches("FOLLOW-NOTE").count(), 1);
            assert_eq!(all.matches("AFTER-END").count(), 1);
            for j in 1..=5 {
                assert_eq!(all.matches(&format!("문단 F{j}:")).count(), 1);
            }
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
            assert_eq!(e.report.pictures, 2);
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
}
#[test]
fn explicit_page_break_clears_float_instead_of_carrying_old_exclusion() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/float-flow-single.hwpx"),
    )
    .unwrap();
    d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "follow-1")
        .unwrap()
        .attrs
        .insert("pageBreak".into(), "1".into());
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let object = e.report.bounds.iter().find(|b| b.id == "object-0").unwrap();
    let next = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "reflow-line" && b.id == "follow-1")
        .unwrap();
    assert!(next.page > object.page);
    assert!((next.x - 50.).abs() < 0.01);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.foreground_image_overlaps.is_empty());
}
#[test]
fn another_object_clears_the_previous_float_and_continuation_can_move_as_a_group() {
    for force_move in [false, true] {
        let mut d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("samples/reflow/float-flow-single.hwpx"),
        )
        .unwrap();
        resize(&mut d.sections[0], 42000);
        if force_move {
            fn extend(n: &mut Node) {
                if n.name == "#text" && n.text.contains("그림과 도형 주위") {
                    n.text = n.text.repeat(18);
                }
                for c in &mut n.children {
                    extend(c);
                }
            }
            extend(&mut d.sections[0]);
        } else {
            d.sections[0].children.retain(|n| {
                !matches!(n.a("id"), "follow-2" | "follow-3" | "follow-4" | "follow-5")
            });
        }
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let a = e.report.bounds.iter().find(|b| b.id == "object-0").unwrap();
        if force_move {
            let intro = e
                .report
                .bounds
                .iter()
                .rev()
                .find(|b| b.kind == "line" && b.id == "intro")
                .unwrap();
            let follow = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "line" && b.id == "follow-2")
                .unwrap();
            assert!(a.page > intro.page);
            assert_eq!(a.page, follow.page);
        } else {
            let b = e.report.bounds.iter().find(|b| b.id == "object-1").unwrap();
            assert!(b.page > a.page || b.y >= a.y + a.h);
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty());
        assert!(q.text_overlaps.is_empty());
        assert!(q.border_crossings.is_empty());
        assert!(q.foreground_image_overlaps.is_empty());
    }
}
