//! Caption measurement and placement shared by pictures, textboxes and tables.
use super::*;
pub(super) fn side_caption_height(n: &Node) -> f64 {
    n.children_named("caption")
        .filter(|c| matches!(c.a("side"), "LEFT" | "RIGHT"))
        .filter_map(|c| c.child("subList"))
        .map(content_height)
        .fold(0f64, f64::max)
}
impl<'a> Engine<'a> {
    pub(super) fn caption_body_width(&mut self, n: &mut Node, width: f64) -> Result<f64> {
        let mut side = 0.;
        for c in n
            .children
            .iter_mut()
            .filter(|c| c.name == "caption" && matches!(c.a("side"), "LEFT" | "RIGHT"))
        {
            let w = (c.n("width") / 100.).max(12.).min(width * 0.4);
            if (w - c.n("width") / 100.).abs() > 0.01 {
                self.report
                    .warnings
                    .insert("Side caption width fitted to leave printable object space".into());
            }
            c.attrs.insert("width".into(), (w * 100.).to_string());
            side += w + c.n("gap") / 100.;
        }
        anyhow::ensure!(
            side == 0. || width - side > 12.,
            "Side captions leave no printable object width"
        );
        Ok(width - side)
    }
    pub(super) fn prepare_captions(&mut self, n: &mut Node, nesting: usize) -> Result<()> {
        let body = val(n.child("sz"), "width");
        if n.child("caption").is_some() {
            n.attrs.insert("reflowCaptions".into(), "1".into());
        }
        for c in n.children.iter_mut().filter(|c| c.name == "caption") {
            let width = if matches!(c.a("side"), "LEFT" | "RIGHT") {
                c.n("width") / 100.
            } else {
                body
            };
            if let Some(list) = c.children.iter_mut().find(|c| c.name == "subList") {
                self.reflow_list(list, width, nesting + 1)?;
            }
        }
        Ok(())
    }
    pub(super) fn paint_captions(
        &mut self,
        n: &Node,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Result<()> {
        let m = margin(n, "outMargin");
        for caption in n.children_named("caption") {
            if let Some(list) = caption.child("subList") {
                let gap = caption.n("gap") / 100.;
                let ch = content_height(list);
                let cw = if matches!(caption.a("side"), "LEFT" | "RIGHT") {
                    caption.n("width") / 100.
                } else if caption.a("expandToMargin") == "1" {
                    w + m[0] + m[1]
                } else {
                    w
                };
                let remaining = if n.a("reflowCaptions") == "1" {
                    (h - ch).max(0.)
                } else {
                    h - ch
                };
                let cy = y + match list.a("vertAlign") {
                    "CENTER" => remaining / 2.,
                    "BOTTOM" => remaining,
                    _ => 0.,
                };
                let (cx, cy) = match caption.a("side") {
                    "LEFT" => (x - gap - cw, cy),
                    "RIGHT" => (x + w + gap, cy),
                    "TOP" => (x - (cw - w) / 2., y - gap - ch),
                    _ => (x - (cw - w) / 2., y + h + gap),
                };
                self.local(list, cx, cy, cw)?;
                if n.a("reflowCaptions") == "1" {
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: "caption".into(),
                        id: n.a("id").into(),
                        x: cx,
                        y: cy,
                        w: cw,
                        h: ch,
                    });
                }
            }
        }
        Ok(())
    }
}
