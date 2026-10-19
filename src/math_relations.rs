//! Explicit scale scopes and labelled relation arrows from the HWP equation grammar.
use crate::{
    math::{MathBox, group, layout_inner, moved},
    pdf::{Fonts, Op},
};
use anyhow::{Result, ensure};
pub(crate) fn parse(
    tail: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
    style: crate::math::style::Style,
) -> Result<Option<(MathBox, usize)>> {
    let lower = tail.to_ascii_lowercase();
    if lower.starts_with("scale ") {
        let (ratio, rest) = group(&tail[5..]);
        let Ok(ratio) = ratio.parse::<f64>() else {
            return Ok(None);
        };
        ensure!(
            ratio.is_finite() && ratio > 0. && ratio <= 1000.,
            "Invalid equation scale"
        );
        let (body, rest) = group(rest);
        let b = layout_inner(body, size * ratio / 100., fonts, depth, style)?;
        return Ok(Some((b, tail.len() - rest.len())));
    }
    let (key, bottom) = if lower.starts_with("buildrel ") {
        ("buildrel", false)
    } else if lower.starts_with("rel ") {
        ("rel", true)
    } else {
        return Ok(None);
    };
    let (arrow, rest) = group(&tail[key.len()..]);
    if !matches!(arrow, "->" | "<-" | "<->" | "=>" | "<=" | "<=>") {
        return Ok(None);
    }
    let (top, rest) = group(rest);
    let (below, rest) = if bottom { group(rest) } else { ("", rest) };
    let a = layout_inner(top, size, fonts, depth, style)?;
    let b = layout_inner(below, size, fonts, depth, style)?;
    let width = a.width.max(b.width).max(size * 1.6) + size * 0.4;
    let ay = -size * 0.5 - a.descent;
    let by = b.ascent - size * 0.15;
    let mut result = MathBox {
        width,
        ascent: (a.ascent - ay).max(size * 0.85),
        descent: if bottom { by + b.descent } else { size * 0.2 },
        ops: Vec::new(),
    };
    result.ops.extend(
        a.ops
            .into_iter()
            .map(|o| moved(o, (width - a.width) / 2., ay)),
    );
    if bottom {
        result.ops.extend(
            b.ops
                .into_iter()
                .map(|o| moved(o, (width - b.width) / 2., by)),
        );
    }
    let y = -size * 0.25;
    let double = arrow.contains('=');
    for offset in if double {
        vec![-size * 0.055, size * 0.055]
    } else {
        vec![0.]
    } {
        result.ops.push(Op::Line {
            x: 0.,
            y: y + offset,
            x2: width,
            y2: y + offset,
            width: 0.5,
            color: style.color,
        });
    }
    for (tip, sign) in [(0., 1.), (width, -1.)] {
        if (tip == 0. && arrow.starts_with('<')) || (tip > 0. && arrow.ends_with('>')) {
            result.ops.push(Op::Path {
                points: vec![
                    (tip + sign * size * 0.25, y - size * 0.14),
                    (tip, y),
                    (tip + sign * size * 0.25, y + size * 0.14),
                ],
                closed: false,
                width: 0.5,
                color: style.color,
                fill: None,
            });
        }
    }
    Ok(Some((result, tail.len() - rest.len())))
}
