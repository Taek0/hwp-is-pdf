//! Paint paragraph-below floating tables after their anchor paragraph's text.
use super::*;
pub(super) fn trailing_table(p: &Node, items: &[Item]) -> Option<(usize, Node)> {
    if p.a("reflowed") == "1" {
        return None;
    }
    let ls = lines(p);
    let first = *ls.first()?;
    let last = *ls.last()?;
    if ls.windows(2).any(|w| w[1].n("vertpos") < w[0].n("vertpos")) {
        return None;
    }
    if !items.iter().any(|i|matches!(i.kind,ItemKind::Char(c) if !c.is_whitespace()))
        || items.iter().any(|i|matches!(&i.kind,ItemKind::Control(n) if matches!(n.name.as_str(),"footNote"|"endNote"|"fieldBegin"|"fieldEnd"|"hyperlink"))) {return None;}
    let mut objects = items
        .iter()
        .filter(|i| matches!(i.kind, ItemKind::Object(_)));
    let item = objects.next()?;
    if objects.next().is_some() {
        return None;
    }
    let ItemKind::Object(n) = &item.kind else {
        return None;
    };
    let pos = n.child("pos")?;
    let end = (last.n("vertpos") + last.n("vertsize") - first.n("vertpos")) / 100.;
    let first_end = ls.get(1).map(|l| l.u("textpos")).unwrap_or(usize::MAX);
    if n.name != "tbl"
        || n.a("binaryHwp") != "1"
        || n.a("textWrap") != "TOP_AND_BOTTOM"
        || pos.a("treatAsChar") == "1"
        || pos.a("vertRelTo") != "PARA"
        || !matches!(pos.a("vertAlign"), "" | "TOP")
        || item.pos >= first_end
        || pos.signed("vertOffset") / 100. < end - 0.01
    {
        return None;
    }
    Some((item.pos, n.clone()))
}
impl<'a> Engine<'a> {
    pub(super) fn paint_trailing_saved_table(
        &mut self,
        n: &Node,
        p: &Node,
        ox: f64,
        oy: f64,
        width: f64,
    ) -> Result<()> {
        let first = lines(p)[0];
        let y = oy + first.n("vertpos") / 100.;
        self.object(n, ox, y, y + first.n("baseline") / 100., ox, width)?;
        // The anchor text has already completed. A following cached page reset
        // belongs after the table, rather than being consumed by its anchor's
        // text continuation as in the normal inline painting path.
        if let Some((_, _, continued)) = &mut self.flow_floor {
            *continued = false;
        }
        Ok(())
    }
}
