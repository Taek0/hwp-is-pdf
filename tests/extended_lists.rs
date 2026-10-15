use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
#[test]
fn extended_formats_restarts_and_image_bullets_have_measured_space() {
    for name in ["formats", "hierarchy", "image"] {
        let d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("samples/reflow/list-extended-{name}.hwpx")),
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
        let labels: Vec<_> = e
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
            .filter(|s| s.chars().count() > 1)
            .collect();
        if name == "formats" {
            for label in [
                "3.", "③.", "III.", "iii.", "C.", "c.", "Ⓒ.", "ⓒ.", "다.", "㉰.", "ㄷ.", "㉢.",
                "삼.", "三.", "㊂.",
            ] {
                assert!(labels.contains(&label), "{label}");
            }
        }
        if name == "hierarchy" {
            assert_eq!(labels, vec!["III.", "III.e.", "III.f.", "IV.", "IV.e."]);
        }
        if name == "image" {
            assert_eq!(e.report.pictures, 4);
            assert!(e.pages.len() > 1);
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{name}");
        assert!(q.text_overlaps.is_empty(), "{name} {:?}", q.text_overlaps);
        assert!(
            q.foreground_image_overlaps.is_empty(),
            "{name} {:?}",
            q.foreground_image_overlaps
        );
    }
}
