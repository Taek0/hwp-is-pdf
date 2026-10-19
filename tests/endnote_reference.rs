use hwp_engine::{layout::Engine, model::Document, pdf::Op};
use std::path::Path;
#[test]
fn native_endnote_page_retains_blank_height_but_not_trailing_leading_or_an_extra_rule_gap() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/tables-notes/endnote-01.hwp");
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !path.exists() || !fonts.exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let line = d.sections[0]
        .find("endNotePr")
        .unwrap()
        .child("noteLine")
        .unwrap();
    assert!(line.n("length") > 32767.);
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    assert_eq!(e.pages.len(), 5);
    let first = e.pages[4]
        .ops
        .iter()
        .find_map(|op| match op {
            Op::Text { text, y, .. } if text.starts_with("1)") => Some(*y),
            _ => None,
        })
        .unwrap();
    assert!((first - 107.76).abs() < 0.15, "{first}");
    assert!(e.pages[4].ops.iter().any(
        |op| matches!(op,Op::Line{x,x2,y,..} if (y-94.44).abs()<0.15 && (x2-x-482.04).abs()<0.2)
    ));
    assert_eq!(
        e.report
            .bounds
            .iter()
            .filter(|b| b.kind == "endnote")
            .count(),
        6
    );
}
