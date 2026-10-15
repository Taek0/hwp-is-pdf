use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
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
fn note_reservation_consumes_cached_break_but_preserves_explicit_break() {
    for (explicit, pages) in [(0, 2), (1, 3)] {
        let mut d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="59528" height="40000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>BEFORE</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="49528"/></linesegarray></p><p paraPrIDRef="0"><run charPrIDRef="0"><t>ANCHOR</t><ctrl><footNote number="1"><subList><p paraPrIDRef="0"><run charPrIDRef="0"><t>NOTE</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="10000" baseline="1000" horzsize="49528"/></linesegarray></p></subList></footNote></ctrl></run><linesegarray><lineseg textpos="0" vertpos="24000" vertsize="2000" baseline="1000" horzsize="49528"/></linesegarray></p><p pageBreak="{explicit}" paraPrIDRef="0"><run charPrIDRef="0"><t>AFTER</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="49528"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        assert_eq!(e.pages.len(), pages);
        let text: String = e.pages[pages - 1]
            .ops
            .iter()
            .filter_map(|op| {
                if let Op::Text { text, .. } = op {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(text.contains("AFTER"));
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty());
        assert!(q.outside_page.is_empty());
    }
}
#[test]
fn cached_inline_table_reserves_trailing_line_spacing_without_losing_content() {
    for (spacing, pages) in [(0, 1), (1000, 2)] {
        let mut d = Document::open(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let cell = |col| {
            format!(
                r#"<tc borderFillIDRef="0"><cellAddr colAddr="{col}" rowAddr="0"/><cellSpan colSpan="1" rowSpan="1"/><cellSz width="10000" height="20000"/><subList><p paraPrIDRef="0"><run charPrIDRef="0"><t>CELL-{col}</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="10000"/></linesegarray></p></subList></tc>"#
            )
        };
        d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="59528" height="40000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>BEFORE</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="49528"/></linesegarray></p><p id="inline" paraPrIDRef="0"><run charPrIDRef="0"><tbl id="atomic" rowCnt="1" colCnt="2" pageBreak="NONE"><sz width="20000" height="20000"/><pos treatAsChar="1"/><tr>{}{}</tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="10000" vertsize="20000" spacing="{spacing}" baseline="17000" horzsize="49528"/></linesegarray></p></sec>"#,cell(0),cell(1)).as_bytes()).unwrap()];
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        assert_eq!(e.pages.len(), pages);
        let page_text = |index: usize| {
            e.pages[index]
                .ops
                .iter()
                .filter_map(|op| {
                    if let Op::Text { text, .. } = op {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>()
        };
        assert!(page_text(0).contains("BEFORE"));
        assert!(page_text(pages - 1).contains("CELL-0"));
        assert!(page_text(pages - 1).contains("CELL-1"));
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty());
        assert!(q.text_overlaps.is_empty());
    }
}
