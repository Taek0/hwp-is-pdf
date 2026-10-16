//! Layout responsibility: table pagination.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn table_slices(
        &mut self,
        n: &Node,
        cells: &[&Node],
        cw: &[f64],
        rh: &[f64],
        x: f64,
        mut top: f64,
    ) -> Result<()> {
        let total = rh.iter().sum::<f64>();
        let mut contents = Vec::new();
        for c in cells {
            let a = c.child("cellAddr").unwrap();
            let s = c.child("cellSpan").unwrap();
            let row = a.u("rowAddr");
            let col = a.u("colAddr");
            let cy = rh[..row].iter().sum::<f64>();
            let ch = rh[row..row + s.u("rowSpan")].iter().sum::<f64>();
            let cx = x + cw[..col].iter().sum::<f64>();
            let ww = cw[col..col + s.u("colSpan")].iter().sum::<f64>();
            let m = if c.a("hasMargin") == "1" {
                margin(c, "cellMargin")
            } else {
                margin(n, "inMargin")
            };
            let mut flow = Vec::new();
            if let Some(l) = c.child("subList") {
                let (min, max) = floating_extent(l);
                let spare = (ch - m[2] - m[3] - (max - min)).max(0.);
                let shift = match l.a("vertAlign") {
                    "CENTER" => spare / 2.,
                    "BOTTOM" => spare,
                    _ => 0.,
                };
                for (p, i, yy) in flowing_lines(l) {
                    flow.push((p, i, cy + m[2] + shift - min + yy));
                }
            }
            contents.push((*c, cx, cy, ww, ch, m, flow));
        }
        let mut start = 0.;
        let header_rows = table_headers::band_rows(n).unwrap_or(0);
        let header_height = rh[..header_rows].iter().sum::<f64>();
        while start < total - 0.01 {
            if header_rows > 0 && start >= header_height - 0.01 {
                let header_cells: Vec<Node> = cells
                    .iter()
                    .filter(|c| c.child("cellAddr").unwrap().u("rowAddr") < header_rows)
                    .map(|c| (*c).clone())
                    .collect();
                if !header_cells.is_empty()
                    && header_cells.iter().all(|c| c.find("footNote").is_none())
                    && header_height < (self.ph - self.top - self.bottom) / 3.
                {
                    let mut header = n.clone();
                    header
                        .attrs
                        .insert("rowCnt".into(), header_rows.to_string());
                    header.attrs.insert("repeatHeader".into(), "0".into());
                    header.attrs.insert("pageBreak".into(), "NONE".into());
                    header.children.retain(|c| c.name != "tr");
                    header.children.push(Node {
                        name: "tr".into(),
                        children: header_cells,
                        ..Default::default()
                    });
                    self.depth += 1;
                    self.table(&header, x, top, cw.iter().sum(), header_height)?;
                    self.depth -= 1;
                    top += header_height;
                }
            }
            let mut cut = (start + self.ph - self.bottom - top).min(total);
            for _ in 0..64 {
                let mut notes = Vec::new();
                for (_, _, _, _, _, _, flow) in &contents {
                    for (p, i, yy) in flow {
                        if *yy >= start - 0.01 && *yy < cut {
                            let ls = lines(p);
                            let end = ls.get(i + 1).map(|l| l.u("textpos")).unwrap_or(usize::MAX);
                            for it in Self::items(p) {
                                if it.pos >= ls[*i].u("textpos")
                                    && it.pos < end
                                    && let ItemKind::Control(n) = it.kind
                                    && n.name == "footNote"
                                {
                                    notes.push(n);
                                }
                            }
                        }
                    }
                }
                let refs: Vec<_> = notes.iter().collect();
                let reserve = self.footnote_reservation(&refs, x);
                let mut next = (start + self.ph - self.bottom - top - reserve).min(cut);
                for (_, _, _, _, _, _, flow) in &contents {
                    for (p, i, yy) in flow {
                        let end = yy + lines(p)[*i].n("vertsize") / 100. + 0.5;
                        if *yy < next && end > next {
                            next = next.min(*yy);
                        }
                    }
                }
                if (next - cut).abs() < 0.01 {
                    break;
                }
                cut = next;
            }
            if cut <= start + 0.1 {
                if top > self.top + 0.1 {
                    self.finish()?;
                    let sec = &self.doc.sections[self.active_section];
                    self.decorations(sec, false, self.active_columns, self.active_gap)?;
                    top = self.column_top();
                    continue;
                }
                bail!("Table line with footnotes exceeds a full page");
            }
            for (c, cx, cy, ww, ch, m, flow) in &contents {
                let a = cy.max(start);
                let b = (cy + ch).min(cut);
                if b <= a {
                    continue;
                }
                self.border(c.a("borderFillIDRef"), *cx, top + a - start, *ww, b - a);
                self.depth += 1;
                for (p, i, yy) in flow {
                    if *yy >= start - 0.01 && *yy < cut - 0.01 {
                        self.line(
                            p,
                            *i,
                            cx + m[0],
                            top + yy - start - lines(p)[*i].n("vertpos") / 100.,
                            ww - m[0] - m[1],
                        )?;
                    }
                }
                self.depth -= 1;
            }
            self.last_table_end = top + cut - start;
            start = cut;
            if start < total - 0.01 {
                self.finish()?;
                let sec = &self.doc.sections[self.active_section];
                self.decorations(sec, false, self.active_columns, self.active_gap)?;
                top = self.column_top();
            }
        }
        self.report.warnings.insert("Oversized table row split at shared text-line boundaries; merged cell borders continued".into());
        Ok(())
    }
    /// Reflow consumes columns before advancing a page; saved layouts keep
    /// their original page-only splitting behavior.
    pub(super) fn advance_table_fragment(&mut self, x: &mut f64) -> Result<()> {
        if !self.column_widths.is_empty()
            && let Some(mut col) = self.reflow_table_column
        {
            let offset = *x - self.column_x(col);
            let section = &self.doc.sections[self.active_section];
            let mut y = self.column_top();
            self.advance_complex_column(section, &mut col, &mut y)?;
            *x = self.column_x(col) + offset;
            self.reflow_table_column = Some(col);
            return Ok(());
        }
        if let Some(col) = self.reflow_table_column {
            let width = (self.pw
                - self.left
                - self.right
                - self.active_gap * (self.active_columns - 1) as f64)
                / self.active_columns as f64;
            let next = (col + 1) % self.active_columns;
            *x += (next as f64 - col as f64) * (width + self.active_gap);
            self.reflow_table_column = Some(next);
            if next != 0 {
                return Ok(());
            }
        }
        self.finish()?;
        self.page_float_floor = self.top;
        let section = &self.doc.sections[self.active_section];
        self.decorations(section, false, self.active_columns, self.active_gap)?;
        Ok(())
    }
}
