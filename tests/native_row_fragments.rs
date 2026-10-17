use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
    qa,
};
use std::path::Path;
#[test]
fn split_rows_repeat_padding_and_keep_short_centered_cells_in_first_fragment() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    let paragraph = |label: &str, y: usize| {
        format!(
            r#"<p paraPrIDRef="plain"><run charPrIDRef="0"><t>{label}</t></run><linesegarray><lineseg textpos="0" vertpos="{y}" vertsize="1000" baseline="850" horzsize="17000" spacing="600"/></linesegarray></p>"#
        )
    };
    let cell = |row: usize, col: usize, content: String| {
        format!(
            r#"<tc borderFillIDRef="1" header="{}"><cellAddr rowAddr="{row}" colAddr="{col}"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="18000" height="2000"/><subList vertAlign="CENTER">{content}</subList></tc>"#,
            usize::from(row == 0)
        )
    };
    let heads = format!(
        "{}{}",
        cell(0, 0, paragraph("HEADL", 0)),
        cell(0, 1, paragraph("HEADR", 0))
    );
    let body = format!(
        "{}{}",
        cell(1, 0, paragraph("ONLY", 0)),
        cell(
            1,
            1,
            (0..10)
                .map(|i| paragraph(&format!("LINE{i:02}"), i * 1600))
                .collect()
        )
    );
    d.paras.insert(
        "plain".into(),
        xml(br#"<paraPr><align horizontal="LEFT"/></paraPr>"#).unwrap(),
    );
    d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="plain"><run charPrIDRef="0"><secPr><pagePr width="40000" height="26000"><margin left="2000" right="2000" top="2000" bottom="2000"/></pagePr></secPr><tbl id="fragment" binaryHwp="1" pageBreak="TABLE" repeatHeader="1" rowCnt="2" colCnt="2"><sz width="36000" height="20000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="500" right="500" top="1000" bottom="1000"/><tr>{heads}</tr><tr>{body}</tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="11000" vertsize="1000" baseline="850" horzsize="36000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
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
    e.render().unwrap();
    assert_eq!(e.pages.len(), 2);
    let texts: Vec<String> = e
        .pages
        .iter()
        .map(|p| {
            p.ops
                .iter()
                .filter_map(|o| {
                    if let Op::Text { text, .. } = o {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    assert!(texts[0].contains("ONLY"));
    assert!(!texts[1].contains("ONLY"));
    for i in 0..10 {
        assert_eq!(texts.concat().matches(&format!("LINE{i:02}")).count(), 1);
    }
    for t in &texts {
        assert!(t.contains("HEADL") && t.contains("HEADR"));
    }
    assert_eq!(original, d.sections);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(q.outside_page.is_empty());
    assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
}

#[test]
fn text_after_nested_top_bottom_table_starts_below_its_bottom_border() {
    let d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/tables-notes/inner-table-01.hwp"),
    )
    .unwrap();
    let mut e = Engine::new(
        &d,
        Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    assert_eq!(e.pages.len(), 3);
    let last = e
        .report
        .bounds
        .iter()
        .rfind(|b| b.kind == "tbl" && b.page == 3)
        .unwrap();
    let y = e.pages[2]
        .ops
        .iter()
        .find_map(|o| match o {
            Op::Text { text, y, .. } if text == "끝" => Some(*y),
            _ => None,
        })
        .unwrap();
    assert!(
        y > last.y + last.h,
        "following baseline {y}, table bottom {}",
        last.y + last.h
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.border_crossings.is_empty());
    assert!(q.text_overlaps.is_empty());
}
