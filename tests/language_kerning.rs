use hwp_engine::{
    layout::Engine,
    model::Document,
    pdf::{Fonts, Op},
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
#[test]
fn opentype_pair_kerning_is_scaled_and_does_not_invent_ligatures() {
    let mut f = Fonts::new(fonts());
    let id = f.get("arial.ttf").unwrap();
    let k = f.kerning(id, 'A', 'V', 20.);
    assert!(k < -0.2 && k > -5.);
    assert!((f.kerning(id, 'A', 'V', 40.) - 2. * k).abs() < 1e-9);
    assert!(f.kerning(id, 'f', 'i', 20.).is_finite());
}
#[test]
fn language_faces_and_ratios_survive_saved_and_rebuilt_text() {
    let d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/language-kerning.hwpx"),
    )
    .unwrap();
    for reflow in [false, true] {
        let mut e = Engine::new(&d, fonts());
        if reflow {
            e.render_reflow().unwrap()
        } else {
            e.render().unwrap()
        }
        let find = |c: &str| {
            e.pages
                .iter()
                .flat_map(|p| &p.ops)
                .find_map(|o| {
                    if let Op::Text {
                        text, font, scale, ..
                    } = o
                    {
                        (text == c).then_some((&e.fonts.list[*font].name, *scale))
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let (hanja, ratio) = find("漢");
        assert!(hanja.to_lowercase().contains("simsun"));
        assert!((ratio - 1.2).abs() < 0.01);
        let (japanese, ratio) = find("か");
        assert!(japanese.to_lowercase().contains("msgothic"));
        assert!((ratio - 0.9).abs() < 0.01);
        assert!(find("★").0.to_lowercase().contains("seguisym"));
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
        assert!(
            e.report
                .warnings
                .iter()
                .all(|w| !w.starts_with("Missing glyph"))
        );
    }
}
