//! Unruled stacks and binomial coefficients.
use crate::{
    math::{MathBox, group, layout_inner, moved},
    pdf::{Fonts, Op},
};
use anyhow::Result;
pub(crate) fn compose(
    top: &str,
    bottom: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
    brackets: bool,
) -> Result<MathBox> {
    let a = layout_inner(top, size * 0.85, fonts, depth)?;
    let b = layout_inner(bottom, size * 0.85, fonts, depth)?;
    let gap = size * 0.15;
    let padding = if brackets { size * 0.4 } else { size * 0.1 };
    let width = a.width.max(b.width) + padding * 2.;
    let ay = -gap - a.descent;
    let by = gap + b.ascent;
    let mut result = MathBox {
        width,
        ascent: a.ascent - ay,
        descent: by + b.descent,
        ..Default::default()
    };
    result.ops.extend(
        a.ops
            .into_iter()
            .map(|o| moved(o, (width - a.width) / 2., ay)),
    );
    result.ops.extend(
        b.ops
            .into_iter()
            .map(|o| moved(o, (width - b.width) / 2., by)),
    );
    if brackets {
        let h = result.ascent + result.descent;
        for right in [false, true] {
            let edge = if right { width } else { 0. };
            let sign = if right { -1. } else { 1. };
            result.ops.push(Op::Bezier {
                start: (edge + sign * padding * 0.7, -result.ascent),
                segments: vec![[
                    edge + sign * padding * 0.05,
                    -result.ascent + h * 0.2,
                    edge + sign * padding * 0.05,
                    -result.ascent + h * 0.8,
                    edge + sign * padding * 0.7,
                    result.descent,
                ]],
                closed: false,
                width: 0.5,
                color: [0.; 3],
                fill: None,
            });
        }
    }
    Ok(result)
}
pub(crate) fn prefix(
    tail: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
) -> Result<Option<(MathBox, usize)>> {
    if !tail
        .get(..5)
        .is_some_and(|s| s.eq_ignore_ascii_case("binom"))
        || !tail[5..]
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || c == '{')
    {
        return Ok(None);
    }
    let (a, rest) = group(&tail[5..]);
    let (b, rest) = group(rest);
    Ok(Some((
        compose(a, b, size, fonts, depth, true)?,
        tail.len() - rest.len(),
    )))
}
