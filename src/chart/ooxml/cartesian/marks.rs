//! Explicit/default series colors and markers for source-backed charts.
use super::*;
pub(super) fn color(n: &Node, index: usize) -> [f64; 3] {
    let index = n.child("idx").map(|n| n.u("val")).unwrap_or(index);
    const COLORS: [[u8; 3]; 5] = [
        [97, 130, 214],
        [255, 132, 58],
        [178, 178, 178],
        [255, 216, 0],
        [108, 171, 74],
    ];
    n.child("spPr")
        .and_then(|n| {
            n.child("solidFill")
                .or_else(|| n.child("ln").and_then(|n| n.child("solidFill")))
        })
        .and_then(|n| n.child("srgbClr"))
        .map(|n| rgb(n.a("val")))
        .unwrap_or_else(|| COLORS[index % COLORS.len()].map(|v| v as f64 / 255.))
}
pub(super) fn line_visible(n: &Node) -> bool {
    n.child("spPr")
        .and_then(|n| n.child("ln"))
        .is_none_or(|n| n.child("noFill").is_none())
}
pub(super) fn line_width(n: &Node) -> f64 {
    n.child("spPr")
        .and_then(|n| n.child("ln"))
        .map(|n| n.n("w") / 12700.)
        .filter(|v| *v > 0.)
        .unwrap_or(2.25)
}
pub(super) fn marker(n: &Node, index: usize, x: f64, y: f64) -> Vec<Op> {
    let index = n.child("idx").map(|n| n.u("val")).unwrap_or(index);
    let marker = n.child("marker");
    let symbol = marker
        .and_then(|n| n.child("symbol"))
        .map(|n| n.a("val"))
        .unwrap_or("auto");
    let symbol = if symbol == "auto" {
        ["diamond", "square", "triangle", "x", "circle"][index % 5]
    } else {
        symbol
    };
    if symbol == "none" {
        return Vec::new();
    }
    let r = marker
        .and_then(|n| n.child("size"))
        .map(|n| n.n("val"))
        .filter(|v| *v > 0.)
        .unwrap_or(7.)
        * 0.375;
    let c = color(n, index);
    if symbol == "x" {
        return vec![
            Op::Line {
                x: x - r,
                y: y - r,
                x2: x + r,
                y2: y + r,
                width: 0.75,
                color: c,
            },
            Op::Line {
                x: x - r,
                y: y + r,
                x2: x + r,
                y2: y - r,
                width: 0.75,
                color: c,
            },
        ];
    }
    let points = match symbol {
        "square" => vec![
            (x - r, y - r),
            (x + r, y - r),
            (x + r, y + r),
            (x - r, y + r),
        ],
        "triangle" => vec![(x, y - r), (x + r, y + r), (x - r, y + r)],
        "circle" => (0..32)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 32.;
                (x + r * a.cos(), y + r * a.sin())
            })
            .collect(),
        _ => vec![(x, y - r), (x + r, y), (x, y + r), (x - r, y)],
    };
    vec![Op::Path {
        points,
        closed: true,
        width: 0.,
        color: c,
        fill: Some(c),
    }]
}
