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
#[test]
fn character_boxes_join_runs_and_paint_fill_before_text_without_crossing_neighbors() {
    for label in ["BOX", "gyp"] {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let mut plain = d.chars["0"].clone();
        plain.attrs.insert("height".into(), "2000".into());
        plain.attrs.remove("borderFillIDRef");
        plain.children.retain(|n| n.name != "fontRef");
        plain
            .children
            .push(xml(br#"<fontRef latin="0"/>"#).unwrap());
        d.fonts.insert(("LATIN".into(), "0".into()), "Arial".into());
        let mut boxed = plain.clone();
        boxed.attrs.insert("borderFillIDRef".into(), "box".into());
        d.chars.insert("plain".into(), plain);
        d.chars.insert("box".into(), boxed);
        d.borders.insert("box".into(),xml(br##"<borderFill><leftBorder type="SOLID" width="0.2 mm" color="#999999"/><rightBorder type="SOLID" width="0.2 mm" color="#999999"/><topBorder type="SOLID" width="0.2 mm" color="#999999"/><bottomBorder type="SOLID" width="0.2 mm" color="#999999"/><fillBrush><winBrush faceColor="#F2F2F2"/></fillBrush></borderFill>"##).unwrap());
        d.paras.insert(
            "plain".into(),
            xml(br#"<paraPr><align horizontal="LEFT"/></paraPr>"#).unwrap(),
        );
        d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="plain"><run charPrIDRef="plain"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>A</t></run><run charPrIDRef="box"><t>{first}</t></run><run charPrIDRef="box"><t>{last}</t></run><run charPrIDRef="plain"><t>B</t></run><linesegarray><lineseg textpos="0" vertpos="1000" vertsize="2000" baseline="1700" horzsize="30000"/></linesegarray></p></sec>"#,first=&label[..2],last=&label[2..]).as_bytes()).unwrap()];
        d.masters.clear();
        let original = d.sections.clone();
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let ops = &e.pages[0].ops;
        let fills: Vec<_> = ops
            .iter()
            .enumerate()
            .filter_map(|(i, o)| {
                if let Op::Rect { x, y, w, h, fill } = o {
                    (fill[0] > 0.9).then_some((i, *x, *y, *w, *h))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(fills.len(), 1);
        let f = fills[0];
        let chars: Vec<_> = ops
            .iter()
            .enumerate()
            .filter_map(|(i, o)| {
                if let Op::Text { text, x, y, .. } = o {
                    Some((i, text.as_str(), *x, *y))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            chars.iter().map(|c| c.1).collect::<String>(),
            format!("A{label}B")
        );
        for c in &chars[1..4] {
            assert!(c.0 > f.0);
            assert!(c.2 >= f.1 && c.2 < f.1 + f.3);
            assert!(c.3 > f.2 && c.3 < f.2 + f.4);
        }
        let audit = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(
            audit.border_crossings.is_empty(),
            "{:?}",
            audit.border_crossings
        );
        assert!(audit.text_overlaps.is_empty());
        assert_eq!(original, d.sections);
    }
}
#[test]
fn native_character_border_reference_is_preserved() {
    let d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
    )
    .unwrap();
    let c = &d.chars["14"];
    let border = &d.borders[c.a("borderFillIDRef")];
    assert_eq!(border.child("topBorder").unwrap().a("type"), "SOLID");
    assert_eq!(border.find("winBrush").unwrap().a("faceColor"), "#F2F2F2");
}
