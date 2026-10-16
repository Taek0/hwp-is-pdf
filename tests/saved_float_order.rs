use hwp_engine::{
    layout::Engine,
    model::{Document, text, xml},
    pdf::Op,
    qa,
};
use std::path::Path;
fn fonts() -> &'static Path {
    Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
#[test]
fn below_paragraph_float_keeps_all_anchor_lines_before_a_multipage_table() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    let rows=(0..40).map(|i|format!(r#"<p id="row-{i}" paraPrIDRef="0"><run charPrIDRef="0"><t>ROW-{i:02}</t></run><linesegarray><lineseg textpos="0" vertpos="{}" vertsize="1200" baseline="1000" spacing="400" horzsize="29800"/></linesegarray></p>"#,i*1600)).collect::<String>();
    d.sections=vec![xml(format!(r#"<sec><p id="anchor" paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="50000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><tbl id="float" binaryHwp="1" textWrap="TOP_AND_BOTTOM" pageBreak="TABLE" rowCnt="1" colCnt="1"><sz width="30000" height="20000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" vertAlign="TOP" vertOffset="6000"/><inMargin left="100" right="100" top="100" bottom="100"/><tr><tc borderFillIDRef="1"><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="30000" height="70000"/><subList>{rows}</subList></tc></tr></tbl><t>FIRST SECOND THIRD</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="30000"/><lineseg textpos="14" vertpos="1600" vertsize="1200" baseline="1000" horzsize="30000"/><lineseg textpos="21" vertpos="3200" vertsize="1200" baseline="1000" horzsize="30000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
    d.masters.clear();
    let original = d.sections.clone();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
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
    assert!(e.pages.len() > 1);
    assert!(all.find("THIRD").unwrap() < all.find("ROW-00").unwrap());
    assert!(
        e.report
            .bounds
            .iter()
            .filter(|b| b.kind == "line" && b.id == "anchor")
            .all(|b| b.page == 1)
    );
    for i in 0..40 {
        assert_eq!(all.matches(&format!("ROW-{i:02}")).count(), 1);
    }
    assert_eq!(d.sections, original);
    let mut a: Vec<_> = all.chars().filter(|c| !c.is_whitespace()).collect();
    let mut b: Vec<_> = text(&d.sections[0])
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
    assert!(q.outside_page.is_empty());
}

#[test]
fn report_anchor_precedes_table_and_research_table_returns_to_reference_page() {
    let d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
    )
    .unwrap();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 50);
    let page_text = |i: usize| {
        e.pages[i]
            .ops
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
    assert!(page_text(34).contains("이 과정에서"));
    assert!(!page_text(41).contains("이 과정에서"));
    assert!(page_text(45).contains("제목(년도)"));
    let research: std::collections::BTreeSet<_> = e
        .report
        .bounds
        .iter()
        .filter(|b| b.kind == "tbl" && b.id == "hwp-6909f")
        .map(|b| b.page)
        .collect();
    assert_eq!(research, [46, 47].into_iter().collect());
    let note_pages: std::collections::BTreeSet<_> = e
        .report
        .bounds
        .iter()
        .filter(|b| b.kind.starts_with("footnote") && b.id == "44")
        .map(|b| b.page)
        .collect();
    assert_eq!(note_pages, [44, 45].into_iter().collect());
    for (page, count) in [(44, 3), (45, 1)] {
        assert_eq!(
            e.report
                .bounds
                .iter()
                .filter(|b| b.kind == "footnote-fragment" && b.id == "44" && b.page == page)
                .count(),
            count
        );
    }
    let tail = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "tbl" && b.id == "hwp-44c83" && b.page == 42)
        .unwrap();
    assert!(
        tail.h < 100.,
        "staggered merged cells should not leave whole-row gaps: {}",
        tail.h
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
    assert!(q.outside_page.is_empty());
}
