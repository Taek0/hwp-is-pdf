use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::path::Path;

#[test]
fn justification_expands_inner_spaces_without_moving_the_leading_padding() {
    let mut positions = Vec::new();
    for align in ["LEFT", "JUSTIFY", "DISTRIBUTE_SPACE"] {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        d.masters.clear();
        d.paras.insert(
            "probe".into(),
            xml(format!(r#"<paraPr><align horizontal="{align}"/></paraPr>"#).as_bytes()).unwrap(),
        );
        d.sections = vec![xml(br#"<sec><p id="probe" paraPrIDRef="probe"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1020" horzsize="10000"/></linesegarray></p><p id="text" paraPrIDRef="probe"><run charPrIDRef="0"><t>   A B C D</t></run><linesegarray><lineseg textpos="0" vertpos="2000" vertsize="1200" textheight="1200" baseline="1020" horzpos="0" horzsize="10000"/><lineseg textpos="8" vertpos="4000" vertsize="1200" textheight="1200" baseline="1020" horzpos="0" horzsize="10000"/></linesegarray></p></sec>"#).unwrap()];
        let mut e = Engine::new(
            &d,
            Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        e.render().unwrap();
        let at = |ch: &str| {
            e.pages[0]
                .ops
                .iter()
                .find_map(|o| match o {
                    Op::Text { text, x, .. } if text == ch => Some(*x),
                    _ => None,
                })
                .unwrap()
        };
        positions.push((at("A"), at("B"), at("C")));
    }
    for &(a, b, c) in &positions[1..] {
        assert!((a - positions[0].0).abs() < 0.001);
        assert!(
            b > positions[0].1 + 1. && c > positions[0].2 + 2.,
            "{positions:?}"
        );
    }
}
