use hwp_engine::{graphics, layout::Engine, model::Document, pdf::Op};
#[test]
fn supplied_vertical_script_columns_match_hancom_origins() {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-Four-Priorities-20261019/03_04_replacement_print/03b_vertical_all.hwpx");
    if !input.exists() {
        eprintln!("SKIP: user-owned reference fixture unavailable");
        return;
    }
    let doc = Document::open(&input).unwrap();
    let mut engine = Engine::new(
        &doc,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    engine.render().unwrap();
    assert_eq!(engine.pages.len(), 2);
    let mut stack = Vec::new();
    let mut matrix = graphics::IDENTITY;
    let mut origins = Vec::new();
    let mut words = Vec::new();
    for op in &engine.pages[1].ops {
        match op {
            Op::TransformBegin { matrix: m } => {
                stack.push(matrix);
                matrix = graphics::compose(matrix, *m);
            }
            Op::TransformEnd => matrix = stack.pop().unwrap(),
            Op::Text { x, y, text, .. } if text == "가" => {
                origins.push(graphics::point(matrix, *x, *y))
            }
            Op::Text { x, y, text, .. } if text == "한" => {
                words.push(graphics::point(matrix, *x, *y))
            }
            _ => {}
        }
    }
    assert_eq!(origins.len(), 4);
    assert_eq!(words.len(), 4);
    for ((first, word), advance) in origins.iter().zip(&words).zip([81., 58.08, 58.08, 84.]) {
        assert!((word.1 - first.1 - advance).abs() < 0.15);
    }

    for (p, expected) in origins.iter().zip([491.887389, 440.16, 365.88, 311.28]) {
        assert!((p.0 - expected).abs() < 0.15, "{} != {}", p.0, expected);
    }
}

#[test]
fn supplied_horizontal_script_rows_keep_nominal_leading_and_reference_baselines() {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-Four-Priorities-20261019/01_print/02_character_effects.hwpx");
    if !input.exists() {
        eprintln!("SKIP: user-owned reference fixture unavailable");
        return;
    }
    let doc = Document::open(&input).unwrap();
    let mut engine = Engine::new(
        &doc,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    engine.render().unwrap();
    assert_eq!(engine.pages.len(), 2);
    let ys: Vec<_> = engine.pages[1]
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Text { text, y, .. } = o {
                (text == "가").then_some(*y)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(ys.len(), 4);
    for (y, expected) in ys.iter().zip([122.88, 173.04, 247.32, 310.08]) {
        assert!((*y - expected).abs() < 0.15, "{y} != {expected}");
    }
}
