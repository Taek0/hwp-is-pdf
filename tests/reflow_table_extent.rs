use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
};
use std::path::Path;

#[test]
fn section_heading_table_spans_columns_and_uses_ink_plus_outside_margins() {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.masters.clear();
    d.chars
        .get_mut("0")
        .unwrap()
        .attrs
        .insert("height".into(), "1000".into());
    d.paras.insert("plain".into(), xml(br#"<paraPr><align horizontal="LEFT"/><lineSpacing type="PERCENT" value="160"/></paraPr>"#).unwrap());
    d.sections = vec![xml(br#"<sec><p id="heading" paraPrIDRef="plain"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="4000" right="4000" top="4000" bottom="4000"/></pagePr></secPr><ctrl><colPr colCount="2" sameGap="2000"/></ctrl><tbl id="wide" rowCnt="1" colCnt="1" textWrap="TOP_AND_BOTTOM"><sz width="30000" height="1400"/><pos treatAsChar="0" horzRelTo="COLUMN" vertRelTo="PARA" horzAlign="LEFT" vertAlign="TOP"/><outMargin left="300" right="300" top="400" bottom="500"/><inMargin left="200" right="200" top="200" bottom="200"/><tr><tc hasMargin="0"><cellAddr colAddr="0" rowAddr="0"/><cellSpan colSpan="1" rowSpan="1"/><cellSz width="30000" height="1400"/><subList><p id="cell" paraPrIDRef="plain"><run charPrIDRef="0"><t>HEADING</t></run></p></subList></tc></tr></tbl></run></p><p id="following" paraPrIDRef="plain"><run charPrIDRef="0"><t>AFTER</t></run></p></sec>"#).unwrap()];
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    fn extend(n: &mut hwp_engine::model::Node) {
        if n.name == "#text" && n.text == "AFTER" {
            n.text = "AFTER ".repeat(110);
        }
        for child in &mut n.children {
            extend(child);
        }
    }
    extend(&mut d.sections[0]);
    let mut e = Engine::new(&d, fonts);
    e.render_reflow().unwrap();
    let table = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "tbl" && b.id == "wide")
        .unwrap();
    assert!((table.w - 300.).abs() < 0.01);
    assert!((table.x - 43.).abs() < 0.01 && (table.y - 44.).abs() < 0.01);
    assert!(table.h >= 14. && table.h < 17., "height {}", table.h);
    let after = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "line" && b.id == "following")
        .unwrap();
    assert!(after.y >= table.y + table.h + 5. - 0.01);
    let right = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "reflow-line" && b.id == "following" && b.page == 1 && b.x > 200.)
        .unwrap();
    assert!(right.y >= table.y + table.h + 5. - 0.01);
}
