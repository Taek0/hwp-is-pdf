//! Vector border styles shared by cells, paragraphs and page frames.
use super::*;
pub(super) fn stroke(style: &Node, start: (f64, f64), end: (f64, f64)) -> Vec<Op> {
    let width = style
        .a("width")
        .split_whitespace()
        .next()
        .unwrap_or("0.1")
        .parse::<f64>()
        .unwrap_or(0.1)
        * 72.
        / 25.4;
    let color = color(style.a("color"));
    if matches!(style.a("type"), "THICK3D" | "THICKREV3D" | "3D" | "REV3D") {
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let length = dx.hypot(dy);
        if length <= 0. {
            return Vec::new();
        }
        let light = color.map(|v| v + (1. - v) * 0.65);
        let dark = color.map(|v| v * 0.5);
        let (a, b) = if style.a("type").contains("REV") {
            (dark, light)
        } else {
            (light, dark)
        };
        let bands = if style.a("type").starts_with("THICK") {
            vec![(-0.4, 0.2, a), (0., 0.6, color), (0.4, 0.2, b)]
        } else {
            vec![(-0.25, 0.5, a), (0.25, 0.5, b)]
        };
        return bands
            .into_iter()
            .map(|(offset, weight, color)| {
                let x = -dy / length * offset * width;
                let y = dx / length * offset * width;
                Op::Line {
                    x: start.0 + x,
                    y: start.1 + y,
                    x2: end.0 + x,
                    y2: end.1 + y,
                    width: weight * width,
                    color,
                }
            })
            .collect();
    }
    let names = [
        "SOLID",
        "DASH",
        "DOT",
        "DASH_DOT",
        "DASH_DOT_DOT",
        "LONG_DASH",
        "CIRCLE",
        "DOUBLE_SLIM",
        "SLIM_THICK",
        "THICK_SLIM",
        "SLIM_THICK_SLIM",
        "WAVE",
        "DOUBLEWAVE",
    ];
    let index = names
        .iter()
        .position(|&n| n == style.a("type"))
        .unwrap_or(0);
    if index == 0 {
        return vec![Op::Line {
            x: start.0,
            y: start.1,
            x2: end.0,
            y2: end.1,
            width,
            color,
        }];
    }
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let length = dx.hypot(dy);
    if length <= 0. || width <= 0. {
        return Vec::new();
    }
    let scale = width / 0.45;
    // Border width is the complete band, unlike character underline strokes.
    // Center multiple rules on the boundary so they do not invade adjacent ink.
    let (band, center) = match index {
        7 => (2., 0.8),
        8 => (2.4, 1.),
        9 => (2.4, 0.6),
        10 => (3.6, 1.6),
        11 => (2.65, 0.),
        12 => (5.05, 1.2),
        _ => (0.45, 0.),
    };
    let across = width / band;
    let point = |x: f64, y: f64| {
        (
            start.0 + (dx * x * scale - dy * (y - center) * across) / length,
            start.1 + (dy * x * scale + dx * (y - center) * across) / length,
        )
    };
    crate::decoration::underline(0., length / scale, 0., index, color)
        .into_iter()
        .map(|op| match op {
            Op::Path { points, width, .. } => {
                let a = point(points[0].0, points[0].1);
                let b = point(points[1].0, points[1].1);
                Op::Line {
                    x: a.0,
                    y: a.1,
                    x2: b.0,
                    y2: b.1,
                    width: width * across,
                    color,
                }
            }
            Op::Bezier {
                start,
                segments,
                closed,
                width,
                fill,
                ..
            } => Op::Bezier {
                start: point(start.0, start.1),
                segments: segments
                    .into_iter()
                    .map(|s| {
                        let a = point(s[0], s[1]);
                        let b = point(s[2], s[3]);
                        let c = point(s[4], s[5]);
                        [a.0, a.1, b.0, b.1, c.0, c.1]
                    })
                    .collect(),
                closed,
                width: width * across,
                color,
                fill,
            },
            _ => unreachable!(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dash_has_gaps_double_has_two_rules_and_waves_are_curved() {
        let style = |kind: &str| {
            crate::model::xml(
                format!(r##"<topBorder type="{kind}" width="0.2 mm" color="#000000"/>"##)
                    .as_bytes(),
            )
            .unwrap()
        };
        let dash = stroke(&style("DASH"), (0., 0.), (100., 0.));
        let covered: f64 = dash
            .iter()
            .map(|o| {
                if let Op::Line { x, x2, .. } = o {
                    x2 - x
                } else {
                    0.
                }
            })
            .sum();
        assert!(covered > 40. && covered < 80.);
        let double = stroke(&style("DOUBLE_SLIM"), (20., 10.), (20., 100.));
        assert_eq!(double.len(), 2);
        assert!(double.iter().all(|o|matches!(o,Op::Line{x,x2,y,y2,..} if (x-x2).abs()<1e-9 && *y==10. && (*y2-100.).abs()<1e-9)));
        assert!(matches!(
            stroke(&style("WAVE"), (0., 0.), (100., 0.))[0],
            Op::Bezier { .. }
        ));
    }
}
