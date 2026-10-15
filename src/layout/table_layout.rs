//! Layout responsibility: table layout.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn table(&mut self, n: &Node, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        let repaired = self.repair_saved_cell_text(n)?;
        let n = repaired.as_ref().unwrap_or(n);
        let rows = n.u("rowCnt");
        let cols = n.u("colCnt");
        let cells: Vec<_> = n
            .children_named("tr")
            .flat_map(|r| r.children_named("tc"))
            .collect();
        let (cw, rh, total) = self.measure_table_grid(n, h)?;
        if self.depth == 0
            && (self.reflow_table_column.is_some()
                || (n.a("reflowTable") == "1" && n.child("caption").is_some()))
        {
            anyhow::ensure!(
                self.split_nested_vectors(n, x, y, w, total)?,
                "This table structure cannot yet be split across reflow columns"
            );
            return Ok(());
        }
        // Saved text tables can cross a page within a cell. Measure their actual
        // ink and note anchors, as for mixed tables, instead of moving whole rows.
        if self.depth == 0
            && n.a("binaryHwp") == "1"
            && n.a("pageBreak") == "TABLE"
            && n.child("pos").is_some_and(|p| {
                p.a("treatAsChar") == "0"
                    && p.a("vertRelTo") == "PARA"
                    && p.a("vertAlign") != "BOTTOM"
            })
            && total > self.ph - self.bottom - y
            && cells
                .iter()
                .filter_map(|c| c.child("subList"))
                .flat_map(|l| l.children_named("p"))
                .all(|p| {
                    Self::items(p)
                        .iter()
                        .all(|i| !matches!(i.kind, ItemKind::Object(_)))
                })
            && self.split_nested_vectors(n, x, y, w, total)?
        {
            return Ok(());
        }
        let fixed_bottom = n.child("pos").is_some_and(|p| p.a("vertAlign") == "BOTTOM");
        let nested = cells.iter().filter_map(|c| c.child("subList")).any(|l| {
            l.children_named("p").any(|p| {
                Self::items(p)
                    .iter()
                        .any(|i| matches!(&i.kind, ItemKind::Object(o) if effective_height(o) > self.ph-self.top-self.bottom))
            })
        });
        if self.depth == 0 && nested && total > self.ph - self.top - self.bottom {
            if self.split_nested_vectors(n, x, y, w, total)? {
                return Ok(());
            }
            let target_y = y.max(self.top).min(self.ph - self.bottom - 1.);
            let factor = ((self.ph - self.bottom - target_y) / total).min(1.);
            let matrix = [
                factor,
                0.,
                0.,
                factor,
                x * (1. - factor),
                target_y - factor * y,
            ];
            let begin = self.report.bounds.len();
            self.current.ops.push(Op::TransformBegin { matrix });
            self.depth += 1;
            self.table(n, x, y, w, total)?;
            self.depth -= 1;
            self.current.ops.push(Op::TransformEnd);
            for b in &mut self.report.bounds[begin..] {
                let r = crate::graphics::rect(matrix, [b.x, b.y, b.x + b.w, b.y + b.h]);
                b.x = r[0];
                b.y = r[1];
                b.w = r[2] - r[0];
                b.h = r[3] - r[1];
            }
            self.last_table_end = target_y + total * factor;
            self.flow_floor = Some((self.pages.len(), self.last_table_end, false));
            if let Some(b) = self.report.bounds[..begin]
                .iter_mut()
                .rev()
                .find(|b| b.kind == "tbl" && b.id == n.a("id") && b.page == self.pages.len() + 1)
            {
                b.x = x;
                b.y = target_y;
                b.w = w * factor;
                b.h = total * factor;
            }
            self.report.warnings.insert(format!("Oversized nested table {} fitted proportionally at {:.1}%; structural splitting remains limited",n.a("id"),factor*100.));
            return Ok(());
        }
        if self.depth == 0
            && !fixed_bottom
            && cells.len() > 1
            && rh.iter().any(|h| *h > self.ph - self.top - self.bottom)
            && cells
                .iter()
                .filter_map(|c| c.child("subList"))
                .flat_map(|l| l.children_named("p"))
                .all(|p| {
                    Self::items(p)
                        .iter()
                        .all(|i| !matches!(i.kind, ItemKind::Object(_)))
                })
        {
            return self.table_slices(n, &cells, &cw, &rh, x, y);
        }
        if self.depth == 0
            && !fixed_bottom
            && cells.len() == 1
            && cols == 1
            && total > self.ph - self.bottom - y
        {
            let cell = cells[0];
            let list = cell.child("subList").context("Cell paragraphs")?;
            let m = if cell.a("hasMargin") == "1" {
                margin(cell, "cellMargin")
            } else {
                margin(n, "inMargin")
            };
            let flow = flowing_lines(list);
            let mut at = 0;
            let mut top = y;
            while at < flow.len() {
                let start = flow[at].2;
                let capacity = self.ph - self.bottom - top - m[2] - m[3];
                let mut end = at;
                while end < flow.len() {
                    let (p, i, yy) = flow[end];
                    if end > at && yy - start + lines(p)[i].n("vertsize") / 100. > capacity {
                        break;
                    }
                    end += 1;
                }
                let (p, i, yy) = flow[end - 1];
                let height = yy - start + lines(p)[i].n("vertsize") / 100. + m[2] + m[3];
                self.border(cell.a("borderFillIDRef"), x, top, w, height);
                self.depth += 1;
                for (p, i, yy) in &flow[at..end] {
                    self.line(
                        p,
                        *i,
                        x + m[0],
                        top + m[2] + yy - start - lines(p)[*i].n("vertpos") / 100.,
                        w - m[0] - m[1],
                    )?;
                }
                self.depth -= 1;
                at = end;
                if at < flow.len() {
                    self.finish()?;
                    let section = &self.doc.sections[self.active_section];
                    self.decorations(section, false, 1, 0.)?;
                    top = self.top;
                }
            }
            self.report
                .warnings
                .insert("Long single-cell table split at line boundaries".into());
            return Ok(());
        }
        // Cached table heights include minimum/automatic row growth; keep them
        // instead of stretching cells to a stale object rectangle.
        let header_cells: Vec<_> = cells
            .iter()
            .filter(|c| c.child("cellAddr").is_some_and(|a| a.u("rowAddr") == 0))
            .map(|c| (*c).clone())
            .collect();
        let mut page_row = 0;
        let mut page_y = y;
        let mut previous_row = None;
        let captured_header = if self.capture_table_headers
            && n.a("repeatHeader") == "1"
            && rows > 1
            && rh[0] < (self.ph - self.top - self.bottom) / 3.
            && header_cells.iter().all(|c| {
                c.child("cellSpan").is_some_and(|s| s.u("rowSpan") == 1)
                    && c.find("footNote").is_none()
            }) {
            let index = self.table_headers.len();
            self.table_headers.push(TableHeaderPaint {
                y,
                height: rh[0],
                end: y + rh.iter().sum::<f64>(),
                ops: Vec::new(),
            });
            Some(index)
        } else {
            None
        };
        for c in &cells {
            let a = c.child("cellAddr").unwrap();
            let s = c.child("cellSpan").unwrap();
            let col = a.u("colAddr");
            let row = a.u("rowAddr");
            let mut band_end = if row == 0 && n.a("repeatHeader") == "1" && rows > 1 {
                2
            } else {
                row + 1
            };
            loop {
                let next = cells
                    .iter()
                    .filter(|c| {
                        let r = c.child("cellAddr").unwrap().u("rowAddr");
                        r >= row && r < band_end
                    })
                    .map(|c| {
                        c.child("cellAddr").unwrap().u("rowAddr")
                            + c.child("cellSpan").unwrap().u("rowSpan")
                    })
                    .max()
                    .unwrap_or(band_end);
                if next == band_end {
                    break;
                }
                band_end = next;
            }
            let mut incoming_notes = Vec::new();
            for c in cells.iter().filter(|c| {
                let r = c.child("cellAddr").unwrap().u("rowAddr");
                r >= row && r < band_end
            }) {
                c.all("footNote", &mut incoming_notes);
            }
            let reserve = self.footnote_reservation(&incoming_notes, x);
            let band_height = rh[row..band_end].iter().sum::<f64>();
            if self.depth == 0
                && !fixed_bottom
                && (n.a("pageBreak") == "CELL"
                    || (n.a("pageBreak") == "TABLE" && (total > h + 0.5 || reserve > 0.))
                    || total > self.ph - self.top - self.bottom)
                && previous_row != Some(row)
                && (row > page_row || page_y > self.top + 0.1)
                && band_height < self.ph - self.top - self.bottom
                && page_y + rh[page_row..band_end].iter().sum::<f64>()
                    > self.ph - self.bottom - reserve
            {
                let merged_crossing = cells.iter().any(|cell| {
                    let a = cell.child("cellAddr").unwrap();
                    let s = cell.child("cellSpan").unwrap();
                    a.u("rowAddr") < row && a.u("rowAddr") + s.u("rowSpan") > row
                });
                if !merged_crossing {
                    self.finish()?;
                    let section = &self.doc.sections[self.active_section];
                    let columns = section_columns(section);
                    self.decorations(
                        section,
                        false,
                        columns.map(|n| n.u("colCount")).unwrap_or(1),
                        val(columns, "sameGap"),
                    )?;
                    page_y = self.top;
                    page_row = row;
                    if n.a("repeatHeader") == "1"
                        && header_cells
                            .iter()
                            .all(|c| c.child("cellSpan").unwrap().u("rowSpan") == 1)
                    {
                        let mut header = n.clone();
                        header.attrs.insert("rowCnt".into(), "1".into());
                        header.attrs.insert("pageBreak".into(), "NONE".into());
                        header.attrs.insert("repeatHeader".into(), "0".into());
                        header.children.retain(|c| c.name != "tr");
                        header.children.push(Node {
                            name: "tr".into(),
                            children: header_cells.clone(),
                            ..Default::default()
                        });
                        self.table(&header, x, page_y, w, rh[0])?;
                        page_y += rh[0];
                    }
                    self.report
                        .warnings
                        .insert("Long table paginated by row with repeated header".into());
                }
            }
            previous_row = Some(row);
            let xx = x + cw[..col].iter().sum::<f64>();
            let yy = page_y + rh[page_row..row].iter().sum::<f64>();
            let ww = cw[col..col + s.u("colSpan")].iter().sum::<f64>();
            let hh = rh[row..row + s.u("rowSpan")].iter().sum::<f64>();
            let cell_paint_start = self.current.ops.len();
            self.border(c.a("borderFillIDRef"), xx, yy, ww, hh);
            if let Some(list) = c.child("subList") {
                let m = if c.a("hasMargin") == "1" {
                    margin(c, "cellMargin")
                } else {
                    margin(n, "inMargin")
                };
                let (min, mut max) = floating_extent(list);
                max = max.max(self.measured_height(list)?);
                let spare = (hh - m[2] - m[3] - (max - min)).max(0.);
                let shift = match list.a("vertAlign") {
                    "CENTER" => spare / 2.,
                    "BOTTOM" => spare,
                    _ => 0.,
                };
                self.local(list, xx + m[0], yy + m[2] + shift - min, ww - m[0] - m[1])?;
            }
            if row == 0
                && let Some(index) = captured_header
            {
                self.table_headers[index]
                    .ops
                    .extend_from_slice(&self.current.ops[cell_paint_start..]);
            }
        }
        let _ = (w, h);
        if self.depth == 0 {
            self.last_table_end = page_y + rh[page_row..].iter().sum::<f64>();
        }
        Ok(())
    }
}
