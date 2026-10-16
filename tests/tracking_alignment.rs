use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
    qa,
};
use std::path::Path;
#[test]
fn centered_and_right_text_exclude_final_tracking_for_both_signs() {
    for align in ["CENTER", "RIGHT"] {
        for spacing in [20, -10] {
            let mut d = Document::open(
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
            )
            .unwrap();
            d.paras
                .get_mut("0")
                .unwrap()
                .children
                .iter_mut()
                .find(|n| n.name == "align")
                .unwrap()
                .attrs
                .insert("horizontal".into(), align.into());
            d.chars
                .get_mut("0")
                .unwrap()
                .children
                .retain(|n| n.name != "spacing");
            d.chars
                .get_mut("0")
                .unwrap()
                .children
                .push(xml(format!(r#"<spacing latin="{spacing}"/>"#).as_bytes()).unwrap());
            d.sections=vec![xml(br#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="20000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>AB</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="10000"/></linesegarray></p></sec>"#).unwrap()];
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
            let mut extents = Vec::new();
            for op in &e.pages[0].ops {
                if let Op::Text {
                    x,
                    text,
                    font,
                    size,
                    scale,
                    ..
                } = op
                {
                    extents.push((*x, *x + e.fonts.width(*font, text, *size) * scale));
                }
            }
            assert_eq!(extents.len(), 2);
            let right = extents[1].1;
            if align == "CENTER" {
                assert!(((extents[0].0 + right) / 2. - 100.).abs() < 0.001);
            } else {
                assert!((right - 150.).abs() < 0.001);
            }
            assert!(
                qa::inspect(&e.pages, &e.fonts, &e.report.bounds)
                    .text_overlaps
                    .is_empty()
            );
        }
    }
}
