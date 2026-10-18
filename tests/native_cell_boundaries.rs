use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::path::Path;
#[test]
fn cell_y_resets_do_not_prevent_same_count_soft_line_repair() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.header.attrs.insert("binaryHwp".into(), "1".into());
    d.fonts
        .insert(("LATIN".into(), "probe".into()), "굴림체".into());
    d.chars.insert(
        "probe".into(),
        xml(br##"<charPr height="1000" textColor="#000000"><fontRef latin="probe"/></charPr>"##)
            .unwrap(),
    );
    d.paras.insert("probe".into(),xml(br#"<paraPr binaryHwp="1"><align horizontal="LEFT"/><breakSetting breakLatinWord="BREAK_WORD"/></paraPr>"#).unwrap());
    d.sections=vec![xml(br#"<sec><p paraPrIDRef="probe"><run charPrIDRef="probe"><secPr><pagePr width="10000" height="14000"><margin left="2000" right="2000" top="2000" bottom="2000"/></pagePr></secPr><tbl binaryHwp="1" pageBreak="NONE" rowCnt="1" colCnt="1"><sz width="2000" height="7000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><tr><tc><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="2000" height="0"/><subList lineWrap="BREAK" vertAlign="TOP"><p paraPrIDRef="probe"><run charPrIDRef="probe"><t>TOP</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="2000"/></linesegarray></p><p paraPrIDRef="probe"><run charPrIDRef="probe"><t>ABCDEFGHIJKL</t></run><linesegarray><lineseg textpos="0" vertpos="2000" vertsize="1000" baseline="850" spacing="600" horzsize="2000"/><lineseg textpos="3" vertpos="3600" vertsize="1000" baseline="850" spacing="600" horzsize="2000"/><lineseg textpos="8" vertpos="0" vertsize="1000" baseline="850" spacing="600" horzsize="2000"/></linesegarray></p></subList></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="6000"/></linesegarray></p></sec>"#).unwrap()];
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
    let y = |target: &str| {
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .find_map(|o| match o {
                Op::Text { text, y, .. } if text == target => Some(*y),
                _ => None,
            })
            .unwrap()
    };
    assert!((y("A") - y("D")).abs() < 0.01);
    assert!((y("E") - y("A") - 16.).abs() < 0.01);
    assert!((y("I") - y("E") - 16.).abs() < 0.01);
    assert_eq!(original, d.sections);
}

#[test]
fn normal_wrap_cell_grows_a_stale_single_line_without_squeezing_its_font() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
    )
    .unwrap();
    let text = "3. 「장애인복지법」 제2조제2항에 따른 장애인";
    d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="24"><secPr><pagePr width="30000" height="40000"><margin left="2000" right="2000" top="2000" bottom="2000"/></pagePr></secPr><tbl binaryHwp="1" rowCnt="1" colCnt="1" pageBreak="NONE"><sz width="23208" height="4000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="566" right="566" top="566" bottom="566"/><tr><tc><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="23208" height="0"/><subList lineWrap="BREAK" vertAlign="TOP"><p paraPrIDRef="0"><run charPrIDRef="24"><t>{text}</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" textheight="1000" baseline="850" spacing="600" horzsize="22076"/></linesegarray></p></subList></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="26000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
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
    let glyphs: Vec<_> = e
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|o| match o {
            Op::Text { text, y, .. } => Some((text.as_str(), *y)),
            _ => None,
        })
        .collect();
    assert_eq!(glyphs.iter().map(|g| g.0).collect::<String>(), text);
    assert_eq!(glyphs.last().unwrap().0, "인");
    assert!((glyphs.last().unwrap().1 - glyphs[0].1 - 16.).abs() < 0.01);
    assert_eq!(original, d.sections);
}
