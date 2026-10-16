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
fn doc() -> Document {
    Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/text-style/emphasis-marks.hwpx"),
    )
    .unwrap()
}

#[test]
fn native_binary_marks_render_without_changing_logical_text() {
    let d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/text-style/emphasis-marks.hwp"),
    )
    .unwrap();
    let mut plain = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/text-style/emphasis-marks.hwp"),
    )
    .unwrap();
    for style in plain.chars.values_mut() {
        style.attrs.remove("symMark");
    }
    let mut marked = Engine::new(&d, fonts());
    marked.render().unwrap();
    let mut baseline = Engine::new(&plain, fonts());
    baseline.render().unwrap();
    let content = |e: &Engine| {
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>()
    };
    assert_eq!(content(&marked), content(&baseline));
    assert!(
        marked
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .any(|o| matches!(o, Op::Path { closed: true, .. }))
    );
    assert!(
        qa::inspect(&marked.pages, &marked.fonts, &marked.report.bounds)
            .outside_page
            .is_empty()
    );
}

#[test]
fn all_named_marks_are_vectors_without_duplicating_text_or_marking_spaces() {
    let d = doc();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let ops = &e.pages[0].ops;
    assert_eq!(
        ops.iter().filter(|o| matches!(o, Op::Path { .. })).count(),
        44
    );
    assert_eq!(
        ops.iter()
            .filter(|o| matches!(o, Op::Bezier { .. }))
            .count(),
        8
    );
    assert_eq!(
        ops.iter()
            .filter(|o| matches!(o, Op::TransformBegin { .. }))
            .count(),
        48
    );
    let text = ops
        .iter()
        .filter_map(|o| match o {
            Op::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(text.matches("한글 Aa").count(), 13);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.outside_page.is_empty());
}

#[test]
fn shaped_clusters_keep_logical_text_and_emphasis() {
    let mut d = doc();
    d.sections=vec![xml("<sec><p paraPrIDRef=\"0\"><run charPrIDRef=\"1\"><secPr><pagePr width=\"40000\" height=\"30000\"><margin left=\"5000\" right=\"5000\" top=\"5000\" bottom=\"5000\"/></pagePr></secPr><t>אבג</t></run><linesegarray><lineseg textpos=\"0\" vertpos=\"0\" vertsize=\"3000\" baseline=\"2200\" horzsize=\"30000\"/></linesegarray></p></sec>".as_bytes()).unwrap()];
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(
        e.pages[0]
            .ops
            .iter()
            .filter(|o| matches!(o, Op::Path { .. }))
            .count(),
        3
    );
    assert!(
        e.pages[0]
            .ops
            .iter()
            .any(|o| matches!(o,Op::ActualTextBegin{text} if text=="אבג"))
    );
    assert!(
        qa::inspect(&e.pages, &e.fonts, &e.report.bounds)
            .outside_page
            .is_empty()
    );
}

#[test]
fn marks_stay_with_glyphs_in_multipage_table_fragments() {
    let mut d = doc();
    let rows = |col: usize| {
        (0..20).map(|i|format!(r#"<p paraPrIDRef="0"><run charPrIDRef="1"><t>C{col}R{i:02}</t></run><linesegarray><lineseg textpos="0" vertpos="{}" vertsize="3000" baseline="2300" horzsize="14000"/></linesegarray></p>"#,i*3000)).collect::<String>()
    };
    let cells=(0..2).map(|col|format!(r#"<tc borderFillIDRef="0"><cellAddr rowAddr="0" colAddr="{col}"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="15000" height="62000"/><subList>{}</subList></tc>"#,rows(col))).collect::<String>();
    d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="label"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><tbl id="marked-table" binaryHwp="1" pageBreak="TABLE" rowCnt="1" colCnt="2"><sz width="30000" height="62000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="500" right="500" top="1000" bottom="1000"/><tr>{cells}</tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="30000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert!(e.pages.len() > 1);
    let text = e
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|o| match o {
            Op::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    for col in 0..2 {
        for i in 0..20 {
            assert_eq!(text.matches(&format!("C{col}R{i:02}")).count(), 1);
        }
    }
    assert_eq!(
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter(|o| matches!(o, Op::Path { closed: true, .. }))
            .count(),
        200
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
