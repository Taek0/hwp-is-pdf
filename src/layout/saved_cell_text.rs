//! Recover overflowing plain-text cell caches that explicitly allow wrapping.
use super::*;
impl<'a> Engine<'a> {
    fn repair_nested_saved_tables(&mut self, n: &mut Node) -> Result<bool> {
        let mut changed = false;
        for child in &mut n.children {
            if child.name == "tbl" {
                if let Some(copy) = self.repair_saved_cell_text(child)? {
                    *child = copy;
                    changed = true;
                }
            } else {
                changed |= self.repair_nested_saved_tables(child)?;
            }
        }
        Ok(changed)
    }
    pub(super) fn repair_saved_cell_text(&mut self, n: &Node) -> Result<Option<Node>> {
        if n.a("reflowTable") == "1" || n.a("savedTextRepaired") == "1" {
            return Ok(None);
        }
        let mut base = n.clone();
        let changed = self.repair_nested_saved_tables(&mut base)?;
        let n = &base;
        let mut repaired = changed.then(|| base.clone());
        for (ri, row) in n
            .children
            .iter()
            .enumerate()
            .filter(|(_, r)| r.name == "tr")
        {
            for (ci, cell) in row
                .children
                .iter()
                .enumerate()
                .filter(|(_, c)| c.name == "tc")
            {
                let Some(list) = cell.child("subList").filter(|l| l.a("lineWrap") == "BREAK")
                else {
                    continue;
                };
                if list.children_named("p").any(|p| {
                    p.a("reflowed") == "1"
                        || Self::items(p)
                            .iter()
                            .any(|i| !matches!(i.kind, ItemKind::Char(_)))
                }) {
                    continue;
                }
                let m = if cell.a("hasMargin") == "1" {
                    margin(cell, "cellMargin")
                } else {
                    margin(n, "inMargin")
                };
                let width = val(cell.child("cellSz"), "width") - m[0] - m[1];
                if width <= 1. {
                    continue;
                }
                let mut overflow = false;
                for p in list.children_named("p") {
                    let items = Self::items(p);
                    let ls = lines(p);
                    for (i, l) in ls.iter().enumerate() {
                        let end = ls.get(i + 1).map(|l| l.u("textpos")).unwrap_or(usize::MAX);
                        let mut measured = 0.;
                        let mut trailing = 0.;
                        for it in items
                            .iter()
                            .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                        {
                            if let ItemKind::Char(c) = it.kind {
                                let g = self.glyph(c, &it.style)?;
                                measured += g.width;
                                trailing = g.tracking;
                            }
                        }
                        measured -= trailing;
                        let available = if l.n("horzsize") > 0. {
                            (l.n("horzsize") / 100.).min(width)
                        } else {
                            width
                        };
                        // Native caches often deliberately fit slightly wider
                        // replacement fonts. Rebuild only a materially stale
                        // line there; small changes must retain saved pagination.
                        // A centered Latin label with explicit word wrapping
                        // should wrap even a small overflow, not be squeezed to
                        // the cached width. Keep the native tolerance for body
                        // prose/negative tracking where saved fitting is common.
                        let word_label = ls.len() == 1
                            && self.doc.paras.get(p.a("paraPrIDRef")).is_some_and(|s| {
                                s.child("align")
                                    .is_some_and(|a| a.a("horizontal") == "CENTER")
                                    && s.child("breakSetting")
                                        .is_some_and(|b| b.a("breakLatinWord") == "KEEP_WORD")
                            })
                            && items
                                .iter()
                                .all(|i| matches!(i.kind,ItemKind::Char(c) if c.is_ascii()))
                            && items.iter().any(|i| matches!(i.kind, ItemKind::Char(' ')));
                        let tolerance = if word_label {
                            1.0
                        } else if n.a("binaryHwp") == "1" {
                            1.20
                        } else {
                            1.02
                        };
                        overflow |= measured > available * tolerance + 0.1;
                    }
                }
                if overflow {
                    let copy = repaired.get_or_insert_with(|| n.clone());
                    let list = node_mut(&mut copy.children[ri].children[ci], "subList");
                    self.reflow_list(list, width, self.depth + 1)?;
                    // A row's advance includes line spacing; the final line's
                    // ink box does not. Keep that distinction in saved caches.
                    for p in list.children.iter_mut().filter(|p| p.name == "p") {
                        let items = Self::items(p);
                        let original: Vec<Node> = lines(p).into_iter().cloned().collect();
                        for (i, l) in original.iter().enumerate() {
                            let end = original
                                .get(i + 1)
                                .map(|l| l.u("textpos"))
                                .unwrap_or(usize::MAX);
                            let (mut ascent, mut descent, mut em) = (0f64, 0f64, 0f64);
                            for it in items
                                .iter()
                                .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            {
                                if let ItemKind::Char(c) = it.kind {
                                    let g = self.glyph(c, &it.style)?;
                                    ascent = ascent.max(g.ascent);
                                    descent = descent.max(g.descent);
                                    em = em.max(g.size);
                                }
                            }
                            let height = (ascent + descent).max(em);
                            if height <= 0. {
                                continue;
                            }
                            let leading = (l.n("vertsize") / 100. - height).max(0.);
                            let seg = &mut node_mut(p, "linesegarray").children[i];
                            seg.attrs
                                .insert("vertsize".into(), (height * 100.).to_string());
                            seg.attrs
                                .insert("textheight".into(), (height * 100.).to_string());
                            seg.attrs.insert(
                                "spacing".into(),
                                (l.n("spacing") + leading * 100.).to_string(),
                            );
                            seg.attrs.insert(
                                "baseline".into(),
                                (l.n("baseline") - leading * 50.).to_string(),
                            );
                        }
                    }
                }
            }
        }
        if let Some(copy) = &mut repaired {
            let (_, rows, _) = self.measure_table_grid(copy, val(copy.child("sz"), "height"))?;
            node_mut(copy, "sz").attrs.insert(
                "height".into(),
                (rows.iter().sum::<f64>() * 100.).to_string(),
            );
            copy.attrs.insert("savedTextRepaired".into(), "1".into());
            self.report.warnings.insert("Rewrapped overflowing plain-text cells with lineWrap=BREAK using resolved font metrics".into());
        }
        Ok(repaired)
    }
}
