//! Position measured equation drawing operations without changing their semantics.
use super::{MathBox, Op};
pub(crate) fn moved(op: Op, dx: f64, dy: f64) -> Op {
    match op {
        Op::Bezier {
            start,
            segments,
            closed,
            width,
            color,
            fill,
        } => Op::Bezier {
            start: (start.0 + dx, start.1 + dy),
            segments: segments
                .into_iter()
                .map(|s| {
                    [
                        s[0] + dx,
                        s[1] + dy,
                        s[2] + dx,
                        s[3] + dy,
                        s[4] + dx,
                        s[5] + dy,
                    ]
                })
                .collect(),
            closed,
            width,
            color,
            fill,
        },
        Op::Shaped {
            x,
            y,
            text,
            font,
            size,
            scale,
            color,
            glyphs,
        } => Op::Shaped {
            x: x + dx,
            y: y + dy,
            text,
            font,
            size,
            scale,
            color,
            glyphs,
        },
        Op::Path {
            points,
            closed,
            width,
            color,
            fill,
        } => Op::Path {
            points: points.into_iter().map(|(x, y)| (x + dx, y + dy)).collect(),
            closed,
            width,
            color,
            fill,
        },
        Op::Text {
            x,
            y,
            text,
            font,
            size,
            scale,
            color,
        } => Op::Text {
            x: x + dx,
            y: y + dy,
            text,
            font,
            size,
            scale,
            color,
        },
        Op::Line {
            x,
            y,
            x2,
            y2,
            width,
            color,
        } => Op::Line {
            x: x + dx,
            y: y + dy,
            x2: x2 + dx,
            y2: y2 + dy,
            width,
            color,
        },
        other => other,
    }
}
pub fn place(b: MathBox, x: f64, baseline: f64, w: f64, h: f64) -> Vec<Op> {
    let sx = if b.width > 0. { w / b.width } else { 1. };
    let sy = if b.ascent + b.descent > h {
        h / (b.ascent + b.descent)
    } else {
        1.
    };
    b.ops
        .into_iter()
        .map(|op| match op {
            Op::Bezier {
                start,
                segments,
                closed,
                width,
                color,
                fill,
            } => Op::Bezier {
                start: (x + start.0 * sx, baseline + start.1 * sy),
                segments: segments
                    .into_iter()
                    .map(|s| {
                        [
                            x + s[0] * sx,
                            baseline + s[1] * sy,
                            x + s[2] * sx,
                            baseline + s[3] * sy,
                            x + s[4] * sx,
                            baseline + s[5] * sy,
                        ]
                    })
                    .collect(),
                closed,
                width,
                color,
                fill,
            },
            Op::Shaped {
                x: xx,
                y,
                text,
                font,
                size,
                scale,
                color,
                mut glyphs,
            } => {
                for g in &mut glyphs {
                    g.x *= sx;
                    g.y *= sy;
                }
                Op::Shaped {
                    x: x + xx * sx,
                    y: baseline + y * sy,
                    text,
                    font,
                    size: size * sy,
                    scale: scale * sx / sy,
                    color,
                    glyphs,
                }
            }
            Op::Path {
                points,
                closed,
                width,
                color,
                fill,
            } => Op::Path {
                points: points
                    .into_iter()
                    .map(|(xx, yy)| (x + xx * sx, baseline + yy * sy))
                    .collect(),
                closed,
                width,
                color,
                fill,
            },
            Op::Text {
                x: xx,
                y,
                text,
                font,
                size,
                scale,
                color,
            } => Op::Text {
                x: x + xx * sx,
                y: baseline + y * sy,
                text,
                font,
                size: size * sy,
                scale: scale * sx / sy,
                color,
            },
            Op::Line {
                x: xx,
                y,
                x2,
                y2,
                width,
                color,
            } => Op::Line {
                x: x + xx * sx,
                y: baseline + y * sy,
                x2: x + x2 * sx,
                y2: baseline + y2 * sy,
                width,
                color,
            },
            o => o,
        })
        .collect()
}
