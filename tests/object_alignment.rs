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
fn document(body: String) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.sections = vec![xml(body.as_bytes()).unwrap()];
    d.masters.clear();
    d
}
const PAGE: &str = r#"<secPr><pagePr width="40000" height="40000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr>"#;
#[test]
fn fresh_empty_block_picture_anchor_requires_the_complete_reserved_gap() {
    for (cache_y, expected_y) in [(10000, 50.), (15000, 200.)] {
        let mut d = document(format!(
            r#"<sec><p columnStart="1" paraPrIDRef="0"><run charPrIDRef="0">{PAGE}<pic id="probe" binaryHwp="1" textWrap="TOP_AND_BOTTOM"><sz width="10000" height="10000"/><pos treatAsChar="0" vertRelTo="PARA" vertAlign="TOP" horzRelTo="PARA"/><img binaryItemIDRef="probe"/></pic></run><linesegarray><lineseg textpos="0" vertpos="{cache_y}" vertsize="1000" textheight="1000" baseline="850" horzsize="30000"/></linesegarray></p></sec>"#
        ));
        d.header.attrs.insert("binaryHwp".into(), "1".into());
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([0, 0, 255])))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        d.binaries.insert("probe".into(), bytes.into_inner());
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let image = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "pic" && b.id == "probe")
            .unwrap();
        assert!((image.y - expected_y).abs() < 0.01, "{}", image.y);
    }
}
#[test]
fn hwpx_inline_table_cache_reserves_its_complete_outside_box() {
    let d = document(format!(
        r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0">{PAGE}<tbl id="probe" rowCnt="1" colCnt="1"><sz width="10000" height="4000"/><pos treatAsChar="1"/><outMargin top="200" bottom="600"/><tr><tc borderFillIDRef="1"><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="10000" height="4000"/><subList><p paraPrIDRef="0"><run charPrIDRef="0"><t>X</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="10000"/></linesegarray></p></subList></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="4800" textheight="4800" baseline="4080" horzsize="30000"/></linesegarray></p></sec>"#
    ));
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let table = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "tbl" && b.id == "probe")
        .unwrap();
    assert!((table.y - 52.).abs() < 0.01, "{}", table.y);
}
#[test]
fn native_inline_picture_baseline_includes_margins_and_vertical_captions() {
    for side in ["NONE", "TOP", "BOTTOM"] {
        let caption = if side == "NONE" {
            String::new()
        } else {
            format!(
                r#"<caption side="{side}" gap="600" width="10000"><subList><p paraPrIDRef="0"><run charPrIDRef="0"><t>CAPTION</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" horzsize="10000"/></linesegarray></p></subList></caption>"#
            )
        };
        let height = if side == "NONE" { 11200 } else { 12800 };
        let baseline = height as f64 * 0.85;
        let mut d = document(format!(
            r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0">{PAGE}<pic id="picture" binaryHwp="1"><sz width="10000" height="10000"/><pos treatAsChar="1"/><outMargin top="400" bottom="800"/><img binaryItemIDRef="probe"/>{caption}</pic></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="{height}" baseline="{baseline}" horzsize="30000"/></linesegarray></p></sec>"#
        ));
        let mut png = std::io::Cursor::new(vec![]);
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([0, 0, 255])))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        d.binaries.insert("probe".into(), png.into_inner());
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let b = e.report.bounds.iter().find(|b| b.id == "picture").unwrap();
        assert!(
            (b.y - if side == "TOP" { 70. } else { 54. }).abs() < 0.001,
            "{side}: {}",
            b.y
        );
    }
}
#[test]
fn native_inline_table_baseline_includes_asymmetric_outside_margins() {
    let d = document(format!(
        r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0">{PAGE}<tbl id="native" binaryHwp="1" rowCnt="1" colCnt="1" pageBreak="NONE"><sz width="30000" height="10000"/><pos treatAsChar="1"/><outMargin top="400" bottom="800"/><tr><tc borderFillIDRef="1"><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="30000" height="10000"/><subList><p paraPrIDRef="0"><run charPrIDRef="0"><t>CELL</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="30000"/></linesegarray></p></subList></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="11200" baseline="9520" horzsize="30000"/></linesegarray></p></sec>"#
    ));
    let original = d.sections.clone();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let b = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "tbl" && b.id == "native")
        .unwrap();
    assert!((b.y - 54.).abs() < 0.001);
    assert_eq!(original, d.sections);
    assert!(
        qa::inspect(&e.pages, &e.fonts, &e.report.bounds)
            .text_overlaps
            .is_empty()
    );
}
#[test]
fn text_box_vertical_alignment_uses_content_height_and_both_margins() {
    for (align, expected) in [("TOP", 60.), ("CENTER", 89.), ("BOTTOM", 118.)] {
        let d = document(format!(
            r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0">{PAGE}<rect id="box"><sz width="20000" height="10000"/><pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><drawText><textMargin top="1000" bottom="2000" left="100" right="100"/><subList vertAlign="{align}"><p id="inside" paraPrIDRef="0"><run charPrIDRef="0"><t>TEXT</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="19800"/></linesegarray></p></subList></drawText></rect></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="30000"/></linesegarray></p></sec>"#
        ));
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let b = e
            .report
            .bounds
            .iter()
            .find(|b| b.kind == "line" && b.id == "inside")
            .unwrap();
        assert!((b.y - expected).abs() < 0.001, "{align}: {}", b.y);
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty());
        assert!(q.border_crossings.is_empty());
        assert!(q.outside_page.is_empty());
    }
}
#[test]
fn supplied_title_baselines_match_reference_positions() {
    for (name, page, label, size, baseline) in [
        ("복학원서.hwp", 0, "복", 24., 86.64),
        ("21_언어_기출_편집가능본.hwp", 7, "언", 25., 127.189),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(name);
        if !path.exists() {
            continue;
        }
        let d = Document::open(&path).unwrap();
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let y = e.pages[page]
            .ops
            .iter()
            .find_map(|op| match op {
                Op::Text {
                    text, size: s, y, ..
                } if text == label && (*s - size).abs() < 0.01 => Some(*y),
                _ => None,
            })
            .unwrap();
        assert!((y - baseline).abs() < 0.1, "{name}: {y}");
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty());
        assert!(q.border_crossings.is_empty());
    }
}

#[test]
fn native_center_baseline_aligns_equal_boxes_with_different_outside_margins() {
    let cell = |id: &str, margin: i32| {
        format!(
            r#"<tbl id="{id}" binaryHwp="1" rowCnt="1" colCnt="1" pageBreak="NONE"><sz width="10000" height="2448"/><pos treatAsChar="1"/><outMargin top="{margin}" bottom="{margin}"/><tr><tc><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="10000" height="2448"/><subList><p paraPrIDRef="0"><run charPrIDRef="0"><t>CELL</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="10000"/></linesegarray></p></subList></tc></tr></tbl>"#
        )
    };
    let body = format!(
        r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0">{PAGE}{}{}</run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="3014" textheight="3014" baseline="1507" horzsize="30000"/></linesegarray></p></sec>"#,
        cell("with-margin", 283),
        cell("no-margin", 0)
    );
    let d = document(body);
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let a = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "with-margin")
        .unwrap();
    let b = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "no-margin")
        .unwrap();
    assert!((a.y - 52.83).abs() < 0.01);
    assert!((a.y - b.y).abs() < 0.01);
}

#[test]
fn native_inline_rectangle_baseline_includes_its_bottom_margin() {
    let d = document(format!(
        r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0">{PAGE}<rect id="box" binaryHwp="1"><sz width="10000" height="3610"/><pos treatAsChar="1"/><outMargin bottom="283"/><lineShape style="SOLID"/></rect></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="3893" textheight="3893" baseline="3309" horzsize="30000"/></linesegarray></p></sec>"#
    ));
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let b = e.report.bounds.iter().find(|b| b.id == "box").unwrap();
    assert!((b.y - 50.).abs() < 0.01);
}
#[test]
fn supplied_law_input_boxes_share_the_printed_baseline() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("21_언어_기출_편집가능본.hwp");
    if !input.exists() {
        return;
    }
    let d = Document::open(&input).unwrap();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 15);
    for (id, y) in [
        ("hwp-1c0c", 200.04),
        ("hwp-1d72", 200.04),
        ("hwp-18fd", 188.4),
    ] {
        let b = e.report.bounds.iter().find(|b| b.id == id).unwrap();
        assert!((b.y - y).abs() < 0.15, "{id}: {} != {y}", b.y);
    }
}
