//! Source-based continuation across columns with different printable widths.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn reflow_complex_columns(
        &mut self,
        section: &Node,
    ) -> Result<(f64, usize, f64, f64)> {
        let paragraphs: Vec<_> = section.children_named("p").collect();
        let mut col = 0;
        let mut y = self.top;
        let mut band_end = y;
        let mut band_limit = self.ph - self.bottom;
        // Changing widths invalidates carried outlines. Objects remain indivisible
        // blocks in this path until a cross-width contour solver is available.
        self.general_wrap = false;
        self.flow_floor = None;
        for (index, original) in paragraphs.iter().enumerate() {
            if index > 0
                && let Some(definition) = paragraph_columns(original)
            {
                self.column_rules(band_end);
                let next_page = !self.footnotes.is_empty() || !self.continued_notes.is_empty();
                if next_page {
                    self.column_definition = None;
                    self.finish()?;
                    self.drain_note_tail()?;
                    band_end = self.top;
                    self.page_float_floor = self.top;
                }
                self.configure_columns(Some(definition))?;
                if next_page {
                    self.decorations(section, false, self.active_columns, self.active_gap)?;
                }
                self.column_band_top = band_end;
                col = 0;
                y = band_end;
            }
            self.reflow_column = col;
            if index == 0 || paragraph_columns(original).is_some() {
                let end = paragraphs
                    .iter()
                    .enumerate()
                    .skip(index + 1)
                    .find(|(_, p)| paragraph_columns(p).is_some())
                    .map(|(i, _)| i)
                    .unwrap_or(paragraphs.len());
                band_limit = self.balanced_column_bottom(&paragraphs[index..end])?;
            }
            self.refresh_shared_notes(&paragraphs, index)?;
            let mut source = (*original).clone();
            // Convert floating anchors to measured blocks; do not mutate source XML.
            for run in source.children.iter_mut().filter(|r| r.name == "run") {
                for object in run.children.iter_mut().filter(|n| n.child("pos").is_some()) {
                    if object
                        .child("pos")
                        .is_some_and(|p| p.a("treatAsChar") != "1")
                    {
                        object
                            .attrs
                            .insert("textWrap".into(), "TOP_AND_BOTTOM".into());
                    }
                }
            }
            let mut start = 0;
            loop {
                self.reflow_column = col;
                source.attrs.insert("reflowStart".into(), start.to_string());
                let width = self.column_width(col);
                let (p, ls, settings) = self.reflow_paragraph(&source, width, 0)?;
                if start == 0
                    && (p.a("pageBreak") == "1" || settings.page)
                    && (y > self.top + 0.01 || col > 0)
                {
                    col = self.active_columns - 1;
                    self.advance_complex_column(section, &mut col, &mut y)?;
                    band_end = y;
                    band_limit = self.ph - self.bottom;
                    continue;
                }
                let height =
                    settings.before + ls.iter().map(|l| l.height).sum::<f64>() + settings.after;
                let limit = band_limit - self.footnote_reservation(&[], self.column_x(col));
                if settings.keep
                    && height <= limit - self.column_top()
                    && y + height > limit
                    && y > self.column_top() + 0.01
                {
                    let old_page = self.pages.len();
                    self.advance_complex_column(section, &mut col, &mut y)?;
                    if old_page != self.pages.len() {
                        band_end = y;
                        band_limit = self.ph - self.bottom;
                    }
                    continue;
                }
                y += settings.before;
                let items = Self::items(&p);
                let mut at = 0;
                let mut replan = false;
                while at < ls.len() {
                    let mut end = at + 1;
                    while end < ls.len() && ls[end - 1].height == 0. {
                        end += 1;
                    }
                    let stop = ls.get(end).map(|l| l.pos).unwrap_or(usize::MAX);
                    let incoming: Vec<_> = items
                        .iter()
                        .filter(|i| i.pos >= ls[at].pos && i.pos < stop)
                        .filter_map(|i| match &i.kind {
                            ItemKind::Control(n) if n.name == "footNote" => Some(n),
                            _ => None,
                        })
                        .collect();
                    let row_height = ls[at..end].iter().map(|l| l.height).sum::<f64>();
                    let x = self.column_x(col);
                    let reserve = self.footnote_reservation(&incoming, x);
                    let object = if ls[at].block {
                        items
                            .iter()
                            .filter(|i| i.pos >= ls[at].pos && i.pos < stop)
                            .find_map(|i| {
                                if let ItemKind::Object(n) = &i.kind {
                                    Some(n)
                                } else {
                                    None
                                }
                            })
                    } else {
                        None
                    };
                    let splittable = object.is_some_and(|n| n.name == "tbl");
                    if !splittable && y + row_height > band_limit - reserve + 0.001 {
                        anyhow::ensure!(
                            y > self.top + 0.01,
                            "A reflow row exceeds column capacity"
                        );
                        start = ls[at].pos;
                        let old_page = self.pages.len();
                        self.advance_complex_column(section, &mut col, &mut y)?;
                        if old_page != self.pages.len() {
                            band_end = y;
                            band_limit = self.ph - self.bottom;
                        }
                        self.refresh_shared_notes(&paragraphs, index)?;
                        replan = true;
                        break;
                    }
                    if let Some(object) = object {
                        let cap = caption_extent(object);
                        let old_page = self.pages.len();
                        self.reflow_table_column = (object.name == "tbl").then_some(col);
                        self.object(object, x, y + cap[2], y + cap[2], x, width)?;
                        if object.name == "tbl" {
                            y = self.last_table_end
                                + if self.split_caption_handled {
                                    0.
                                } else {
                                    cap[3]
                                };
                        } else {
                            y += row_height;
                        }
                        if let Some(last) = self.reflow_table_column.take() {
                            col = last;
                        }
                        if old_page != self.pages.len() {
                            band_end = self.column_top();
                            band_limit = self.ph - self.bottom;
                        }
                        self.flow_floor = None;
                        if end < ls.len() && (self.column_width(col) - width).abs() > 0.01 {
                            start = ls[end].pos;
                            replan = true;
                            break;
                        }
                    } else {
                        for (i, line) in ls.iter().enumerate().take(end).skip(at) {
                            self.line(&p, i, x, y - lines(&p)[i].n("vertpos") / 100., width)?;
                            self.report.bounds.push(Bounds {
                                page: self.pages.len() + 1,
                                kind: "reflow-line".into(),
                                id: p.a("id").into(),
                                x: x + line.left,
                                y,
                                w: line.width,
                                h: line.height,
                            });
                            y += line.height;
                        }
                    }
                    band_end = band_end.max(y);
                    at = end;
                }
                if replan {
                    continue;
                }
                y += settings.after;
                band_end = band_end.max(y);
                break;
            }
        }
        self.report.warnings.insert("Unequal-column reflow rebuilds text at each column width; floating objects use measured blocks".into());
        Ok((y, col, self.column_width(col), self.active_gap))
    }
}
