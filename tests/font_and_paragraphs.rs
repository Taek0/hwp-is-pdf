use hwp_engine::{
    layout::Engine,
    model::{Document, text, xml},
    pdf::{Fonts, Op},
    qa,
};
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).into()
}
fn fonts() -> &'static std::path::Path {
    std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
#[test]
fn named_faces_select_collection_members_and_real_bold_before_synthesis() {
    let mut f = Fonts::new(fonts());
    let regular = f.family("굴림", false, false).unwrap().unwrap();
    let mono = f.family("굴림체", false, false).unwrap().unwrap();
    assert_ne!(regular, mono);
    assert!(f.list[mono].name.ends_with("#1"));
    let face = ttf_parser::Face::parse(&f.list[mono].bytes, 0).unwrap();
    assert!(
        face.names()
            .into_iter()
            .filter_map(|n| n.to_string())
            .any(|n| n == "GulimChe")
    );
    let native = f.family("함초롬바탕", true, false).unwrap().unwrap();
    assert!(!f.list[native].synthetic_bold);
    assert!(
        ttf_parser::Face::parse(&f.list[native].bytes, 0)
            .unwrap()
            .is_bold()
    );
    let plain = f.family("바탕", false, false).unwrap().unwrap();
    let bold = f.family("바탕", true, false).unwrap().unwrap();
    assert_ne!(plain, bold);
    assert!(f.list[bold].synthetic_bold);
    assert_eq!(f.list[plain].bytes, f.list[bold].bytes);
    assert_eq!(f.family("바탕", true, false).unwrap(), Some(bold));
    assert_eq!(
        f.family("missing-font-family-928371", false, false)
            .unwrap(),
        None
    );
    let arial = f.family("Arial", false, false).unwrap().unwrap();
    assert_eq!(
        ttf_parser::Face::parse(&f.list[arial].bytes, 0)
            .unwrap()
            .width()
            .to_number(),
        5,
        "Arial Narrow must not win the shared legacy family name"
    );
}
#[test]
fn duplicate_border_definitions_join_without_a_rule_through_text() {
    let mut d = Document::open(&root().join("samples/reflow/contour-single.hwpx")).unwrap();
    for id in ["100", "101"] {
        d.borders.insert(id.into(),xml(format!(r##"<borderFill id="{id}"><leftBorder type="SOLID" width="0.1 mm" color="#000000"/><rightBorder type="SOLID" width="0.1 mm" color="#000000"/><topBorder type="SOLID" width="0.1 mm" color="#000000"/><bottomBorder type="SOLID" width="0.1 mm" color="#000000"/></borderFill>"##).as_bytes()).unwrap());
    }
    for (id, left) in [("100", 2000), ("101", 1000)] {
        d.paras.insert(id.into(),xml(format!(r#"<paraPr><margin><left value="{left}"/></margin><border borderFillIDRef="{id}" connect="1" ignoreMargin="0" offsetLeft="200" offsetRight="200" offsetTop="200" offsetBottom="200"/></paraPr>"#).as_bytes()).unwrap());
    }
    d.sections=vec![xml(br#"<sec><p id="first" paraPrIDRef="100"><run charPrIDRef="0"><secPr><pagePr width="40000" height="40000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>FIRST</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1400" baseline="1100" horzpos="2000" horzsize="28000"/></linesegarray></p><p id="second" paraPrIDRef="101"><run charPrIDRef="0"><t>SECOND</t></run><linesegarray><lineseg textpos="0" vertpos="2000" vertsize="1400" baseline="1100" horzpos="1000" horzsize="29000"/></linesegarray></p></sec>"#).unwrap()];
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let horizontal: Vec<_> = e.pages[0]
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Line { x, y, x2, y2, .. } = o {
                ((y - y2).abs() < 0.01).then_some((*x, *y, *x2))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(horizontal.len(), 2);
    assert_eq!(horizontal[0], (48., 48., 352.));
    assert_eq!(horizontal[1], (48., 86., 352.));
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.border_crossings.is_empty());
}
#[test]
fn native_paragraph_terminator_does_not_add_a_blank_reflow_line() {
    let d = Document::open(&root().join("samples/features/fields.hwp")).unwrap();
    let p = d
        .sections
        .iter()
        .flat_map(|s| s.children_named("p"))
        .find(|p| !text(p).trim().is_empty())
        .unwrap();
    assert!(p.find("hwpParagraphEnd").is_some());
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    assert_eq!(
        e.report
            .bounds
            .iter()
            .filter(|b| b.kind == "reflow-line" && b.id == p.a("id"))
            .count(),
        1
    );
}

#[test]
fn fixed_half_em_spaces_and_missing_bold_family_are_preserved() {
    for use_font in [0, 1] {
        let mut d = Document::open(&root().join("samples/reflow/contour-single.hwpx")).unwrap();
        d.fonts.insert(
            ("LATIN".into(), "0".into()),
            "Unavailable Test Family B".into(),
        );
        d.paras.insert(
            "0".into(),
            xml(br#"<paraPr><align horizontal="LEFT"/></paraPr>"#).unwrap(),
        );
        d.chars.insert("0".into(),xml(format!(r##"<charPr height="1200" textColor="#000000" useFontSpace="{use_font}"><fontRef latin="0" hangul="0"/></charPr>"##).as_bytes()).unwrap());
        d.sections=vec![xml(br#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="40000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>A A</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1600" baseline="1200" horzsize="30000"/></linesegarray></p></sec>"#).unwrap()];
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let aa: Vec<_> = e.pages[0]
            .ops
            .iter()
            .filter_map(|op| {
                if let Op::Text { text, x, font, .. } = op {
                    (text == "A").then_some((*x, *font))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(aa.len(), 2);
        let font = aa[0].1;
        assert!(
            ttf_parser::Face::parse(&e.fonts.list[font].bytes, 0)
                .unwrap()
                .is_bold()
        );
        let space_x = e.pages[0]
            .ops
            .iter()
            .find_map(|op| {
                if let Op::Text { text, x, .. } = op {
                    (text == " ").then_some(*x)
                } else {
                    None
                }
            })
            .unwrap();
        let space = aa[1].0 - space_x;
        let expected = if use_font == 0 {
            6.
        } else {
            e.fonts.width(font, " ", 12.)
        };
        assert!(
            (space - expected).abs() < 0.001,
            "font-space={use_font}: got {space}, expected {expected}"
        );
    }
}
