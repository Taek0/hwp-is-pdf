//! Recover native cell line boundaries when resolved fonts fit no more lines.
//! Retain source vertical metrics, explicit breaks, and inter-paragraph gaps.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn repair_native_cell_cache(&mut self, table: &mut Node) -> Result<bool> {
        let defaults = margin(table, "inMargin");
        let mut changed = false;
        for row in table.children.iter_mut().filter(|n| n.name == "tr") {
            for cell in row.children.iter_mut().filter(|n| n.name == "tc") {
                let margins = if cell.a("hasMargin") == "1" {
                    margin(cell, "cellMargin")
                } else {
                    defaults
                };
                let width = val(cell.child("cellSz"), "width") - margins[0] - margins[1];
                let Some(list) = cell
                    .child("subList")
                    .filter(|l| l.a("lineWrap") == "BREAK" && l.a("savedColumns") != "1")
                else {
                    continue;
                };
                if width <= 1.
                    || list.children_named("p").any(|p| {
                        p.a("reflowed") == "1"
                            || p.a("pageBreak") == "1"
                            || p.a("columnBreak") == "1"
                            || Self::items(p)
                                .iter()
                                .any(|i| !matches!(i.kind, ItemKind::Char(_)))
                    })
                {
                    continue;
                }
                // Cell-local Y can reset at an automatic page boundary. Rebuild
                // continuous positions before replacing soft line boundaries;
                // explicit breaks and nested columns are excluded above.
                let flow = flowing_lines(list);
                let mut replacements = Vec::new();
                let mut any = false;
                for p in list.children_named("p") {
                    let saved = lines(p);
                    let mut starts = None;
                    let style = self.doc.paras.get(p.a("paraPrIDRef"));
                    let mut settings = crate::reflow::settings(style);
                    let heading = style
                        .and_then(|s| s.child("heading"))
                        .map(|h| h.a("type"))
                        .unwrap_or("");
                    let mut label_x = None;
                    if saved.len() > 1
                        && heading != "NUMBER"
                        && p.find("lineBreak").is_none()
                        && saved.iter().all(|l| {
                            l.a("hardBreak") != "1"
                                && ["vertsize", "baseline", "spacing"]
                                    .iter()
                                    .all(|key| (l.n(key) - saved[0].n(key)).abs() < 0.01)
                        })
                    {
                        if heading == "BULLET"
                            && let Some((_, _, reserved)) = self.paragraph_number(p, false)?
                        {
                            let anchor = (settings.left + settings.indent).max(0.);
                            label_x = Some(anchor);
                            settings.left = settings.left.max(anchor + reserved);
                            settings.indent = 0.;
                        }
                        let units = self.paragraph_units(p)?;
                        let wrapped = crate::reflow::wrap(&units, width, &settings)?;
                        if wrapped.len() <= saved.len()
                            && (wrapped.len() < saved.len()
                                || wrapped
                                    .iter()
                                    .zip(&saved)
                                    .any(|(a, b)| a.pos != b.u("textpos")))
                        {
                            starts = Some(
                                wrapped
                                    .iter()
                                    .map(|l| (l.pos, l.left, l.width))
                                    .collect::<Vec<_>>(),
                            );
                            any = true;
                        }
                    }
                    let ys: Vec<_> = flow
                        .iter()
                        .filter(|(node, _, _)| std::ptr::eq(*node, p))
                        .map(|(_, _, y)| *y)
                        .collect();
                    replacements.push((starts, ys, label_x));
                }
                if !any {
                    continue;
                }
                let mut copy = list.clone();
                let mut old_end = None;
                let mut new_end = 0.;
                for (p, (starts, ys, label_x)) in copy
                    .children
                    .iter_mut()
                    .filter(|n| n.name == "p")
                    .zip(replacements)
                {
                    if ys.is_empty() {
                        continue;
                    }
                    let saved: Vec<_> = lines(p).into_iter().cloned().collect();
                    let first = ys[0];
                    let origin = old_end
                        .map(|e: f64| new_end + (first - e).max(0.))
                        .unwrap_or(first);
                    let last = saved.last().unwrap();
                    old_end =
                        Some(ys[ys.len() - 1] + (last.n("vertsize") + last.n("spacing")) / 100.);
                    let mut cache = if let Some(starts) = starts {
                        if let Some(anchor) = label_x {
                            p.attrs.insert("reflowed".into(), "1".into());
                            p.attrs.insert("reflowNumberX".into(), anchor.to_string());
                        }
                        starts
                            .iter()
                            .enumerate()
                            .map(|(i, (pos, left, width))| {
                                let mut l = saved[0].clone();
                                l.attrs.insert("textpos".into(), pos.to_string());
                                l.attrs.insert("horzpos".into(), (left * 100.).to_string());
                                l.attrs
                                    .insert("horzsize".into(), (width * 100.).to_string());
                                l.attrs.insert(
                                    "vertpos".into(),
                                    ((origin
                                        + i as f64 * (l.n("vertsize") + l.n("spacing")) / 100.)
                                        * 100.)
                                        .to_string(),
                                );
                                l
                            })
                            .collect::<Vec<_>>()
                    } else {
                        saved
                            .iter()
                            .zip(&ys)
                            .map(|(l, y)| {
                                let mut l = l.clone();
                                l.attrs.insert(
                                    "vertpos".into(),
                                    ((origin + y - first) * 100.).to_string(),
                                );
                                l
                            })
                            .collect()
                    };
                    if let Some(last) = cache.last() {
                        new_end =
                            (last.n("vertpos") + last.n("vertsize") + last.n("spacing")) / 100.;
                    }
                    node_mut(p, "linesegarray").children = std::mem::take(&mut cache);
                }
                *node_mut(cell, "subList") = copy;
                changed = true;
            }
        }
        Ok(changed)
    }
}
