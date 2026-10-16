use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
};
use std::path::Path;

#[test]
fn expanded_spanning_table_reserves_bottom_margin_in_both_columns_only_on_its_page() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    let text = |id: &str, y: usize, page: usize| {
        format!(
            r#"<p id="{id}" paraPrIDRef="0" pageBreak="{page}"><run charPrIDRef="0"><t>{id}</t></run><linesegarray><lineseg textpos="0" vertpos="{y}" vertsize="1000" baseline="850" spacing="600" horzsize="14000"/></linesegarray></p>"#
        )
    };
    d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="40000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><ctrl><colPr colCount="2" sameGap="2000"><colLine type="SOLID"/></colPr></ctrl><tbl id="wide" rowCnt="1" colCnt="1" pageBreak="NONE"><sz width="30000" height="2000"/><pos treatAsChar="0" horzRelTo="COLUMN" vertRelTo="PARA" horzAlign="LEFT" vertAlign="TOP"/><outMargin top="200" bottom="300"/><tr><tc borderFillIDRef="1"><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="30000" height="6000"/><subList/></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="14000"/></linesegarray></p>{}{}{}{}</sec>"#,text("LEFT",2500,0),text("LATER",4500,0),text("RIGHT",2500,0),text("NEXT_PAGE",0,1)).as_bytes()).unwrap()];
    d.masters.clear();
    let original = d.sections.clone();
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    for (id, page, x, y) in [
        ("LEFT", 1, 50., 115.),
        ("RIGHT", 1, 210., 115.),
        ("NEXT_PAGE", 2, 50., 50.),
    ] {
        let b = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == id)
            .unwrap();
        assert_eq!(b.page, page);
        assert!(
            (b.x - x).abs() < 0.001 && (b.y - y).abs() < 0.001,
            "{id}: {},{}",
            b.x,
            b.y
        );
    }
    assert_eq!(d.sections, original);
    assert!(e.pages[0].ops.iter().any(|op| matches!(op,
        hwp_engine::pdf::Op::Line {x,y,x2,y2,..}
        if (*x-200.).abs()<0.001 && (*x2-200.).abs()<0.001
            && (*y-115.).abs()<0.001 && (*y2-350.).abs()<0.001)));
}
