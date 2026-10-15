//! Normalize negative and absolute anchors without changing the source document.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn prepare_reflow_anchors(
        &mut self,
        p: &mut Node,
        width: f64,
        settings: &mut crate::reflow::Settings,
    ) -> Result<()> {
        let mut notes = Vec::new();
        p.all("footNote", &mut notes);
        let reserve = self.footnote_reservation(&notes, self.left);
        let body = self.ph - self.top - self.bottom;
        let absolute = p
            .children_named("run")
            .flat_map(|r| &r.children)
            .filter(|n| n.a("reflowGeneral") == "1")
            .any(|n| {
                n.child("pos").is_some_and(|p| {
                    matches!(p.a("horzRelTo"), "PAGE" | "PAPER")
                        || matches!(p.a("vertRelTo"), "PAGE" | "PAPER")
                })
            });
        let original_before = settings.before;
        let mut before_extra = 0f64;
        let mut floor = self.top;
        for run in p.children.iter_mut().filter(|r| r.name == "run") {
            for n in run
                .children
                .iter_mut()
                .filter(|n| n.a("reflowGeneral") == "1")
            {
                let m = margin(n, "outMargin");
                let pos = n.child("pos").context("Float position")?.clone();
                n.attrs.insert(
                    "reflowMaxHeight".into(),
                    (body - reserve - original_before - 32.)
                        .max(24.)
                        .to_string(),
                );
                n.attrs.insert("reflowNegativeAllowed".into(), "1".into());
                if absolute {
                    let mut w = val(n.child("sz"), "width");
                    let mut h = effective_height(n);
                    let scale = (self.pw / w.max(0.01))
                        .min((body - reserve - 32.) / h.max(0.01))
                        .clamp(0.001, 1.);
                    if scale < 1. {
                        n.attrs.insert("reflowScale".into(), scale.to_string());
                        w *= scale;
                        h *= scale;
                    }
                    let (xo, xw) = match pos.a("horzRelTo") {
                        "PAPER" => (0., self.pw),
                        "PAGE" => (self.left, self.pw - self.left - self.right),
                        _ => (self.left, width),
                    };
                    let (yo, yh) = match pos.a("vertRelTo") {
                        "PAPER" => (0., self.ph),
                        "PAGE" => (self.top, body),
                        _ => (self.top + original_before, body),
                    };
                    let x = xo
                        + pos.signed("horzOffset") / 100.
                        + match pos.a("horzAlign") {
                            "CENTER" => (xw - w) / 2.,
                            "RIGHT" => xw - w,
                            _ => 0.,
                        };
                    let offset = pos.signed("vertOffset") / 100.;
                    let y = object_position::vertical(
                        yo,
                        yh,
                        h,
                        pos.a("vertRelTo"),
                        pos.a("vertAlign"),
                        offset,
                    );
                    anyhow::ensure!(
                        y >= 0. && y + h <= self.ph + 0.01,
                        "Absolute float lies outside the paper"
                    );
                    let target = node_mut(n, "pos");
                    for (k, v) in [
                        ("horzRelTo", "PARA"),
                        ("vertRelTo", "PARA"),
                        ("horzAlign", "LEFT"),
                        ("vertAlign", "TOP"),
                    ] {
                        target.attrs.insert(k.into(), v.into());
                    }
                    target.attrs.insert(
                        "horzOffset".into(),
                        ((x - self.left - m[0]) * 100.).to_string(),
                    );
                    target.attrs.insert(
                        "vertOffset".into(),
                        ((y - self.top - original_before - m[2]) * 100.).to_string(),
                    );
                    n.attrs.insert("pageAbsolute".into(), "1".into());
                    floor = floor.max(y + h + m[3] + 2.);
                } else {
                    before_extra = before_extra.max(-(pos.signed("vertOffset") / 100. + m[2]));
                }
            }
        }
        if absolute {
            settings.page = true;
            p.attrs.insert("pageBreak".into(), "1".into());
            p.attrs.insert(
                "pageFloatFloor".into(),
                floor.min(self.ph - self.bottom).to_string(),
            );
            self.report.warnings.insert(
                "Absolute anchors start a reflow page; later columns clear their page-wide floor"
                    .into(),
            );
        } else if before_extra > 0. {
            settings.before += before_extra;
            p.attrs
                .insert("anchorBeforeExtra".into(), before_extra.to_string());
        }
        Ok(())
    }
}
