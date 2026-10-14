//! Hancom character underline styles (format table 25/30).
use crate::pdf::Op;
/// HWPX names are not HWP binary enum integers (notably DASH/DOT).
pub fn style(node: &crate::model::Node) -> Option<usize> {
    if !node.a("shape").is_empty() {
        return match node.a("shape") {
            "SOLID" => Some(0),
            "DASH" => Some(1),
            "DOT" => Some(2),
            "DASH_DOT" => Some(3),
            "DASH_DOT_DOT" => Some(4),
            "LONG_DASH" => Some(5),
            "CIRCLE" => Some(6),
            "DOUBLE_SLIM" => Some(7),
            "SLIM_THICK" => Some(8),
            "THICK_SLIM" => Some(9),
            "SLIM_THICK_SLIM" => Some(10),
            "WAVE" => Some(11),
            "DOUBLEWAVE" => Some(12),
            _ => None,
        };
    }
    let s = node.a("style").parse::<usize>().ok()?;
    (s <= 12).then_some(s)
}
pub fn underline(x: f64, end: f64, y: f64, style: usize, color: [f64; 3]) -> Vec<Op> {
    let mut out = Vec::new();
    if style == 11 || style == 12 {
        for offset in if style == 12 {
            &[0., 2.4][..]
        } else {
            &[0.][..]
        } {
            let mut segments = Vec::new();
            let mut at = x;
            let mut direction = -1.;
            while at < end {
                let next = (at + 2.).min(end);
                let amplitude = (next - at) * 0.55 * direction;
                segments.push([
                    at + (next - at) / 3.,
                    y + offset + amplitude,
                    next - (next - at) / 3.,
                    y + offset + amplitude,
                    next,
                    y + offset,
                ]);
                at = next;
                direction = -direction;
            }
            out.push(Op::Bezier {
                start: (x, y + offset),
                segments,
                closed: false,
                width: 0.45,
                color,
                fill: None,
            });
        }
        return out;
    }
    let mut stroke = |a, b, yy, width| {
        out.push(Op::Path {
            points: vec![(a, yy), (b, yy)],
            closed: false,
            width,
            color,
            fill: None,
        })
    };
    match style {
        1..=6 => {
            let pattern: &[f64] = match style {
                1 => &[3., 2.],
                2 => &[0.5, 1.5],
                3 => &[3., 1.5, 0.5, 1.5],
                4 => &[3., 1.5, 0.5, 1.5, 0.5, 1.5],
                5 => &[6., 2.],
                _ => &[1.2, 1.8],
            };
            let mut at = x;
            let mut i = 0;
            while at < end {
                let next = (at + pattern[i % pattern.len()]).min(end);
                if i % 2 == 0 {
                    stroke(at, next, y, 0.45);
                }
                at = next;
                i += 1;
            }
        }
        7..=9 => {
            stroke(x, end, y, if style == 9 { 1.2 } else { 0.4 });
            stroke(x, end, y + 1.6, if style == 8 { 1.2 } else { 0.4 });
        }
        10 => {
            stroke(x, end, y, 0.4);
            stroke(x, end, y + 1.6, 1.2);
            stroke(x, end, y + 3.2, 0.4);
        }
        _ => stroke(x, end, y, 0.45),
    }
    out
}
