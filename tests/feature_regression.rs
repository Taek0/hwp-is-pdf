use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn fonts() -> PathBuf {
    if cfg!(windows) {
        "C:/Windows/Fonts".into()
    } else {
        "/mnt/c/Windows/Fonts".into()
    }
}

#[test]
fn native_chart_integer_cells_preserve_data_and_labels() {
    let doc = Document::open(&root().join("samples/features/chart.hwp")).unwrap();
    let chart = doc.sections.iter().find_map(|s| s.find("chart")).unwrap();
    assert_eq!(chart.a("nativeType"), "1");
    assert_eq!(chart.a("title"), "차트 제목");
    let categories: Vec<_> = chart
        .children_named("category")
        .map(|n| n.a("name"))
        .collect();
    assert_eq!(categories, ["행1", "행2", "행3", "행4", "행5"]);
    let series: Vec<_> = chart.children_named("series").collect();
    assert_eq!(series.len(), 4);
    assert_eq!(series[0].a("name"), "열1");
    assert_eq!(
        series[0].children_named("value").next().unwrap().n("n"),
        92.
    );
    assert!(
        series
            .iter()
            .all(|s| s.children_named("value").count() == 5)
    );
}

#[test]
fn feature_documents_render_without_geometry_collisions() {
    assert!(
        fonts().exists(),
        "This integration suite requires Windows fonts"
    );
    for (name, count) in [
        ("chart", 1),
        ("curves", 2),
        ("distribution", 2),
        ("equation", 1),
        ("fields", 1),
        ("group", 1),
        ("header-footer", 2),
        ("notes", 1),
        ("numbering", 1),
        ("ole", 1),
        ("overlay", 1),
        ("polygon", 1),
        ("ruby", 1),
        ("shapes", 1),
    ] {
        let path = root().join(format!("samples/features/{name}.hwp"));
        let original = std::fs::read(&path).unwrap();
        let doc = Document::open(&path).unwrap();
        let mut engine = Engine::new(&doc, &fonts());
        engine.render().unwrap_or_else(|e| panic!("{name}: {e:#}"));
        assert_eq!(engine.pages.len(), count, "{name}");
        assert!(
            engine.pages.iter().any(|p| !p.ops.is_empty()),
            "{name}: empty rendering"
        );
        if name == "chart" {
            assert_eq!(engine.report.charts, 1);
        }
        if name == "equation" {
            assert_eq!(engine.report.equations, 1);
        }
        if name == "ole" {
            assert_eq!(engine.report.pictures, 2);
        }
        let audit = qa::inspect(&engine.pages, &engine.fonts, &engine.report.bounds);
        assert!(audit.outside_page.is_empty(), "{name}: outside page");
        assert!(audit.text_overlaps.is_empty(), "{name}: overlapping text");
        assert!(audit.border_crossings.is_empty(), "{name}: crossing rule");
        assert!(
            audit.foreground_image_overlaps.is_empty(),
            "{name}: foreground image overlap"
        );
        assert_eq!(original, std::fs::read(path).unwrap());
    }
}

#[test]
fn structured_equations_emit_content_and_reject_excessive_nesting() {
    let mut fonts = hwp_engine::pdf::Fonts::new(&fonts());
    for script in [
        "sqrt {x}",
        "root {3} of {8}",
        "bmatrix {a & b # c & d}",
        "cases {x & x>0 # 0 & x=0}",
    ] {
        let b = hwp_engine::math::layout(script, 12., &mut fonts).unwrap();
        assert!(b.width > 0. && b.ascent > 0. && b.descent >= 0.);
        assert!(
            b.ops.iter().any(|op| matches!(op, Op::Path { .. })),
            "{script}"
        );
        assert!(
            b.ops.iter().any(|op| matches!(op, Op::Text { .. })),
            "{script}"
        );
    }
    let nested = format!("{}x{}", "sqrt {".repeat(70), "}".repeat(70));
    assert!(
        hwp_engine::math::layout(&nested, 12., &mut fonts)
            .err()
            .unwrap()
            .to_string()
            .contains("nesting")
    );
}

#[test]
fn public_documents_keep_negative_tracking_from_colliding() {
    for (name, pages) in [("government-notice", 30), ("government-faq", 14)] {
        let doc = Document::open(&root().join(format!("samples/web/{name}.hwp"))).unwrap();
        let mut engine = Engine::new(&doc, &fonts());
        engine.render().unwrap();
        assert_eq!(engine.report.pages, pages);
        let audit = qa::inspect(&engine.pages, &engine.fonts, &engine.report.bounds);
        assert!(
            audit.text_overlaps.is_empty(),
            "{name}: {:?}",
            audit.text_overlaps
        );
        assert!(audit.outside_page.is_empty(), "{name}: outside page");
        assert!(audit.border_crossings.is_empty(), "{name}: crossing rule");
        assert!(
            audit.foreground_image_overlaps.is_empty(),
            "{name}: foreground image overlap"
        );
    }
}
