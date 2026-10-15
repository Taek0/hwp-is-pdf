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
        while start < total - 0.01 {
            if start > 0. && n.a("repeatHeader") == "1" {
                let header_cells: Vec<Node> = cells
                    .iter()
                    .filter(|c| c.child("cellAddr").unwrap().u("rowAddr") == 0)
                    .map(|c| (*c).clone())
                    .collect();
                if !header_cells.is_empty()
                    && header_cells.iter().all(|c| {
                        c.child("cellSpan").unwrap().u("rowSpan") == 1
                            && c.find("footNote").is_none()
                    })
                    && rh[0] < (self.ph - self.top - self.bottom) / 3.
                {
                    let mut header = n.clone();
                    header.attrs.insert("rowCnt".into(), "1".into());
                    header.attrs.insert("repeatHeader".into(), "0".into());
                    header.attrs.insert("pageBreak".into(), "NONE".into());
                    header.children.retain(|c| c.name != "tr");
                    header.children.push(Node {
                        name: "tr".into(),
                        children: header_cells,
                        ..Default::default()
                    });
                    self.depth += 1;
                    self.table(&header, x, top, cw.iter().sum(), rh[0])?;
                    self.depth -= 1;
                    top += rh[0];
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
                    top = self.top;
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
                top = self.top;
            }
        }
        self.report.warnings.insert("Oversized table row split at shared text-line boundaries; merged cell borders continued".into());
        Ok(())
    }
    /// Reflow consumes columns before advancing a page; saved layouts keep
    /// their original page-only splitting behavior.
    pub(super) fn advance_table_fragment(&mut self, x: &mut f64) -> Result<()> {
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
        let section = &self.doc.sections[self.active_section];
        self.decorations(section, false, self.active_columns, self.active_gap)?;
        Ok(())
    }
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
        let captions: Vec<_> = n.children_named("caption").collect();
        let mut all_captions = Vec::new();
        n.all("caption", &mut all_captions);
        if all_captions.len() != captions.len()
            || captions
                .iter()
                .any(|c| !matches!(c.a("side"), "TOP" | "BOTTOM"))
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
            .filter(|t| t.a("repeatHeader") == "1")
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
        self.table_note_anchors.clear();
        self.depth += 1;
        let rendered = self.table(n, x, 0., w, total);
        self.depth -= 1;
        self.capture_table_headers = false;
        let mut headers = std::mem::take(&mut self.table_headers);
        let mut planned_notes = std::mem::take(&mut self.table_note_anchors);
        headers.sort_by(|a, b| a.y.total_cmp(&b.y));
        let canvas = std::mem::replace(&mut self.current, previous);
        let bounds = self.report.bounds.split_off(bound_start);
        rendered?;
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
        if top + first_min > self.ph - self.bottom {
            self.advance_table_fragment(&mut x)?;
            top = self.top + above;
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
            let mut capacity = self.ph - self.bottom - top - self.footnote_reservation(&[], x);
            if start + capacity >= total {
                capacity -= below;
            }
            let mut cut = (start + capacity).min(total);
            for _ in 0..planned_notes.len() + 2 {
                let incoming: Vec<_> = planned_notes
                    .iter()
                    .filter(|n| n.y >= start - 0.01 && n.y < cut)
                    .map(|n| &n.node)
                    .collect();
                let reserve = self.footnote_reservation(&incoming, x);
                let next = (start + self.ph
                    - self.bottom
                    - top
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
                    .filter(|v| *v > start + capacity * 0.5 && *v <= cut && safe(*v))
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
            if below > 0.
                && cut < total - 0.01
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
                top = self.top;
                continue;
            }
            let shift = top - start;
            for note in planned_notes
                .iter()
                .filter(|n| n.y >= start - 0.01 && n.y < cut - 0.01)
            {
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
                top = self.top;
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
