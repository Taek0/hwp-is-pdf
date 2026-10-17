//! Fragment-local layout for native plain-text rows without body row-spans.
//! Each fragment owns its padding and vertical alignment; consumed cell text
//! is not repeated on following pages.
use super::*;
struct Cell<'b> {
    node: &'b Node,
    flow: Vec<(&'b Node, usize, f64, f64)>,
    at: usize,
    x: f64,
    width: f64,
    margin: [f64; 4],
}
impl Cell<'_> {
    fn content_height(&self, count: usize) -> f64 {
        if count == 0 {
            return 0.;
        }
        let (_, _, origin, _) = self.flow[self.at];
        let (p, i, y, ink) = self.flow[self.at + count - 1];
        let logical = lines(p)[i].n("vertsize") / 100.;
        y - if self.at == 0 { 0. } else { origin } + if logical > 0. { logical } else { ink }
    }
    fn ink_height(&self, count: usize) -> f64 {
        if count == 0 {
            return 0.;
        }
        let origin = if self.at == 0 {
            0.
        } else {
            self.flow[self.at].2
        };
        self.flow[self.at..self.at + count]
            .iter()
            .map(|(_, _, y, h)| y - origin + h)
            .fold(0., f64::max)
    }
    fn required(&self, count: usize) -> f64 {
        if count == 0 {
            0.
        } else {
            self.margin[2]
                + (self.content_height(count) + self.margin[3]).max(self.ink_height(count))
        }
    }
    fn fit(&self, height: f64) -> usize {
        let mut count = 0;
        while self.at + count < self.flow.len() && self.required(count + 1) <= height + 0.01 {
            count += 1;
        }
        count
    }
}
struct Row<'b> {
    cells: Vec<Cell<'b>>,
    minimum: f64,
}
impl Row<'_> {
    fn remaining(&self) -> f64 {
        self.cells
            .iter()
            .map(|c| c.required(c.flow.len() - c.at))
            .fold(self.minimum, f64::max)
    }
    fn done(&self) -> bool {
        self.minimum <= 0.01 && self.cells.iter().all(|c| c.at == c.flow.len())
    }
}
impl<'a> Engine<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn native_table_rows(
        &mut self,
        n: &Node,
        mut x: f64,
        mut top: f64,
        w: f64,
        cw: &[f64],
        rh: &[f64],
        total: f64,
    ) -> Result<bool> {
        if self.depth != 0
            || self.active_columns != 1
            || n.a("binaryHwp") != "1"
            || n.a("reflowTable") == "1"
            || n.a("pageBreak") != "TABLE"
            || total <= self.ph - self.bottom - top
            || n.find("footNote").is_some()
            || n.find("endNote").is_some()
            || !n.child("pos").is_some_and(|p| {
                p.a("treatAsChar") == "0"
                    && p.a("vertRelTo") == "PARA"
                    && p.a("vertAlign") != "BOTTOM"
            })
            || n.children_named("caption")
                .any(|c| !matches!(c.a("side"), "TOP" | "BOTTOM"))
        {
            return Ok(false);
        }
        let header_rows = table_headers::band_rows(n).unwrap_or(0);
        let cells: Vec<_> = n
            .children_named("tr")
            .flat_map(|r| r.children_named("tc"))
            .collect();
        if n.a("repeatHeader") == "1" && cells.iter().all(|c| c.a("header").is_empty()) {
            return Ok(false);
        }
        if cells.iter().any(|c| {
            let a = c.child("cellAddr").unwrap();
            (a.u("rowAddr") >= header_rows && c.child("cellSpan").unwrap().u("rowSpan") != 1)
                || c.child("subList").is_some_and(|l| {
                    l.a("savedColumns") == "1"
                        || l.children_named("p").any(|p| {
                            Self::items(p)
                                .iter()
                                .any(|i| !matches!(i.kind, ItemKind::Char(_) | ItemKind::Tab(_)))
                                || self.doc.paras.get(p.a("paraPrIDRef")).is_some_and(|s| {
                                    s.child("breakSetting").is_some_and(|b| {
                                        ["keepLines", "keepWithNext", "widowOrphan"]
                                            .iter()
                                            .any(|k| b.a(k) == "1")
                                    }) || s
                                        .child("border")
                                        .and_then(|b| self.doc.borders.get(b.a("borderFillIDRef")))
                                        .is_some_and(|b| {
                                            b.children.iter().any(|s| {
                                                s.name.ends_with("Border") && s.a("type") != "NONE"
                                            }) || b
                                                .find("winBrush")
                                                .is_some_and(|f| f.a("faceColor") != "none")
                                        })
                                })
                        })
                })
        }) {
            return Ok(false);
        }
        let outside = margin(n, "outMargin");
        let header_height = rh[..header_rows].iter().sum::<f64>();
        let maximum = self.ph - self.top - self.bottom - outside[2] - outside[3] - header_height;
        if maximum <= 24. {
            return Ok(false);
        }
        let defaults = margin(n, "inMargin");
        let mut rows = Vec::new();
        for row in header_rows..rh.len() {
            let mut content = Vec::new();
            let mut minimum = 0f64;
            for c in cells
                .iter()
                .filter(|c| c.child("cellAddr").unwrap().u("rowAddr") == row)
            {
                let col = c.child("cellAddr").unwrap().u("colAddr");
                let span = c.child("cellSpan").unwrap().u("colSpan");
                let m = if c.a("hasMargin") == "1" {
                    margin(c, "cellMargin")
                } else {
                    defaults
                };
                let flow = if let Some(list) = c.child("subList") {
                    self.measured_flow(list)?
                } else {
                    Vec::new()
                };
                if flow.iter().any(|(p, i, _, h)| {
                    (lines(p)[*i].n("vertsize") / 100.).max(*h) + m[2] + m[3] > maximum
                }) {
                    return Ok(false);
                }
                minimum = minimum.max(val(c.child("cellSz"), "height"));
                content.push(Cell {
                    node: c,
                    flow,
                    at: 0,
                    x: cw[..col].iter().sum(),
                    width: cw[col..col + span].iter().sum(),
                    margin: m,
                });
            }
            rows.push(Row {
                cells: content,
                minimum,
            });
        }
        let declared = if n.a("nativeDeclaredHeight").is_empty() {
            val(n.child("sz"), "height")
        } else {
            n.n("nativeDeclaredHeight") / 100.
        };
        let natural = header_height + rows.iter().map(Row::remaining).sum::<f64>();
        // Preserve explicit extra table height with the existing vector paginator.
        // Assigning it to the final row can manufacture a header-only page.
        if declared > natural + 0.01 {
            return Ok(false);
        }
        let mut header = n.clone();
        header
            .attrs
            .insert("rowCnt".into(), header_rows.to_string());
        header.attrs.insert("repeatHeader".into(), "0".into());
        header.attrs.insert("pageBreak".into(), "NONE".into());
        header.children.retain(|c| c.name != "tr");
        header
            .children
            .extend(n.children_named("tr").take(header_rows).cloned());
        node_mut(&mut header, "sz")
            .attrs
            .insert("height".into(), (header_height * 100.).to_string());
        let mut above = n.clone();
        above
            .children
            .retain(|c| c.name != "caption" || c.a("side") == "TOP");
        let mut below = n.clone();
        below
            .children
            .retain(|c| c.name != "caption" || c.a("side") == "BOTTOM");
        let cap = caption_extent(n);
        let mut row = 0usize;
        let mut page_top = top;
        let mut painted = false;
        let mut first = true;
        let first_bound = self
            .report
            .bounds
            .iter()
            .rposition(|b| b.kind == "tbl" && b.id == n.a("id"));
        for _ in 0..10000 {
            if row == rows.len() {
                break;
            }
            let reserve = self.footnote_reservation(&[], x);
            let available = self.ph
                - self.bottom
                - top
                - outside[3]
                - reserve
                - if painted { 0. } else { header_height };
            let last_room = if row + 1 == rows.len() {
                (available - cap[3]).max(0.)
            } else {
                available
            };
            let full = rows[row].remaining() <= last_room + 0.01;
            let mut takes: Vec<_> = rows[row]
                .cells
                .iter()
                .map(|c| {
                    if full {
                        c.flow.len() - c.at
                    } else {
                        c.fit(last_room)
                    }
                })
                .collect();
            // A fixed-height row may outlive its text. Retain the final visible
            // line for its last fragment instead of creating a header-only tail.
            if !full
                && rows[row].minimum > last_room + 0.01
                && takes
                    .iter()
                    .zip(&rows[row].cells)
                    .all(|(take, c)| *take == c.flow.len() - c.at)
                && let Some(i) = takes.iter().rposition(|take| *take > 0)
            {
                takes[i] -= 1;
            }
            let content = takes
                .iter()
                .zip(&rows[row].cells)
                .map(|(take, c)| c.required(*take))
                .fold(0., f64::max);
            let height = if full {
                rows[row].remaining()
            } else {
                content.max(rows[row].minimum.min(last_room.max(0.)))
            };
            if height <= 0.1
                || (takes.iter().all(|n| *n == 0)
                    && rows[row].cells.iter().any(|c| c.at < c.flow.len()))
            {
                anyhow::ensure!(
                    painted || top > self.top + outside[2] + cap[2] + 0.1 || reserve > 0.,
                    "Native table line cannot fit a page"
                );
                if painted {
                    self.native_row_fragment_bound(n, x, page_top, top, w, first_bound, first);
                    first = false;
                }
                self.advance_table_fragment(&mut x)?;
                top = self.column_top() + outside[2] + if first { cap[2] } else { 0. };
                page_top = top;
                painted = false;
                continue;
            }
            if !painted {
                if first {
                    self.paint_captions(&above, x, top, w, 0.)?;
                }
                if header_rows > 0 {
                    self.depth += 1;
                    let result = self.table(&header, x, top, w, header_height);
                    self.depth -= 1;
                    result?;
                }
                top += header_height;
                painted = true;
            }
            for (cell, take) in rows[row].cells.iter_mut().zip(takes) {
                self.border(
                    cell.node.a("borderFillIDRef"),
                    x + cell.x,
                    top,
                    cell.width,
                    height,
                );
                if take > 0 {
                    let list = cell.node.child("subList").unwrap();
                    let content = cell
                        .content_height(take)
                        .max(cell.ink_height(take) - cell.margin[3]);
                    let spare = (height - cell.margin[2] - cell.margin[3] - content).max(0.);
                    let alignment = match list.a("vertAlign") {
                        "CENTER" => spare / 2.,
                        "BOTTOM" => spare,
                        _ => 0.,
                    };
                    let origin = if cell.at == 0 {
                        0.
                    } else {
                        cell.flow[cell.at].2
                    };
                    self.depth += 1;
                    for (p, i, y, _) in &cell.flow[cell.at..cell.at + take] {
                        self.line(
                            p,
                            *i,
                            x + cell.x + cell.margin[0],
                            top + cell.margin[2] + alignment + y
                                - origin
                                - lines(p)[*i].n("vertpos") / 100.,
                            cell.width - cell.margin[0] - cell.margin[1],
                        )?;
                    }
                    self.depth -= 1;
                    cell.at += take;
                }
            }
            rows[row].minimum = (rows[row].minimum - height).max(0.);
            top += height;
            if rows[row].done() {
                row += 1;
            } else {
                self.native_row_fragment_bound(n, x, page_top, top, w, first_bound, first);
                first = false;
                self.advance_table_fragment(&mut x)?;
                top = self.column_top() + outside[2];
                page_top = top;
                painted = false;
            }
        }
        anyhow::ensure!(row == rows.len(), "Native table exceeded fragment limit");
        if painted {
            self.native_row_fragment_bound(n, x, page_top, top, w, first_bound, first);
        }
        self.paint_captions(&below, x, top, w, 0.)?;
        self.last_table_end = top + cap[3];
        self.split_caption_handled = n.child("caption").is_some();
        self.flow_floor = Some((self.pages.len(), self.last_table_end, false));
        self.report.warnings.insert("Native plain-text rows paginated with fragment-local cell padding and vertical alignment".into());
        Ok(true)
    }
    #[allow(clippy::too_many_arguments)]
    fn native_row_fragment_bound(
        &mut self,
        n: &Node,
        x: f64,
        top: f64,
        bottom: f64,
        width: f64,
        first_bound: Option<usize>,
        first: bool,
    ) {
        let b = Bounds {
            page: self.pages.len() + 1,
            kind: "tbl".into(),
            id: n.a("id").into(),
            x,
            y: top,
            w: width,
            h: bottom - top,
        };
        if first && let Some(i) = first_bound {
            self.report.bounds[i] = b;
        } else {
            self.report.bounds.push(b);
        }
    }
}
