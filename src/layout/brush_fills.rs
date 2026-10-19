//! Rectangular brush fills, kept separate from outline decoration.
use super::*;
pub(super) fn complex(n: &Node) -> bool {
    let n = n.child("fillBrush").unwrap_or(n);
    n.child("gradation").is_some()
        || n.child("imgBrush").is_some_and(|b| b.a("mode") != "NONE")
        || n.child("winBrush")
            .is_some_and(|b| !matches!(b.a("hatchStyle"), "" | "NONE"))
}
pub(super) fn paint(
    n: &Node,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    images: &BTreeMap<String, Vec<u8>>,
) -> Vec<Op> {
    if !complex(n) || w <= 0. || h <= 0. {
        return Vec::new();
    }
    let padding = val(
        n.child("lineShape").filter(|l| l.a("style") != "NONE"),
        "width",
    );
    let n = n.child("fillBrush").unwrap_or(n);
    let mut ops = vec![
        Op::TransformBegin {
            matrix: crate::graphics::IDENTITY,
        },
        Op::ClipRect { x, y, w, h },
    ];
    if let Some(g) = n.find("gradation") {
        let colors: Vec<_> = g
            .children_named("color")
            .map(|c| color(c.a("value")))
            .collect();
        if colors.len() >= 2 {
            ops.push(Op::Gradient {
                x,
                y,
                w,
                h,
                kind: g.a("type").into(),
                angle: g.n("angle"),
                center: [g.n("centerX") / 100., g.n("centerY") / 100.],
                padding,
                steps: if g.a("step").is_empty() {
                    50
                } else {
                    g.u("step").clamp(2, 256)
                },
                step_center: if g.a("stepCenter").is_empty() {
                    0.5
                } else {
                    (g.n("stepCenter") / 100.).clamp(0.001, 0.999)
                },
                colors,
            });
        }
    } else if let Some(b) = n.find("imgBrush").filter(|b| b.a("mode") != "NONE") {
        if let Some(image) = b.child("img") {
            if b.a("mode") == "TILE" {
                let dimensions = images
                    .get(image.a("binaryItemIDRef"))
                    .and_then(|bytes| image::load_from_memory(bytes).ok())
                    .map(|im| (im.width() as f64 * 0.75, im.height() as f64 * 0.75))
                    .unwrap_or((0., 0.));
                ops.push(Op::ImageTile {
                    x,
                    y,
                    w,
                    h,
                    id: image.a("binaryItemIDRef").into(),
                    tile_w: dimensions.0,
                    tile_h: dimensions.1,
                });
            } else {
                ops.push(Op::Image {
                    x,
                    y,
                    w,
                    h,
                    id: image.a("binaryItemIDRef").into(),
                });
            }
        }
    } else if let Some(b) = n.find("winBrush") {
        if !matches!(b.a("faceColor"), "" | "none") {
            ops.push(Op::Rect {
                x,
                y,
                w,
                h,
                fill: color(b.a("faceColor")),
            });
        }
        let c = color(b.a("hatchColor"));
        let spacing = 5.76;
        let width = 0.72;
        let mut line = |a: (f64, f64), b: (f64, f64)| {
            ops.push(Op::DecorativeLine {
                x: a.0,
                y: a.1,
                x2: b.0,
                y2: b.1,
                width,
                color: c,
            })
        };
        let style = b.a("hatchStyle");
        if matches!(style, "HORIZONTAL" | "CROSS") {
            for i in 0..=(h / spacing).ceil().min(10000.) as usize {
                let yy = y + i as f64 * spacing;
                line((x, yy), (x + w, yy));
            }
        }
        if matches!(style, "VERTICAL" | "CROSS") {
            for i in 0..=(w / spacing).ceil().min(10000.) as usize {
                let xx = x + i as f64 * spacing;
                line((xx, y), (xx, y + h));
            }
        }
        for diagonal in ["SLASH", "BACK_SLASH"] {
            if style == diagonal || style == "CROSS_DIAGONAL" {
                for i in 0..=((w + h) / spacing).ceil().min(10000.) as usize {
                    let xx = x - h + i as f64 * spacing;
                    if diagonal == "SLASH" {
                        line((xx, y + h), (xx + h, y));
                    } else {
                        line((xx, y), (xx + h, y + h));
                    }
                }
            }
        }
    }
    ops.push(Op::TransformEnd);
    ops
}
