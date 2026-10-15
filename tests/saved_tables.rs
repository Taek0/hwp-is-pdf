use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
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
fn document(binary: bool, wrap: &str) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.sections=vec![xml(format!(r#"<sec><p id="body" paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="50000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><tbl id="table" binaryHwp="{}" rowCnt="1" colCnt="1" pageBreak="NONE"><sz width="9000" height="4000"/><pos treatAsChar="0" horzRelTo="COLUMN" vertRelTo="PARA"/><outMargin left="500" top="700"/><inMargin left="100" right="100" top="100" bottom="100"/><tr><tc borderFillIDRef="1"><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="9000" height="4000"/><subList lineWrap="{wrap}"><p id="text" paraPrIDRef="0"><run charPrIDRef="0"><t>Receiving Official long label</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzpos="0" horzsize="8800"/></linesegarray></p></subList></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzpos="0" horzsize="30000"/></linesegarray></p></sec>"#,usize::from(binary)).as_bytes()).unwrap()];
    d.masters.clear();
    d
}
#[test]
fn native_break_cells_recover_lines_but_squeeze_cells_keep_one_line() {
    for wrap in ["BREAK", "SQUEEZE"] {
        let d = document(true, wrap);
        let source = d.sections.clone();
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let mut rows = std::collections::BTreeSet::new();
        let mut text = String::new();
        for op in &e.pages[0].ops {
            if let Op::Text { text: t, y, .. } = op {
                rows.insert((y * 100.).round() as i64);
                text.push_str(t);
            }
        }
        assert_eq!(text.replace(' ', ""), "ReceivingOfficiallonglabel");
        assert_eq!(rows.len() > 1, wrap == "BREAK");
        assert_eq!(d.sections, source);
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.border_crossings.is_empty());
        assert!(q.outside_page.is_empty());
    }
}
#[test]
fn saved_floating_hwpx_table_uses_outer_margin_box_origin() {
    let d = document(false, "BREAK");
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let b = e.report.bounds.iter().find(|b| b.id == "table").unwrap();
    assert_eq!((b.x, b.y), (55., 57.));
}
