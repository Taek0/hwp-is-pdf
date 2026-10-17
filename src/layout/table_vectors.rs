//! Paginate measured table vectors at shared text and object boundaries.
use super::*;
impl<'a> Engine<'a> {
    /// Paginate measured vectors at safe row/ink boundaries. Only rules and
    /// fills are clipped; glyphs and transformed objects remain indivisible.
    pub(super) fn split_nested_vectors(
        &mut self,
        n: &Node,
        mut x: f64,
        mut top: f64,
        w: f64,
        total: f64,
    ) -> Result<bool> {
        let canvas_x = x;
        let outside = if n.a("binaryHwp") == "1"
            && n.a("reflowTable") != "1"
            && n.child("pos").is_some_and(|p| p.a("treatAsChar") == "0")
        {
            margin(n, "outMargin")
        } else {
            [0.; 4]
        };
        let captions: Vec<_> = n.children_named("caption").collect();
        let mut all_captions = Vec::new();
        n.all("caption", &mut all_captions);
        if n.a("reflowTable") != "1"
            && (all_captions.len() != captions.len()
                || captions
                    .iter()
                    .any(|c| !matches!(c.a("side"), "TOP" | "BOTTOM")))
        {
            return Ok(false);
        }
        let caption_height = |side: &str| {
            captions
                .iter()
                .filter(|c| c.a("side") == side)
                .map(|c| c.child("subList").map(content_height).unwrap_or(0.) + c.n("gap") / 100.)
                .sum::<f64>()
        };
        let above = caption_height("TOP");
        let below = caption_height("BOTTOM");
        let mut bottom_notes = Vec::new();
        for c in captions.iter().filter(|c| c.a("side") == "BOTTOM") {
            c.all("footNote", &mut bottom_notes);
        }
        if above + below > (self.ph - self.top - self.bottom) * 0.5 {
            return Ok(false);
        }
        if n.find("endNote").is_some()
            || n.child("rotationInfo").is_some_and(|r| r.n("angle") != 0.)
            || n.child("flip")
                .is_some_and(|r| r.a("horizontal") == "1" || r.a("vertical") == "1")
        {
            return Ok(false);
        }
        let previous = std::mem::take(&mut self.current);
        let mut table_nodes = Vec::new();
        n.all("tbl", &mut table_nodes);
        let requested_headers = table_nodes
            .iter()
            .filter(|t| {
                table_headers::band_rows(t).is_some()
                    // Native saved tables with a non-repeatable header flag
                    // retain their established row/line fallback and cache flow.
                    || (n.a("binaryHwp")=="1" && n.a("reflowTable")!="1"
                        && t.a("repeatHeader")=="1"
                        && !t.children_named("tr").flat_map(|r|r.children_named("tc")).any(|c|c.attrs.contains_key("header")))
            })
            .count();
        let bound_start = self.report.bounds.len();
        let counts = (
            self.report.tables,
            self.report.pictures,
            self.report.equations,
            self.report.charts,
        );
        self.capture_table_headers = true;
        self.table_headers.clear();
        self.table_cells.clear();
        self.table_note_anchors.clear();
        self.depth += 1;
        let rendered = self.table(n, x, 0., w, total).and_then(|()| {
            if n.a("reflowTable") == "1" {
                let mut side = n.clone();
                side.children
                    .retain(|c| c.name != "caption" || matches!(c.a("side"), "LEFT" | "RIGHT"));
                self.paint_captions(&side, x, 0., w, total)?;
            }
            Ok(())
        });
        self.depth -= 1;
        self.capture_table_headers = false;
        let mut headers = std::mem::take(&mut self.table_headers);
        let mut cells = std::mem::take(&mut self.table_cells);
        let mut planned_notes = std::mem::take(&mut self.table_note_anchors);
        // Native saved floating-table notes belong to the table's final fragment.
        // Reflowed and inline tables retain anchor-fragment note placement.
        let notes_at_end = n.a("binaryHwp") == "1"
            && n.a("reflowTable") != "1"
            && n.child("pos").is_some_and(|p| p.a("treatAsChar") == "0");
        headers.sort_by(|a, b| a.y.total_cmp(&b.y));
        let mut canvas = std::mem::replace(&mut self.current, previous);
        let mut bounds = self.report.bounds.split_off(bound_start);
        rendered?;
        let mut total = if n.a("reflowTable") == "1" {
            total.max(side_caption_height(n))
        } else {
            total
        };
        if notes_at_end
            && n.a("pageBreak") == "TABLE"
            && bottom_notes.is_empty()
            && table_nodes.len() == 1
            && n.children_named("tr")
                .flat_map(|r| r.children_named("tc"))
                .filter_map(|c| c.child("subList"))
                .flat_map(|l| l.children_named("p"))
                .all(|p| {
                    Self::items(p)
                        .iter()
                        .all(|i| !matches!(i.kind, ItemKind::Object(_)))
                })
        {
            for c in &mut cells {
                c.bounds.start -= bound_start;
                c.bounds.end -= bound_start;
            }
            let original_total = total;
            total = table_cell_split::pack_cells(
                &mut canvas.ops,
                &mut bounds,
                &mut cells,
                &mut headers,
                &mut planned_notes,
                &self.fonts,
                total,
                self.ph - self.bottom - top.max(self.top + above) - outside[3],
                self.ph - self.bottom - self.top - outside[2] - outside[3],
            );
            if total > original_total + 0.001 {
                self.report.warnings.insert("Staggered native table cell lines continued independently; merged borders extended without scaling".into());
            }
        }
        let atoms = crate::pagination::atoms(&canvas.ops, &self.fonts)?;
        let supported = atoms.iter().all(|a| {
            a.bottom - a.top
                < self.ph
                    - self.top
                    - self.bottom
                    - above
                    - below
                    - headers.iter().map(|h| h.height).sum::<f64>()
        });
        let mut owner = vec![None; canvas.ops.len()];
        for (id, atom) in atoms.iter().enumerate() {
            for slot in &mut owner[atom.start..atom.end] {
                *slot = Some(id);
            }
        }
        for note in &mut planned_notes {
            if let Some(id) = owner[note.op_index] {
                let atom = &atoms[id];
                note.top = atom.top;
                note.y = (atom.top + atom.bottom) / 2.;
            }
        }
        if !supported
            || headers.len() != requested_headers
            || headers.iter().map(|h| h.height).sum::<f64>()
                > self.ph - self.top - self.bottom - above - below - 24.
        {
            (
                self.report.tables,
                self.report.pictures,
                self.report.equations,
                self.report.charts,
            ) = counts;
            return Ok(false);
        }
        let mut ink = Vec::new();
        ink.extend(atoms.iter().map(|a| (a.top - 0.6, a.bottom + 0.6)));
        ink.extend(planned_notes.iter().map(|n| (n.top, n.y + 0.6)));
        ink.extend(
            bounds
                .iter()
                .filter(|b| {
                    matches!(
                        b.kind.as_str(),
                        "pic" | "rect" | "group" | "polygon" | "curve" | "arc" | "equation"
                    )
                })
                .map(|b| (b.y - 0.6, b.y + b.h + 0.6)),
        );
        let mut boundaries = Vec::new();
        for (op_index, op) in canvas.ops.iter().enumerate() {
            if owner[op_index].is_some() {
                continue;
            }
            match op {
                Op::Text {
                    font,
                    size,
                    x,
                    scale,
                    y,
                    text,
                    ..
                } => {
                    let face = ttf_parser::Face::parse(&self.fonts.list[*font].bytes, 0).unwrap();
                    let unit = size / face.units_per_em() as f64;
                    let right = x + self.fonts.width(*font, text, *size) * scale;
                    let decoration_bottom = canvas
                        .ops
                        .iter()
                        .filter_map(|o| match o {
                            Op::Path { points, .. }
                                if points.len() == 2
                                    && points[0].1 >= *y
                                    && points[0].1 < y + size * 0.5 + 4.
                                    && points[0].0.min(points[1].0) < right
                                    && points[0].0.max(points[1].0) > *x =>
                            {
                                Some(points[0].1 + 0.6)
                            }
                            _ => None,
                        })
                        .fold(*y, f64::max);
                    let decoration_top = canvas
                        .ops
                        .iter()
                        .filter_map(|o| match o {
                            Op::Path { points, .. }
                                if points.len() == 2
                                    && points[0].1 <= *y
                                    && points[0].1 > y - size * 1.1
                                    && points[0].0.min(points[1].0) < right
                                    && points[0].0.max(points[1].0) > *x =>
                            {
                                Some(points[0].1 - 0.6)
                            }
                            _ => None,
                        })
                        .fold(*y, f64::min);
                    for c in text.chars().filter(|c| !c.is_whitespace()) {
                        if let Some(bb) = crate::pdf::glyph_id(&face, c)
                            .and_then(|id| face.glyph_bounding_box(id))
                        {
                            ink.push((
                                (y - bb.y_max as f64 * unit).min(decoration_top) - 0.6,
                                (y - bb.y_min as f64 * unit).max(decoration_bottom) + 0.6,
                            ));
                        }
                    }
                }
                Op::Line { y, y2, .. } if (y - y2).abs() < 0.001 => boundaries.push(*y),
                _ => {}
            }
        }
        let safe = |cut: f64| !ink.iter().any(|(a, b)| *a < cut && *b > cut);
        let mut start = 0.;
        let mut first = true;
        top = top.max(self.top + above);
        let first_min = headers.iter().map(|h| h.y + h.height).fold(0., f64::max) + 24.;
        let mut caption_notes = Vec::new();
        for c in &captions {
            c.all("footNote", &mut caption_notes);
        }
        if top + first_min > self.ph - self.bottom - self.footnote_reservation(&caption_notes, x) {
            self.advance_table_fragment(&mut x)?;
            top = self.column_top() + outside[2] + above;
        }
        let mut caption_y = top - above;
        for c in captions.iter().filter(|c| c.a("side") == "TOP") {
            if let Some(list) = c.child("subList") {
                let h = content_height(list);
                self.local(list, x, caption_y, w)?;
                self.report.bounds.push(Bounds {
                    page: self.pages.len() + 1,
                    kind: "caption".into(),
                    id: n.a("id").into(),
                    x,
                    y: caption_y,
                    w,
                    h,
                });
                caption_y += h + c.n("gap") / 100.;
            }
        }
        while start < total - 0.01 {
            let mut header_height = 0.;
            let mut prior_y = None;
            let mut group_top = top;
            let mut group_height = 0f64;
            if !first {
                for header in headers
                    .iter()
                    .filter(|h| start >= h.y + h.height - 0.01 && start < h.end - 0.01)
                {
                    if prior_y.is_some_and(|y: f64| (y - header.y).abs() > 0.01) {
                        header_height += group_height;
                        group_top = top + header_height;
                        group_height = 0.;
                    }
                    prior_y = Some(header.y);
                    group_height = group_height.max(header.height);
                    self.current.ops.push(Op::TransformBegin {
                        matrix: [1., 0., 0., 1., x - canvas_x, group_top - header.y],
                    });
                    self.current.ops.extend(header.ops.clone());
                    self.current.ops.push(Op::TransformEnd);
                }
                header_height += group_height;
            }
            top += header_height;
            let mut capacity = self.ph
                - self.bottom
                - top
                - outside[3]
                - self.footnote_reservation(&bottom_notes, x);
            if start + capacity >= total {
                capacity -= below;
            }
            let mut cut = (start + capacity).min(total);
            for _ in 0..planned_notes.len() + 2 {
                let incoming: Vec<_> = planned_notes
                    .iter()
                    .filter(|n| {
                        if notes_at_end {
                            cut >= total - 0.01
                        } else {
                            n.y >= start - 0.01 && n.y < cut
                        }
                    })
                    .map(|n| &n.node)
                    .chain(bottom_notes.iter().copied())
                    .collect();
                let reserve = self.footnote_reservation(&incoming, x);
                let next = (start + self.ph
                    - self.bottom
                    - top
                    - outside[3]
                    - reserve
                    - if start + capacity >= total { below } else { 0. })
                .min(cut);
                if (next - cut).abs() < 0.01 {
                    break;
                }
                cut = next;
            }
            if cut < total - 0.01 {
                if let Some(row) = boundaries
                    .iter()
                    .copied()
                    .filter(|v| {
                        !(n.a("binaryHwp") == "1"
                            && n.a("reflowTable") != "1"
                            && n.a("pageBreak") == "TABLE")
                            && *v > start + capacity * 0.5
                            && *v <= cut
                            && safe(*v)
                    })
                    .max_by(f64::total_cmp)
                {
                    cut = row;
                } else {
                    for _ in 0..ink.len() + 1 {
                        let next = ink
                            .iter()
                            .filter(|(a, b)| *a < cut && *b > cut)
                            .map(|(a, _)| *a)
                            .fold(cut, f64::min);
                        if (next - cut).abs() < 0.001 {
                            break;
                        }
                        cut = next;
                    }
                }
            }
            // Do not orphan a repeated header and an empty row-height tail.
            // Keep the last content row with the final table fragment.
            if cut < total - 0.01
                && !ink.iter().any(|(a, _)| *a >= cut)
                && let Some(last) = ink.iter().map(|(a, _)| *a).max_by(f64::total_cmp)
            {
                cut = boundaries
                    .iter()
                    .copied()
                    .filter(|b| *b > start + 0.1 && *b <= last && safe(*b))
                    .max_by(f64::total_cmp)
                    .unwrap_or(last);
                for _ in 0..ink.len() + 1 {
                    let next = ink
                        .iter()
                        .filter(|(a, b)| *a < cut && *b > cut)
                        .map(|(a, _)| *a)
                        .fold(cut, f64::min);
                    if (next - cut).abs() < 0.001 {
                        break;
                    }
                    cut = next;
                }
            }
            if cut <= start + 0.1 {
                anyhow::ensure!(
                    top > self.top + header_height + 0.1,
                    "Nested-table text line exceeds printable page"
                );
                self.advance_table_fragment(&mut x)?;
                top = self.column_top() + outside[2];
                continue;
            }
            let shift = top - start;
            for note in planned_notes.iter().filter(|n| {
                if notes_at_end {
                    cut >= total - 0.01
                } else {
                    n.y >= start - 0.01 && n.y < cut - 0.01
                }
            }) {
                let (nx, nw) = if self.reflow_table_column.is_some() {
                    self.footnote_area(x)
                } else {
                    (note.x, note.width)
                };
                self.footnotes.push((note.node.clone(), nx, nw));
            }
            self.current.ops.push(Op::TransformBegin {
                matrix: [1., 0., 0., 1., x - canvas_x, shift],
            });
            for (op_index, op) in canvas.ops.iter().enumerate() {
                if let Some(id) = owner[op_index] {
                    let atom = &atoms[id];
                    if op_index == atom.start
                        && atom.bottom > start - 0.001
                        && atom.top < cut - 0.001
                    {
                        self.current
                            .ops
                            .extend_from_slice(&canvas.ops[atom.start..atom.end]);
                    }
                    continue;
                }
                let clipped = match op {
                    Op::Link { anchor_y, .. } | Op::Destination { anchor_y, .. } => {
                        if *anchor_y >= start && *anchor_y < cut {
                            Some(op.clone())
                        } else {
                            None
                        }
                    }
                    Op::Text { y, .. } => {
                        if *y >= start && *y < cut {
                            Some(op.clone())
                        } else {
                            None
                        }
                    }
                    Op::Line {
                        x,
                        y,
                        x2,
                        y2,
                        width,
                        color,
                    } => {
                        if (y - y2).abs() < 0.001 {
                            if *y >= start - 0.001 && *y <= cut + 0.001 {
                                Some(op.clone())
                            } else {
                                None
                            }
                        } else {
                            let a = y.min(*y2).max(start);
                            let b = y.max(*y2).min(cut);
                            (b > a).then_some(Op::Line {
                                x: *x,
                                y: a,
                                x2: *x2,
                                y2: b,
                                width: *width,
                                color: *color,
                            })
                        }
                    }
                    Op::Rect { x, y, w, h, fill } => {
                        let a = y.max(start);
                        let b = (y + h).min(cut);
                        (b > a).then_some(Op::Rect {
                            x: *x,
                            y: a,
                            w: *w,
                            h: b - a,
                            fill: *fill,
                        })
                    }
                    Op::Path { points, .. } => {
                        if points.first().is_some_and(|p| p.1 >= start && p.1 < cut) {
                            Some(op.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(op) = clipped {
                    self.current.ops.push(op);
                }
            }
            if notes_at_end && cut < total - 0.01 {
                self.current
                    .ops
                    .extend(table_cell_split::bottom_rules(&canvas.ops, &cells, cut));
            }
            self.current.ops.push(Op::TransformEnd);
            for b in &bounds {
                let a = b.y.max(start);
                let end = (b.y + b.h).min(cut);
                if end > a {
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: b.kind.clone(),
                        id: b.id.clone(),
                        x: b.x + x - canvas_x,
                        y: a + shift,
                        w: b.w,
                        h: end - a,
                    });
                }
            }
            if first {
                if let Some(b) = self.report.bounds[..bound_start]
                    .iter_mut()
                    .rev()
                    .find(|b| b.kind == "tbl" && b.id == n.a("id"))
                {
                    b.x = x;
                    b.y = top;
                    b.page = self.pages.len() + 1;
                    b.h = cut - start;
                }
            } else {
                self.report.bounds.push(Bounds {
                    page: self.pages.len() + 1,
                    kind: "tbl".into(),
                    id: n.a("id").into(),
                    x,
                    y: top,
                    w,
                    h: cut - start,
                });
            }
            self.last_table_end = top + cut - start;
            start = cut;
            first = false;
            if start < total - 0.01 {
                self.advance_table_fragment(&mut x)?;
                top = self.column_top() + outside[2];
            }
        }
        let mut caption_y = self.last_table_end;
        for c in captions.iter().filter(|c| c.a("side") == "BOTTOM") {
            if let Some(list) = c.child("subList") {
                caption_y += c.n("gap") / 100.;
                let h = content_height(list);
                self.local(list, x, caption_y, w)?;
                self.report.bounds.push(Bounds {
                    page: self.pages.len() + 1,
                    kind: "caption".into(),
                    id: n.a("id").into(),
                    x,
                    y: caption_y,
                    w,
                    h,
                });
                caption_y += h;
            }
        }
        self.last_table_end = caption_y;
        self.split_caption_handled = !captions.is_empty();
        self.flow_floor = Some((self.pages.len(), self.last_table_end, true));
        self.report.warnings.insert("Nested text table split at safe row/ink boundaries without scaling; border/fill fragments continued".into());
        Ok(true)
    }
}
