//! Scoped equation accents from Hancom equation format revision 1.3.
use crate::{
    math::{MathBox, group, layout_inner, moved},
    pdf::{Fonts, Op},
};
use anyhow::Result;
pub(crate) fn parse(
    tail: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
) -> Result<Option<(MathBox, usize)>> {
    let lower = tail.to_ascii_lowercase();
    let Some(key) = [
        "overline",
        "underline",
        "check",
        "tilde",
        "acute",
        "grave",
        "ddot",
        "hat",
        "dot",
        "bar",
        "vec",
        "dyad",
        "under",
    ]
    .into_iter()
    .find(|key| {
        lower.starts_with(key)
            && tail[key.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_whitespace() || c == '{')
    }) else {
        return Ok(None);
    };
    let (body, rest) = group(&tail[key.len()..]);
    let mut b = layout_inner(body, size, fonts, depth)?;
    let width = b.width.max(size * 0.4);
    let dx = (width - b.width) / 2.;
    b.ops = b.ops.into_iter().map(|o| moved(o, dx, 0.)).collect();
    b.width = width;
    let top = -b.ascent - size * 0.2;
    let h = size * 0.18;
    let line = |x, y, x2, y2| Op::Line {
        x,
        y,
        x2,
        y2,
        width: 0.45,
        color: [0.; 3],
    };
    let path = |points| Op::Path {
        points,
        closed: false,
        width: 0.45,
        color: [0.; 3],
        fill: None,
    };
    match key {
        "under" | "underline" => {
            let y = b.descent + size * 0.16;
            b.ops.push(line(0., y, width, y));
            b.descent = y + 0.3;
        }
        "bar" | "overline" => {
            b.ops.push(line(0., top, width, top));
            b.ascent = -top + 0.3;
        }
        "hat" | "check" => {
            let sign = if key == "hat" { 1. } else { -1. };
            b.ops.push(path(vec![
                (0., top + sign * h / 2.),
                (width / 2., top - sign * h / 2.),
                (width, top + sign * h / 2.),
            ]));
            b.ascent = -top + h / 2. + 0.3;
        }
        "acute" | "grave" => {
            let direction = if key == "acute" { 1. } else { -1. };
            b.ops.push(line(
                width / 2. - direction * h / 2.,
                top + h / 2.,
                width / 2. + direction * h / 2.,
                top - h / 2.,
            ));
            b.ascent = -top + h / 2. + 0.3;
        }
        "tilde" => {
            b.ops.push(Op::Bezier {
                start: (0., top + h / 3.),
                segments: vec![[
                    width * 0.25,
                    top - h,
                    width * 0.75,
                    top + h,
                    width,
                    top - h / 3.,
                ]],
                closed: false,
                width: 0.45,
                color: [0.; 3],
                fill: None,
            });
            b.ascent = -top + h + 0.3;
        }
        "dot" | "ddot" => {
            let centers = if key == "dot" {
                vec![width / 2.]
            } else {
                vec![width * 0.35, width * 0.65]
            };
            let radius = (size * 0.045).max(0.35);
            for x in centers {
                b.ops.push(Op::Path {
                    points: (0..16)
                        .map(|i| {
                            let a = i as f64 * std::f64::consts::TAU / 16.;
                            (x + radius * a.cos(), top + radius * a.sin())
                        })
                        .collect(),
                    closed: true,
                    width: 0.1,
                    color: [0.; 3],
                    fill: Some([0.; 3]),
                });
            }
            b.ascent = -top + radius + 0.2;
        }
        "vec" | "dyad" => {
            let arrow = (size * 0.22).min(width * 0.4);
            b.ops.push(line(0., top, width, top));
            b.ops.push(path(vec![
                (width - arrow, top - h / 2.),
                (width, top),
                (width - arrow, top + h / 2.),
            ]));
            if key == "dyad" {
                b.ops.push(path(vec![
                    (arrow, top - h / 2.),
                    (0., top),
                    (arrow, top + h / 2.),
                ]));
            }
            b.ascent = -top + h / 2. + 0.3;
        }
        _ => unreachable!(),
    }
    Ok(Some((b, tail.len() - rest.len())))
}
