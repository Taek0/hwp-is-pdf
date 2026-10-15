//! Resolve document tab definitions and paint optional tab leaders.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn paragraph_tabs(&self, p: &Node) -> (Vec<crate::tabs::Stop>, f64) {
        let id = self
            .doc
            .paras
            .get(p.a("paraPrIDRef"))
            .map(|n| n.a("tabPrIDRef"))
            .unwrap_or("");
        let mut defs = Vec::new();
        self.doc.header.all("tabPr", &mut defs);
        let mut stops = defs
            .into_iter()
            .find(|d| d.a("id") == id)
            .map(|d| {
                d.children_named("tabItem")
                    .map(|t| crate::tabs::Stop {
                        pos: t.n("pos") / 100.,
                        kind: t.a("type").into(),
                        leader: t.a("leader").into(),
                    })
                    .filter(|s| s.pos.is_finite() && s.pos >= 0.)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        stops.sort_by(|a, b| a.pos.total_cmp(&b.pos));
        let step = self.doc.sections[self.active_section]
            .find("secPr")
            .map(|s| s.n("tabStop") / 100.)
            .filter(|v| *v > 0.)
            .unwrap_or(40.);
        (stops, step)
    }
    pub(super) fn paint_tab_leader(
        &mut self,
        p: &Node,
        x: f64,
        base: f64,
        advance: f64,
        ox: f64,
        style: &str,
    ) -> Result<()> {
        let (stops, _) = self.paragraph_tabs(p);
        let Some(stop) = stops.iter().find(|s| s.pos > x - ox + 0.01) else {
            return Ok(());
        };
        if advance <= 3. || matches!(stop.leader.as_str(), "" | "NONE" | "0") {
            return Ok(());
        }
        let g = self.glyph(' ', style)?;
        let y = base - g.size * 0.2;
        let (length, step) = match stop.leader.as_str() {
            "DOT" | "DOTTED" | "3" => (0.5, 3.5),
            "DASH" | "2" => (3., 5.),
            _ => (advance - 2., advance),
        };
        let mut at = x + 1.;
        while at + length < x + advance {
            self.current.ops.push(Op::Line {
                x: at,
                y,
                x2: at + length,
                y2: y,
                width: 0.3,
                color: g.color,
            });
            at += step;
        }
        Ok(())
    }
}
