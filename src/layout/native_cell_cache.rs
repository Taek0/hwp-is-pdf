//! Rebuild native cell line boundaries while retaining source vertical metrics.
//! Retain source vertical metrics, explicit breaks, and inter-paragraph gaps.
use super::*;
/// A native saved line can differ by a fraction of a point after font
/// realization. Grow only when visible advances exceed the wrap area by 0.1pt.
fn materially_overflows(
    units: &[crate::reflow::Unit],
    saved: &[&Node],
    width: f64,
    s: &crate::reflow::Settings,
) -> bool {
    saved.iter().enumerate().any(|(i, line)| {
        let end = saved
            .get(i + 1)
            .map(|l| l.u("textpos"))
            .unwrap_or(usize::MAX);
        let mut part: Vec<_> = units
            .iter()
            .filter(|u| u.pos >= line.u("textpos") && u.pos < end)
            .collect();
        while part.last().is_some_and(|u| u.ch == Some(' ')) {
            part.pop();
        }
        let measured = part.iter().map(|u| u.width).sum::<f64>()
            - part.last().map(|u| u.trailing).unwrap_or(0.)
            - part
                .iter()
                .filter(|u| u.ch == Some(' '))
                .map(|u| u.width.max(0.) * s.condense)
                .sum::<f64>();
        let available = width - s.left - s.right - if i == 0 { s.indent } else { 0. };
        measured > available + 0.1
    })
}
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
                    let can_grow = matches!(heading, "" | "NONE")
                        || (heading == "BULLET"
                            && self.list_auto_indent(p)
                            && settings.indent == 0.);
                    if !saved.is_empty()
                        && Self::items(p)
                            .iter()
                            .any(|i| matches!(i.kind,ItemKind::Char(c) if !c.is_whitespace()))
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
                        // Recovery must not reject an otherwise usable saved
                        // layout when its inherited margins define no wrap area.
                        if width > settings.left + settings.right + settings.indent.max(0.) + 0.01 {
                            let units = self.paragraph_units(p)?;
                            let wrapped = crate::reflow::wrap(&units, width, &settings)?;
                            let saved_fits_space_budget = settings.condense > 0.
                                && !materially_overflows(&units, &saved, width, &settings);
                            let grow =
                                can_grow && materially_overflows(&units, &saved, width, &settings);
                            if !saved_fits_space_budget
                                && (wrapped.len() <= saved.len() || grow)
                                && (wrapped.len() != saved.len()
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_growth_ignores_subpoint_fit_noise_and_wrapping_spaces() {
        let line = crate::model::xml(br#"<lineseg textpos="0"/>"#).unwrap();
        let mut units = vec![crate::reflow::Unit {
            pos: 0,
            ch: Some('A'),
            width: 32.,
            trailing: 2.,
            ..Default::default()
        }];
        let settings = crate::reflow::Settings::default();
        assert!(!materially_overflows(&units, &[&line], 29.95, &settings));
        assert!(materially_overflows(&units, &[&line], 29.8, &settings));
        units.push(crate::reflow::Unit {
            pos: 1,
            ch: Some(' '),
            width: 5.,
            ..Default::default()
        });
        assert!(!materially_overflows(&units, &[&line], 29.95, &settings));
    }
    #[test]
    fn optional_cell_recovery_preserves_unusable_margin_regions() {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        d.paras.insert(
            "wide".into(),
            crate::model::xml(
                br#"<paraPr binaryHwp="1"><margin><left value="2000"/></margin></paraPr>"#,
            )
            .unwrap(),
        );
        for value in ["", "A"] {
            let mut t=crate::model::xml(format!(r#"<tbl><tr><tc><cellSz width="1000" height="1000"/><subList lineWrap="BREAK"><p paraPrIDRef="wide"><run charPrIDRef="0"><t>{value}</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1000" baseline="850" spacing="600" horzsize="1000"/></linesegarray></p></subList></tc></tr></tbl>"#).as_bytes()).unwrap();
            let original = t.clone();
            let mut e = Engine::new(&d, Path::new("."));
            assert!(!e.repair_native_cell_cache(&mut t).unwrap());
            assert_eq!(original, t);
        }
    }
}
