use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
    qa,
};
use std::path::Path;

#[test]
fn staggered_cell_lines_cross_pages_without_losing_or_clipping_text() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    let cell = |col: usize, row: usize, height: usize, content: String| {
        format!(
            r#"<tc borderFillIDRef="1"><cellAddr rowAddr="{row}" colAddr="{col}"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="10000" height="{height}"/><subList>{content}</subList></tc>"#
        )
    };
    let paragraph = |s: String, y: usize| {
        format!(
            r#"<p paraPrIDRef="0"><run charPrIDRef="0"><t>{s}</t></run><linesegarray><lineseg textpos="0" vertpos="{y}" vertsize="1000" baseline="850" horzsize="9800" spacing="200"/></linesegarray></p>"#
        )
    };
    let headers = (0..3)
        .map(|c| cell(c, 0, 2000, paragraph(format!("HEADER{c}"), 0)))
        .collect::<String>();
    let rows = (0..3)
        .map(|c| {
            cell(
                c,
                1,
                51000,
                (0..40)
                    .map(|i| paragraph(format!("C{c}R{i:02}"), i * 1200 + c * 400))
                    .collect(),
            )
        })
        .collect::<String>();
    d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><tbl id="staggered" binaryHwp="1" pageBreak="TABLE" repeatHeader="1" rowCnt="2" colCnt="3"><sz width="30000" height="53000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="100" right="100" top="100" bottom="100"/><tr>{headers}</tr><tr>{rows}</tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="30000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
    d.masters.clear();
    let original = d.sections.clone();
    let mut e = Engine::new(
        &d,
        Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    assert_eq!(e.pages.len(), 3);
    let texts: Vec<_> = e
        .pages
        .iter()
        .map(|p| {
            p.ops
                .iter()
                .filter_map(|op| match op {
                    Op::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>()
        })
        .collect();
    let all = texts.concat();
    for col in 0..3 {
        assert!(texts.iter().all(|t| t.contains(&format!("HEADER{col}"))));
        for row in 0..40 {
            assert_eq!(all.matches(&format!("C{col}R{row:02}")).count(), 1);
        }
    }
    assert_eq!(original, d.sections);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
    assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
}
