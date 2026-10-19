//! Large operators and their attached limits, before ordinary symbol tokenization.
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
    let Some((key, symbol, integral)) = [
        ("oint", "∮", true),
        ("int", "∫", true),
        ("sum", "∑", false),
        ("prod", "∏", false),
        ("union", "⋃", false),
        ("inter", "⋂", false),
    ]
    .into_iter()
    .filter(|(k, _, _)| {
        tail.get(..k.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(k))
    })
    .max_by_key(|(k, _, _)| k.len()) else {
        return Ok(None);
    };
    let mut rest = &tail[key.len()..];
    let mut upper = None;
    let mut lower = None;
    loop {
        let next = rest.trim_start();
        let Some(c) = next.chars().next() else { break };
        if c != '_' && c != '^' {
            break;
        }
        let input = &next[1..];
        let (body, after) = if input.starts_with('{') {
            group(input)
        } else {
            let end = input
                .char_indices()
                .find_map(|(i, c)| {
                    (c.is_whitespace() || (i > 0 && matches!(c, '_' | '^'))).then_some(i)
                })
                .unwrap_or(input.len());
            (&input[..end], &input[end..])
        };
        let b = layout_inner(body, size * 0.68, fonts, depth + 1, style)?;
        if c == '_' {
            lower = Some(b);
        } else {
            upper = Some(b);
        }
        rest = after;
    }
    let mut font = if integral {
        if let Some(font) = fonts.family("Cambria Math", style.bold, false)? {
            font
        } else {
            style.font(fonts, symbol.chars().next().unwrap())?
        }
    } else {
        style.font(fonts, symbol.chars().next().unwrap())?
    };
    if !fonts.has(font, symbol.chars().next().unwrap()) {
        font = fonts.get("seguisym.ttf")?;
    }
    let face = ttf_parser::Face::parse(&fonts.list[font].bytes, 0)?;
    let bounds = face
        .glyph_index(symbol.chars().next().unwrap())
        .and_then(|g| face.glyph_bounding_box(g));
    let (units, ink_height) = bounds
        .map(|b| {
            (
                face.units_per_em() as f64,
                (b.y_max as f64 - b.y_min as f64).max(1.),
            )
        })
        .unwrap_or((1000., 1000.));
    let glyph_size = size * if integral { 2. } else { 1.23 } * units / ink_height;
    let sw = size * if integral { 1.23 } else { 1.3 };
    let scale = sw / fonts.width(font, symbol, glyph_size).max(0.01);
    let u = glyph_size / units;
    let (top, bottom) = bounds
        .map(|b| (-b.y_max as f64 * u, -b.y_min as f64 * u))
        .unwrap_or((-glyph_size * 0.85, glyph_size * 0.2));
    let base = -size / 3. - (top + bottom) / 2.;
    let (top, bottom) = (top + base, bottom + base);
    let lw = lower.as_ref().map(|b| b.width).unwrap_or(0.);
    let uw = upper.as_ref().map(|b| b.width).unwrap_or(0.);
    let width = if integral {
        sw + uw.max(lw) * 0.85
    } else {
        sw.max(lw).max(uw)
    };
    let sx = if integral { 0. } else { (width - sw) / 2. };
    let mut out = MathBox {
        width: width + size * 0.12,
        ascent: (-top).max(size * 0.85),
        descent: bottom.max(size * 0.2),
        ops: vec![Op::Text {
            x: sx,
            y: base,
            text: symbol.into(),
            font,
            size: glyph_size,
            scale,
            color: style.color,
        }],
    };
    if let Some(b) = upper {
        let x = if integral { sw } else { (width - b.width) / 2. };
        let y = if integral {
            top + size * 0.34
        } else {
            top - size * 0.03 - b.descent
        };
        out.ascent = out.ascent.max(b.ascent - y);
        out.ops.extend(b.ops.into_iter().map(|o| moved(o, x, y)));
    }
    if let Some(b) = lower {
        let x = if integral {
            sw * 0.65
        } else {
            (width - b.width) / 2.
        };
        let y = if integral {
            bottom + size * 0.23
        } else {
            bottom + size * 0.03 + b.ascent
        };
        out.descent = out.descent.max(y + b.descent);
        out.ops.extend(b.ops.into_iter().map(|o| moved(o, x, y)));
    }
    Ok(Some((out, tail.len() - rest.len())))
}
