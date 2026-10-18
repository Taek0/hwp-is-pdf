//! Drawing dash patterns in path distance, independent of curve tessellation.
use super::*;

pub(super) fn apply(ops: &mut Vec<Op>, start: usize, line: Option<&Node>) {
    let style = line.map(|n| n.a("style")).unwrap_or("SOLID");
    let pattern: &[f64] = match style {
        "DASH" => &[6., 3.],
        "DOT" => &[1., 2.],
        "DASH_DOT" => &[6., 2., 1., 2.],
        "DASH_DOT_DOT" => &[6., 2., 1., 2., 1., 2.],
        "LONG_DASH" => &[12., 3.],
        "NONE" => &[],
        _ => return,
    };
    let original: Vec<_> = ops.drain(start..).collect();
    for op in original {
        let (mut points, closed, width, color, filled) = match &op {
            Op::Line {
                x,
                y,
                x2,
                y2,
                width,
                color,
            } => (vec![(*x, *y), (*x2, *y2)], false, *width, *color, false),
            Op::Path {
                points,
                closed,
                width,
                color,
                fill,
            } => (points.clone(), *closed, *width, *color, fill.is_some()),
            Op::Bezier {
                start,
                segments,
                closed,
                width,
                color,
                fill,
            } => (
                crate::graphics::flatten_cubics(*start, segments),
                *closed,
                *width,
                *color,
                fill.is_some(),
            ),
            _ => {
                ops.push(op);
                continue;
            }
        };
        if filled {
            let mut fill = op.clone();
            match &mut fill {
                Op::Path { width, .. } | Op::Bezier { width, .. } => *width = 0.,
                _ => {}
            }
            ops.push(fill);
        }
        if style == "NONE" || width <= 0. {
            continue;
        }
        if closed
            && points.first() != points.last()
            && let Some(p) = points.first().copied()
        {
            points.push(p);
        }
        ops.extend(dashed(&points, pattern, width, color));
    }
}

fn dashed(points: &[(f64, f64)], pattern: &[f64], width: f64, color: [f64; 3]) -> Vec<Op> {
    let unit = width.max(0.5);
    let mut phase = 0;
    let mut remaining = pattern[0] * unit;
    let mut out = Vec::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let length = (b.0 - a.0).hypot(b.1 - a.1);
        if !length.is_finite() || length <= 1e-9 {
            continue;
        }
        let mut at = 0.;
        while at < length - 1e-9 {
            let next = (at + remaining).min(length);
            if phase % 2 == 0 {
                let point = |t: f64| {
                    (
                        a.0 + (b.0 - a.0) * t / length,
                        a.1 + (b.1 - a.1) * t / length,
                    )
                };
                let (p, q) = (point(at), point(next));
                out.push(Op::Line {
                    x: p.0,
                    y: p.1,
                    x2: q.0,
                    y2: q.1,
                    width,
                    color,
                });
            }
            remaining -= next - at;
            at = next;
            if remaining < 1e-9 {
                phase = (phase + 1) % pattern.len();
                remaining = pattern[phase] * unit;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dash_phase_crosses_vertices_and_zero_length_edges() {
        let ops = dashed(
            &[(0., 0.), (4., 0.), (4., 0.), (4., 8.)],
            &[6., 3.],
            1.,
            [0.; 3],
        );
        let lengths: Vec<_> = ops
            .iter()
            .map(|o| match o {
                Op::Line { x, y, x2, y2, .. } => (x2 - x).hypot(y2 - y),
                _ => 0.,
            })
            .collect();
        assert_eq!(lengths, vec![4., 2., 3.]);
        assert!(matches!(ops[2], Op::Line { y: 5., y2: 8., .. }));
    }
    #[test]
    fn invisible_outline_retains_fill_without_hairline() {
        let line = crate::model::xml(b"<lineShape style=\"NONE\"/>").unwrap();
        let mut ops = vec![Op::Path {
            points: vec![(0., 0.), (10., 0.), (10., 10.)],
            closed: true,
            width: 1.,
            color: [0.; 3],
            fill: Some([1.; 3]),
        }];
        apply(&mut ops, 0, Some(&line));
        assert_eq!(ops.len(), 1);
        assert!(matches!(
            ops[0],
            Op::Path {
                width: 0.,
                fill: Some(_),
                ..
            }
        ));
    }
}
