//! Structured operators from Hancom equation format rev 1.3: roots, arrays and cases.
use crate::{
    math::{MathBox, group, layout_inner, moved},
    pdf::{Fonts, Op},
};
use anyhow::Result;
fn split(s: &str, sep: char) -> Vec<&str> {
    let mut depth = 0;
    let mut start = 0;
    let mut out = vec![];
    for (i, c) in s.char_indices() {
        if c == '{' {
            depth += 1
        } else if c == '}' {
            depth -= 1
        } else if c == sep && depth == 0 {
            out.push(&s[start..i]);
            start = i + c.len_utf8();
        }
    }
    out.push(&s[start..]);
    out
}
pub fn parse(
    tail: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
    style: crate::math::style::Style,
) -> Result<Option<(MathBox, usize)>> {
    let lower = tail.to_ascii_lowercase();
    if lower.starts_with("sqrt") || lower.starts_with("root") {
        let mut degree = None;
        let (mut body, mut rest) = group(&tail[4..]);
        if lower.starts_with("root") {
            degree = Some(body);
            let next = rest.trim_start();
            (body, rest) = group(next.strip_prefix("of").unwrap_or(next));
        }
        let b = layout_inner(body, size, fonts, depth, style)?;
        let top = -b.ascent - size * 0.12;
        let mut out = MathBox {
            width: b.width + size * 0.65,
            ascent: -top,
            descent: b.descent,
            ops: vec![Op::Path {
                points: vec![
                    (0., -size * 0.2),
                    (size * 0.15, 0.),
                    (size * 0.1, top),
                    (size * 0.6 + b.width, top),
                ],
                closed: false,
                width: 0.6,
                color: style.color,
                fill: None,
            }],
        };
        out.ops
            .extend(b.ops.into_iter().map(|o| moved(o, size * 0.6, 0.)));
        if let Some(d) = degree {
            let index = layout_inner(d, size * 0.45, fonts, depth, style)?;
            out.ops
                .extend(index.ops.into_iter().map(|o| moved(o, 0., top * 0.55)));
        }
        return Ok(Some((out, tail.len() - rest.len())));
    }
    let Some(kind) = [
        "pmatrix", "bmatrix", "dmatrix", "matrix", "cases", "eqalign", "lpile", "rpile", "pile",
    ]
    .iter()
    .find(|k| lower.starts_with(**k)) else {
        return Ok(None);
    };
    let (body, rest) = group(&tail[kind.len()..]);
    let mut columns = Vec::new();
    let mut cursor = body.trim();
    while let Some(key) = ["lcol", "rcol", "col"].into_iter().find(|key| {
        cursor
            .get(..key.len())
            .is_some_and(|s| s.eq_ignore_ascii_case(key))
            && cursor[key.len()..].starts_with([' ', '{'])
    }) {
        let (content, remaining) = group(&cursor[key.len()..]);
        columns.push((key, split(content, '#')));
        cursor = remaining.trim();
    }
    let matrix_rows: Vec<Vec<&str>> = if !columns.is_empty() && cursor.is_empty() {
        let count = columns.iter().map(|(_, c)| c.len()).max().unwrap_or(0);
        (0..count)
            .map(|i| {
                columns
                    .iter()
                    .map(|(_, c)| c.get(i).copied().unwrap_or(""))
                    .collect()
            })
            .collect()
    } else {
        columns.clear();
        split(body, '#')
            .into_iter()
            .map(|r| split(r, '&'))
            .collect()
    };
    let mut rows = vec![];
    let mut widths = vec![];
    let mut heights = vec![];
    for row in matrix_rows {
        let mut cells = vec![];
        let mut ascent = 0f64;
        let mut descent = 0f64;
        for (i, cell) in row.iter().enumerate() {
            let b = layout_inner(cell, size, fonts, depth, style)?;
            if i >= widths.len() {
                widths.push(0f64);
            }
            widths[i] = widths[i].max(b.width);
            ascent = ascent.max(b.ascent);
            descent = descent.max(b.descent);
            cells.push(b);
        }
        rows.push(cells);
        heights.push((ascent, descent));
    }
    let padding = if *kind == "matrix" || kind.ends_with("pile") || *kind == "eqalign" {
        0.
    } else {
        size * 0.5
    };
    let width = widths.iter().sum::<f64>()
        + size * 0.28 * widths.len().saturating_sub(1) as f64
        + padding * 2.;
    let total = heights.iter().map(|(a, d)| a + d).sum::<f64>()
        + size * 0.1 * rows.len().saturating_sub(1) as f64;
    let ascent = total / 2. + size * 0.25;
    let mut out = MathBox {
        width,
        ascent,
        descent: (total - ascent).max(0.),
        ops: vec![],
    };
    let mut yy = -ascent;
    for (i, row) in rows.into_iter().enumerate() {
        let mut xx = padding;
        for (j, cell) in row.into_iter().enumerate() {
            let dx = if matches!(*kind, "cases" | "lpile")
                || columns.get(j).is_some_and(|c| c.0 == "lcol")
            {
                0.
            } else if *kind == "rpile" || columns.get(j).is_some_and(|c| c.0 == "rcol") {
                widths[j] - cell.width
            } else {
                (widths[j] - cell.width) / 2.
            };
            out.ops.extend(
                cell.ops
                    .into_iter()
                    .map(|o| moved(o, xx + dx, yy + heights[i].0)),
            );
            xx += widths[j] + size * 0.28;
        }
        yy += heights[i].0 + heights[i].1 + size * 0.1;
    }
    if padding > 0. {
        for side in [0., width] {
            if *kind == "cases" && side > 0. {
                continue;
            }
            if *kind == "cases" {
                let font = fonts.get("times.ttf")?;
                let face = ttf_parser::Face::parse(&fonts.list[font].bytes, 0)?;
                if let Some(bb) = face
                    .glyph_index('{')
                    .and_then(|g| face.glyph_bounding_box(g))
                {
                    let u = total / (bb.y_max as f64 - bb.y_min as f64).max(1.);
                    let glyph_size = u * face.units_per_em() as f64;
                    out.ops.push(Op::Text {
                        x: side,
                        y: -ascent + bb.y_max as f64 * u,
                        text: "{".into(),
                        font,
                        size: glyph_size,
                        scale: padding * 0.7 / fonts.width(font, "{", glyph_size).max(0.01),
                        color: style.color,
                    });
                }
                continue;
            }
            let inset = if side == 0. {
                size * 0.22
            } else {
                -size * 0.22
            };
            let points = if *kind == "dmatrix" {
                vec![(side, -ascent), (side, total - ascent)]
            } else if *kind == "pmatrix" {
                (0..=20)
                    .map(|i| {
                        let t = i as f64 / 20.;
                        (side + inset * (2. * t - 1.).powi(2), -ascent + total * t)
                    })
                    .collect()
            } else {
                vec![
                    (side + inset, -ascent),
                    (side, -ascent),
                    (side, total - ascent),
                    (side + inset, total - ascent),
                ]
            };
            out.ops.push(Op::Path {
                points,
                closed: false,
                width: 0.5,
                color: style.color,
                fill: None,
            });
        }
    }
    Ok(Some((out, tail.len() - rest.len())))
}
