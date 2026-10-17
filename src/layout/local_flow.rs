//! Flow and measure already-positioned nested paragraph lists.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn local(&mut self, list: &Node, x: f64, y: f64, w: f64) -> Result<()> {
        self.depth += 1;
        if self.depth > 64 {
            bail!("Excessive nested layout")
        }
        for (p, i, flow_y, _) in self.measured_flow(list)? {
            self.line(p, i, x, y + flow_y - lines(p)[i].n("vertpos") / 100., w)?;
        }
        self.depth -= 1;
        Ok(())
    }
    pub(super) fn measured_height(&mut self, list: &Node) -> Result<f64> {
        Ok(self
            .measured_flow(list)?
            .iter()
            .map(|(_, _, y, h)| y + h)
            .fold(0f64, f64::max))
    }
    pub(super) fn measured_flow<'b>(
        &mut self,
        list: &'b Node,
    ) -> Result<Vec<(&'b Node, usize, f64, f64)>> {
        if self.doc.header.a("binaryHwp") != "1"
            || list.children_named("p").all(|p| p.a("reflowed") == "1")
        {
            return Ok(flowing_lines(list)
                .into_iter()
                .map(|(p, i, y)| (p, i, y, lines(p)[i].n("vertsize") / 100.))
                .collect());
        }
        let mut out = Vec::new();
        let mut bottom = 0f64;
        let mut column_bottoms = BTreeMap::<usize, f64>::new();
        for (p, i, y) in flowing_lines(list) {
            let ls = lines(p);
            let l = ls[i];
            if list.a("savedColumns") == "1" {
                bottom = column_bottoms
                    .get(&l.u("savedColumn"))
                    .copied()
                    .unwrap_or(0.);
            }
            let items = Self::items(p);
            let end = ls.get(i + 1).map(|n| n.u("textpos")).unwrap_or(usize::MAX);
            let mut ascent = l.n("baseline") / 100.;
            let mut descent = 0f64;
            let mut block_height = 0f64;
            for it in items
                .iter()
                .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
            {
                if let ItemKind::Object(n) = &it.kind
                    && n.a("textWrap") == "TOP_AND_BOTTOM"
                    && n.child("pos").is_some_and(|pos| {
                        pos.a("treatAsChar") == "0" && pos.a("vertRelTo") == "PARA"
                    })
                {
                    let m = margin(n, "outMargin");
                    let cap = caption_extent(n);
                    let height = if n.name == "tbl" {
                        self.measure_table_grid(n, effective_height(n))?.2
                    } else {
                        effective_height(n)
                    };
                    block_height = block_height.max(
                        n.child("pos").unwrap().signed("vertOffset") / 100.
                            + m[2]
                            + height
                            + m[3]
                            + cap[2]
                            + cap[3],
                    );
                }
                if let ItemKind::Char(c) = it.kind {
                    let g = self.glyph(c, &it.style)?;
                    ascent = ascent.max(g.ascent);
                    descent = descent.max(g.descent);
                }
            }
            let h = (ascent + descent)
                .max(l.n("vertsize") / 100.)
                .max(block_height);
            let actual = y.max(bottom);
            out.push((p, i, actual, h));
            bottom = actual + h;
            if list.a("savedColumns") == "1" {
                column_bottoms.insert(l.u("savedColumn"), bottom);
            }
        }
        Ok(out)
    }
}
