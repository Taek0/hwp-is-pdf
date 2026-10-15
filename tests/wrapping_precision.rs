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
fn alpha_curve_group_and_multiple_fragments_preserve_text() {
    for name in ["precision-single", "precision-columns"] {
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
        let lines: Vec<_> = e
            .report
            .bounds
            .iter()
            .filter(|b| b.kind == "reflow-line")
            .collect();
        assert!(
            lines.iter().any(|a| lines.iter().any(|b| a.page == b.page
                && a.id == b.id
                && (a.y - b.y).abs() < 0.01
                && (a.x - b.x).abs() > 10.)),
            "no same-row fragments"
        );
        let ring = e
            .report
            .bounds
            .iter()
            .find(|b| b.id == "alpha-ring")
            .unwrap();
        assert_eq!(ring.kind, "pic-alpha");
        assert!(
            lines.iter().any(|b| b.page == ring.page
                && b.y > ring.y + ring.h * 0.35
                && b.y < ring.y + ring.h * 0.65
                && b.x > ring.x + ring.w * 0.2
                && b.x < ring.x + ring.w * 0.7),
            "alpha hole unused"
        );
        assert!(e.report.bounds.iter().any(|b| b.id == "cubic-contour"));
        assert!(e.report.bounds.iter().any(|b| b.id == "group-child-1"));
        assert_eq!(e.report.pictures, 1);
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
#[test]
fn rotated_cropped_transparency_uses_the_displayed_mask() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/precision-single.hwpx"),
    )
    .unwrap();
    fn edit(n: &mut hwp_engine::model::Node) {
        if n.a("id") == "alpha-ring" {
            for (name, attrs) in [
                (
                    "imgClip",
                    vec![
                        ("left", "2400"),
                        ("right", "7200"),
                        ("top", "0"),
                        ("bottom", "9600"),
                    ],
                ),
                ("curSz", vec![("width", "16000"), ("height", "16000")]),
                ("rotationInfo", vec![("angle", "30")]),
            ] {
                n.children.push(hwp_engine::model::Node {
                    name: name.into(),
                    attrs: attrs
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                    ..Default::default()
                });
            }
        }
        for c in &mut n.children {
            edit(c);
        }
    }
    edit(&mut d.sections[0]);
    let mut e = Engine::new(&d, fonts());
    e.render_contours().unwrap();
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(
        q.foreground_image_overlaps.is_empty(),
        "{:?}",
        q.foreground_image_overlaps
    );
    assert_eq!(e.report.pictures, 1);
}
