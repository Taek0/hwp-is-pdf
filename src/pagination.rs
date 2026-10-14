//! Indivisible vector groups used when splitting mixed-content tables.
use crate::{
    graphics::{self, Matrix},
    pdf::{Fonts, Op},
};
use anyhow::{Result, ensure};
pub(crate) struct Atom {
    pub start: usize,
    pub end: usize,
    pub top: f64,
    pub bottom: f64,
}
fn bounds(op: &Op, fonts: &Fonts) -> Option<[f64; 4]> {
    match op {
        Op::Text {
            x,
            y,
            size,
            scale,
            font,
            text,
            ..
        } => Some([
            *x - size * 0.2,
            y - size * 1.2,
            x + fonts.width(*font, text, *size) * scale + size * 0.2,
            y + size * 0.4,
        ]),
        Op::Rect { x, y, w, h, .. }
        | Op::Image { x, y, w, h, .. }
        | Op::ImageClip { x, y, w, h, .. } => Some([*x, *y, x + w, y + h]),
        Op::Line {
            x,
            y,
            x2,
            y2,
            width,
            ..
        } => Some([
            x.min(*x2) - width / 2.,
            y.min(*y2) - width / 2.,
            x.max(*x2) + width / 2.,
            y.max(*y2) + width / 2.,
        ]),
        Op::Path { points, width, .. } => points_bounds(points, *width),
        Op::Bezier {
            start,
            segments,
            width,
            ..
        } => {
            let mut p = vec![*start];
            for s in segments {
                p.extend([(s[0], s[1]), (s[2], s[3]), (s[4], s[5])]);
            }
            points_bounds(&p, *width)
        }
        _ => None,
    }
}
fn points_bounds(p: &[(f64, f64)], width: f64) -> Option<[f64; 4]> {
    if p.is_empty() {
        return None;
    }
    Some([
        p.iter().map(|p| p.0).fold(f64::INFINITY, f64::min) - width / 2.,
        p.iter().map(|p| p.1).fold(f64::INFINITY, f64::min) - width / 2.,
        p.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max) + width / 2.,
        p.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) + width / 2.,
    ])
}
pub(crate) fn atoms(ops: &[Op], fonts: &Fonts) -> Result<Vec<Atom>> {
    let mut out = Vec::new();
    let mut stack: Vec<Matrix> = Vec::new();
    let mut matrix = graphics::IDENTITY;
    let mut start = 0;
    let mut top = f64::INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::TransformBegin { matrix: m } => {
                if stack.is_empty() {
                    start = i;
                    top = f64::INFINITY;
                    bottom = f64::NEG_INFINITY;
                }
                stack.push(matrix);
                matrix = graphics::compose(matrix, *m);
            }
            Op::TransformEnd => {
                matrix = stack
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("Unbalanced table transform"))?;
                if stack.is_empty() {
                    out.push(Atom {
                        start,
                        end: i + 1,
                        top: if top.is_finite() { top } else { 0. },
                        bottom: if bottom.is_finite() { bottom } else { 0. },
                    });
                }
            }
            _ => {
                let atomic = !stack.is_empty()
                    || match op {
                        Op::Image { .. } | Op::ImageClip { .. } | Op::Bezier { .. } => true,
                        Op::Path {
                            points,
                            closed,
                            fill,
                            ..
                        } => {
                            *closed
                                || fill.is_some()
                                || points.len() != 2
                                || (points[0].1 - points[1].1).abs() > 0.001
                        }
                        Op::Line { x, y, x2, y2, .. } => {
                            (x - x2).abs() > 0.001 && (y - y2).abs() > 0.001
                        }
                        _ => false,
                    };
                if atomic && let Some(b) = bounds(op, fonts) {
                    let r = graphics::rect(matrix, b);
                    if stack.is_empty() {
                        out.push(Atom {
                            start: i,
                            end: i + 1,
                            top: r[1],
                            bottom: r[3],
                        });
                    } else {
                        top = top.min(r[1]);
                        bottom = bottom.max(r[3]);
                    }
                }
            }
        }
    }
    ensure!(stack.is_empty(), "Unclosed table transform");
    Ok(out)
}
