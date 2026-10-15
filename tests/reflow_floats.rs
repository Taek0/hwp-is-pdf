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
fn wide_square_picture_clears_text_below_instead_of_failing() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/float-single.hwpx"),
    )
    .unwrap();
    fn widen(n: &mut hwp_engine::model::Node) {
        if n.a("id") == "object-0" {
            n.children
                .iter_mut()
                .find(|n| n.name == "sz")
                .unwrap()
                .attrs
                .insert("width".into(), "49000".into());
        }
        for c in &mut n.children {
            widen(c);
        }
    }
    widen(&mut d.sections[0]);
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.foreground_image_overlaps.is_empty());
    assert!(q.outside_page.is_empty());
    assert_eq!(e.report.pictures, 2);
}
#[test]
fn leading_floats_wrap_restore_width_and_stay_with_their_text() {
    for (name, height) in [
        ("float-single", 60000),
        ("float-columns", 60000),
        ("float-single", 40000),
        ("float-columns", 40000),
    ] {
        let mut d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/{name}.hwpx")),
        )
        .unwrap();
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
        let all = e
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        for i in 0..4 {
            assert_eq!(all.matches(&format!("START-{i}")).count(), 1);
            assert_eq!(all.matches(&format!("END-{i}")).count(), 1);
            let object = e
                .report
                .bounds
                .iter()
                .find(|b| b.id == format!("object-{i}"))
                .unwrap();
            let lines: Vec<_> = e
                .report
                .bounds
                .iter()
                .filter(|b| b.kind == "reflow-line" && b.id == format!("float-{i}"))
                .collect();
            assert_eq!(object.page, lines[0].page);
            for line in &lines {
                if line.page == object.page && line.y < object.y + object.h {
                    assert!(
                        line.x >= object.x + object.w || line.x + line.w <= object.x,
                        "line crosses float"
                    );
                }
            }
            if i < 3 {
                assert!(
                    lines.iter().any(|l| l.w > lines[0].w + 30.),
                    "width not restored {name} {i}"
                );
            }
        }
        let last = e.report.bounds.iter().find(|b| b.id == "object-3").unwrap();
        let after = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == "after")
            .unwrap();
        if last.page == after.page && after.y < last.y + last.h {
            assert!(after.x + after.w <= last.x || after.x >= last.x + last.w);
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
        assert_eq!(all.matches("FLOAT-NOTE").count(), 1);
        let note = e
            .report
            .bounds
            .iter()
            .find(|b| b.id == "float-note-body" && b.kind == "line")
            .unwrap();
        let object = e.report.bounds.iter().find(|b| b.id == "object-0").unwrap();
        assert_eq!(note.page, object.page);
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
