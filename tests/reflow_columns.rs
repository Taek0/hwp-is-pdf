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
fn mixed_tables_continue_through_columns_without_losing_notes_or_images() {
    for (name, height) in [
        ("mixed-two-column", 84188),
        ("mixed-three-column", 84188),
        ("mixed-two-column", 65000),
        ("mixed-three-column", 65000),
    ] {
        let mut d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/{name}.hwpx")),
        )
        .unwrap();
        fn resize(n: &mut hwp_engine::model::Node, height: usize) {
            if n.name == "pagePr" {
                n.attrs.insert("height".into(), height.to_string());
            }
            for c in &mut n.children {
                resize(c, height)
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
        for marker in [
            "INTRO-START",
            "INLINE-END",
            "CELL-START",
            "CELL-END",
            "BODY-END",
        ] {
            assert_eq!(all.matches(marker).count(), 1, "{name} {marker}");
        }
        for i in 1..=60 {
            assert_eq!(
                all.matches(&format!("행 R {i:03}")).count(),
                1,
                "{name} row {i}"
            );
        }
        for i in 1..=12 {
            assert_eq!(all.matches(&format!("각주 N{i:03}:")).count(), 1);
        }
        for i in 1..=4 {
            assert_eq!(all.matches(&format!("본문 각주 B{i:03}:")).count(), 1);
        }
        assert_eq!(e.report.pictures, 2);
        let counts = |s: &str| {
            let mut m = std::collections::BTreeMap::new();
            for c in s.chars().filter(|c| !c.is_whitespace()) {
                *m.entry(c).or_insert(0usize) += 1;
            }
            m
        };
        let drawn = counts(&all);
        for (c, n) in counts(&text(&d.sections[0])) {
            assert!(drawn.get(&c).copied().unwrap_or(0) >= n, "lost {c}");
        }
        // The same table must occupy multiple columns on at least one page.
        let tables: Vec<_> = e.report.bounds.iter().filter(|b| b.kind == "tbl").collect();
        assert!(
            tables.iter().any(|a| tables
                .iter()
                .any(|b| a.id == b.id && a.page == b.page && (a.x - b.x).abs() > 100.)),
            "no column transition"
        );
        let reference = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == "1131")
            .unwrap();
        let note = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == "3001")
            .unwrap();
        assert_eq!(
            reference.page, note.page,
            "note moved off its reference page"
        );
        let page = d.sections[0].find("pagePr").unwrap();
        let cols = if name == "mixed-two-column" { 2. } else { 3. };
        let step = (page.n("width") / 100. - 100. - 24. * (cols - 1.)) / cols + 24.;
        assert_eq!(
            ((reference.x - 50.) / step).floor(),
            ((note.x - 50.) / step).floor(),
            "note moved to wrong column"
        );
        if height == 84188 {
            let last = tables.iter().rev().find(|b| b.id == "mixed-outer").unwrap();
            let after = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "line" && b.id == "500")
                .unwrap();
            assert_eq!(last.page, after.page);
            assert!((last.x - after.x).abs() < 0.01 && after.y >= last.y + last.h);
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
fn variable_columns_preserve_source_order_and_geometry() {
    for name in ["unequal", "transition", "right", "balanced"] {
        let d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/columns-{name}.hwpx")),
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
        let actual = e
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
        let compact = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        assert_eq!(compact(&actual), compact(&text(&d.sections[0])), "{name}");
        let rows: Vec<_> = e
            .report
            .bounds
            .iter()
            .filter(|b| b.kind == "reflow-line")
            .collect();
        assert!(rows.iter().any(|b| b.x > 200.), "{name}");
        for row in &rows {
            assert!(
                row.x >= 49.99 && row.x + row.w <= 545.29,
                "{name}: {}",
                row.x + row.w
            );
        }
        if name == "right" {
            assert!(rows[0].x > 200.);
        }
        if name == "balanced" {
            assert_eq!(e.pages.len(), 1);
            let a = rows
                .iter()
                .filter(|b| b.x < 200.)
                .map(|b| b.y + b.h)
                .fold(0f64, f64::max);
            let b = rows
                .iter()
                .filter(|b| b.x > 200.)
                .map(|b| b.y + b.h)
                .fold(0f64, f64::max);
            assert!((a - b).abs() < 30., "unbalanced: {a} {b}");
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{name}");
        assert!(q.text_overlaps.is_empty(), "{name}");
    }
}

#[test]
fn shared_notes_reserve_space_before_later_column_references() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/columns-merged.hwpx"),
    )
    .unwrap();
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    let notes: Vec<_> = e
        .report
        .bounds
        .iter()
        .filter(|b| b.kind == "footnote")
        .collect();
    assert_eq!(notes.len(), 1);
    assert!((notes[0].w - 495.28).abs() < 0.01);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.outside_page.is_empty());
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
    assert_eq!(all.matches("NOTE-1:").count(), 1);
    for i in 0..10 {
        assert_eq!(all.matches(&format!("BEGIN-{i}")).count(), 1);
        assert_eq!(all.matches(&format!("END-{i}")).count(), 1);
    }
}

#[test]
fn mixed_table_fragments_follow_unequal_column_origins() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/mixed-two-column.hwpx"),
    )
    .unwrap();
    fn set_columns(n: &mut hwp_engine::model::Node) {
        if n.name == "colPr" {
            n.children
                .push(hwp_engine::model::xml(br#"<colSz width="20000" gap="2400"/>"#).unwrap());
            n.children
                .push(hwp_engine::model::xml(br#"<colSz width="27128" gap="0"/>"#).unwrap());
        }
        for c in &mut n.children {
            set_columns(c);
        }
    }
    set_columns(&mut d.sections[0]);
    let mut e = Engine::new(&d, fonts());
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
    for i in 1..=60 {
        assert_eq!(all.matches(&format!("행 R {i:03}")).count(), 1);
    }
    assert_eq!(e.report.pictures, 2);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(
        q.foreground_image_overlaps.is_empty(),
        "{:?}",
        q.foreground_image_overlaps
    );
}
