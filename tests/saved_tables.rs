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
fn document(binary: bool, wrap: &str) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.sections=vec![xml(format!(r#"<sec><p id="body" paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="50000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><tbl id="table" binaryHwp="{}" rowCnt="1" colCnt="1" pageBreak="NONE"><sz width="9000" height="4000"/><pos treatAsChar="0" horzRelTo="COLUMN" vertRelTo="PARA"/><outMargin left="500" top="700"/><inMargin left="100" right="100" top="100" bottom="100"/><tr><tc borderFillIDRef="1"><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="9000" height="4000"/><subList lineWrap="{wrap}"><p id="text" paraPrIDRef="0"><run charPrIDRef="0"><t>Receiving Official long label</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzpos="0" horzsize="8800"/></linesegarray></p></subList></tc></tr></tbl></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzpos="0" horzsize="30000"/></linesegarray></p></sec>"#,usize::from(binary)).as_bytes()).unwrap()];
    d.masters.clear();
    d
}
#[test]
fn native_break_cells_recover_lines_but_squeeze_cells_keep_one_line() {
    for wrap in ["BREAK", "SQUEEZE"] {
        let d = document(true, wrap);
        let source = d.sections.clone();
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let mut rows = std::collections::BTreeSet::new();
        let mut text = String::new();
        for op in &e.pages[0].ops {
            if let Op::Text { text: t, y, .. } = op {
                rows.insert((y * 100.).round() as i64);
                text.push_str(t);
            }
        }
        assert_eq!(text.replace(' ', ""), "ReceivingOfficiallonglabel");
        assert_eq!(rows.len() > 1, wrap == "BREAK");
        assert_eq!(d.sections, source);
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.border_crossings.is_empty());
        assert!(q.outside_page.is_empty());
    }
}
#[test]
fn saved_floating_hwpx_table_uses_outer_margin_box_origin() {
    let d = document(false, "BREAK");
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let b = e.report.bounds.iter().find(|b| b.id == "table").unwrap();
    assert_eq!((b.x, b.y), (55., 57.));
}

#[test]
fn centered_latin_word_labels_wrap_small_overflow_only_when_break_is_enabled() {
    let mut f = hwp_engine::pdf::Fonts::new(fonts());
    let id = f.get("arial.ttf").unwrap();
    let available = f.width(id, "Receiving Official long label", 12.) - 0.5;
    for wrap in ["BREAK", "SQUEEZE", "KEEP"] {
        let mut d = document(true, wrap);
        let style = d.paras.get_mut("0").unwrap();
        style
            .children
            .iter_mut()
            .find(|n| n.name == "align")
            .unwrap()
            .attrs
            .insert("horizontal".into(), "CENTER".into());
        style
            .children
            .push(xml(br#"<breakSetting breakLatinWord="KEEP_WORD"/>"#).unwrap());
        fn widths(n: &mut hwp_engine::model::Node, w: f64) {
            if matches!(n.name.as_str(), "cellSz" | "sz") {
                n.attrs
                    .insert("width".into(), ((w + 2.) * 100.).to_string());
            }
            if n.name == "lineseg" && n.n("horzsize") == 8800. {
                n.attrs.insert("horzsize".into(), (w * 100.).to_string());
            }
            for child in &mut n.children {
                widths(child, w);
            }
        }
        widths(&mut d.sections[0], available);
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let rows: std::collections::BTreeSet<_> = e.pages[0]
            .ops
            .iter()
            .filter_map(|o| {
                if let Op::Text { y, .. } = o {
                    Some((y * 100.).round() as i64)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(rows.len() > 1, wrap == "BREAK");
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty());
        assert!(q.border_crossings.is_empty());
    }
}

#[test]
fn supplied_form_receiving_label_uses_two_english_rows_without_losing_text() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("복학원서.hwp");
    if !path.exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 2);
    let mut rows = std::collections::BTreeMap::<i64, String>::new();
    for op in &e.pages[0].ops {
        if let Op::Text { text, y, .. } = op {
            rows.entry((y * 100.).round() as i64)
                .or_default()
                .push_str(text);
        }
    }
    assert!(
        rows.values()
            .any(|r| r.contains("(Receiving") && !r.contains("Official)"))
    );
    assert!(
        rows.values()
            .any(|r| r.contains("Official)") && !r.contains("Receiving"))
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
}
