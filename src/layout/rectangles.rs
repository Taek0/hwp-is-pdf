//! Rectangular and rounded rectangular fills/strokes in object coordinates.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn paint_rectangle(&mut self, n: &Node, x: f64, y: f64, w: f64, h: f64) {
        let paint_start = self.current.ops.len();
        let complex_brush = brush_fills::complex(n);
        let fill = n
            .child("fillBrush")
            .and_then(|b| b.child("winBrush"))
            .or_else(|| n.child("winBrush"))
            .filter(|f| f.a("faceColor") != "none" && !complex_brush)
            .map(|f| color(f.a("faceColor")));
        let line = n.child("lineShape");
        let stroke = line.is_none_or(|s| s.a("style") != "NONE");
        let width = if stroke {
            val(line, "width").max(0.1)
        } else {
            0.
        };
        let c = color(line.map(|s| s.a("color")).unwrap_or("#000000"));
        let ratio = if n.a("cornerRatio").is_empty() {
            n.n("ratio")
        } else {
            n.n("cornerRatio")
        };
        let r = w.min(h).max(0.) * ratio.clamp(0., 50.) / 100.;
        if r > 0.01 {
            if !stroke && fill.is_none() && !complex_brush {
                return;
            }
            let k = 0.5522847498307936 * r;
            self.current.ops.push(Op::Bezier {
                start: (x + r, y),
                segments: vec![
                    [x + r, y, x + w - r, y, x + w - r, y],
                    [x + w - r + k, y, x + w, y + r - k, x + w, y + r],
                    [x + w, y + r, x + w, y + h - r, x + w, y + h - r],
                    [x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h],
                    [x + w - r, y + h, x + r, y + h, x + r, y + h],
                    [x + r - k, y + h, x, y + h - r + k, x, y + h - r],
                    [x, y + h - r, x, y + r, x, y + r],
                    [x, y + r - k, x + r - k, y, x + r, y],
                ],
                closed: true,
                width,
                color: c,
                fill,
            });
        } else {
            if let Some(fill) = fill {
                self.current.ops.push(Op::Rect { x, y, w, h, fill });
            }
            if stroke {
                if line.is_some_and(|s| {
                    matches!(
                        s.a("style"),
                        "DASH" | "DOT" | "DASH_DOT" | "DASH_DOT_DOT" | "LONG_DASH"
                    )
                }) {
                    self.current.ops.push(Op::Path {
                        points: vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)],
                        closed: true,
                        width,
                        color: c,
                        fill: None,
                    });
                } else {
                    self.box_lines(x, y, w, h, width, c);
                }
            }
        }
        let clip = if complex_brush {
            self.current.ops[paint_start..].iter().find_map(|o| {
                if let Op::Bezier {
                    start, segments, ..
                } = o
                {
                    Some(crate::graphics::flatten_cubics(*start, segments))
                } else {
                    None
                }
            })
        } else {
            None
        };
        drawing_strokes::apply(&mut self.current.ops, paint_start, line);
        if complex_brush {
            let mut fill = brush_fills::paint(n, x, y, w, h, &self.doc.binaries);
            if let Some(points) = clip
                && !fill.is_empty()
            {
                fill.insert(2, Op::ClipPath { points });
            }
            self.current.ops.splice(paint_start..paint_start, fill);
        }
    }
}
