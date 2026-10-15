use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
fn fonts() -> &'static std::path::Path {
    std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn doc(name: &str) -> Document {
    Document::open(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name)).unwrap()
}
fn visible(ops: &[Op]) -> String {
    ops.iter()
        .filter_map(|o| {
            if let Op::Text { text, .. } = o {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn empty_toc_and_fields_resolve_to_final_pages_and_link_only_the_field() {
    let d = doc("samples/reflow/fields-toc.hwpx");
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    assert_eq!(e.pages.len(), 3);
    let first = visible(&e.pages[0].ops);
    assert!(first.contains("TOTAL-PAGES: 3"), "{first}");
    assert!(first.contains("SECOND-PAGE: 2"), "{first}");
    let destinations: Vec<_> = e
        .pages
        .iter()
        .enumerate()
        .flat_map(|(page, p)| {
            p.ops.iter().filter_map(move |o| {
                if let Op::Destination { name, level, .. } = o {
                    Some((page, name.as_str(), *level))
                } else {
                    None
                }
            })
        })
        .collect();
    assert_eq!(
        destinations
            .iter()
            .filter(|(_, _, level)| *level != usize::MAX)
            .count(),
        6
    );
    assert!(
        destinations
            .iter()
            .any(|(page, name, _)| *page == 1 && *name == "chapter-two")
    );
    assert_eq!(
        e.pages[0]
            .ops
            .iter()
            .filter(|o| matches!(o,Op::Link{target,..} if target=="https://www.hancom.com"))
            .count(),
        "한컴 홈페이지".chars().count()
    );
    assert!(
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .all(|o| !matches!(o, Op::FieldText { .. }))
    );
    for (page, name, level) in destinations
        .iter()
        .filter(|(_, _, level)| *level != usize::MAX)
    {
        let links: Vec<_> = e.pages[0]
            .ops
            .iter()
            .filter_map(|o| {
                if let Op::Link { target, rect, .. } = o {
                    (target == &format!("#{name}")).then_some(rect)
                } else {
                    None
                }
            })
            .collect();
        assert!(!links.is_empty(), "missing TOC link {name} level {level}");
        let row = links[0][1];
        let row_text: String = e.pages[0]
            .ops
            .iter()
            .filter_map(|o| {
                if let Op::Text { text, y, .. } = o {
                    ((*y - row).abs() < 20.).then_some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            row_text.contains(&(page + 1).to_string()),
            "wrong TOC page: {row_text}"
        );
    }
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
}
#[test]
fn populated_custom_fields_and_toc_remain_authoritative() {
    let d = doc("samples/reflow/fields-cached.hwpx");
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    let first = visible(&e.pages[0].ops);
    assert!(first.contains("TOTAL-PAGES: 99"), "{first}");
    assert!(first.contains("보존된 사용자 목차 ........ 99"));
    assert!(
        !e.report
            .warnings
            .iter()
            .any(|s| s.starts_with("Empty TOC rebuilt"))
    );
}
#[test]
fn native_hwp_fields_keep_cached_text_in_saved_and_reflow_modes() {
    let d = doc("samples/features/fields.hwp");
    for reflow in [false, true] {
        let mut e = Engine::new(&d, fonts());
        if reflow {
            e.render_reflow().unwrap();
        } else {
            e.render().unwrap();
        }
        assert!(!visible(&e.pages[0].ops).trim().is_empty());
        assert!(
            e.pages
                .iter()
                .flat_map(|p| &p.ops)
                .all(|o| !matches!(o, Op::FieldText { .. }))
        );
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty());
        assert!(q.text_overlaps.is_empty());
    }
}
