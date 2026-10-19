use hwp_engine::{
    math,
    pdf::{Fonts, Op},
};
fn fonts() -> Fonts {
    Fonts::new(std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    }))
}
#[test]
fn math_styles_are_scoped_and_inherited_by_structures() {
    let mut f = fonts();
    let b = math::layout("a {rm b} c", 18., &mut f).unwrap();
    let styles: Vec<_> = b
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Text { text, font, .. } = o {
                Some((
                    text.as_str(),
                    ttf_parser::Face::parse(&f.list[*font].bytes, 0)
                        .unwrap()
                        .is_italic(),
                ))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(styles, vec![("a", true), ("b", false), ("c", true)]);
    let b = math::layout("bold {a over b}", 18., &mut f).unwrap();
    for o in b.ops {
        if let Op::Text { font, size, .. } = o {
            let face = ttf_parser::Face::parse(&f.list[font].bytes, 0).unwrap();
            assert!(face.is_bold() && face.is_italic());
            assert_eq!(size, 18.);
        }
    }
}
#[test]
fn upper_and_lower_scripts_share_one_attachment_column() {
    let mut f = fonts();
    let b = math::layout("x_{a}^{b}", 18., &mut f).unwrap();
    let mut x = Vec::new();
    for o in b.ops {
        if let Op::Text { x: xx, text, .. } = o
            && matches!(text.as_str(), "a" | "b")
        {
            x.push(xx);
        }
    }
    assert_eq!(x.len(), 2);
    assert!((x[0] - x[1]).abs() < 0.001);
}

#[test]
fn named_functions_quotes_spaces_and_color_have_distinct_semantics() {
    let mut f = fonts();
    let b = math::layout("sin x + {COLOR {255,0,255} y} + z", 18., &mut f).unwrap();
    for op in &b.ops {
        if let Op::Text {
            text, font, color, ..
        } = op
        {
            if text == "sin" {
                assert!(
                    !ttf_parser::Face::parse(&f.list[*font].bytes, 0)
                        .unwrap()
                        .is_italic()
                );
            }
            if text == "y" {
                assert_eq!(*color, [1., 0., 1.]);
            }
            if text == "z" {
                assert_eq!(*color, [0.; 3]);
            }
        }
    }
    let b = math::layout("\"sin over x\"", 18., &mut f).unwrap();
    let text: String = b
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Text { text, .. } = o {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(text, "sin over x");
    let a = math::layout("~", 18., &mut f).unwrap();
    let b = math::layout("`", 18., &mut f).unwrap();
    assert!((a.width - 4. * b.width).abs() < 0.001);
    let b = math::layout("int_0^1 x", 18., &mut f).unwrap();
    assert!(
        b.ops
            .iter()
            .any(|o| matches!(o,Op::Text{text,..} if text=="∫"))
    );
}

#[test]
fn unbraced_words_follow_hancom_scope_while_braces_delimit_scripts() {
    let mut f = fonts();
    let b = math::layout("a^2+b^2", 18., &mut f).unwrap();
    assert!(
        b.ops
            .iter()
            .any(|o| matches!(o,Op::Text{text,size,..} if text=="b" && (*size-12.24).abs()<0.001))
    );
    let b = math::layout("a^{2}+b^{2}", 18., &mut f).unwrap();
    assert!(
        b.ops
            .iter()
            .any(|o| matches!(o,Op::Text{text,size,..} if text=="b" && *size==18.))
    );
}

#[test]
fn sums_have_large_symbols_with_centered_limits_and_normal_body_size() {
    let mut f = fonts();
    let b = math::layout("sum_{i=1}^{n} k^2", 18., &mut f).unwrap();
    let (font, size) = b
        .ops
        .iter()
        .find_map(|o| {
            if let Op::Text {
                text, font, size, ..
            } = o
            {
                (text == "∑").then_some((*font, *size))
            } else {
                None
            }
        })
        .unwrap();
    let face = ttf_parser::Face::parse(&f.list[font].bytes, 0).unwrap();
    let bb = face
        .glyph_bounding_box(face.glyph_index('∑').unwrap())
        .unwrap();
    assert!(
        ((bb.y_max as f64 - bb.y_min as f64) * size / face.units_per_em() as f64 - 18. * 1.23)
            .abs()
            < 0.001
    );
    assert!(
        b.ops
            .iter()
            .any(|o| matches!(o,Op::Text{text,size,..} if text=="k" && *size==18.))
    );
    let n = b
        .ops
        .iter()
        .find_map(|o| {
            if let Op::Text { text, y, .. } = o {
                (text == "n").then_some(*y)
            } else {
                None
            }
        })
        .unwrap();
    let i = b
        .ops
        .iter()
        .find_map(|o| {
            if let Op::Text { text, y, .. } = o {
                (text == "i").then_some(*y)
            } else {
                None
            }
        })
        .unwrap();
    assert!(n < 0. && i > 0.);
}
