use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::path::Path;
#[test]
fn outlined_and_following_plain_text_keep_positions_and_searchable_content() {
    let mut doc = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    let mut outlined = doc.chars["0"].clone();
    outlined
        .children
        .push(xml(b"<outline type=\"SOLID\"/>").unwrap());
    doc.chars.insert("outlined".into(), outlined);
    doc.masters.clear();
    doc.sections=vec![xml(br#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>A</t></run><run charPrIDRef="outlined"><t>B</t></run><run charPrIDRef="0"><t>C</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1400" baseline="1100" horzsize="30000"/></linesegarray></p></sec>"#).unwrap()];
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut engine = Engine::new(&doc, fonts);
    engine.render().unwrap();
    let mut stack = Vec::new();
    let mut active = false;
    let mut texts = Vec::new();
    for op in &engine.pages[0].ops {
        match op {
            Op::TransformBegin { .. } => stack.push(active),
            Op::TransformEnd => active = stack.pop().unwrap(),
            Op::TextOutline { .. } => active = true,
            Op::Text { text, x, .. } => texts.push((text.clone(), *x, active)),
            _ => {}
        }
    }
    assert_eq!(
        texts.iter().map(|v| v.0.as_str()).collect::<String>(),
        "ABC"
    );
    assert_eq!(
        texts.iter().map(|v| v.2).collect::<Vec<_>>(),
        vec![false, true, false]
    );
    assert!(texts.windows(2).all(|v| v[0].1 < v[1].1));
    assert!(stack.is_empty());
}
