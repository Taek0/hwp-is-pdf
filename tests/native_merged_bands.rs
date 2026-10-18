use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
    qa,
};
use std::path::Path;
#[test]
fn merged_bands_use_preflight_and_split_without_duplicating_content() {
    for fits in [true, false] {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        d.header.attrs.insert("binaryHwp".into(), "1".into());
        d.chars
            .get_mut("0")
            .unwrap()
            .attrs
            .insert("height".into(), "1000".into());
        d.paras.insert(
            "plain".into(),
            xml(br#"<paraPr><align horizontal="LEFT"/></paraPr>"#).unwrap(),
        );
        let p = |label: String, y: usize| {
            format!(
                r#"<p paraPrIDRef="plain"><run charPrIDRef="0"><t>{label}</t></run><linesegarray><lineseg textpos="0" vertpos="{y}" vertsize="1000" baseline="850" spacing="600" horzsize="8800"/></linesegarray></p>"#
            )
        };
        let cell = |row: usize, col: usize, span: usize, height: usize, body: String| {
            format!(
                r#"<tc header="{}" borderFillIDRef="1"><cellAddr rowAddr="{row}" colAddr="{col}"/><cellSpan rowSpan="{span}" colSpan="1"/><cellSz width="9000" height="{height}"/><subList vertAlign="CENTER">{body}</subList></tc>"#,
                usize::from(row == 0)
            )
        };
        let band_row = if fits { 2000 } else { 9000 };
        let rows = format!(
            "<tr>{}{}</tr><tr>{}{}</tr><tr>{}{}</tr><tr>{}</tr><tr>{}{}</tr>",
            cell(0, 0, 1, 2000, p("H0".into(), 0)),
            cell(0, 1, 1, 2000, p("H1".into(), 0)),
            cell(1, 0, 1, 0, p("SHORT".into(), 0)),
            cell(
                1,
                1,
                1,
                0,
                (0..10)
                    .map(|i| p(format!("LINE{i:02}"), i * 1600))
                    .collect()
            ),
            cell(2, 0, 2, band_row * 2, p("MERGED".into(), 0)),
            cell(2, 1, 1, band_row, p("UPPER".into(), 0)),
            cell(3, 1, 1, band_row, p("LOWER".into(), 0)),
            cell(4, 0, 1, 2000, p("TAIL0".into(), 0)),
            cell(4, 1, 1, 2000, p("TAIL1".into(), 0))
        );
        let height = 19600 + band_row * 2;
        d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="plain"><run charPrIDRef="0"><secPr><pagePr width="22000" height="26000"><margin left="2000" right="2000" top="2000" bottom="2000"/></pagePr></secPr><tbl id="band-probe" binaryHwp="1" pageBreak="TABLE" repeatHeader="1" rowCnt="5" colCnt="2"><sz width="18000" height="{height}"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="100" right="100" top="100" bottom="100"/>{rows}</tbl></run><linesegarray><lineseg textpos="0" vertpos="8000" vertsize="1000" baseline="850" horzsize="18000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
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
        let used = e
            .report
            .warnings
            .iter()
            .any(|w| w.contains("merged row bands paginated"));
        assert!(
            used,
            "both fitting and split bands must use the preflighted path"
        );
        let texts: Vec<String> = e
            .pages
            .iter()
            .map(|p| {
                p.ops
                    .iter()
                    .filter_map(|o| match o {
                        Op::Text { text, .. } | Op::Shaped { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect()
            })
            .collect();
        let all = texts.concat();
        for word in ["SHORT", "MERGED", "UPPER", "LOWER", "TAIL0", "TAIL1"] {
            assert_eq!(all.matches(word).count(), 1, "{fits}: {word}");
        }
        for i in 0..10 {
            assert_eq!(all.matches(&format!("LINE{i:02}")).count(), 1);
        }
        if fits {
            assert_eq!(e.pages.len(), 2);
            assert!(
                texts[1].contains("MERGED")
                    && texts[1].contains("UPPER")
                    && texts[1].contains("LOWER")
            );
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
        assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
        assert!(q.outside_page.is_empty());
        assert_eq!(original, d.sections);
    }
}

#[test]
fn long_spanning_cell_keeps_every_line_once_across_remaining_row_grids() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.header.attrs.insert("binaryHwp".into(), "1".into());
    d.paras.insert(
        "plain".into(),
        xml(br#"<paraPr><align horizontal="LEFT"/></paraPr>"#).unwrap(),
    );
    d.chars
        .get_mut("0")
        .unwrap()
        .attrs
        .insert("height".into(), "1000".into());
    let p = |label: String, y: usize| {
        format!(
            r#"<p paraPrIDRef="plain"><run charPrIDRef="0"><t>{label}</t></run><linesegarray><lineseg textpos="0" vertpos="{y}" vertsize="1000" baseline="850" spacing="600" horzsize="8800"/></linesegarray></p>"#
        )
    };
    let cell = |r: usize, c: usize, span: usize, body: String| {
        format!(
            r#"<tc header="0" borderFillIDRef="1"><cellAddr rowAddr="{r}" colAddr="{c}"/><cellSpan rowSpan="{span}" colSpan="1"/><cellSz width="9000" height="0"/><subList vertAlign="CENTER">{body}</subList></tc>"#
        )
    };
    let left = cell(
        0,
        0,
        2,
        (0..23)
            .map(|i| p(format!("LINE{i:02}"), i * 1600))
            .collect(),
    );
    let upper = cell(0, 1, 1, p("UPPER".into(), 0));
    let lower = cell(1, 1, 1, p("LOWER".into(), 0));
    d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="plain"><run charPrIDRef="0"><secPr><pagePr width="22000" height="22000"><margin left="2000" right="2000" top="2000" bottom="2000"/></pagePr></secPr><tbl id="long-band" binaryHwp="1" pageBreak="TABLE" repeatHeader="0" rowCnt="2" colCnt="2"><sz width="18000" height="0"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="100" right="100" top="100" bottom="100"/><tr>{left}{upper}</tr><tr>{lower}</tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="18000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
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
    assert_eq!(e.pages.len(), 3);
    assert!(
        e.report
            .warnings
            .iter()
            .any(|s| s.contains("merged row bands paginated"))
    );
    let text: String = e
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|o| match o {
            Op::Text { text, .. } | Op::Shaped { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    for i in 0..23 {
        assert_eq!(text.matches(&format!("LINE{i:02}")).count(), 1);
    }
    for label in ["UPPER", "LOWER"] {
        assert_eq!(text.matches(label).count(), 1);
    }
    assert_eq!(d.sections, original);
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(
        q.text_overlaps.is_empty() && q.border_crossings.is_empty() && q.outside_page.is_empty()
    );
}
