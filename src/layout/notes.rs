//! Layout responsibility: notes.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn item_footnotes(items: &[Item], start: usize, end: usize) -> Vec<&Node> {
        let mut notes = Vec::new();
        for item in items.iter().filter(|i| i.pos >= start && i.pos < end) {
            match &item.kind {
                ItemKind::Control(n) | ItemKind::Object(n) => n.all("footNote", &mut notes),
                _ => {}
            }
        }
        notes
    }
    pub(super) fn note_spec(&self, end: bool) -> Option<&Node> {
        self.doc.sections[self.active_section]
            .find("secPr")
            .and_then(|s| s.child(if end { "endNotePr" } else { "footNotePr" }))
    }
    pub(super) fn note_gap(&self, end: bool, key: &str, default: f64) -> f64 {
        self.note_spec(end)
            .and_then(|s| s.child("noteSpacing"))
            .map(|s| s.n(key) / 100.)
            .unwrap_or(default)
    }
    pub(super) fn note_label(&self, n: &Node, end: bool) -> String {
        let number = if n.a("number").is_empty() {
            n.find("autoNum").map(|a| a.u("num")).unwrap_or(1)
        } else {
            n.u("number")
        };
        let format = self.note_spec(end).and_then(|s| s.child("autoNumFormat"));
        let label = crate::number_format::format(number, format.map(|f| f.a("type")).unwrap_or(""))
            .unwrap_or_else(|| number.to_string());
        format!(
            "{}{}{}",
            format.map(|f| f.a("prefixChar")).unwrap_or(""),
            label,
            format.map(|f| f.a("suffixChar")).unwrap_or(")")
        )
    }
    pub(super) fn auto_label(&self, n: &Node) -> String {
        let number = if n.a("numType") == "PAGE" {
            (self.pages.len() as i64 + self.page_base).max(1) as usize
        } else {
            n.u("num")
        };
        let f = n.child("autoNumFormat");
        let label = crate::number_format::format(number, f.map(|f| f.a("type")).unwrap_or(""))
            .unwrap_or_else(|| number.to_string());
        if matches!(n.a("numType"), "FOOTNOTE" | "ENDNOTE") {
            format!(
                "{}{}{}",
                f.map(|f| f.a("prefixChar")).unwrap_or(""),
                label,
                f.map(|f| f.a("suffixChar")).unwrap_or(")")
            )
        } else {
            label
        }
    }
    pub(super) fn footnote_area(&self, x: f64) -> (f64, f64) {
        let count = self.active_columns.max(1);
        let width = (self.pw - self.left - self.right - self.active_gap * (count - 1) as f64)
            / count as f64;
        let placement = self
            .note_spec(false)
            .and_then(|s| s.child("placement"))
            .map(|p| p.a("place"))
            .unwrap_or("EACH_COLUMN");
        if placement == "MERGED_COLUMN" {
            return (self.left, self.pw - self.left - self.right);
        }
        if !self.column_widths.is_empty() {
            let col = (0..count)
                .filter(|&i| placement == "RIGHT_MOST_COLUMN" || self.column_x(i) <= x + 0.01)
                .max_by(|&a, &b| self.column_x(a).total_cmp(&self.column_x(b)))
                .unwrap_or(0);
            return (self.column_x(col), self.column_width(col));
        }
        let col = if placement == "RIGHT_MOST_COLUMN" {
            count - 1
        } else {
            ((x - self.left) / (width + self.active_gap))
                .floor()
                .max(0.) as usize
        }
        .min(count - 1);
        (self.left + col as f64 * (width + self.active_gap), width)
    }
    pub(super) fn footnote_reservation(&self, extra: &[&Node], x: f64) -> f64 {
        let (nx, _) = self.footnote_area(x);
        let shared = if self.column_widths.is_empty() {
            0.
        } else {
            self.shared_note_reserve
        };
        let mut count = 0;
        let mut h = 0.;
        let pending = self.continued_notes.get(&((nx * 100.).round() as i64));
        if let Some((_, _, rows)) = pending {
            h += rows.iter().map(|r| r.2).sum::<f64>();
            count += 1;
        }
        for (n, xx, _) in &self.footnotes {
            if (nx - xx).abs() < 0.1
                && let Some(l) = n.child("subList")
            {
                h += content_height(l);
                count += 1;
            }
        }
        for n in extra {
            if let Some(l) = n.child("subList") {
                h += content_height(l);
                count += 1;
            }
        }
        if count == 0 {
            return shared;
        }
        let full = h
            + self.note_gap(false, "betweenNotes", 4.) * (count - 1) as f64
            + self.note_gap(false, "aboveLine", 6.)
            + self.note_gap(false, "belowLine", 4.)
            + 1.;
        let body = self.ph - self.top - self.bottom;
        let reservation = if pending.is_some() {
            full.min(body * 0.5)
        } else if full > body - 48. {
            body * 0.5
        } else {
            full
        };
        reservation.max(shared)
    }
    pub(super) fn note_separator(&mut self, end: bool, x: f64, y: f64, width: f64) {
        let line = self.note_spec(end).and_then(|s| s.child("noteLine"));
        if line.is_some_and(|l| l.a("type") == "NONE" || l.n("length") == 0.) {
            return;
        }
        let length = line.map(|l| l.n("length")).unwrap_or(-1.);
        let length = if length < 0. {
            width / 3.
        } else {
            (length / 100.).min(width)
        };
        let thickness = line
            .map(|l| {
                if l.a("width").ends_with("mm") {
                    l.a("width")
                        .trim_end_matches("mm")
                        .trim()
                        .parse::<f64>()
                        .unwrap_or(0.12)
                        * 72.
                        / 25.4
                } else {
                    [
                        0.1, 0.12, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 1., 1.5, 2., 3., 4.,
                        5.,
                    ][l.u("widthIndex").min(15)]
                        * 72.
                        / 25.4
                }
            })
            .unwrap_or(0.34);
        let c = color(line.map(|l| l.a("color")).unwrap_or("#000000"));
        self.current.ops.push(Op::Line {
            x,
            y,
            x2: x + length,
            y2: y,
            width: thickness,
            color: c,
        });
    }
    pub(super) fn next_note_column(
        &mut self,
        col: &mut usize,
        y: &mut f64,
        gap: f64,
    ) -> Result<()> {
        *col += 1;
        if *col >= self.active_columns.max(1) {
            self.finish()?;
            *col = 0;
            let section = &self.doc.sections[self.active_section];
            self.decorations(section, false, self.active_columns, gap)?;
        }
        *y = self.top;
        Ok(())
    }
    pub(super) fn render_endnotes(
        &mut self,
        start: (f64, usize, f64, f64),
        section_only: Option<usize>,
    ) -> Result<()> {
        let (notes, pending): (Vec<_>, Vec<_>) = std::mem::take(&mut self.notes)
            .into_iter()
            .partition(|n| section_only.is_none_or(|s| n.u("sourceSection") == s));
        self.notes = pending;
        if notes.is_empty() {
            return Ok(());
        }
        if notes.iter().any(|n| n.a("reflowEndnote") == "1") {
            return self.render_reflow_endnotes(notes, start);
        }
        let (mut y, mut col, width, gap) = start;
        let above = self.note_gap(true, "aboveLine", 0.);
        let below = self.note_gap(true, "belowLine", 0.);
        let between = self.note_gap(true, "betweenNotes", 8.);
        let mut separator = true;
        self.note_rendering = true;
        for n in notes {
            if let Some(list) = n.child("subList") {
                let h = content_height(list);
                if y + h + if separator { above + below + 1. } else { 0. } > self.ph - self.bottom
                    && y > self.top + 0.1
                {
                    self.next_note_column(&mut col, &mut y, gap)?;
                    self.note_rendering = true;
                    separator = true;
                }
                if separator {
                    self.note_separator(
                        true,
                        self.left + col as f64 * (width + gap),
                        y + above,
                        width,
                    );
                    y += above + below + 1.;
                    separator = false;
                }
                let x = self.left + col as f64 * (width + gap);
                if y + h <= self.ph - self.bottom {
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: "endnote".into(),
                        id: n.a("number").into(),
                        x,
                        y,
                        w: width,
                        h,
                    });
                    self.local(list, x, y, width)?;
                    y += h + between;
                } else {
                    let mut previous = 0.;
                    self.depth += 1;
                    for (p, i, yy) in flowing_lines(list) {
                        let line = lines(p)[i];
                        let height = line.n("vertsize") / 100.;
                        y += (yy - previous).max(0.);
                        if y + height > self.ph - self.bottom {
                            self.next_note_column(&mut col, &mut y, gap)?;
                            self.note_rendering = true;
                            self.note_separator(
                                true,
                                self.left + col as f64 * (width + gap),
                                y + above,
                                width,
                            );
                            y += above + below + 1.;
                        }
                        self.line(
                            p,
                            i,
                            self.left + col as f64 * (width + gap),
                            y - line.n("vertpos") / 100.,
                            width,
                        )?;
                        y += height;
                        previous = yy + height;
                    }
                    self.depth -= 1;
                    y += between;
                }
            }
        }
        self.note_rendering = false;
        self.report.warnings.insert("Endnotes follow saved lines with configured spacing and column count; exact Hancom pagination remains unverified".into());
        Ok(())
    }
    pub(super) fn finish(&mut self) -> Result<()> {
        let fields = std::mem::take(&mut self.field_stack);
        let result = self.finish_page_content();
        self.field_stack = fields;
        result
    }
    fn finish_page_content(&mut self) -> Result<()> {
        self.column_rules(self.ph - self.bottom - self.shared_note_reserve);
        let body = self.ph - self.top - self.bottom;
        let has_long_group = self.footnotes.iter().any(|(_, x, _)| {
            self.footnotes
                .iter()
                .filter(|(_, xx, _)| (xx - x).abs() < 0.1)
                .map(|(n, _, _)| {
                    n.child("subList").map(content_height).unwrap_or(0.)
                        + self.note_gap(false, "betweenNotes", 4.)
                })
                .sum::<f64>()
                - self.note_gap(false, "betweenNotes", 4.)
                + self.note_gap(false, "aboveLine", 6.)
                + self.note_gap(false, "belowLine", 4.)
                + 1.
                > body - 48.
        });
        if has_long_group || !self.continued_notes.is_empty() {
            return self.finish_long_notes();
        }
        let notes = std::mem::take(&mut self.footnotes);
        if !notes.is_empty() {
            let mut groups: BTreeMap<i64, Vec<(Node, f64, f64)>> = BTreeMap::new();
            for n in notes {
                groups
                    .entry((n.1 * 100.).round() as i64)
                    .or_default()
                    .push(n);
            }
            for notes in groups.into_values() {
                let gap = self.note_gap(false, "betweenNotes", 4.);
                let height = notes
                    .iter()
                    .filter_map(|(n, _, _)| n.child("subList"))
                    .map(content_height)
                    .sum::<f64>()
                    + gap * notes.len().saturating_sub(1) as f64;
                let mut y = self.ph - self.bottom - height;
                self.note_separator(
                    false,
                    notes[0].1,
                    y - self.note_gap(false, "belowLine", 4.),
                    notes[0].2,
                );
                self.note_rendering = true;
                for (n, x, w) in notes {
                    if let Some(list) = n.child("subList") {
                        self.report.bounds.push(Bounds {
                            page: self.pages.len() + 1,
                            kind: "footnote".into(),
                            id: n.a("number").into(),
                            x,
                            y,
                            w,
                            h: content_height(list),
                        });
                        self.local(list, x, y, w)?;
                        y += content_height(list) + gap;
                    }
                }
                self.note_rendering = false;
            }
        }

        self.current.width = self.pw;
        self.current.height = self.ph;
        self.pages.push(std::mem::take(&mut self.current));
        Ok(())
    }
    /// Drain long notes onto dedicated continuation pages before resuming body flow.
    /// Lines are indivisible; numbering is already present in their first line.
    pub(super) fn finish_long_notes(&mut self) -> Result<()> {
        let mut groups = std::mem::take(&mut self.continued_notes);
        for (note, x, w) in std::mem::take(&mut self.footnotes) {
            if let Some(list) = note.child("subList") {
                let flow = flowing_lines(list);
                let entry = groups.entry((x * 100.).round() as i64).or_insert((
                    x,
                    w,
                    std::collections::VecDeque::new(),
                ));
                for (idx, (p, line, y)) in flow.iter().enumerate() {
                    let height = flow
                        .get(idx + 1)
                        .map(|next| next.2 - y)
                        .unwrap_or_else(|| {
                            (lines(p)[*line].n("vertsize") + lines(p)[*line].n("spacing")) / 100.
                        })
                        .max(lines(p)[*line].n("vertsize") / 100.)
                        .max(0.1);
                    entry
                        .2
                        .push_back(((*p).clone(), *line, height, note.a("number").into()));
                }
                if let Some(last) = entry.2.back_mut() {
                    last.2 += self.note_gap(false, "betweenNotes", 4.);
                }
            }
        }
        let mut first = !self.draining_notes;
        loop {
            let overhead =
                self.note_gap(false, "aboveLine", 6.) + self.note_gap(false, "belowLine", 4.) + 1.;
            let capacity = (if first {
                let mut text_bottom = self.top;
                let mut matrix = crate::graphics::IDENTITY;
                let mut stack = Vec::new();
                for op in &self.current.ops {
                    match op {
                        Op::TransformBegin { matrix: m } => {
                            stack.push(matrix);
                            matrix = crate::graphics::compose(matrix, *m);
                        }
                        Op::TransformEnd => {
                            matrix = stack.pop().unwrap_or(crate::graphics::IDENTITY)
                        }
                        Op::Text { .. } | Op::Shaped { .. } => {
                            if let Some(b) = crate::pagination::bounds(op, &self.fonts) {
                                let b = crate::graphics::rect(matrix, b);
                                if b[1] >= self.top && b[3] < self.ph - self.bottom {
                                    text_bottom = text_bottom.max(b[3]);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let object_bottom = self
                    .report
                    .bounds
                    .iter()
                    .filter(|b| b.page == self.pages.len() + 1 && b.kind != "watermark")
                    .map(|b| b.y + b.h)
                    .fold(self.top, f64::max);
                // Body layout already reserved half a page. At finalization the
                // unoccupied space above it can safely hold more complete lines.
                (self.ph - self.bottom - text_bottom.max(object_bottom))
                    .max((self.ph - self.top - self.bottom) * 0.5)
            } else {
                self.ph - self.top - self.bottom
            }) - overhead;
            anyhow::ensure!(capacity > 0., "No printable area for footnote continuation");
            self.note_rendering = true;
            for (x, w, rows) in groups.values_mut() {
                let mut batch = Vec::new();
                let mut height = 0.;
                while let Some(row) = rows.front() {
                    if height + row.2 > capacity + 0.01 {
                        break;
                    }
                    let row = rows.pop_front().unwrap();
                    height += row.2;
                    batch.push(row);
                }
                if batch.is_empty() && !rows.is_empty() {
                    anyhow::ensure!(first, "A single footnote line exceeds a printable page");
                    continue;
                }
                if batch.is_empty() {
                    continue;
                }
                let mut y = self.ph - self.bottom - height;
                self.note_separator(false, *x, y - self.note_gap(false, "belowLine", 4.), *w);
                for (p, line, h, id) in batch {
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: "footnote-fragment".into(),
                        id,
                        x: *x,
                        y,
                        w: *w,
                        h,
                    });
                    self.depth += 1;
                    self.line(&p, line, *x, y - lines(&p)[line].n("vertpos") / 100., *w)?;
                    self.depth -= 1;
                    y += h;
                }
            }
            self.note_rendering = false;
            self.current.width = self.pw;
            self.current.height = self.ph;
            self.pages.push(std::mem::take(&mut self.current));
            groups.retain(|_, (_, _, rows)| !rows.is_empty());
            if !self.draining_notes {
                self.continued_notes = groups;
                break;
            }
            if groups.values().all(|(_, _, r)| r.is_empty()) {
                break;
            }
            let section = &self.doc.sections[self.active_section];
            self.decorations(section, false, self.active_columns, self.active_gap)?;
            first = false;
        }
        self.report.warnings.insert("Long footnotes share following body pages with a bounded note area; remaining section-end notes drain before the next section".into());
        Ok(())
    }
    pub(super) fn drain_note_tail(&mut self) -> Result<()> {
        if self.continued_notes.is_empty() {
            return Ok(());
        }
        let section = &self.doc.sections[self.active_section];
        self.decorations(section, false, self.active_columns, self.active_gap)?;
        self.draining_notes = true;
        let result = self.finish_long_notes();
        self.draining_notes = false;
        result
    }
}
