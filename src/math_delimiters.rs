//! Stretch LEFT/RIGHT delimiters around the measured expression.
use crate::{
    math::{MathBox, layout_inner, moved},
    pdf::{Fonts, Op},
};
use anyhow::Result;
pub(crate) fn parse(
    tail: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
    style: crate::math::style::Style,
) -> Result<Option<(MathBox, usize)>> {
    if !tail
        .get(..4)
        .is_some_and(|s| s.eq_ignore_ascii_case("left"))
    {
        return Ok(None);
    }
    let after = tail[4..].trim_start();
    let Some(left) = after.chars().next() else {
        return Ok(None);
    };
    if !"([{<|.".contains(left) {
        return Ok(None);
    }
    let body = &after[left.len_utf8()..];
    let mut level = 1;
    let mut closing = None;
    for (i, _) in body.char_indices() {
        let t = &body[i..];
        if t.get(..4).is_some_and(|v| v.eq_ignore_ascii_case("left")) {
            level += 1;
        }
        if t.get(..5).is_some_and(|v| v.eq_ignore_ascii_case("right")) {
            level -= 1;
            if level == 0 {
                closing = Some(i);
                break;
            }
        }
    }
    let Some(at) = closing else { return Ok(None) };
    let rest = body[at + 5..].trim_start();
    let Some(right) = rest.chars().next() else {
        return Ok(None);
    };
    if !")]} >|.".contains(right) {
        return Ok(None);
    }
    let expression = layout_inner(&body[..at], size, fonts, depth, style)?;
    let pad = size * 0.625;
    let ascent = expression.ascent.max(size * 0.8);
    let descent = expression.descent.max(size * 0.2);
    let width = expression.width + 2. * pad;
    let mut result = MathBox {
        width,
        ascent,
        descent,
        ops: expression
            .ops
            .into_iter()
            .map(|o| moved(o, pad, 0.))
            .collect(),
    };
    for (symbol, x, sign) in [(left, 0., 1.), (right, width, -1.)] {
        if symbol == '.' {
            continue;
        }
        let font = fonts.get("times.ttf")?;
        let face = ttf_parser::Face::parse(&fonts.list[font].bytes, 0)?;
        if let Some(bounds) = face
            .glyph_index(symbol)
            .and_then(|id| face.glyph_bounding_box(id))
        {
            let height = (bounds.y_max as f64 - bounds.y_min as f64).max(1.);
            let scale_y = (ascent + descent) / height;
            let size = scale_y * face.units_per_em() as f64;
            let advance = fonts.width(font, &symbol.to_string(), size).max(0.01);
            let scale = pad / advance;
            let xx = if sign > 0. { x } else { x - pad };
            let baseline = -ascent + bounds.y_max as f64 * scale_y;
            result.ops.push(Op::Text {
                x: xx,
                y: baseline,
                text: symbol.to_string(),
                font,
                size,
                scale,
                color: style.color,
            });
        }
    }
    let remaining = &rest[right.len_utf8()..];
    Ok(Some((result, tail.len() - remaining.len())))
}
