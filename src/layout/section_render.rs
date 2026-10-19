//! Section dispatch and saved-layout rendering.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn render_mode(&mut self, force_reflow: bool) -> Result<()> {
        if self.render_vertical(force_reflow)? {
            return Ok(());
        }
        let mut reflow_used = false;
        let mut note_start = (0., 0usize, 0., 0.);
        let mut prepared = self.prepare_field_sections()?;
        if self.has_paragraph_borders() {
            let sections = prepared.get_or_insert_with(|| self.doc.sections.clone());
            for section in sections {
                self.prepare_border_lists(section);
            }
        }
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
            let mode = sec.u("engineVerticalMode") as u8;
            if self.vertical_mode != mode {
                self.glyph_cache.clear();
                self.vertical_mode = mode;
            }
            if let Some(start) = sec.child("startNum").filter(|n| n.u("page") > 0) {
                self.page_base = start.u("page") as i64 - self.pages.len() as i64;
            }
            // pgct changes the displayed starting number, not the physical
            // sheet count. Supplied EVEN/ODD prints retain both body pages.
            if let Some(control) = section
                .children_named("p")
                .next()
                .into_iter()
                .flat_map(|p| p.children_named("run"))
                .flat_map(|r| r.children_named("ctrl"))
                .find_map(|c| c.child("pageNumCtrl"))
            {
                let number = self.pages.len() as i64 + self.page_base;
                let mismatch = match control.a("pageStartsOn") {
                    "EVEN" => number.rem_euclid(2) != 0,
                    "ODD" => number.rem_euclid(2) != 1,
                    _ => false,
                };
                if mismatch {
                    self.page_base += 1;
                }
            }
            let p = sec.child("pagePr").context("Missing page size")?;
            self.pw = p.n("width") / 100.;
            self.ph = p.n("height") / 100.;
            let m = margin(p, "margin");
            self.left = m[0];
            self.right = m[1];
            self.top = m[2] + val(p.child("margin"), "header");
            self.bottom = m[3] + val(p.child("margin"), "footer");
            let gutter = val(p.child("margin"), "gutter");
            if gutter > 0. {
                if matches!(p.a("gutterType"), "" | "LEFT_ONLY") {
                    self.left += gutter;
                } else {
                    self.report.warnings.insert(
                        "Nonzero facing/top binding gutter requires page-side-aware layout".into(),
                    );
                }
            }
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
            let rebuild =
                force_reflow || self.vertical_mode > 0 || Self::section_needs_reflow(section);
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
                let mut blank_flow = self
                    .recover_blank_columns(section, cols)
                    .then(saved_blank_flow::BlankColumnFlow::new);
                if blank_flow.is_some() {
                    self.report.warnings.insert("Rebuilt saved column flow across empty-paragraph cache boundaries; hideFirstEmptyLine suppresses up to two leading empty lines after automatic breaks".into());
                }
                let mut col = 0;
                let mut bands = (self.doc.header.a("binaryHwp") == "1"
                    && section
                        .children_named("p")
                        .any(|p| p.a("columnStart") == "1" && paragraph_columns(p).is_some()))
                .then(saved_column_bands::ColumnBands::default);
                let mut last = None;
                let mut flow_shift = 0f64;
                let mut table_reflowed = false;
                let mut repage_saved_tail = false;
                let mut previous_end = self.top;
                let mut spanning_floor: Option<(usize, f64)> = None;
                let mut spanning_shift: Option<(usize, f64)> = None;
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
                    let original_last = lines(p).last().map(|l| l.n("vertpos"));
                    let native_repaired = if cols == 1 {
                        self.repair_native_paragraph_cache(p, cw)?
                    } else {
                        None
                    };
                    let p = native_repaired.as_ref().unwrap_or(p);
                    // Logical blank-column recovery can propagate the changed
                    // line count without trusting the following stale origins.
                    let repaired = if blank_flow.is_some() {
                        self.repair_numbered_cache(p)?
                    } else {
                        None
                    };
                    let p = repaired.as_ref().unwrap_or(p);
                    let ls = lines(p);
                    if ls.is_empty() {
                        bail!("Missing saved layout cache in paragraph {}", p.a("id"))
                    }
                    if let Some(def) = paragraph_columns(p) {
                        let next_count = def.u("colCount").max(1);
                        let next_gap = val(Some(def), "sameGap");
                        if next_count != cols
                            || next_gap != gap
                            || (bands.is_some() && last.is_some() && p.a("columnStart") == "1")
                        {
                            blank_flow = None;
                            spanning_floor = None;
                            spanning_shift = None;
                            repage_saved_tail = false;
                            if (bands.is_none() && last.is_some_and(|y| ls[0].n("vertpos") < y))
                                || p.a("pageBreak") == "1"
                            {
                                self.finish()?;
                                last = None;
                                flow_shift = 0.;
                            }
                            if let Some(band) = &mut bands {
                                flow_shift = band.start(
                                    self.pages.len(),
                                    ls[0].n("vertpos") / 100.,
                                    sec.n("spaceColumns") / 100.,
                                );
                                if flow_shift > 0.
                                    && self.top
                                        + flow_shift
                                        + (ls[0].n("vertpos") + ls[0].n("vertsize")) / 100.
                                        > self.ph - self.bottom
                                {
                                    self.finish()?;
                                    flow_shift = band.start(
                                        self.pages.len(),
                                        ls[0].n("vertpos") / 100.,
                                        sec.n("spaceColumns") / 100.,
                                    );
                                }
                                last = None;
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
                        let cached_break = blank_flow.is_none()
                            && last.is_some_and(|last| {
                                y < last
                                    || (bands.is_some()
                                        && cols > 1
                                        && i > 0
                                        && y == last
                                        && l.n("horzpos") == ls[i - 1].n("horzpos"))
                            })
                            && !table_consumed_break;
                        let explicit_break = last.is_some()
                            && i == 0
                            && (p.a("pageBreak") == "1"
                                || ((blank_flow.is_some() || repage_saved_tail)
                                    && self
                                        .doc
                                        .paras
                                        .get(p.a("paraPrIDRef"))
                                        .and_then(|s| s.child("breakSetting"))
                                        .is_some_and(|b| {
                                            matches!(b.a("pageBreakBefore"), "1" | "true")
                                        })));
                        let explicit_column = last.is_some() && i == 0 && p.a("columnBreak") == "1";
                        if cached_break && repage_saved_tail && !explicit_break && !explicit_column
                        {
                            // Changed table/note geometry invalidated the saved page boundary.
                            // Reuse cached line positions as continuous flow until an explicit break.
                            flow_shift = previous_end - self.top - y / 100.;
                        } else if cached_break || explicit_break || explicit_column {
                            repage_saved_tail = false;
                            col += 1;
                            flow_shift = 0.;
                            table_reflowed = false;
                            if explicit_break {
                                col = cols
                            }
                            if col >= cols {
                                self.finish()?;
                                col = 0;
                                self.decorations(section, false, cols, gap)?;
                            }
                            if let Some(band) = &mut bands {
                                flow_shift = band.origin(self.pages.len());
                            }
                            if let Some((_, shift)) =
                                spanning_shift.filter(|(page, _)| *page == self.pages.len())
                            {
                                // A table covering multiple columns can grow beyond
                                // its saved reservation. All covered columns on this
                                // page share the repaired reservation.
                                flow_shift = shift;
                            }
                        }
                        if let Some(flow) = &mut blank_flow {
                            if explicit_break || explicit_column {
                                flow.break_column(false);
                                previous_end = spanning_floor
                                    .filter(|(page, _)| *page == self.pages.len())
                                    .map(|(_, bottom)| bottom)
                                    .unwrap_or(self.top)
                                    .max(self.top);
                            }
                            flow_shift = previous_end + flow.gap(y / 100.) - self.top - y / 100.;
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
                            if continued && cols == 1 && blank_flow.is_none() {
                                // A continued table invalidates cached body-page
                                // boundaries after shrinking as well as growing.
                                repage_saved_tail = true;
                            }
                            if spanning_floor.is_some_and(|(page, end)| {
                                page == self.pages.len() && (end - bottom).abs() < 0.001
                            }) {
                                spanning_shift = Some((self.pages.len(), flow_shift));
                            }
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
                        let reserve = self.saved_line_note_reservation(
                            &incoming,
                            self.left + col as f64 * (cw + gap),
                            self.top + flow_shift + (y + l.n("vertsize")) / 100.,
                        );
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
                            })
                            || (blank_flow.is_some()
                                && self
                                    .doc
                                    .paras
                                    .get(p.a("paraPrIDRef"))
                                    .and_then(|s| s.child("heading"))
                                    .is_some_and(|h| {
                                        matches!(h.a("type"), "NUMBER" | "BULLET" | "OUTLINE")
                                    }));
                        let inline_object = its.iter().filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            .any(|it| matches!(&it.kind,ItemKind::Object(n) if n.name=="tbl" && n.a("pageBreak")=="NONE" && n.child("pos").is_some_and(|p|p.a("treatAsChar")=="1")));
                        let line_slot = l.n("vertsize")
                            + if inline_object {
                                l.n("spacing").max(0.)
                            } else {
                                0.
                            };
                        let fit_slot = if blank_flow.is_some() {
                            l.n("vertsize")
                        } else {
                            line_slot
                        };
                        if (visible || blank_flow.is_some() || repage_saved_tail)
                            && (blank_flow.is_some()
                                || repage_saved_tail
                                || reserve > 0.
                                || fallback_flow
                                || table_reflowed
                                || (inline_object
                                    && self.top + flow_shift + (y + l.n("vertsize")) / 100.
                                        <= self.ph - self.bottom
                                    && line_slot / 100. <= self.ph - self.top - self.bottom))
                            && self.top + flow_shift + (y + fit_slot) / 100.
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
                            if let Some(flow) = &mut blank_flow {
                                flow.break_column(true);
                                if let Some((_, bottom)) =
                                    spanning_floor.filter(|(page, _)| *page == self.pages.len())
                                {
                                    flow_shift += bottom - self.top;
                                }
                            }
                            if cols == 1 && (reserve > 0. || table_reflowed) {
                                repage_saved_tail = true;
                            }
                        }
                        if let Some(flow) = &mut blank_flow {
                            flow.observe(y / 100., (l.n("vertsize") + l.n("spacing")) / 100.);
                            if flow.hide(!visible) {
                                previous_end = self.top + flow_shift + y / 100.;
                                last = Some(y);
                                note_start = (previous_end, col, cw, gap);
                                continue;
                            }
                        }
                        self.line(
                            p,
                            i,
                            self.left + col as f64 * (cw + gap),
                            self.top + flow_shift,
                            cw,
                        )?;
                        if cols > 1
                            && col == 0
                            && its.iter().any(|it| {
                                it.pos >= l.u("textpos") && it.pos < end
                                    && matches!(&it.kind, ItemKind::Object(n)
                                        if n.name == "tbl"
                                        && val(n.child("sz"), "width")
                                            + margin(n, "outMargin")[0]
                                            + margin(n, "outMargin")[1]
                                            >= self.pw - self.left - self.right - 0.01
                                        && n.child("pos").is_some_and(|p| p.a("treatAsChar") == "0"))
                            })
                        {
                            spanning_floor = self.flow_floor.map(|(page, bottom, _)| (page, bottom));
                        }
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
                                            (effective_height(n)
                                                - val(n.child("sz"), "height")
                                                    .max(l.n("vertsize") / 100.))
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
                        if let Some(band) = &mut bands {
                            band.observe(
                                self.pages.len(),
                                flow_shift + (y + l.n("vertsize")) / 100.,
                            );
                        }
                        if fallback_flow {
                            flow_shift += (l.n("vertsize") + l.n("spacing")) / 100.;
                        }
                    }
                    if native_repaired.is_some()
                        && let (Some(old), Some(new)) = (original_last, ls.last())
                    {
                        flow_shift += (new.n("vertpos") - old) / 100.;
                        last = Some(old);
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
