use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
fn fixture() -> Document {
    Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/text-style/scripts-offsets.hwpx"),
    )
    .unwrap()
}
fn fonts() -> &'static std::path::Path {
    std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
#[test]
fn scripts_offsets_and_decoration_follow_the_requested_baseline() {
    let doc = fixture();
    let mut e = Engine::new(&doc, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 1);
    let texts: Vec<_> = e.pages[0]
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Text { text, y, size, .. } = o {
                Some((text.as_str(), *y, *size))
            } else {
                None
            }
        })
        .collect();
    assert!(
        texts
            .iter()
            .any(|(t, y, s)| *t == "2" && (*y - 196.08).abs() < 0.01 && (*s - 11.52).abs() < 0.01)
    );
    assert!(
        texts
            .iter()
            .any(|(t, y, s)| *t == "2" && (*y - 271.16).abs() < 0.01 && (*s - 11.52).abs() < 0.01)
    );
    for (t, offset) in [("가", -3.6), ("A", 1.8), ("漢", 5.4), ("あ", 7.2)] {
        assert!(
            texts
                .iter()
                .any(|(ch, y, _)| *ch == t && (*y - (529. + offset)).abs() < 0.01),
            "{t}"
        );
    }
    // Decoration must move with the affected run, not stay on the normal baseline.
    assert!(e.pages[0].ops.iter().any(|o|matches!(o,Op::Path{points,color,..} if color[2]>0.7 && points[0].1>329. && points[0].1<334.)));
    assert!(e.pages[0].ops.iter().any(
        |o| matches!(o,Op::Path{points,color,..} if color[0]>0.7 && (points[0].1-399.).abs()<0.01)
    ));
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
#[test]
fn korean_slant_is_scoped_and_latin_native_italic_not_slanted_twice() {
    let doc = fixture();
    let mut e = Engine::new(&doc, fonts());
    e.render().unwrap();
    let mut slanted = false;
    let mut seen_korean = false;
    for op in &e.pages[0].ops {
        match op {
            Op::TransformBegin { matrix } => {
                assert!(!slanted);
                slanted = true;
                assert!(matrix[2] < 0.);
            }
            Op::TransformEnd => {
                assert!(slanted);
                slanted = false;
            }
            Op::Text { text, y, .. } if *y > 130. && *y < 150. => {
                if text == "한" {
                    assert!(slanted);
                    seen_korean = true;
                }
                if text == "A" {
                    assert!(!slanted);
                }
            }
            _ => {}
        }
    }
    assert!(seen_korean);
    assert!(!slanted);
}
