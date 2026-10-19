//! Ellipse outlines and shape-clipped brushes.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn paint_ellipse(&mut self, n: &Node, x: f64, y: f64, w: f64, h: f64) {
        let start = self.current.ops.len();
        let line = n.child("lineShape");
        let complex = brush_fills::complex(n);
        let points: Vec<_> = (0..64)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 64.;
                (x + w / 2. + w / 2. * a.cos(), y + h / 2. + h / 2. * a.sin())
            })
            .collect();
        if let Some(fill) = n
            .child("fillBrush")
            .and_then(|b| b.child("winBrush"))
            .filter(|b| b.a("faceColor") != "none" && !complex)
            .map(|b| color(b.a("faceColor")))
        {
            self.current.ops.push(Op::Path {
                points: points.clone(),
                closed: true,
                width: 0.,
                color: fill,
                fill: Some(fill),
            });
        }
        let stroke = color(line.map(|n| n.a("color")).unwrap_or("#000000"));
        if line.is_some_and(|s| !matches!(s.a("style"), "" | "SOLID")) {
            self.current.ops.push(Op::Path {
                points: points.clone(),
                closed: true,
                width: val(line, "width").max(0.1),
                color: stroke,
                fill: None,
            });
            drawing_strokes::apply(&mut self.current.ops, start, line);
        } else {
            for i in 0..64 {
                let a = i as f64 * std::f64::consts::TAU / 64.;
                let b = (i + 1) as f64 * std::f64::consts::TAU / 64.;
                self.current.ops.push(Op::Line {
                    x: x + w / 2. + w / 2. * a.cos(),
                    y: y + h / 2. + h / 2. * a.sin(),
                    x2: x + w / 2. + w / 2. * b.cos(),
                    y2: y + h / 2. + h / 2. * b.sin(),
                    width: val(line, "width").max(0.1),
                    color: stroke,
                });
            }
        }
        if complex {
            let mut fill = brush_fills::paint(n, x, y, w, h, &self.doc.binaries);
            if !fill.is_empty() {
                fill.insert(2, Op::ClipPath { points });
                self.current.ops.splice(start..start, fill);
            }
        }
    }
}
