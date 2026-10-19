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
fn paragraph(label: &str) -> String {
    format!(r#"<p paraPrIDRef="0"><run charPrIDRef="0"><t>{label}</t></run></p>"#)
}
fn cell(row: usize, col: usize, span: usize, label: String) -> String {
    format!(
        r#"<tc borderFillIDRef="1" header="{}"><cellAddr rowAddr="{row}" colAddr="{col}"/><cellSpan rowSpan="{span}" colSpan="1"/><cellSz width="15000" height="{}"/><subList>{label}</subList></tc>"#,
        usize::from(row < 2),
        span * 2200
    )
}
fn document(rows: usize, cols: usize, content: String, columns: usize) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><ctrl><colPr colCount="{columns}" sameGap="1000"/></ctrl><tbl id="test-table" rowCnt="{rows}" colCnt="{cols}" repeatHeader="1" pageBreak="TABLE"><sz width="30000" height="{}"/><pos treatAsChar="1" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="100" right="100" top="100" bottom="100"/>{content}</tbl></run></p></sec>"#,rows*2200).as_bytes()).unwrap()];
    d.masters.clear();
    d
}
fn text(e: &Engine) -> String {
    e.pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|o| match o {
            Op::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}
#[test]
fn merged_two_row_header_repeats_as_a_complete_band() {
    let mut rows = format!(
        "<tr>{}{}</tr><tr>{}</tr>",
        cell(0, 0, 2, paragraph("MERGED")),
        cell(0, 1, 1, paragraph("TOP")),
        cell(1, 1, 1, paragraph("LOW"))
    );
    for i in 2..32 {
        rows += &format!(
            "<tr>{}{}</tr>",
            cell(i, 0, 1, paragraph(&format!("L{i:02}"))),
            cell(i, 1, 1, paragraph(&format!("R{i:02}")))
        );
    }
    let d = document(32, 2, rows, 1);
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    assert!(e.pages.len() > 1);
    let out = text(&e);
    for label in ["MERGED", "TOP", "LOW"] {
        assert_eq!(out.matches(label).count(), e.pages.len());
    }
    for i in 2..32 {
        for side in ["L", "R"] {
            assert_eq!(out.matches(&format!("{side}{i:02}")).count(), 1);
        }
    }
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
    assert!(q.outside_page.is_empty());
}
#[test]
fn single_row_repeat_flag_does_not_require_a_header_in_column_reflow() {
    let content = (0..40).map(|i| paragraph(&format!("ROW{i:02}"))).collect();
    let d = document(1, 1, format!("<tr>{}</tr>", cell(0, 0, 1, content)), 2);
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    let out = text(&e);
    for i in 0..40 {
        assert_eq!(out.matches(&format!("ROW{i:02}")).count(), 1);
    }
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.outside_page.is_empty());
}
#[test]
fn supplied_math_full_reflow_keeps_equations_and_pictures() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("2단_수학시험지_변환검증_7826.hwpx");
    if !input.exists() {
        return;
    }
    let d = Document::open(&input).unwrap();
    let mut saved = Engine::new(&d, fonts());
    saved.render().unwrap();
    let mut flowed = Engine::new(&d, fonts());
    flowed.render_reflow().unwrap();
    assert_eq!(saved.report.equations, flowed.report.equations);
    assert_eq!(saved.report.pictures, flowed.report.pictures);
    let q = qa::inspect(&flowed.pages, &flowed.fonts, &flowed.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.outside_page.is_empty());
    assert!(q.border_crossings.is_empty());
}

#[test]
fn numbering_edited_math_full_reflow_retains_nine_pages_and_all_objects() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-Note-Wrap-20261016/01_print/04_math_black_markers_no_list_numbers.hwpx");
    if !input.exists() {
        return;
    }
    let d = Document::open(&input).unwrap();
    let mut saved = Engine::new(&d, fonts());
    saved.render().unwrap();
    let mut flowed = Engine::new(&d, fonts());
    flowed.render_reflow().unwrap();
    assert_eq!(flowed.pages.len(), 9);
    assert_eq!(saved.report.equations, flowed.report.equations);
    assert_eq!(saved.report.pictures, flowed.report.pictures);
    assert_eq!(text(&flowed).matches("[정답]").count(), 26);
    let notes: Vec<_> = flowed
        .report
        .bounds
        .iter()
        .filter(|b| b.kind == "endnote-fragment")
        .collect();
    assert_eq!(notes.len(), 26);
    assert!(notes.iter().all(|b| b.page == 9));
    for pair in notes.windows(2) {
        assert!((pair[1].y - pair[0].y - 10.).abs() < 0.01);
    }
    let q = qa::inspect(&flowed.pages, &flowed.fonts, &flowed.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.outside_page.is_empty());
    assert!(q.border_crossings.is_empty());
}

#[test]
fn merged_body_cells_are_not_promoted_to_headers() {
    for explicit_zero in [true, false] {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/tables-notes/merged-header.hwpx"),
        )
        .unwrap();
        fn clear(n: &mut hwp_engine::model::Node, explicit: bool) {
            if n.name == "tc" {
                if explicit {
                    n.attrs.insert("header".into(), "0".into());
                } else {
                    n.attrs.remove("header");
                }
            }
            for c in &mut n.children {
                clear(c, explicit);
            }
        }
        clear(&mut d.sections[0], explicit_zero);
        let mut e = Engine::new(&d, fonts());
        e.render_reflow().unwrap();
        let out = text(&e);
        for label in ["MERGED", "TOP", "LOW"] {
            assert_eq!(out.matches(label).count(), 1);
        }
        assert!(
            qa::inspect(&e.pages, &e.fonts, &e.report.bounds)
                .text_overlaps
                .is_empty()
        );
    }
}
