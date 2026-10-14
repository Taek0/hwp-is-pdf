use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
#[test]
fn mixed_table_with_long_footnote_keeps_every_fragment() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/overflow/nested-mixed.hwpx"),
    )
    .unwrap();
    fn extend(n: &mut hwp_engine::model::Node) {
        if n.name == "footNote" {
            let list = n.children.iter_mut().find(|c| c.name == "subList").unwrap();
            let template = list.children[0].clone();
            list.children = (1..=90)
                .map(|i| {
                    let mut p = template.clone();
                    fn edit(n: &mut hwp_engine::model::Node, i: usize) {
                        if n.name == "#text" {
                            n.text = format!("긴각주 L{i:03}: 표 다음 페이지에서도 보존합니다.");
                        }
                        if n.name == "lineseg" {
                            n.attrs
                                .insert("vertpos".into(), ((i - 1) * 1600).to_string());
                        }
                        for c in &mut n.children {
                            edit(c, i)
                        }
                    }
                    edit(&mut p, i);
                    p
                })
                .collect();
        } else {
            for c in &mut n.children {
                extend(c)
            }
        }
    }
    extend(&mut d.sections[0]);
    let mut e = Engine::new(
        &d,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    let pages: Vec<_> = e
        .pages
        .iter()
        .map(|p| {
            p.ops
                .iter()
                .filter_map(|o| {
                    if let Op::Text { text, .. } = o {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>()
        })
        .collect();
    let all = pages.concat();
    for i in 1..=90 {
        assert_eq!(all.matches(&format!("긴각주 L{i:03}:")).count(), 1);
    }
    for i in 1..=60 {
        assert_eq!(all.matches(&format!("행 R {i:03}")).count(), 1);
    }
    assert!(
        pages
            .iter()
            .find(|p| p.contains("행 R 031"))
            .unwrap()
            .contains("긴각주 L001:")
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
}
#[test]
fn note_inside_rotated_object_travels_with_its_reference() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/overflow/nested-mixed.hwpx"),
    )
    .unwrap();
    let note = d.sections[0].find("footNote").unwrap().clone();
    fn remove(n: &mut hwp_engine::model::Node) {
        n.children.retain(|c| c.name != "footNote");
        for c in &mut n.children {
            remove(c)
        }
    }
    remove(&mut d.sections[0]);
    fn attach(n: &mut hwp_engine::model::Node, note: &hwp_engine::model::Node) {
        if n.a("id") == "mixed-rotation" {
            let draw = n
                .children
                .iter_mut()
                .find(|c| c.name == "drawText")
                .unwrap();
            let list = draw
                .children
                .iter_mut()
                .find(|c| c.name == "subList")
                .unwrap();
            let p = list.children.iter_mut().find(|c| c.name == "p").unwrap();
            p.children
                .iter_mut()
                .find(|c| c.name == "run")
                .unwrap()
                .children
                .push(hwp_engine::model::Node {
                    name: "ctrl".into(),
                    children: vec![note.clone()],
                    ..Default::default()
                });
        } else {
            for c in &mut n.children {
                attach(c, note)
            }
        }
    }
    attach(&mut d.sections[0], &note);
    let mut e = Engine::new(
        &d,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    let pages: Vec<_> = e
        .pages
        .iter()
        .map(|p| {
            p.ops
                .iter()
                .filter_map(|o| {
                    if let Op::Text { text, .. } = o {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>()
        })
        .collect();
    let page = pages.iter().find(|p| p.contains("회전 개체")).unwrap();
    assert!(page.contains("각주 N001:"));
    let all = pages.concat();
    for i in 1..=12 {
        assert_eq!(all.matches(&format!("각주 N{i:03}:")).count(), 1);
    }
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
    assert!(q.foreground_image_overlaps.is_empty());
}
#[test]
fn mixed_table_keeps_objects_and_footnote_on_safe_pages() {
    for height in [84188, 72000, 96000] {
        let mut d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("samples/overflow/nested-mixed.hwpx"),
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
        let mut e = Engine::new(
            &d,
            std::path::Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        e.render().unwrap();
        let pages: Vec<_> = e
            .pages
            .iter()
            .map(|p| {
                p.ops
                    .iter()
                    .filter_map(|o| {
                        if let Op::Text { text, .. } = o {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<String>()
            })
            .collect();
        let all = pages.concat();
        for i in 1..=60 {
            assert_eq!(
                all.matches(&format!("행 R {i:03}")).count(),
                1,
                "height={height} row={i}"
            );
        }
        for i in 1..=12 {
            assert_eq!(all.matches(&format!("각주 N{i:03}:")).count(), 1);
        }
        assert_eq!(all.matches("회전 개체").count(), 1);
        let reference = pages.iter().position(|p| p.contains("행 R 031")).unwrap();
        assert!(
            pages[reference].contains("각주 N001:"),
            "reference and footnote separated at {height}"
        );
        let mut images = 0;
        let mut rotations = 0;
        for page in &e.pages {
            let mut depth = 0;
            for op in &page.ops {
                match op {
                    Op::Image { .. } | Op::ImageClip { .. } => images += 1,
                    Op::TransformBegin { matrix } => {
                        depth += 1;
                        if matrix[1].abs() > 0.1 {
                            rotations += 1;
                        }
                    }
                    Op::TransformEnd => {
                        depth -= 1;
                        assert!(depth >= 0);
                    }
                    _ => {}
                }
            }
            assert_eq!(depth, 0);
        }
        assert_eq!(images, 1);
        assert_eq!(rotations, 1);
        assert!(
            !e.report
                .warnings
                .iter()
                .any(|w| w.contains("fitted proportionally"))
        );
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{height}: {:?}", q.outside_page);
        assert!(
            q.text_overlaps.is_empty(),
            "{height}: {:?}",
            q.text_overlaps
        );
        assert!(
            q.border_crossings.is_empty(),
            "{height}: {:?}",
            q.border_crossings
        );
        assert!(
            q.foreground_image_overlaps.is_empty(),
            "{height}: {:?}",
            q.foreground_image_overlaps
        );
    }
}
