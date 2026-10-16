use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::path::Path;

#[test]
fn numbered_paragraphs_keep_note_anchors_and_their_advance() {
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    for kind in ["footNote", "endNote"] {
        for white in [false, true] {
            for reflow in [false, true] {
                let mut d = Document::open(
                    &Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("samples/reflow/contour-single.hwpx"),
                )
                .unwrap();
                d.masters.clear();
                let c = d.chars.get_mut("0").unwrap();
                c.attrs.insert("height".into(), "1000".into());
                c.attrs.insert(
                    "textColor".into(),
                    if white { "#FFFFFF" } else { "#000000" }.into(),
                );
                c.children.retain(|n| n.name != "ratio");
                c.children
                    .push(xml(br#"<ratio hangul="80" latin="80"/>"#).unwrap());
                d.paras.insert("anchor".into(), xml(br#"<paraPr><align horizontal="LEFT"/><heading type="NUMBER" idRef="anchor" level="0"/><lineSpacing type="PERCENT" value="160"/></paraPr>"#).unwrap());
                d.header.children.push(xml(br#"<numbering id="anchor" start="1"><paraHead level="1" charPrIDRef="0" textOffsetType="PERCENT" textOffset="50" numFormat="DIGIT">^1.</paraHead></numbering>"#).unwrap());
                d.sections = vec![xml(format!(r#"<sec><p id="anchor" paraPrIDRef="anchor"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="4000" right="4000" top="4000" bottom="4000"/></pagePr></secPr><t>A</t><ctrl><{kind} number="13"><subList/></{kind}></ctrl><t>B</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" textheight="1000" baseline="850" spacing="600" horzsize="32000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
                let mut e = Engine::new(&d, fonts);
                if reflow {
                    e.render_reflow().unwrap();
                } else {
                    e.render().unwrap();
                }
                let texts: Vec<_> = e
                    .pages
                    .iter()
                    .flat_map(|p| &p.ops)
                    .filter_map(|op| match op {
                        Op::Text {
                            text,
                            x,
                            y,
                            size,
                            scale,
                            color,
                            font,
                        } => Some((text.as_str(), *x, *y, *size, *scale, *color, *font)),
                        _ => None,
                    })
                    .collect();
                let marker = texts
                    .iter()
                    .find(|t| t.0 == "13)")
                    .expect("numbering must not suppress a note anchor");
                let a = texts.iter().find(|t| t.0 == "A").unwrap();
                let b = texts.iter().find(|t| t.0 == "B").unwrap();
                assert!((marker.3 - 7.5).abs() < 0.001);
                assert!((a.2 - marker.2 - 2.).abs() < 0.001);
                assert!((marker.4 - 0.8).abs() < 0.001);
                assert_eq!(marker.5, if white { [1.; 3] } else { [0.; 3] });
                let width = e.fonts.width(marker.6, marker.0, marker.3) * marker.4;
                assert!((b.1 - marker.1 - width).abs() < 0.001);
                assert_eq!(texts.iter().filter(|t| t.0 == "13)").count(), 1);
            }
        }
    }
}
