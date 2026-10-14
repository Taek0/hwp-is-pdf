use hwp_engine::{
    layout::Engine,
    model::Document,
    pdf::{Fonts, Op, glyph_id},
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
fn symbol_cmap_is_shared_by_metrics_pdf_encoding_and_qa() {
    let mut fonts = Fonts::new(&fonts());
    let index = fonts.get("wingding.ttf").unwrap();
    let face = ttf_parser::Face::parse(&fonts.list[index].bytes, 0).unwrap();
    let g = glyph_id(&face, '\u{f06c}').unwrap();
    assert_eq!(glyph_id(&face, 'l'), Some(g));
    assert!(face.glyph_bounding_box(g).is_some());
    assert!(fonts.has(index, '\u{f06c}'));
    assert!(fonts.width(index, "\u{f06c}", 12.) > 0.);
    let page = hwp_engine::pdf::Page {
        width: 100.,
        height: 100.,
        ops: vec![Op::Text {
            x: 10.,
            y: 20.,
            text: "\u{f06c}".into(),
            font: index,
            size: 12.,
            scale: 1.,
            color: [0.; 3],
        }],
    };
    let audit = qa::inspect(&[page], &fonts, &[]);
    assert_eq!(audit.glyphs_checked, 1);
}
#[test]
fn legacy_bullet_is_visible_without_missing_glyph_warning() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let doc = Document::open(&root.join("samples/features/numbering.hwp")).unwrap();
    let mut e = Engine::new(&doc, &fonts());
    e.render().unwrap();
    assert!(
        !e.report
            .warnings
            .iter()
            .any(|s| s.starts_with("Missing glyph"))
    );
    let bullets: Vec<_> = e
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|op| {
            if let Op::Text { text, font, .. } = op {
                (text == "\u{f06c}").then_some(*font)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(bullets.len(), 2);
    assert!(bullets.iter().all(|&f| e.fonts.has(f, '\u{f06c}')));
}
#[test]
fn curved_segment_types_survive_binary_normalization() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let doc = Document::open(&root.join("samples/features/curves.hwp")).unwrap();
    let mut curves = Vec::new();
    for s in &doc.sections {
        s.all("curve", &mut curves);
    }
    assert_eq!(curves.len(), 4);
    for c in curves {
        assert_eq!(
            c.children_named("segment").count() + 1,
            c.children_named("point").count()
        );
        assert!(c.children_named("segment").any(|s| s.a("type") == "1"));
    }
}

#[test]
fn public_typography_fixtures_have_glyphs_and_no_collisions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (name, pages) in [("lists-bullet", 3), ("facename2", 1)] {
        let doc = Document::open(&root.join(format!("samples/typography/{name}.hwp"))).unwrap();
        let mut e = Engine::new(&doc, &fonts());
        e.render().unwrap();
        assert_eq!(e.pages.len(), pages);
        assert!(
            !e.report
                .warnings
                .iter()
                .any(|s| s.starts_with("Missing glyph")),
            "{name}"
        );
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(
            q.outside_page.is_empty()
                && q.text_overlaps.is_empty()
                && q.border_crossings.is_empty()
                && q.foreground_image_overlaps.is_empty(),
            "{name}"
        );
    }
}
