use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};

#[test]
fn table_notes_fit_when_reading_from_wide_to_narrow_columns() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/mixed-two-column.hwpx"),
    )
    .unwrap();
    fn columns(n: &mut hwp_engine::model::Node) {
        if n.name == "colPr" {
            n.attrs.insert("layout".into(), "RIGHT".into());
            n.children
                .push(hwp_engine::model::xml(br#"<colSz width="18000" gap="2400"/>"#).unwrap());
            n.children
                .push(hwp_engine::model::xml(br#"<colSz width="29128" gap="0"/>"#).unwrap());
        }
        for c in &mut n.children {
            columns(c);
        }
    }
    columns(&mut d.sections[0]);
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
    for i in 1..=12 {
        assert_eq!(all.matches(&format!("각주 N{i:03}:")).count(), 1);
    }
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
}
#[test]
fn captions_cells_and_notes_remain_complete_and_separate() {
    for name in [
        "picture-left",
        "textbox-right",
        "table-side",
        "table-note",
        "table-nested",
    ] {
        let d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/captions-{name}.hwpx")),
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
        for marker in ["INTRO:", "CAPTION-START:", "CAPTION-END", "AFTER:"] {
            assert_eq!(all.matches(marker).count(), 1, "{name} {marker}");
        }
        if name.starts_with("table-") {
            for i in 0..42 {
                assert_eq!(all.matches(&format!("ROW-{i:02}:")).count(), 1, "{name}");
            }
            assert!(e.pages.len() > 1);
        }
        if name == "table-note" {
            assert_eq!(all.matches("CAPTION-NOTE:").count(), 1);
            let note = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "footnote")
                .unwrap();
            let caption = e
                .report
                .bounds
                .iter()
                .rev()
                .find(|b| b.kind == "caption" && b.id == "object")
                .unwrap();
            assert_eq!(note.page, caption.page);
        }
        let caption = e
            .report
            .bounds
            .iter()
            .filter(|b| b.kind == "caption" && b.id == "object")
            .max_by(|a, b| {
                a.page
                    .cmp(&b.page)
                    .then((a.y + a.h).total_cmp(&(b.y + b.h)))
            })
            .unwrap();
        let after = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == "after")
            .unwrap();
        assert!(
            after.page > caption.page
                || (after.page == caption.page && after.y >= caption.y + caption.h - 0.01),
            "{name}: following paragraph overlaps caption"
        );
        if name == "table-nested" {
            for marker in ["INNER-BOX:", "INNER-CAP:"] {
                assert_eq!(all.matches(marker).count(), 1);
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
