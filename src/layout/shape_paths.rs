//! Polygon, curve and arc geometry; source-space path transforms.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn path_shape(&mut self, n: &Node, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        let paint_start = self.current.ops.len();
        let ow = if n.n("originalWidth") > 0. {
            n.n("originalWidth")
        } else {
            n.child("orgSz").map(|s| s.n("width")).unwrap_or(w * 100.)
        };
        let oh = if n.n("originalHeight") > 0. {
            n.n("originalHeight")
        } else {
            n.child("orgSz").map(|s| s.n("height")).unwrap_or(h * 100.)
        };
        let sx = w / (ow / 100.).max(0.01);
        let sy = h / (oh / 100.).max(0.01);
        let mut points: Vec<_> = n
            .children
            .iter()
            .filter(|p| p.name == "point" || p.name == "pt")
            .map(|p| (x + p.n("x") / 100. * sx, y + p.n("y") / 100. * sy))
            .collect();
        let mut kinds: Vec<_> = n.children_named("segment").map(|s| s.u("type")).collect();
        if points.is_empty() && n.name == "curve" {
            for seg in n.children_named("seg") {
                if points.is_empty() {
                    points.push((x + seg.n("x1") / 100. * sx, y + seg.n("y1") / 100. * sy));
                }
                points.push((x + seg.n("x2") / 100. * sx, y + seg.n("y2") / 100. * sy));
                kinds.push(usize::from(seg.a("type") == "CURVE"));
            }
        }
        if n.name == "arc" {
            let c = n.child("center").context("arc center")?;
            let a = n
                .child("axis1")
                .or_else(|| n.child("ax1"))
                .context("arc axis")?;
            let b = n
                .child("axis2")
                .or_else(|| n.child("ax2"))
                .context("arc axis")?;
            for i in 0..=32 {
                let t = i as f64 * std::f64::consts::FRAC_PI_2 / 32.;
                points.push((
                    x + (c.n("x")
                        + (a.n("x") - c.n("x")) * t.cos()
                        + (b.n("x") - c.n("x")) * t.sin())
                        / 100.
                        * sx,
                    y + (c.n("y")
                        + (a.n("y") - c.n("y")) * t.cos()
                        + (b.n("y") - c.n("y")) * t.sin())
                        / 100.
                        * sy,
                ));
            }
            if n.a("type") == "PIE" {
                points.push((x + c.n("x") / 100. * sx, y + c.n("y") / 100. * sy));
            }
        }
        let line = n.child("lineShape");
        let fill = n
            .child("fillBrush")
            .and_then(|b| b.child("winBrush"))
            .or_else(|| n.child("winBrush"))
            .filter(|f| f.a("faceColor") != "none")
            .map(|f| color(f.a("faceColor")));
        if n.name == "curve" && !points.is_empty() {
            let segments = crate::graphics::curve(&points, &kinds);
            self.current.ops.push(Op::Bezier {
                start: points[0],
                segments,
                closed: points.first() == points.last(),
                width: val(line, "width").max(0.1),
                color: color(line.map(|s| s.a("color")).unwrap_or("#000000")),
                fill,
            });
            self.report.warnings.insert("Curve knots rendered as cardinal spline (tension 0.5); exact Hancom interpolation unverified".into());
        } else {
            self.current.ops.push(Op::Path {
                points,
                closed: n.name == "polygon"
                    || (n.name == "arc" && matches!(n.a("type"), "PIE" | "CHORD")),
                width: val(line, "width").max(0.1),
                color: color(line.map(|s| s.a("color")).unwrap_or("#000000")),
                fill,
            });
        }
        drawing_strokes::apply(&mut self.current.ops, paint_start, line);
        Ok(())
    }
}
