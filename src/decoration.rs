//! Hancom character underline styles (format table 25/30).
use crate::pdf::Op;
pub fn underline(x: f64, end: f64, y: f64, style: usize, color: [f64; 3]) -> Vec<Op> {
    let mut out = Vec::new();
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
