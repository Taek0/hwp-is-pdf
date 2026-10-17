use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
    qa,
};
use std::path::Path;
#[test]
fn latin_tracking_uses_half_em_and_follows_horizontal_scale() {
    for spacing in [20, -5] {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let c = d.chars.get_mut("0").unwrap();
        c.attrs.insert("height".into(), "2000".into());
        c.attrs.insert("useKerning".into(), "0".into());
        c.children
            .retain(|n| !matches!(n.name.as_str(), "spacing" | "ratio"));
        c.children.push(
            xml(format!(r#"<spacing latin="{spacing}" hangul="{spacing}"/>"#).as_bytes()).unwrap(),
        );
        c.children
            .push(xml(br#"<ratio latin="80" hangul="80"/>"#).unwrap());
        d.paras.insert(
            "probe".into(),
            xml(br#"<paraPr><align horizontal="LEFT"/></paraPr>"#).unwrap(),
        );
        d.sections=vec![xml(r#"<sec><p paraPrIDRef="probe"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>nn가가</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="2000" baseline="1700" horzsize="30000"/></linesegarray></p></sec>"#.as_bytes()).unwrap()];
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
        let chars: Vec<_> = e.pages[0]
            .ops
            .iter()
            .filter_map(|op| {
                if let Op::Text {
                    x,
                    text,
                    font,
                    size,
                    scale,
                    ..
                } = op
                {
                    Some((*x, text, *font, *size, *scale))
                } else {
                    None
                }
            })
            .collect();
        for (i, factor) in [(0, 0.5), (2, 1.)] {
            let g = &chars[i];
            let natural = e.fonts.width(g.2, g.1, g.3) * g.4;
            let tracking = 20. * 0.8 * spacing as f64 / 100. * factor;
            assert!(
                (chars[i + 1].0 - g.0 - natural - tracking).abs() < 0.001,
                "spacing={spacing} index={i}: advance={} natural={natural} tracking={tracking}",
                chars[i + 1].0 - g.0
            );
        }
    }
}
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
