//! Prepare equation boxes for reflow while leaving saved layouts untouched.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn paint_equation(
        &mut self,
        n: &Node,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Result<()> {
        if n.a("reflowEquation") == "1" {
            self.paint_reflow_equation(n, x, y, w, h)?;
        } else {
            let script = n.child("script").map(text).unwrap_or_default();
            let b = math::layout(&script, n.n("baseUnit") / 100., &mut self.fonts)?;
            self.current
                .ops
                .extend(math::place(b, x, y + h * n.n("baseLine") / 100., w, h));
        }
        self.report.equations += 1;
        Ok(())
    }
    pub(super) fn prepare_equation(&mut self, n: &mut Node, width: f64) -> Result<()> {
        anyhow::ensure!(
            n.child("script")
                .is_some_and(|s| !text(s).trim().is_empty()),
            "Missing equation script in {}",
            n.a("id")
        );
        n.attrs.insert("reflowEquation".into(), "1".into());
        if n.n("baseUnit") <= 0. {
            n.attrs.insert("baseUnit".into(), "1100".into());
        }
        let mut w = val(n.child("sz"), "width");
        let mut h = val(n.child("sz"), "height");
        if w <= 0. || h <= 0. {
            let script = n.child("script").map(text).unwrap_or_default();
            let measured = math::layout(&script, n.n("baseUnit") / 100., &mut self.fonts)?;
            w = measured.width.max(1.);
            h = (measured.ascent + measured.descent).max(1.);
            n.attrs
                .insert("baseLine".into(), (100. * measured.ascent / h).to_string());
        }
        let baseline = if n.a("baseLine").is_empty() {
            85.
        } else {
            n.n("baseLine").clamp(0., 100.)
        };
        n.attrs.insert("baseLine".into(), baseline.to_string());
        let m = margin(n, "outMargin");
        let scale = ((width - m[0] - m[1]).max(1.) / w)
            .min((self.ph - self.top - self.bottom - m[2] - m[3] - 24.).max(1.) / h)
            .min(1.);
        if scale < 1. {
            self.report.warnings.insert(
                "Oversized reflow equation fitted proportionally to the available column".into(),
            );
        }
        let sz = node_mut(n, "sz");
        sz.attrs
            .insert("width".into(), (w * scale * 100.).to_string());
        sz.attrs
            .insert("height".into(), (h * scale * 100.).to_string());
        let pos = node_mut(n, "pos");
        if pos.a("treatAsChar") != "1" {
            for (k, v) in [
                ("treatAsChar", "0"),
                ("horzRelTo", "PARA"),
                ("vertRelTo", "PARA"),
                ("horzAlign", "LEFT"),
                ("vertAlign", "TOP"),
                ("horzOffset", "0"),
                ("vertOffset", "0"),
            ] {
                pos.attrs.insert(k.into(), v.into());
            }
        }
        Ok(())
    }
    pub(super) fn paint_reflow_equation(
        &mut self,
        n: &Node,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Result<()> {
        let script = n.child("script").map(text).unwrap_or_default();
        let b = math::layout(&script, n.n("baseUnit") / 100., &mut self.fonts)?;
        let baseline = (h * n.n("baseLine") / 100.).clamp(0., h);
        let scale = (w / b.width.max(0.01))
            .min(if b.ascent > 0. {
                baseline / b.ascent
            } else {
                1.
            })
            .min(if b.descent > 0. {
                (h - baseline) / b.descent
            } else {
                1.
            })
            .clamp(0.01, 1.);
        let width = b.width * scale;
        let height = (b.ascent + b.descent) * scale;
        self.current
            .ops
            .extend(math::place(b, x, y + baseline, width, height));
        Ok(())
    }
}
