use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
use std::path::Path;
#[test]
fn continued_table_reclaims_a_body_line_but_preserves_the_authored_blank_page() {
    let d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
    )
    .unwrap();
    let mut e = Engine::new(
        &d,
        Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    assert_eq!(e.pages.len(), 50);
    let text = |page: usize| {
        e.pages[page - 1]
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Text { text, .. } | Op::Shaped { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .flat_map(str::chars)
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    assert!(text(4).contains("더불어(2)개별법령상"));
    assert_eq!(text(6), "-6-");
    assert!(text(7).contains("2.연구의목적및필요성"));
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
    assert!(q.outside_page.is_empty());
}
