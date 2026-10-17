//! Hancom baseline measurements, independently read from the supplied PDF.
use hwp_engine::{layout::Engine, model::Document, pdf::Op};
use std::path::Path;

#[test]
fn all_math_question_starts_match_reference_pages_and_baselines() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("2단_수학시험지_변환검증_7826.hwpx");
    if !input.exists() {
        return;
    }
    let fonts = Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !fonts.exists() {
        return;
    }
    let d = Document::open(&input).unwrap();
    let mut e = Engine::new(&d, fonts);
    e.render().unwrap();
    let expected = [
        (1, 181.314),
        (1, 438.956),
        (1, 197.287),
        (1, 535.084),
        (2, 122.274),
        (2, 458.522),
        (2, 122.274),
        (2, 467.887),
        (3, 122.274),
        (3, 391.804),
        (3, 122.274),
        (3, 503.398),
        (4, 122.274),
        (4, 138.357),
        (5, 138.357),
        (5, 505.688),
        (5, 138.357),
        (5, 505.564),
        (6, 138.357),
        (6, 459.715),
        (6, 170.167),
        (6, 516.602),
        (7, 195.724),
        (7, 505.688),
        (7, 186.359),
        (7, 550.797),
    ];
    for (i, (page, baseline)) in expected.into_iter().enumerate() {
        let label = format!("{}.", i + 1);
        let matches: Vec<_> = e
            .pages
            .iter()
            .enumerate()
            .flat_map(|(p, page)| {
                page.ops.iter().filter_map(move |op| {
                    if let Op::Text { text, x, y, .. } | Op::Shaped { text, x, y, .. } = op {
                        Some((p + 1, text, *x, *y))
                    } else {
                        None
                    }
                })
            })
            .filter(|(_, text, x, _)| {
                **text == label && ((*x - 42.52).abs() < 0.1 || (*x - 308.94).abs() < 0.1)
            })
            .collect();
        assert_eq!(matches.len(), 1, "{label}");
        assert_eq!(matches[0].0, page, "{label}");
        assert!(
            (matches[0].3 - baseline).abs() < 0.15,
            "{label}: {} vs {baseline}",
            matches[0].3
        );
    }
    assert_eq!(e.pages.len(), 9);
}
