//! Section dispatch and saved-layout rendering.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn render_mode(&mut self, force_reflow: bool) -> Result<()> {
        let mut reflow_used = false;
        let mut note_start = (0., 0usize, 0., 0.);
        let prepared = self.prepare_field_sections()?;
        let sections = prepared.as_deref().unwrap_or(&self.doc.sections);
        for (section_index, section) in sections.iter().enumerate() {
            self.active_section = section_index;
            self.field_stack.clear();
            if let Some(number) = section.find("newNumber").filter(|n| n.a("type") == "0") {
                self.page_base = number.u("number") as i64 - self.pages.len() as i64;
            }
            let sec = section
                .find("secPr")
                .context("Missing section properties")?;
            let p = sec.child("pagePr").context("Missing page size")?;
            self.pw = p.n("width") / 100.;
            self.ph = p.n("height") / 100.;
            let m = margin(p, "margin");
            self.left = m[0];
            self.right = m[1];
            self.top = m[2] + val(p.child("margin"), "header");
            self.bottom = m[3] + val(p.child("margin"), "footer");
            self.page_float_floor = self.top;
            self.header = val(p.child("margin"), "header");
            self.footer = val(p.child("margin"), "footer");
            let col = section_columns(section);
            let mut cols = col.map(|c| c.u("colCount")).unwrap_or(1).max(1);
            let mut gap = val(col, "sameGap");
            let mut cw = (self.pw - self.left - self.right - gap * (cols - 1) as f64) / cols as f64;
            self.active_columns = cols;
            self.active_gap = gap;
            self.column_widths.clear();
            self.column_origins.clear();
            self.column_definition = None;
            self.shared_note_reserve = 0.;
            self.column_band_top = self.top;
            self.reflow_column = 0;
            let rebuild = force_reflow || Self::section_needs_reflow(section);
            if !force_reflow && rebuild && section.children_named("p").all(|p| !lines(p).is_empty())
            {
                self.report.warnings.insert(
                    "Partial nested line cache recovered by rebuilding the section flow".into(),
                );
            }
            self.prepare_furniture(section, rebuild)?;
            self.page_float_floor = self.top;
            self.column_band_top = self.top;
            if rebuild && Self::complex_columns(section) {
                self.configure_columns(col)?;
            }
            self.decorations(section, true, cols, gap)?;
            if rebuild {
                reflow_used = true;
                note_start = self.reflow_text_section(section, cw, cols, gap)?;
            } else {
                let mut col = 0;
                let mut last = None;
                let mut flow_shift = 0f64;
                let mut table_reflowed = false;
                let mut repage_saved_tail = false;
                let mut previous_end = self.top;
                let fallback_flow = section
                    .children_named("p")
                    .filter(|p| !text(p).trim().is_empty())
                    .count()
                    > 1
                    && section
                        .children_named("p")
                        .flat_map(lines)
                        .all(|l| l.n("vertpos") == 0.);
                if fallback_flow {
                    self.report.warnings.insert(
                        "Rebuilt paragraph flow: all cached paragraph origins were zero".into(),
                    );
                }
                for p in section.children_named("p") {
                    let ls = lines(p);
                    if ls.is_empty() {
                        bail!("Missing saved layout cache in paragraph {}", p.a("id"))
                    }
                    if let Some(def) = paragraph_columns(p) {
                        let next_count = def.u("colCount").max(1);
                        let next_gap = val(Some(def), "sameGap");
                        if next_count != cols || next_gap != gap {
                            repage_saved_tail = false;
                            if last.is_some_and(|y| ls[0].n("vertpos") < y)
                                || p.a("pageBreak") == "1"
                            {
                                self.finish()?;
                                last = None;
                                flow_shift = 0.;
                            }
                            cols = next_count;
                            gap = next_gap;
                            cw = (self.pw - self.left - self.right - gap * (cols - 1) as f64)
                                / cols as f64;
                            col = 0;
                            self.active_columns = cols;
                            self.active_gap = gap;
                        }
                    }
                    for (i, l) in ls.iter().enumerate() {
                        let y = l.n("vertpos");
                        let floor = self.flow_floor.take().filter(|f| f.0 == self.pages.len());
                        let table_consumed_break =
                            floor.is_some_and(|f| f.2) && p.a("pageBreak") != "1";
                        let cached_break =
                            last.is_some_and(|last| y < last) && !table_consumed_break;
                        let explicit_break = last.is_some() && i == 0 && p.a("pageBreak") == "1";
                        if cached_break && repage_saved_tail && !explicit_break {
                            // Changed table/note geometry invalidated the saved page boundary.
                            // Reuse cached line positions as continuous flow until an explicit break.
                            flow_shift = previous_end - self.top - y / 100.;
                        } else if cached_break || explicit_break {
                            repage_saved_tail = false;
                            col += 1;
                            flow_shift = 0.;
                            table_reflowed = false;
                            if i == 0 && p.a("pageBreak") == "1" {
                                col = cols
                            }
                            if col >= cols {
                                self.finish()?;
                                col = 0;
                                self.decorations(section, false, cols, gap)?;
                            }
                        }
                        if let Some((_, bottom, continued)) =
                            floor.filter(|f| f.0 == self.pages.len())
                        {
                            let shift = bottom - self.top - y / 100.;
                            // The table already crossed the cached page boundary. Its
                            // actual last fragment, not the stale continuation Y, owns
                            // the following flow origin (including upward movement).
                            flow_shift = if continued {
                                shift
                            } else {
                                flow_shift.max(shift)
                            };
                            table_reflowed = true;
                        }
                        let end = ls.get(i + 1).map(|l| l.u("textpos")).unwrap_or(usize::MAX);
                        let its = Self::items(p);
                        let incoming: Vec<_> = its
                            .iter()
                            .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            .filter_map(|it| match &it.kind {
                                ItemKind::Control(n) if n.name == "footNote" => Some(n),
                                _ => None,
                            })
                            .collect();
                        let reserve = self
                            .footnote_reservation(&incoming, self.left + col as f64 * (cw + gap));
                        let visible = its
                            .iter()
                            .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            .any(|it| match &it.kind {
                                ItemKind::Char(c) => !c.is_whitespace(),
                                ItemKind::Object(_) => true,
                                ItemKind::Control(n) => {
                                    matches!(n.name.as_str(), "footNote" | "endNote" | "autoNum")
                                }
                                _ => false,
                            });
                        let inline_object = its.iter().filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            .any(|it| matches!(&it.kind,ItemKind::Object(n) if n.name=="tbl" && n.a("pageBreak")=="NONE" && n.child("pos").is_some_and(|p|p.a("treatAsChar")=="1")));
                        let line_slot = l.n("vertsize")
                            + if inline_object {
                                l.n("spacing").max(0.)
                            } else {
                                0.
                            };
                        if visible
                            && (repage_saved_tail
                                || reserve > 0.
                                || fallback_flow
                                || table_reflowed
                                || (inline_object
                                    && self.top + flow_shift + (y + l.n("vertsize")) / 100.
                                        <= self.ph - self.bottom
                                    && line_slot / 100. <= self.ph - self.top - self.bottom))
                            && self.top + flow_shift + (y + line_slot) / 100.
                                > self.ph - self.bottom - reserve
                            && self.top + flow_shift + y / 100. > self.top + 0.1
                        {
                            col += 1;
                            if col >= cols {
                                self.finish()?;
                                col = 0;
                                self.decorations(section, false, cols, gap)?;
                            }
                            flow_shift = -y / 100.;
                            if cols == 1 && (reserve > 0. || table_reflowed) {
                                repage_saved_tail = true;
                            }
                        }
                        self.line(
                            p,
                            i,
                            self.left + col as f64 * (cw + gap),
                            self.top + flow_shift,
                            cw,
                        )?;
                        let end = ls.get(i + 1).map(|n| n.u("textpos")).unwrap_or(usize::MAX);
                        let growth = Self::items(p)
                            .iter()
                            .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            .filter_map(|it| {
                                if let ItemKind::Object(n) = &it.kind {
                                    if n.child("pos")
                                        .map(|p| p.a("treatAsChar") == "1")
                                        .unwrap_or(false)
                                    {
                                        Some(
                                            (effective_height(n) - val(n.child("sz"), "height"))
                                                .max(0.),
                                        )
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .fold(0., f64::max);
                        if growth > 0.01 {
                            self.report.warnings.insert(format!("Repaired stale inline shape height: paragraph {}, +{growth:.2}pt; subsequent lines shifted",p.a("id")));
                            flow_shift += growth;
                        }
                        note_start = (
                            self.top + flow_shift + (y + l.n("vertsize") + l.n("spacing")) / 100.,
                            col,
                            cw,
                            gap,
                        );
                        last = Some(y);
                        previous_end =
                            self.top + flow_shift + (y + l.n("vertsize") + l.n("spacing")) / 100.;
                        if fallback_flow {
                            flow_shift += (l.n("vertsize") + l.n("spacing")) / 100.;
                        }
                    }
                }
            }
            if self
                .note_spec(true)
                .and_then(|n| n.child("placement"))
                .is_some_and(|n| n.a("place") == "END_OF_SECTION")
            {
                self.render_endnotes(note_start, Some(section_index))?;
            }
            if section_index + 1 < self.doc.sections.len() {
                self.finish()?;
                self.drain_note_tail()?;
            }
        }
        self.render_endnotes(note_start, None)?;
        if !self.current.ops.is_empty() || (reflow_used && self.pages.is_empty()) {
            self.finish()?;
        }
        self.drain_note_tail()?;
        self.resolve_page_fields();
        self.report.pages = self.pages.len();
        if !reflow_used {
            self.report.warnings.insert("Saved-layout renderer: font substitutions can differ from Hancom; no general paragraph reflow".into());
        }
        Ok(())
    }
}
