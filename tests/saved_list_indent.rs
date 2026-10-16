use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::path::Path;

#[test]
fn saved_auto_indent_reserves_scaled_label_once_on_every_line() {
    for (auto, continuation) in [(true, 0), (false, 0), (true, 6000)] {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        d.masters.clear();
        let c = d.chars.get_mut("0").unwrap();
        c.attrs.insert("height".into(), "1000".into());
        c.children.retain(|n| n.name != "ratio");
        c.children
            .push(xml(br#"<ratio hangul="80" latin="80"/>"#).unwrap());
        d.paras.insert("list".into(), xml(br#"<paraPr><align horizontal="LEFT"/><heading type="NUMBER" idRef="test" level="0"/></paraPr>"#).unwrap());
        d.header.children.push(xml(format!(r#"<numbering id="test" start="1"><paraHead level="1" charPrIDRef="0" autoIndent="{}" textOffsetType="PERCENT" textOffset="50" numFormat="DIGIT">^1.</paraHead></numbering>"#, u8::from(auto)).as_bytes()).unwrap());
        d.sections = vec![xml(format!(r#"<sec><p id="test" paraPrIDRef="list"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="4000" right="4000" top="4000" bottom="4000"/></pagePr></secPr><t>ABC</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" spacing="600" horzpos="0" horzsize="32000"/><lineseg textpos="10" vertpos="1600" vertsize="1000" baseline="850" spacing="600" horzpos="{continuation}" horzsize="26000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
        let fonts = Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        });
        let mut e = Engine::new(&d, fonts);
        e.render().unwrap();
        let ops: Vec<_> = e.pages.iter().flat_map(|p| &p.ops).collect();
        let text_x = |wanted| {
            ops.iter()
                .find_map(|o| match o {
                    Op::Text { text, x, .. } if text == wanted => Some(*x),
                    _ => None,
                })
                .unwrap()
        };
        let label = ops
            .iter()
            .find_map(|o| match o {
                Op::Text {
                    text,
                    font,
                    size,
                    scale,
                    ..
                } if text == "1." => Some((*font, *size, *scale)),
                _ => None,
            })
            .unwrap();
        let inset = e.fonts.width(label.0, "1.", label.1) * label.2 + 5.;
        assert!((text_x("A") - 40. - inset).abs() < 0.001);
        let expected = if continuation > 0 {
            100.
        } else if auto {
            40. + inset
        } else {
            40.
        };
        assert!((text_x("C") - expected).abs() < 0.001);
        assert_eq!(
            ops.iter()
                .filter(|o| matches!(o, Op::Text {text,..} if text=="1."))
                .count(),
            1
        );
    }
}
