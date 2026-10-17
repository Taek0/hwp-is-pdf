use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
use std::path::Path;

#[test]
fn supplied_combination_chart_preserves_axes_styles_and_all_values() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-Minimal-Print/02_reference_ready/05_mixed_charts.hwp");
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !input.exists() || !fonts.exists() {
        return;
    }
    let d = Document::open(&input).unwrap();
    let c = d.sections[0].find("chart").unwrap();
    assert_eq!(c.a("nativeType"), "9");
    let layout = c.child("nativeLayout").unwrap();
    let ranges: Vec<_> = layout
        .children_named("axis")
        .map(|a| (a.n("min"), a.n("max"), a.u("divisions")))
        .collect();
    assert_eq!(ranges, vec![(80., 110., 6), (0., 6., 6)]);
    let fs: Vec<_> = layout
        .children_named("axisFont")
        .take(3)
        .map(|f| f.n("size"))
        .collect();
    assert_eq!(fs, vec![7., 10., 10.]);
    assert_eq!(
        layout
            .children_named("axisFont")
            .next()
            .unwrap()
            .u("horizontal"),
        1
    );
    assert_eq!(
        c.children_named("series")
            .map(|s| s.u("painter"))
            .collect::<Vec<_>>(),
        vec![6, 1]
    );
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    assert_eq!(e.pages.len(), 1);
    assert_eq!(e.report.charts, 1);
    let ops = &e.pages[0].ops;
    assert!(ops.iter().any(
        |o| matches!(o,Op::Path {points,color,..} if points.len()==102 && *color==[1.,0.,0.])
    ));
    assert_eq!(ops.iter().filter(|o|matches!(o,Op::Rect {fill,..} if (fill[0]-64./255.).abs()<0.001 && (fill[1]-192./255.).abs()<0.001)).count(),102);
    assert!(
        !ops.iter()
            .any(|o| matches!(o,Op::Text{text,..} if text.contains("미해석")))
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty());
    assert!(q.border_crossings.is_empty());
    assert!(q.outside_page.is_empty());
    let mut malformed = c.clone();
    malformed
        .children
        .iter_mut()
        .find(|n| n.name == "nativeLayout")
        .unwrap()
        .children
        .iter_mut()
        .find(|n| n.name == "axis")
        .unwrap()
        .attrs
        .insert("divisions".into(), "4294967295".into());
    assert!(hwp_engine::chart::draw(&malformed, 0., 0., 400., 250., &mut e.fonts).is_err());
}
