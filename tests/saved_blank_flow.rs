//! Abstract line-origin checks from four independently printed Hancom probes.
use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
};
use std::path::Path;

fn probe(stalled: bool, top: usize, bottom: usize) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.chars
        .get_mut("0")
        .unwrap()
        .attrs
        .insert("height".into(), "1000".into());
    d.paras.insert("blank-probe".into(),xml(br#"<paraPr><align horizontal="LEFT"/><heading type="NONE"/><lineSpacing type="PERCENT" value="160"/></paraPr>"#).unwrap());
    d.masters.clear();
    let mut body = String::from("<sec>");
    for i in 0..32 {
        let y = if stalled {
            (i % 17).min(15) * 1600
        } else {
            (i % 14) * 1600
        };
        let setup = if i == 0 {
            format!(
                r#"<secPr><visibility hideFirstEmptyLine="1"/><pagePr width="40000" height="30000"><margin left="4000" right="4000" top="{top}" bottom="{bottom}"/></pagePr></secPr><ctrl><colPr colCount="2" sameGap="2000"/></ctrl>"#
            )
        } else {
            String::new()
        };
        let text = match i {
            0 => "A START",
            18 => "B AFTER BLANKS",
            30 => "C NEXT PAGE",
            _ => "",
        };
        body += &format!(
            r#"<p id="probe-{i}" paraPrIDRef="blank-probe"><run charPrIDRef="0">{setup}<t>{text}</t></run><linesegarray><lineseg textpos="0" vertpos="{y}" vertsize="1000" textheight="1000" baseline="850" spacing="600" horzpos="0" horzsize="15000"/></linesegarray></p>"#
        );
    }
    body += "</sec>";
    d.sections = vec![xml(body.as_bytes()).unwrap()];
    d
}

#[test]
fn logical_blank_flow_matches_reference_independently_of_saved_column_resets() {
    for (stalled, top, bottom, pages, b_y, c_page, c_x, c_y) in [
        (false, 4000, 4000, 2, 72., 2, 40., 40.),
        (true, 4000, 4000, 2, 72., 2, 40., 40.),
        (false, 4000, 8000, 2, 120., 2, 40., 104.),
        (false, 2000, 4000, 1, 36., 1, 210., 228.),
    ] {
        let d = probe(stalled, top, bottom);
        let original = d.sections.clone();
        let fonts = Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        });
        let mut e = Engine::new(&d, fonts);
        e.render().unwrap();
        assert_eq!(e.pages.len(), pages);
        for (id, page, x, y) in [
            ("probe-0", 1, 40., top as f64 / 100.),
            ("probe-18", 1, 210., b_y),
            ("probe-30", c_page, c_x, c_y),
        ] {
            let b = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "line" && b.id == id)
                .unwrap();
            assert_eq!(b.page, page, "{stalled}/{bottom}/{id}");
            assert!(
                (b.x - x).abs() < 0.001 && (b.y - y).abs() < 0.001,
                "{stalled}/{bottom}/{id}: {},{}",
                b.x,
                b.y
            );
        }
        assert_eq!(d.sections, original);
        assert!(!e.pages.iter().flat_map(|p| &p.ops).any(|op| matches!(op,
            hwp_engine::pdf::Op::Line {x,x2,..} if (*x-200.).abs()<0.001 && (*x2-200.).abs()<0.001)));
    }
}

#[test]
fn repaired_inline_rectangle_height_is_not_counted_twice_when_cached_line_reserves_it() {
    let mut d = probe(false, 4000, 4000);
    d.sections=vec![xml(br##"<sec><p id="slot" paraPrIDRef="blank-probe"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="4000" right="4000" top="4000" bottom="4000"/></pagePr></secPr><rect id="box"><sz width="10000" height="3819"/><curSz width="10000" height="6866"/><pos treatAsChar="1"/><lineShape style="SOLID" width="10" color="#000000"/></rect></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="6866" baseline="5836" spacing="600" horzsize="32000"/></linesegarray></p><p id="next" paraPrIDRef="blank-probe"><run charPrIDRef="0"><t>NEXT</t></run><linesegarray><lineseg textpos="0" vertpos="7466" vertsize="1000" baseline="850" spacing="600" horzsize="32000"/></linesegarray></p></sec>"##).unwrap()];
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    let b = e.report.bounds.iter().find(|b| b.id == "box").unwrap();
    assert!((b.y - 40.).abs() < 0.02);
    let next = e.report.bounds.iter().find(|b| b.id == "next").unwrap();
    assert!((next.y - 114.66).abs() < 0.001);
    assert!(
        !e.report
            .warnings
            .iter()
            .any(|w| w.starts_with("Repaired stale inline shape height"))
    );
}
