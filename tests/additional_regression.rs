use hwp_engine::{layout::Engine, model::Document};
use std::path::PathBuf;

#[test]
fn explicit_none_underline_is_not_drawn() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut doc = Document::open(&root.join("samples/additional/underline-styles.hwp")).unwrap();
    for style in doc.chars.values_mut() {
        for child in &mut style.children {
            if child.name == "underline" {
                child.attrs.insert("type".into(), "NONE".into());
            }
        }
    }
    let fonts = PathBuf::from(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut engine = Engine::new(&doc, &fonts);
    engine.render().unwrap();
    assert!(
        engine
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .all(|op| !matches!(op, hwp_engine::pdf::Op::Path { .. }))
    );
}

fn with_fixture(name: &str, check: impl FnOnce(&Engine<'_>)) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let doc = Document::open(&root.join(format!("samples/additional/{name}.hwp"))).unwrap();
    let fonts = PathBuf::from(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut engine = Engine::new(&doc, &fonts);
    engine.render().unwrap();
    let audit = hwp_engine::qa::inspect(&engine.pages, &engine.fonts, &engine.report.bounds);
    assert!(
        audit.outside_page.is_empty(),
        "{name}: {:?}",
        audit.outside_page
    );
    assert!(
        audit.text_overlaps.is_empty(),
        "{name}: {:?}",
        audit.text_overlaps
    );
    assert!(
        audit.border_crossings.is_empty(),
        "{name}: {:?}",
        audit.border_crossings
    );
    assert!(
        audit.foreground_image_overlaps.is_empty(),
        "{name}: {:?}",
        audit.foreground_image_overlaps
    );
    check(&engine);
}

#[test]
fn captions_stay_with_eight_distinct_tables_and_paginate() {
    with_fixture("table-caption", |e| {
        assert_eq!(e.pages.len(), 2);
        let tables: Vec<_> = e.report.bounds.iter().filter(|b| b.kind == "tbl").collect();
        assert_eq!(tables.len(), 8);
        for (i, a) in tables.iter().enumerate() {
            assert!(a.x >= 0. && a.x + a.w <= e.pages[a.page - 1].width);
            for b in &tables[i + 1..] {
                if a.page == b.page {
                    assert!(
                        a.y + a.h <= b.y
                            || b.y + b.h <= a.y
                            || a.x + a.w <= b.x
                            || b.x + b.w <= a.x
                    );
                }
            }
        }
        assert_eq!(tables.iter().filter(|b| b.page == 2).count(), 2);
    });
}

#[test]
fn block_picture_labels_remain_below_images() {
    with_fixture("sample-5017-pics", |e| {
        assert_eq!(e.pages.len(), 11);
        assert_eq!(e.report.pictures, 11);
        for page in [5, 8] {
            let picture = e
                .report
                .bounds
                .iter()
                .find(|b| b.page == page && matches!(b.kind.as_str(), "pic" | "pic-alpha"))
                .unwrap();
            let label = e
                .report
                .bounds
                .iter()
                .find(|b| b.page == page && b.kind == "line" && b.w > 0.)
                .unwrap();
            assert!(label.y + 0.02 >= picture.y + picture.h);
        }
    });
}

#[test]
fn column_sets_change_within_one_section() {
    with_fixture("multicolumns", |e| {
        assert_eq!(e.pages.len(), 3);
        for (page, count) in [(2, 2), (3, 3)] {
            let x: std::collections::BTreeSet<_> = e
                .report
                .bounds
                .iter()
                .filter(|b| b.page == page && b.kind == "line" && b.w > 0.)
                .map(|b| (b.x * 100.).round() as i64)
                .collect();
            assert_eq!(x.len(), count);
        }
    });
}

#[test]
fn all_eleven_underline_styles_emit_visible_strokes() {
    use hwp_engine::pdf::Op;
    with_fixture("underline-styles", |e| {
        let page = &e.pages[0];
        let baselines: std::collections::BTreeSet<_> = page
            .ops
            .iter()
            .filter_map(|op| {
                if let Op::Text { y, .. } = op {
                    Some((y * 100.).round() as i64)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(baselines.len(), 11);
        for baseline in baselines {
            let y = baseline as f64 / 100.;
            assert!(
                page.ops
                    .iter()
                    .any(|op| if let Op::Path { points, width, .. } = op {
                        *width > 0.
                            && points.len() >= 2
                            && points[0].1 > y
                            && points[0].1 < y + 7.
                            && points[1].0 > points[0].0
                    } else {
                        false
                    }),
                "Missing underline at {y}"
            );
        }
        let strokes: Vec<_> = page
            .ops
            .iter()
            .filter(|op| matches!(op, Op::Path { .. }))
            .collect();
        assert!(
            strokes.len() > 30,
            "Dashed/dotted styles must have separate strokes"
        );
    });
}

#[test]
fn binary_section_orientation_is_applied_to_pdf_page_dimensions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let doc = Document::open(&root.join("samples/additional/pagedefs.hwp")).unwrap();
    let fonts = PathBuf::from(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut engine = Engine::new(&doc, &fonts);
    engine.render().unwrap();
    assert_eq!(engine.pages.len(), 2);
    assert!((engine.pages[0].width - 595.28).abs() < 0.01);
    assert!((engine.pages[0].height - 841.88).abs() < 0.01);
    assert!((engine.pages[1].width - 841.88).abs() < 0.01);
    assert!((engine.pages[1].height - 595.28).abs() < 0.01);
}
