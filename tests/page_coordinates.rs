use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::path::Path;

fn fonts() -> &'static Path {
    Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn document(extra: &str, objects: &str, cached: bool) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    let cache = if cached {
        r#"<linesegarray><lineseg textpos="0" vertpos="2000" vertsize="1000" baseline="850" horzpos="0" horzsize="30000"/></linesegarray>"#
    } else {
        ""
    };
    d.sections = vec![xml(format!(r#"<sec><p id="anchor" paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="50000"><margin left="5000" right="5000" top="4000" bottom="4000" header="1000" footer="1000"/></pagePr>{extra}</secPr>{objects}</run>{cache}</p></sec>"#).as_bytes()).unwrap()];
    d.masters.clear();
    d.borders.clear();
    for (id, color) in [("1", "#FF0000"), ("2", "#0000FF")] {
        d.borders.insert(id.into(), xml(format!(r#"<borderFill><topBorder type="SOLID" width="0.1 mm" color="{color}"/></borderFill>"#).as_bytes()).unwrap());
    }
    d.borders.insert("0".into(), xml(b"<borderFill/>").unwrap());
    d
}
fn border(kind: &str, id: &str, attrs: &str) -> String {
    format!(
        r#"<pageBorderFill type="{kind}" borderFillIDRef="{id}" {attrs}><offset left="500" right="600" top="700" bottom="800"/></pageBorderFill>"#
    )
}
fn top(d: &Document) -> (f64, f64, f64, [f64; 3]) {
    let mut e = Engine::new(d, fonts());
    e.render().unwrap();
    e.pages[0]
        .ops
        .iter()
        .find_map(|o| {
            if let Op::Line {
                x, y, x2, color, ..
            } = o
            {
                Some((*x, *y, *x2, *color))
            } else {
                None
            }
        })
        .unwrap()
}

#[test]
fn border_wire_origin_and_header_inclusion_are_independent_of_fill_area() {
    let body = document(
        &border(
            "BOTH",
            "1",
            r#"textBorder="PAPER" headerInside="1" footerInside="1" fillArea="PAPER""#,
        ),
        "",
        true,
    );
    assert_eq!(top(&body), (45., 33., 356., [1., 0., 0.]));
    let paper = document(&border("BOTH", "1", r#"textBorder="CONTENT""#), "", true);
    assert_eq!(top(&paper), (5., 7., 394., [1., 0., 0.]));
    let native = document(&border("BOTH", "1", r#"borderReference="PAPER""#), "", true);
    assert_eq!(top(&native), top(&paper));
}

#[test]
fn empty_parity_falls_back_but_visible_parity_overrides_both() {
    let common = border("BOTH", "1", r#"textBorder="CONTENT""#);
    let empty = document(&(common.clone() + &border("ODD", "0", "")), "", true);
    assert_eq!(top(&empty).3, [1., 0., 0.]);
    let odd = document(
        &(common + &border("ODD", "2", r#"textBorder="CONTENT""#)),
        "",
        true,
    );
    assert_eq!(top(&odd).3, [0., 0., 1.]);
}

#[test]
fn first_page_visibility_and_background_area_do_not_hide_body() {
    let mut d = document(
        &(border("BOTH", "1", r#"textBorder="PAPER" fillArea="PAPER""#)
            + r#"<visibility border="HIDE_FIRST" fill="SHOW_ALL"/>"#),
        "",
        true,
    );
    d.borders
        .get_mut("1")
        .unwrap()
        .children
        .push(xml(br##"<fillBrush><winBrush faceColor="#EEEEEE"/></fillBrush>"##).unwrap());
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert!(!e.pages[0].ops.iter().any(|o| matches!(o, Op::Line { .. })));
    assert!(
        e.pages[0]
            .ops
            .iter()
            .any(|o| matches!(o,Op::Rect{x,y,w,h,..} if *x==0. && *y==0. && *w==400. && *h==500.))
    );
}

#[test]
fn paragraph_vertical_alignment_does_not_use_page_height_in_saved_or_reflow_paths() {
    for cached in [true, false] {
        for align in ["TOP", "CENTER", "BOTTOM"] {
            // PAPER horizontal origin deliberately sends the rebuilt paragraph
            // through absolute-anchor normalization as well as saved painting.
            let object = format!(
                r##"<rect id="marker" textWrap="SQUARE"><sz width="3000" height="2000"/><pos treatAsChar="0" horzRelTo="PAPER" horzAlign="LEFT" horzOffset="12000" vertRelTo="PARA" vertAlign="{align}" vertOffset="1000"/><lineShape style="SOLID" width="10" color="#008800"/></rect>"##
            );
            let d = document("", &object, cached);
            let mut e = Engine::new(&d, fonts());
            e.render().unwrap();
            let b = e.report.bounds.iter().find(|b| b.id == "marker").unwrap();
            assert!(
                (b.y - if cached { 80. } else { 60. }).abs() < 0.01,
                "cached={cached} align={align} y={}",
                b.y
            );
            assert!((b.x - 120.).abs() < 0.01);
            assert_eq!(b.page, 1);
        }
    }
}
