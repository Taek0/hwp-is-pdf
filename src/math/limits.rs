//! Roman limit operators with centered upper/lower limits.
use super::{MathBox, group, layout_inner, moved, style::Style};
use crate::pdf::{Fonts, Op};
use anyhow::Result;
pub(super) fn parse(
    tail: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
    style: Style,
) -> Result<Option<(MathBox, usize)>> {
    let Some(key) = ["lim", "Lim"].into_iter().find(|key| tail.starts_with(key)) else {
        return Ok(None);
    };
    let mut rest = &tail[key.len()..];
    if !rest.trim_start().starts_with(['_', '^']) {
        return Ok(None);
    }
    let (mut lower, mut upper) = (None, None);
    loop {
        let next = rest.trim_start();
        let Some(c) = next.chars().next() else { break };
        if !matches!(c, '_' | '^') {
            break;
        }
        let (body, after) = group(&next[1..]);
        let child = layout_inner(body, size * 0.68, fonts, depth + 1, style)?;
        if c == '_' {
            lower = Some(child);
        } else {
            upper = Some(child);
        }
        rest = after;
    }
    let mut roman = style;
    roman.italic = Some(false);
    let font = roman.font(fonts, 'l')?;
    let label_size = size * 1.2;
    let baseline = size * 0.055;
    let label_width = fonts.width(font, key, label_size);
    let width = label_width
        .max(lower.as_ref().map(|b| b.width).unwrap_or(0.))
        .max(upper.as_ref().map(|b| b.width).unwrap_or(0.));
    let mut out = MathBox {
        width: width + size * 0.15,
        ascent: label_size * 0.85 - baseline,
        descent: size * 0.2,
        ops: vec![Op::Text {
            x: (width - label_width) / 2.,
            y: baseline,
            text: key.into(),
            font,
            size: label_size,
            scale: 1.,
            color: style.color,
        }],
    };
    if let Some(b) = lower {
        let y = baseline + size * 0.2 + b.ascent;
        out.descent = out.descent.max(y + b.descent);
        out.ops.extend(
            b.ops
                .into_iter()
                .map(|op| moved(op, (width - b.width) / 2., y)),
        );
    }
    if let Some(b) = upper {
        let y = -size * 0.88 - b.descent;
        out.ascent = out.ascent.max(b.ascent - y);
        out.ops.extend(
            b.ops
                .into_iter()
                .map(|op| moved(op, (width - b.width) / 2., y)),
        );
    }
    Ok(Some((out, tail.len() - rest.len())))
}
