use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
#[test]
fn top_caption_moves_with_table_when_page_has_no_room() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/overflow/nested-repeat-caption.hwpx"),
    )
    .unwrap();
    fn edit(n: &mut hwp_engine::model::Node) {
        if n.name == "lineseg" {
            n.attrs.insert("vertpos".into(), "73500".into());
        }
        for c in &mut n.children {
            if c.name != "run" {
                edit(c);
            }
        }
    }
    edit(&mut d.sections[0].children[0]);
    let mut intro = d.sections[0].children[1].clone();
    intro.attrs.insert("id".into(), "intro".into());
    fn label(n: &mut hwp_engine::model::Node) {
        if n.name == "#text" {
            n.text = "앞쪽 본문입니다.".into();
        }
        if n.name == "lineseg" {
            n.attrs.insert("vertpos".into(), "0".into());
        }
        for c in &mut n.children {
            label(c)
        }
    }
    label(&mut intro);
    d.sections[0].children.insert(0, intro);
    let mut e = Engine::new(
        &d,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    let text = |p: &hwp_engine::pdf::Page| {
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
    };
    assert!(text(&e.pages[0]).contains("앞쪽 본문"));
    assert!(!text(&e.pages[0]).contains("위 캡션:"));
    assert!(text(&e.pages[1]).contains("위 캡션:"));
    assert!(text(&e.pages[1]).contains("행 R001"));
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
#[test]
fn bottom_caption_keeps_last_row_across_page_sizes() {
    for height in [68000, 72000, 76000, 80000, 84000, 88000] {
        let mut d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("samples/overflow/nested-repeat-caption.hwpx"),
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
        let caption = pages.iter().find(|p| p.contains("아래 캡션:")).unwrap();
        assert!(caption.contains("행 R060"), "height {height}: {caption}");
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{height}: {:?}", q.outside_page);
        assert!(
            q.text_overlaps.is_empty(),
            "{height}: {:?}",
            q.text_overlaps
        );
    }
}
#[test]
fn nested_headers_repeat_and_captions_stay_on_first_and_last_fragments() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/overflow/nested-repeat-caption.hwpx"),
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
    e.render().unwrap();
    let page_text: Vec<_> = e
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
    assert!(page_text.len() >= 2);
    for p in &page_text {
        assert_eq!(p.matches("반복 머리행: 항목 안내").count(), 1);
    }
    assert!(page_text[0].contains("위 캡션:"));
    assert!(page_text.last().unwrap().contains("아래 캡션:"));
    let text = page_text.concat();
    assert_eq!(text.matches("위 캡션:").count(), 1);
    assert_eq!(text.matches("아래 캡션:").count(), 1);
    for i in 1..=60 {
        assert_eq!(text.matches(&format!("행 R{i:03}")).count(), 1);
    }
    assert!(text.ends_with("캡션까지 끝난 뒤 이어지는 본문입니다."));
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
}
#[test]
fn nested_merged_rows_paginate_without_scaling_or_lost_cells() {
    let d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/overflow/nested-merged-rows.hwpx"),
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
    e.render().unwrap();
    assert!(e.pages.len() >= 2);
    let mut text = String::new();
    for op in e.pages.iter().flat_map(|p| &p.ops) {
        match op {
            Op::Text { text: t, .. } => text += t,
            Op::TransformBegin { matrix } => {
                assert_eq!(matrix[0], 1.);
                assert_eq!(matrix[3], 1.);
            }
            _ => {}
        }
    }
    for i in 1..=60 {
        assert_eq!(text.matches(&format!("행 R{i:03}")).count(), 1);
    }
    for i in 1..=30 {
        assert_eq!(text.matches(&format!("병합 M{i:03}")).count(), 1);
    }
    assert!(text.ends_with("분할 표 다음 본문입니다."));
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
}
#[test]
fn continued_note_precedes_new_note_without_losing_lines() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut d = Document::open(&root.join("samples/overflow/long-footnote.hwpx")).unwrap();
    let mut note = d.sections[0].find("footNote").unwrap().clone();
    note.attrs.insert("number".into(), "2".into());
    note.attrs.insert("id".into(), "second".into());
    let list = note
        .children
        .iter_mut()
        .find(|n| n.name == "subList")
        .unwrap();
    list.children.truncate(3);
    fn replace(n: &mut hwp_engine::model::Node) {
        n.text = n.text.replace("각주", "보조");
        for c in &mut n.children {
            replace(c)
        }
    }
    replace(list);
    let p = d.sections[0]
        .children
        .iter_mut()
        .filter(|n| n.name == "p")
        .nth(1)
        .unwrap();
    p.children
        .iter_mut()
        .find(|n| n.name == "run")
        .unwrap()
        .children
        .push(hwp_engine::model::Node {
            name: "ctrl".into(),
            children: vec![note],
            ..Default::default()
        });
    let mut e = Engine::new(
        &d,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    let text = e
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
    for i in 1..=90 {
        assert_eq!(text.matches(&format!("각주 {i:03}:")).count(), 1);
    }
    for i in 1..=3 {
        assert_eq!(text.matches(&format!("보조 {i:03}:")).count(), 1);
    }
    assert!(text.find("각주 090:").unwrap() < text.find("보조 001:").unwrap());
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
#[test]
fn long_notes_and_oversized_content_preserve_all_text() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in [
        "long-footnote",
        "oversized-nested-table",
        "oversized-object",
    ] {
        let d = Document::open(&root.join(format!("samples/overflow/{name}.hwpx"))).unwrap();
        let mut e = Engine::new(
            &d,
            std::path::Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        e.render().unwrap();
        if name == "oversized-nested-table" {
            assert_eq!(e.pages.len(), 2);
            assert!(
                !e.report
                    .warnings
                    .iter()
                    .any(|s| s.contains("fitted proportionally"))
            );
        }
        let text = e
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
        if name == "long-footnote" {
            for i in 1..=90 {
                assert_eq!(text.matches(&format!("각주 {i:03}:")).count(), 1);
            }
            assert!(e.pages.len() >= 3);
            assert_eq!(text.matches("각주와 함께 이어지는 본문입니다.").count(), 1);
            let shared = e.pages[1]
                .ops
                .iter()
                .filter_map(|o| {
                    if let Op::Text { text, .. } = o {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>();
            assert!(shared.contains("이어지는 본문"));
            assert!(shared.contains("각주 "));
        } else {
            assert!(text.contains("첫 줄"));
            assert!(text.contains("마지막 줄"));
            assert!(text.contains("다음 본문입니다."));
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(
            q.outside_page.is_empty(),
            "{name}: outside {:?}",
            q.outside_page
        );
        assert!(
            q.text_overlaps.is_empty(),
            "{name}: overlap {:?}",
            q.text_overlaps
        );
        assert!(
            q.border_crossings.is_empty(),
            "{name}: borders {:?}",
            q.border_crossings
        );
    }
}
