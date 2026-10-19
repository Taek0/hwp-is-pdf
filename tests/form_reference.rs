use hwp_engine::{graphics, layout::Engine, model::Document, pdf::Op};
#[test]
fn supplied_form_merged_cells_and_caption_origins_match_hancom() {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-Four-Priorities-20261019/01_print/07_form_controls.hwpx");
    if !input.exists() {
        eprintln!("SKIP: user-owned print fixture unavailable");
        return;
    }
    let doc = Document::open(&input).unwrap();
    let mut e = Engine::new(
        &doc,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    assert_eq!(e.pages.len(), 10);
    let mut matrix = graphics::IDENTITY;
    let mut stack = Vec::new();
    let mut positions = std::collections::BTreeMap::new();
    for o in &e.pages[0].ops {
        match o {
            Op::TransformBegin { matrix: m } => {
                stack.push(matrix);
                matrix = graphics::compose(matrix, *m);
            }
            Op::TransformEnd => matrix = stack.pop().unwrap(),
            Op::Text { x, y, text, .. } => {
                positions
                    .entry(text.as_str())
                    .or_insert(graphics::point(matrix, *x, *y));
            }
            _ => {}
        }
    }
    for (text, x, y) in [
        ("원천기술형", 168.12, 115.08),
        ("혁신제품형", 249., 115.08),
        ("세계최초", 196.68, 134.88),
        ("세계최고", 326.28, 134.88),
        ("해당없음", 455.88, 134.88),
    ] {
        let p = positions.get(text).unwrap();
        assert!(
            (p.0 - x).abs() < 0.25 && (p.1 - y).abs() < 0.25,
            "{text}: {p:?} != ({x},{y})"
        );
    }
}
