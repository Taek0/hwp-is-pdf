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
fn document(body: String) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.sections = vec![xml(body.as_bytes()).unwrap()];
    d.masters.clear();
    d
}
const PAGE: &str = r#"<secPr><pagePr width="40000" height="40000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr>"#;
#[test]
fn native_inline_table_baseline_includes_asymmetric_outside_margins() {
    let d = document(format!(
        r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0">{PAGE}<tbl id="native" binaryHwp="1" rowCnt="1" colCnt="1" pageBreak="NONE"><sz width="30000" height="10000"/><pos treatAsChar="1"/><outMargin top="400" bottom="800"/><tr><tc borderFillIDRef="1"><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="30000" height="10000"/><subList><p paraPrIDRef="0"><run charPrIDRef="0"><t>CELL</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="30000"/></linesegarray></p></subList></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="11200" baseline="9520" horzsize="30000"/></linesegarray></p></sec>"#
    ));
    let original = d.sections.clone();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let b = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "tbl" && b.id == "native")
        .unwrap();
    assert!((b.y - 54.).abs() < 0.001);
    assert_eq!(original, d.sections);
    assert!(
        qa::inspect(&e.pages, &e.fonts, &e.report.bounds)
            .text_overlaps
            .is_empty()
    );
}
#[test]
fn text_box_vertical_alignment_uses_content_height_and_both_margins() {
    for (align, expected) in [("TOP", 60.), ("CENTER", 89.), ("BOTTOM", 118.)] {
        let d = document(format!(
            r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0">{PAGE}<rect id="box"><sz width="20000" height="10000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><drawText><textMargin top="1000" bottom="2000" left="100" right="100"/><subList vertAlign="{align}"><p id="inside" paraPrIDRef="0"><run charPrIDRef="0"><t>TEXT</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="19800"/></linesegarray></p></subList></drawText></rect></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="30000"/></linesegarray></p></sec>"#
        ));
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let b = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == "inside")
            .unwrap();
        assert!((b.y - expected).abs() < 0.001, "{align}: {}", b.y);
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty());
        assert!(q.border_crossings.is_empty());
        assert!(q.outside_page.is_empty());
    }
}
#[test]
fn supplied_title_baselines_match_reference_positions() {
    for (name, page, label, size, baseline) in [
        ("복학원서.hwp", 0, "복", 24., 86.64),
        ("21_언어_기출_편집가능본.hwp", 7, "언", 25., 127.189),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(name);
        if !path.exists() {
            continue;
        }
        let d = Document::open(&path).unwrap();
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let y = e.pages[page]
            .ops
            .iter()
            .find_map(|op| match op {
                Op::Text {
                    text, size: s, y, ..
                } if text == label && (*s - size).abs() < 0.01 => Some(*y),
                _ => None,
            })
            .unwrap();
        assert!((y - baseline).abs() < 0.1, "{name}: {y}");
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty());
        assert!(q.border_crossings.is_empty());
    }
}
