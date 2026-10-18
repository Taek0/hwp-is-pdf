use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::path::Path;
#[test]
fn native_fixed_bullet_reserves_an_unscaled_em_and_hangs_continuation_text() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
    )
    .unwrap();
    d.sections=vec![xml(r#"<sec><p paraPrIDRef="27"><run charPrIDRef="15"><secPr><pagePr width="9780" height="20000"><margin left="2000" right="2000" top="2000" bottom="2000"/></pagePr></secPr><t>대지조성사업</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" spacing="200" horzsize="5780"/><lineseg textpos="13" vertpos="1200" vertsize="1000" baseline="850" horzsize="5780"/></linesegarray></p></sec>"#.as_bytes()).unwrap()];
    d.masters.clear();
    let mut e = Engine::new(
        &d,
        Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    let position = |c: &str| {
        e.pages[0]
            .ops
            .iter()
            .find_map(|o| match o {
                Op::Text { text, x, y, .. } if text == c => Some((*x, *y)),
                _ => None,
            })
            .unwrap()
    };
    assert!((position("대").0 - 31.).abs() < 0.01);
    assert!(
        (position("업").0 - position("대").0).abs() < 0.01,
        "positions {:?} {:?}",
        position("대"),
        position("업")
    );
    assert!((position("업").1 - position("대").1 - 12.).abs() < 0.01);
}

#[test]
fn native_hanging_bullet_cell_grows_instead_of_squeezing_a_single_saved_line() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
    )
    .unwrap();
    d.sections=vec![xml(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="15"><secPr><pagePr width="20000" height="20000"><margin left="2000" right="2000" top="2000" bottom="2000"/></pagePr></secPr><tbl binaryHwp="1" rowCnt="1" colCnt="1" pageBreak="NONE"><sz width="6800" height="4000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="510" right="510" top="141" bottom="141"/><tr><tc><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="6800" height="0"/><subList lineWrap="BREAK" vertAlign="TOP"><p id="word" paraPrIDRef="27"><run charPrIDRef="15"><t>대지조성사업</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" spacing="200" horzsize="5780"/></linesegarray></p></subList></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="16000"/></linesegarray></p></sec>"#.as_bytes()).unwrap()];
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
    let glyph = |c: &str| {
        e.pages[0]
            .ops
            .iter()
            .find_map(|o| match o {
                Op::Text { text, y, scale, .. } if text == c => Some((*y, *scale)),
                _ => None,
            })
            .unwrap()
    };
    assert!((glyph("업").0 - glyph("대").0 - 12.).abs() < 0.01);
    assert!((glyph("대").1 - 0.88).abs() < 0.001);
    assert_eq!(
        e.report
            .bounds
            .iter()
            .filter(|b| b.kind == "line" && b.id == "word")
            .count(),
        2
    );
    assert_eq!(original, d.sections);
}
