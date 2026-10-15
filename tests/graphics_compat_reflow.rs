use hwp_engine::{
    layout::Engine,
    math,
    model::Document,
    pdf::{Fonts, Op},
    qa,
};
fn fonts() -> &'static std::path::Path {
    std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
#[test]
fn accents_are_scoped_and_stacks_have_real_geometry() {
    let mut f = Fonts::new(fonts());
    let b = math::layout("BAR {AB} + CD", 16., &mut f).unwrap();
    let bars: Vec<_> = b
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Line { x, x2, .. } = o {
                Some(x2 - x)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bars.len(), 1);
    assert!(bars[0] < b.width * 0.7);
    let c = math::layout("{n} CHOOSE {k}", 16., &mut f).unwrap();
    assert_eq!(
        c.ops
            .iter()
            .filter(|o| matches!(o, Op::Bezier { .. }))
            .count(),
        2
    );
    let placed = math::place(c, 200., 300., 60., 70.);
    for op in placed {
        if let Op::Bezier {
            start, segments, ..
        } = op
        {
            assert!(start.0 > 190. && start.1 > 200.);
            assert!(
                segments
                    .iter()
                    .all(|s| s[0] > 190. && s[2] > 190. && s[4] > 190.)
            );
        }
    }
}
#[test]
fn graphics_background_and_public_chart_survive_reflow() {
    for path in [
        "samples/reflow/graphics-accents.hwpx",
        "samples/reflow/graphics-background-arcs.hwpx",
        "samples/features/chart.hwp",
        "samples/features/shapes.hwp",
    ] {
        let d =
            Document::open(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
        let mut e = Engine::new(&d, fonts());
        e.render_reflow().unwrap();
        if path.contains("accents") {
            assert_eq!(e.report.equations, 14);
        }
        if path.contains("background-arcs") {
            let back = e
                .report
                .bounds
                .iter()
                .find(|b| b.id == "background-box")
                .unwrap();
            let line = e
                .report
                .bounds
                .iter()
                .find(|b| b.kind == "line" && b.id == "background")
                .unwrap();
            assert!(line.y < back.y + 50.);
            assert_eq!(
                e.report
                    .bounds
                    .iter()
                    .filter(|b| b.id == "background-box")
                    .count(),
                1
            );
            assert_eq!(
                e.report.bounds.iter().filter(|b| b.kind == "arc").count(),
                3
            );
        }
        if path.ends_with("chart.hwp") {
            assert_eq!(e.report.charts, 1);
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.outside_page.is_empty(), "{path} {:?}", q.outside_page);
        assert!(q.text_overlaps.is_empty(), "{path} {:?}", q.text_overlaps);
        assert!(
            q.border_crossings.is_empty(),
            "{path} {:?}",
            q.border_crossings
        );
        assert!(q.foreground_image_overlaps.is_empty());
    }
}

#[test]
fn background_does_not_disable_complex_text_shaping() {
    let mut d = Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/reflow/typography-rtl.hwpx"),
    )
    .unwrap();
    let p = d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "arabic-0")
        .unwrap();
    let run = p.children.iter_mut().find(|n| n.name == "run").unwrap();
    run.children.insert(0,hwp_engine::model::xml(br##"<rect id="rtl-background" textWrap="BEHIND_TEXT"><sz width="30000" height="10000"/><pos treatAsChar="0" horzRelTo="PAPER" vertRelTo="PAPER" horzOffset="5000" vertOffset="5000"/><lineShape style="NONE"/><fillBrush><winBrush faceColor="#EDF4FC"/></fillBrush></rect>"##).unwrap());
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    assert_eq!(
        e.report
            .bounds
            .iter()
            .filter(|b| b.id == "rtl-background")
            .count(),
        1
    );
    assert!(
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .any(|o| matches!(o, Op::Shaped { .. }))
    );
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty());
    assert!(q.text_overlaps.is_empty());
}
