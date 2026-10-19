use hwp_engine::{
    graphics,
    layout::Engine,
    model::{Document, xml},
    pdf::{Op, Page},
};
use std::path::Path;
fn fonts() -> &'static Path {
    Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn document(modes: &[&str]) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.masters.clear();
    d.sections=modes.iter().map(|m|xml(format!(r#"<sec><p id="p" paraPrIDRef="0"><run charPrIDRef="0"><secPr textDirection="{m}"><pagePr width="40000" height="30000"><margin left="4000" right="4000" top="4000" bottom="4000"/></pagePr></secPr><t>ABC 가나다</t></run></p></sec>"#).as_bytes()).unwrap()).collect();
    d
}
fn points(page: &Page) -> Vec<(String, (f64, f64))> {
    let mut result = Vec::new();
    let mut stack = Vec::new();
    let mut matrix = graphics::IDENTITY;
    for o in &page.ops {
        match o {
            Op::TransformBegin { matrix: m } => {
                stack.push(matrix);
                matrix = graphics::compose(matrix, *m);
            }
            Op::TransformEnd => matrix = stack.pop().unwrap(),
            Op::Text { x, y, text, .. } => {
                result.push((text.clone(), graphics::point(matrix, *x, *y)))
            }
            _ => {}
        }
    }
    assert!(stack.is_empty());
    result
}
#[test]
fn mixed_sections_preserve_isolated_layout_and_reset_direction_specific_glyph_metrics() {
    let modes = ["HORIZONTAL", "VERTICAL", "VERTICALALL", "HORIZONTAL"];
    let d = document(&modes);
    let original = d.sections.clone();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(d.sections, original);
    assert_eq!(e.pages.len(), 4);
    for (i, mode) in modes.iter().enumerate() {
        let one = document(&[mode]);
        let mut isolated = Engine::new(&one, fonts());
        isolated.render().unwrap();
        assert_eq!((e.pages[i].width, e.pages[i].height), (400., 300.));
        let a = points(&e.pages[i]);
        let b = points(&isolated.pages[0]);
        assert_eq!(a.len(), b.len());
        for ((ta, pa), (tb, pb)) in a.iter().zip(&b) {
            assert_eq!(ta, tb);
            assert!(
                (pa.0 - pb.0).abs() < 0.001 && (pa.1 - pb.1).abs() < 0.001,
                "{mode} {ta}: {pa:?}/{pb:?}"
            );
        }
    }
}

#[test]
fn mixed_sections_share_page_numbers_and_honor_explicit_restarts() {
    let mut d = document(&["HORIZONTAL", "VERTICALALL", "HORIZONTAL"]);
    for section in &mut d.sections {
        section.children[0].children[0]
            .children
            .push(xml(br#"<ctrl><pageNumPos position="5" format="0"/></ctrl>"#).unwrap());
    }
    d.sections[2].children[0].children[0].children[0]
        .children
        .push(xml(br#"<startNum page="7"/>"#).unwrap());
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 3);
    for (page, label) in e.pages.iter().zip(["1", "2", "7"]) {
        assert!(
            page.ops
                .iter()
                .any(|o| matches!(o,Op::Text{text,..} if text==label))
        );
    }
}
