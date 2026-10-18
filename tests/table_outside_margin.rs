use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
};
use std::path::Path;
#[test]
fn continued_floating_table_reserves_bottom_outside_margin_before_body() {
    for same_paragraph in [false, true] {
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
        let paragraph = |text: String, y: usize| {
            format!(
                r#"<p paraPrIDRef="plain"><run charPrIDRef="0"><t>{text}</t></run><linesegarray><lineseg textpos="0" vertpos="{y}" vertsize="1000" spacing="600" baseline="850" horzsize="5800"/></linesegarray></p>"#
            )
        };
        let cell = |row: usize, height: usize, body: String| {
            format!(
                r#"<tc header="{}"><cellAddr rowAddr="{row}" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="6000" height="{height}"/><subList vertAlign="TOP">{body}</subList></tc>"#,
                usize::from(row == 0)
            )
        };
        let header = cell(0, 2000, paragraph("HEAD".into(), 0));
        let body = cell(
            1,
            0,
            (0..10)
                .map(|i| paragraph(format!("ROW{i:02}"), i * 1600))
                .collect(),
        );
        let root_id = if same_paragraph { "after" } else { "anchor" };
        let inline_after = if same_paragraph {
            r#"<t>AFTER</t>"#
        } else {
            ""
        };
        let following = if same_paragraph {
            ""
        } else {
            r#"<p id="after" paraPrIDRef="plain"><run charPrIDRef="0"><t>AFTER</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="6000"/></linesegarray></p>"#
        };
        d.sections=vec![xml(format!(r#"<sec><p id="{root_id}" paraPrIDRef="plain"><run charPrIDRef="0"><secPr><pagePr width="10000" height="24000"><margin left="2000" right="2000" top="2000" bottom="2000"/></pagePr></secPr><tbl id="margin-probe" binaryHwp="1" pageBreak="TABLE" repeatHeader="1" rowCnt="2" colCnt="1"><sz width="6000" height="17600"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><outMargin bottom="1500"/><inMargin left="100" right="100" top="100" bottom="100"/><tr>{header}</tr><tr>{body}</tr></tbl>{inline_after}</run><linesegarray><lineseg textpos="0" vertpos="6000" vertsize="1000" baseline="850" horzsize="6000"/></linesegarray></p>{following}</sec>"#).as_bytes()).unwrap()];
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
        assert_eq!(e.pages.len(), 2);
        let table = e
            .report
            .bounds
            .iter()
            .rfind(|b| b.kind == "tbl" && b.id == "margin-probe")
            .unwrap();
        let after = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == "after")
            .unwrap();
        assert_eq!(table.page, after.page);
        assert!(
            (after.y - table.y - table.h - 15.).abs() < 0.01,
            "table bottom={} following={} outside=15",
            table.y + table.h,
            after.y
        );
    }
}
