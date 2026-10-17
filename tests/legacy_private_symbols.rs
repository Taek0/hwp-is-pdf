use hwp_engine::{layout::Engine, model::Document, pdf::Op, qa};
use std::path::Path;
#[test]
fn public_documents_have_real_glyphs_for_legacy_checks_and_number_symbols() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !fonts.join("wingding.ttf").exists() {
        return;
    }
    for file in [
        "samples/tables-notes/inner-table-01.hwp",
        "samples/web/government-notice.hwp",
    ] {
        let d = Document::open(&root.join(file)).unwrap();
        let mut e = Engine::new(&d, fonts);
        e.render().unwrap();
        let mut count = 0;
        for op in e.pages.iter().flat_map(|p| &p.ops) {
            if let Op::Text { text, font, .. } = op {
                for c in text
                    .chars()
                    .filter(|c| ('\u{f020}'..='\u{f0ff}').contains(c))
                {
                    assert!(e.fonts.has(*font, c), "{file}: missing U+{:04X}", c as u32);
                    count += 1;
                }
            }
        }
        assert!(count > 0);
        assert!(
            !e.report
                .warnings
                .iter()
                .any(|w| w.starts_with("Missing glyph"))
        );
        if file.ends_with("government-notice.hwp") {
            assert!(
                e.pages
                    .iter()
                    .flat_map(|p| &p.ops)
                    .any(|o| matches!(o,Op::Link{target,..} if target=="http://www.sbiz24.kr)"))
            );
            assert!(
                !e.report
                    .warnings
                    .iter()
                    .any(|w| w.starts_with("Unresolved or unsupported PDF link"))
            );
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(q.text_overlaps.is_empty());
        assert!(q.outside_page.is_empty());
        assert!(q.border_crossings.is_empty());
        assert!(q.foreground_image_overlaps.is_empty());
    }
}
