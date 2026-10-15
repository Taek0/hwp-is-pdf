use hwp_engine::{
    layout::Engine,
    model::Document,
    pdf::{Op, Page},
    qa,
};
use std::path::PathBuf;
fn fonts() -> PathBuf {
    if cfg!(windows) {
        "C:/Windows/Fonts".into()
    } else {
        "/mnt/c/Windows/Fonts".into()
    }
}
#[test]
fn hwp_inputs_render_directly_with_no_unintended_geometry_collisions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (name, pages, tables, pictures) in [
        ("복학원서.hwp", 2, 3, 2),
        ("21_언어_기출_편집가능본.hwp", 15, 16, 1),
    ] {
        let path = root.parent().unwrap().join(name);
        if !path.exists() || !fonts().exists() {
            eprintln!("Local user fixture unavailable: {}", path.display());
            continue;
        }
        let before = std::fs::read(&path).unwrap();
        let doc = Document::open(&path).unwrap();
        assert_eq!(doc.header.a("binaryHwp"), "1");
        let mut engine = Engine::new(&doc, &fonts());
        engine.render().unwrap();
        assert_eq!(engine.report.pages, pages);
        assert_eq!(engine.report.tables, tables);
        assert_eq!(engine.report.pictures, pictures);
        assert!(
            !engine
                .report
                .warnings
                .iter()
                .any(|w| w.starts_with("Missing glyph")),
            "{name}: {:?}",
            engine.report.warnings
        );
        let audit = qa::inspect(&engine.pages, &engine.fonts, &engine.report.bounds);
        assert!(
            audit.outside_page.is_empty(),
            "{name}: {:?}",
            audit.outside_page
        );
        assert!(
            audit.text_overlaps.is_empty(),
            "{name}: {:?}",
            audit.text_overlaps
        );
        assert!(
            audit.border_crossings.is_empty(),
            "{name}: {:?}",
            audit.border_crossings
        );
        assert!(
            audit.foreground_image_overlaps.is_empty(),
            "{name}: {:?}",
            audit.foreground_image_overlaps
        );
        if name == "복학원서.hwp" {
            assert_eq!(audit.background_images_exempted, 2)
        } else {
            assert_eq!(audit.background_images_exempted, 0)
        }
        assert_eq!(before, std::fs::read(path).unwrap(), "Input changed");
    }
}
#[test]
fn collision_detector_rejects_a_rule_through_visible_glyphs() {
    if !fonts().exists() {
        return;
    }
    let mut fonts = hwp_engine::pdf::Fonts::new(&fonts());
    let font = fonts.get("arial.ttf").unwrap();
    let page = Page {
        width: 100.,
        height: 100.,
        ops: vec![
            Op::Text {
                x: 10.,
                y: 30.,
                text: "H".into(),
                font,
                size: 20.,
                scale: 1.,
                color: [0.; 3],
            },
            Op::Line {
                x: 0.,
                y: 23.,
                x2: 90.,
                y2: 23.,
                width: 0.5,
                color: [0.; 3],
            },
        ],
    };
    let audit = qa::inspect(&[page], &fonts, &[]);
    assert_eq!(audit.border_crossings.len(), 1);
}
