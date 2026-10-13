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
        let b = layout_inner(body, size, fonts, depth)?;
        let top = -b.ascent - size * 0.12;
        let mut out = MathBox {
            width: b.width + size * 0.65,
            ascent: -top,
            descent: b.descent,
            ops: vec![Op::Path {
                points: vec![
                    (0., -size * 0.2),
                    (size * 0.15, 0.),
                    (size * 0.35, top),
                    (size * 0.6 + b.width, top),
                ],
                closed: false,
                width: 0.6,
                color: [0.; 3],
                fill: None,
            }],
        };
        out.ops
            .extend(b.ops.into_iter().map(|o| moved(o, size * 0.6, 0.)));
        if let Some(d) = degree {
            let index = layout_inner(d, size * 0.45, fonts, depth)?;
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
    let mut rows = vec![];
    let mut widths = vec![];
    let mut heights = vec![];
    for row in split(body, '#') {
        let mut cells = vec![];
        let mut ascent = 0f64;
        let mut descent = 0f64;
        for (i, cell) in split(row, '&').iter().enumerate() {
            let b = layout_inner(cell, size * 0.9, fonts, depth)?;
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
        + size * 0.6 * widths.len().saturating_sub(1) as f64
        + padding * 2.;
    let total = heights.iter().map(|(a, d)| a + d).sum::<f64>()
        + size * 0.35 * rows.len().saturating_sub(1) as f64;
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
            let dx = if matches!(*kind, "cases" | "lpile") {
                0.
            } else if *kind == "rpile" {
                widths[j] - cell.width
            } else {
                (widths[j] - cell.width) / 2.
            };
            out.ops.extend(
                cell.ops
                    .into_iter()
                    .map(|o| moved(o, xx + dx, yy + heights[i].0)),
            );
            xx += widths[j] + size * 0.6;
        }
        yy += heights[i].0 + heights[i].1 + size * 0.35;
    }
    if padding > 0. {
        for side in [0., width] {
            if *kind == "cases" && side > 0. {
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
                color: [0.; 3],
                fill: None,
            });
        }
    }
    Ok(Some((out, tail.len() - rest.len())))
}
