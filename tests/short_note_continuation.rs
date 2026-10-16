use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
    qa,
};
use std::path::Path;

#[test]
fn short_note_keeps_anchor_only_when_first_note_line_fits() {
    for (origin, expected_page) in [(14500, 1), (19000, 2)] {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let notes=(0..6).map(|i|format!(r#"<p paraPrIDRef="0"><run charPrIDRef="0"><t>NOTE-{i}</t></run><linesegarray><lineseg textpos="0" vertpos="{}" vertsize="1200" baseline="1000" spacing="400" horzsize="30000"/></linesegarray></p>"#,i*1600)).collect::<String>();
        d.sections=vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><t>BEFORE</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="30000"/></linesegarray></p><p id="anchor" paraPrIDRef="0"><run charPrIDRef="0"><t>ANCHOR</t><ctrl><footNote number="1"><subList>{notes}</subList></footNote></ctrl></run><linesegarray><lineseg textpos="0" vertpos="{origin}" vertsize="1200" baseline="1000" horzsize="30000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
        d.masters.clear();
        let original = d.sections.clone();
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
        let anchor = e
            .report
            .bounds
            .iter()
            .find(|b| b.id == "anchor" && b.kind == "line")
            .unwrap();
        assert_eq!(anchor.page, expected_page);
        let note_pages: std::collections::BTreeSet<_> = e
            .report
            .bounds
            .iter()
            .filter(|b| b.kind.starts_with("footnote"))
            .map(|b| b.page)
            .collect();
        assert_eq!(
            note_pages,
            if expected_page == 1 {
                [1, 2].into_iter().collect()
            } else {
                [2].into_iter().collect()
            }
        );
        let all = e
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|op| match op {
                Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        for i in 0..6 {
            assert_eq!(all.matches(&format!("NOTE-{i}")).count(), 1);
        }
        assert_eq!(original, d.sections);
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
        assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
    }
}
