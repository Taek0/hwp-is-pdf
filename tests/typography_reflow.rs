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
fn shaped_text_preserves_logical_content_and_table_pagination() {
    for name in ["rtl", "rtl-table"] {
        let d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/typography-{name}.hwpx")),
        )
        .unwrap();
        let mut e = Engine::new(&d, fonts());
        e.render_reflow().unwrap();
        if name == "rtl-table" {
            assert!(e.pages.len() > 1);
        }
        let all = e
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|o| match o {
                Op::Text { text, .. } | Op::ActualTextBegin { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        let compact = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        assert_eq!(compact(&all), compact(&text(&d.sections[0])));
        assert!(e.pages.iter().flat_map(|p|&p.ops).any(|o|matches!(o,Op::Shaped{text,glyphs,..} if text.contains("لا") && glyphs.len()<text.chars().count())));
        for op in e.pages.iter().flat_map(|p| &p.ops) {
            if let Op::Shaped { glyphs, .. } = op {
                assert!(glyphs.iter().all(|g| g.id != 0));
            }
        }
        assert!(
            e.fonts
                .list
                .iter()
                .flat_map(|f| f.used.values())
                .any(|s| s == "لا"),
            "the lam-alef ligature needs a multi-character ToUnicode mapping"
        );
        for op in e.pages.iter().flat_map(|p| &p.ops) {
            if let Op::Shaped {
                text, font, glyphs, ..
            } = op
                && text.is_ascii()
            {
                for g in glyphs {
                    assert!(
                        e.fonts.list[*font]
                            .used
                            .get(&g.id)
                            .is_some_and(|s| s != "\u{fffd}"),
                        "ASCII shaped glyph is unmapped"
                    );
                }
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
    }
}
#[test]
fn tabs_align_to_destination_stops_instead_of_cached_widths() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/typography-tabs.hwpx"),
    )
    .unwrap();
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    for kind in ["LEFT", "RIGHT", "CENTER", "DECIMAL"] {
        let row = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == format!("tab-{kind}"))
            .unwrap();
        let ops: Vec<_> = e.pages[row.page - 1]
            .ops
            .iter()
            .filter_map(|o| {
                if let Op::Text {
                    x,
                    y,
                    text,
                    font,
                    size,
                    scale,
                    ..
                } = o
                {
                    (*y >= row.y
                        && *y <= row.y + row.h
                        && text.chars().all(|c| c.is_ascii_digit() || c == '.'))
                    .then_some((*x, text, *font, *size, *scale))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(ops.len(), 5);
        let last = ops.last().unwrap();
        let end = last.0 + e.fonts.width(last.2, last.1, last.3) * last.4;
        let actual = match kind {
            "LEFT" => ops[0].0,
            "RIGHT" => end,
            "CENTER" => (ops[0].0 + end) / 2.,
            _ => ops.iter().find(|p| p.1 == ".").unwrap().0,
        };
        assert!((actual - 250.).abs() < 0.02, "{kind}: {actual}");
    }
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.outside_page.is_empty());
}
