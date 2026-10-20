use hwp_engine::{layout::Engine, model::Document, pdf::Op};
use std::path::Path;

#[test]
fn native_page_counters_retain_their_printed_rising_diagonals() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("21_언어_기출_편집가능본.hwp");
    if !input.exists() {
        return;
    }
    let d = Document::open(&input).unwrap();
    let border = &d.borders["14"];
    assert_eq!(border.child("slash").unwrap().a("type"), "2");
    assert_eq!(border.child("backSlash").unwrap().a("type"), "0");
    assert_eq!(border.child("diagonal").unwrap().a("width"), "0.12 mm");
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    assert_eq!(e.pages.len(), 15);
    // These are authored page-counter cells on pages 10 and 11, not a
    // repeating footer on every page of this A3 document.
    for i in [9usize, 10] {
        let page = &e.pages[i];
        assert!(
            page.ops.iter().any(|op| matches!(op,
                Op::Line {x,y,x2,y2,width,..} if (*x2 - *x - 50.48).abs() < 0.01
                    && *y > *y2 + 5. && *y2 > page.height - 100. && *width > 0.
            )),
            "missing page {} page-counter diagonal",
            i + 1
        );
    }
}
