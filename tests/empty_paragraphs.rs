use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
    qa,
};
use std::path::Path;

#[test]
fn empty_paragraph_advance_uses_its_declared_character_size() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    for (id, size) in [("small", 800), ("large", 2000)] {
        let mut c = d.chars["0"].clone();
        c.attrs.insert("height".into(), size.to_string());
        d.chars.insert(id.into(), c);
    }
    d.sections=vec![xml(br#"<sec><p id="before" paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="50000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>BEFORE</t></run></p><p id="small" paraPrIDRef="0"><run charPrIDRef="small"><t/></run></p><p id="large" paraPrIDRef="0"><run charPrIDRef="large"><t/></run></p><p id="after" paraPrIDRef="0"><run charPrIDRef="0"><t>AFTER</t></run></p></sec>"#).unwrap()];
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
    e.render_reflow().unwrap();
    let y = |id| {
        e.report
            .bounds
            .iter()
            .find(|b| b.id == id && b.kind == "line")
            .unwrap()
            .y
    };
    assert!((y("large") - y("small") - 12.8).abs() < 0.01);
    assert!((y("after") - y("large") - 32.).abs() < 0.01);
    let text = e
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|o| match o {
            Op::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(text, "BEFOREAFTER");
    assert_eq!(original, d.sections);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
