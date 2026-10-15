//! Layout responsibility: reflow.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn reflow_advance(
        &mut self,
        section: &Node,
        col: &mut usize,
        y: &mut f64,
        cols: usize,
        gap: f64,
    ) -> Result<()> {
        *col += 1;
        if *col >= cols {
            self.finish()?;
            self.page_float_floor = self.top;
            *col = 0;
            self.decorations(section, false, cols, gap)?;
        }
        *y = self.top.max(self.page_float_floor);
        Ok(())
    }
    pub(super) fn general_plans(
        &mut self,
        section: &Node,
        width: f64,
        cols: usize,
        gap: f64,
    ) -> Result<Vec<(Node, Vec<crate::reflow::Line>, crate::reflow::Settings)>> {
        let mut plans: Vec<(Node, Vec<crate::reflow::Line>, crate::reflow::Settings)> = Vec::new();
        let mut carry: Vec<crate::wrap_region::Region> = Vec::new();
        let mut group: Option<(usize, f64)> = None;
        for source in section.children_named("p") {
            if let Some(c) = paragraph_columns(source) {
                anyhow::ensure!(
                    c.u("colCount").max(1) == cols && (val(Some(c), "sameGap") - gap).abs() < 0.01,
                    "Reflow with mid-section column changes is not supported"
                );
            }
            let style = self.doc.paras.get(source.a("paraPrIDRef"));
            let settings = crate::reflow::settings(style);
            let source_items = Self::items(source);
            let supported = Self::general_objects(&source_items);
            let follow = source.a("pageBreak") != "1"
                && !settings.page
                && !settings.keep
                && !settings.next
                && style
                    .and_then(|s| s.child("heading"))
                    .is_none_or(|h| !matches!(h.a("type"), "NUMBER" | "BULLET"))
                && supported
                && !source_items.iter().any(|i|matches!(&i.kind,ItemKind::Object(n) if n.child("pos").is_some_and(|p|matches!(p.a("horzRelTo"),"PAGE"|"PAPER") || matches!(p.a("vertRelTo"),"PAGE"|"PAPER"))));
            if !follow && !carry.is_empty() {
                if let Some((_, _, s)) = plans.last_mut() {
                    s.after += carry.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max);
                }
                carry.clear();
                group = None;
            }
            for r in &mut carry {
                r.shift_y(-settings.before);
            }
            carry.retain(|r| r.bounds()[3] > 0.);
            if carry.is_empty() {
                group = None;
            }
            let mut source = source.clone();
            source.attrs.insert("floatFlow".into(), "1".into());
            if !carry.is_empty() {
                source
                    .attrs
                    .insert("generalCarry".into(), serde_json::to_string(&carry)?);
            }
            let previous_remaining =
                carry.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max) + settings.before;
            let (mut p, mut lines, mut s) = self.reflow_paragraph(&source, width, 0)?;
            if let Some((origin, elapsed)) = group
                && p.n("reflowFloatHeight") > 0.
                && elapsed + s.before + p.n("reflowFloatHeight") + plans[origin].2.before
                    > self.ph - self.top - self.bottom - 24.
            {
                if let Some((_, _, previous)) = plans.last_mut() {
                    previous.after += previous_remaining;
                }
                carry.clear();
                group = None;
                source.attrs.remove("generalCarry");
                (p, lines, s) = self.reflow_paragraph(&source, width, 0)?;
                self.report
                    .warnings
                    .insert("Oversized connected float chain cleared before a later anchor".into());
            }
            if !p.a("generalRegions").is_empty() {
                carry = serde_json::from_str(p.a("generalRegions"))?;
            }
            let remaining = carry.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max);
            if remaining > 0. {
                if let Some((origin, elapsed)) = group {
                    let height = (elapsed + s.before + remaining)
                        .max(plans[origin].0.n("reflowFloatHeight"));
                    plans[origin]
                        .0
                        .attrs
                        .insert("reflowFloatHeight".into(), height.to_string());
                    p.attrs.remove("reflowFloatHeight");
                    p.attrs
                        .insert("reflowFloatCarry".into(), remaining.to_string());
                    group = Some((origin, elapsed + s.before));
                } else {
                    group = Some((plans.len(), 0.));
                }
            }
            let consumed = lines.iter().map(|l| l.height).sum::<f64>() + s.after;
            for r in &mut carry {
                r.shift_y(-consumed);
            }
            carry.retain(|r| r.bounds()[3] > 0.);
            if let Some((origin, elapsed)) = group {
                group = (!carry.is_empty()).then_some((origin, elapsed + consumed));
            }
            plans.push((p, lines, s));
        }
        if let Some((_, _, s)) = plans.last_mut() {
            s.after += carry.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max);
        }
        Ok(plans)
    }
    pub(super) fn reflow_text_section(
        &mut self,
        section: &Node,
        width: f64,
        cols: usize,
        gap: f64,
    ) -> Result<(f64, usize, f64, f64)> {
        if !self.column_widths.is_empty() {
            return self.reflow_complex_columns(section);
        }
        self.general_wrap = self.contour_wrap
            || section.children_named("p").any(|p| {
                let items = Self::items(p);
                let objects: Vec<_> = items
                    .iter()
                    .filter_map(|i| match &i.kind {
                        ItemKind::Object(n)
                            if (n.a("textWrap") == "SQUARE"
                                || (n.a("textWrap") == "TOP_AND_BOTTOM"
                                    && n.child("pos").is_some_and(|p| {
                                        matches!(p.a("horzRelTo"), "PAGE" | "PAPER")
                                            || matches!(p.a("vertRelTo"), "PAGE" | "PAPER")
                                    })))
                                && n.child("pos").is_some_and(|p| p.a("treatAsChar") != "1") =>
                        {
                            Some(n)
                        }
                        _ => None,
                    })
                    .collect();
                let sides: BTreeSet<_> = objects
                    .iter()
                    .map(|n| n.child("pos").unwrap().a("horzAlign"))
                    .collect();
                objects.len() > 2
                    || sides.len() < objects.len()
                    || (!objects.is_empty()
                        && items
                            .iter()
                            .rposition(|i| matches!(i.kind, ItemKind::Object(_)))
                            .is_some_and(|last| {
                                items[..last]
                                    .iter()
                                    .any(|i| matches!(i.kind, ItemKind::Char(_)))
                            }))
                    || objects.iter().any(|n| {
                        matches!(n.name.as_str(), "ellipse" | "polygon" | "curve" | "group")
                            || effective_height(n) > self.ph - self.top - self.bottom
                            || val(n.child("sz"), "width") > width
                            || !matches!(n.a("textFlow"), "" | "BOTH_SIDES")
                            || n.child("pos").is_some_and(|p| {
                                p.a("horzAlign") == "CENTER"
                                    || p.signed("horzOffset") != 0.
                                    || p.signed("vertOffset") != 0.
                                    || matches!(p.a("horzRelTo"), "PAGE" | "PAPER")
                                    || matches!(p.a("vertRelTo"), "PAGE" | "PAPER")
                            })
                    })
            });
        let plans = if self.general_wrap {
            self.general_plans(section, width, cols, gap)?
        } else {
            let mut plans: Vec<(Node, Vec<crate::reflow::Line>, crate::reflow::Settings)> =
                Vec::new();
            let mut carry: Vec<(f64, f64, f64)> = Vec::new();
            for source in section.children_named("p") {
                if let Some(c) = paragraph_columns(source) {
                    anyhow::ensure!(
                        c.u("colCount").max(1) == cols
                            && (val(Some(c), "sameGap") - gap).abs() < 0.01,
                        "Reflow with mid-section column changes is not supported"
                    );
                }
                let style = self.doc.paras.get(source.a("paraPrIDRef"));
                let source_settings = crate::reflow::settings(style);
                let (cut_left, cut_right) = carry
                    .iter()
                    .filter(|(_, _, h)| *h > source_settings.before)
                    .fold((0f64, 0f64), |(l, r), (a, b, _)| (l.max(*a), r.max(*b)));
                let side_width = width
                    - (source_settings.left + source_settings.indent.max(0.)).max(cut_left)
                    - source_settings.right.max(cut_right);
                let can_follow = source.a("pageBreak") != "1"
                    && side_width > 24.
                    && !source_settings.page
                    && !source_settings.keep
                    && !source_settings.next
                    && style
                        .and_then(|s| s.child("heading"))
                        .is_none_or(|h| !matches!(h.a("type"), "NUMBER" | "BULLET"))
                    && Self::items(source)
                        .iter()
                        .all(|i| !matches!(i.kind, ItemKind::Object(_)));
                if !can_follow && !carry.is_empty() {
                    if let Some((_, _, last_settings)) = plans.last_mut() {
                        last_settings.after +=
                            carry.iter().map(|(_, _, h)| *h).fold(0f64, f64::max);
                    }
                    carry.clear();
                }
                let mut source = source.clone();
                source.attrs.insert("floatFlow".into(), "1".into());
                for (_, _, remaining) in &mut carry {
                    *remaining -= source_settings.before;
                }
                carry.retain(|(_, _, h)| *h > 0.);
                if !carry.is_empty() {
                    source
                        .attrs
                        .insert("floatCarryRegions".into(), serde_json::to_string(&carry)?);
                }
                let (p, lines, settings) = self.reflow_paragraph(&source, width, 0)?;
                if p.n("reflowFloatHeight") > 0. {
                    carry = serde_json::from_str(p.a("reflowFloatRegions"))?;
                }
                let consumed = lines.iter().map(|l| l.height).sum::<f64>() + settings.after;
                for (_, _, remaining) in &mut carry {
                    *remaining -= consumed;
                }
                carry.retain(|(_, _, h)| *h > 0.);
                plans.push((p, lines, settings));
            }
            if let Some((_, _, settings)) = plans.last_mut() {
                settings.after += carry.iter().map(|(_, _, h)| *h).fold(0f64, f64::max);
            }

            plans
        };
        let mut col = 0;
        let mut y = self.top;
        let body = self.ph - self.top - self.bottom;
        self.flow_floor = None;
        for (pi, (p, ls, s)) in plans.iter().enumerate() {
            let items = Self::items(p);
            let notes_for = |index: usize| {
                let end = ls.get(index + 1).map(|l| l.pos).unwrap_or(usize::MAX);
                items
                    .iter()
                    .filter(|i| i.pos >= ls[index].pos && i.pos < end)
                    .filter_map(|i| match &i.kind {
                        ItemKind::Control(n) if n.name == "footNote" => Some(n),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            if (p.a("pageBreak") == "1" || s.page) && (y > self.top + 0.01 || col > 0) {
                self.finish()?;
                self.page_float_floor = self.top;
                col = 0;
                y = self.top;
                self.decorations(section, false, cols, gap)?;
            }
            if p.n("pageFloatFloor") > 0. {
                self.page_float_floor = self.page_float_floor.max(p.n("pageFloatFloor"));
            }
            let height = s.before + ls.iter().map(|l| l.height).sum::<f64>() + s.after;
            let mut keep_height = height;
            if s.next {
                let mut j = pi + 1;
                while let Some((next, nl, ns)) = plans.get(j) {
                    if next.a("pageBreak") == "1" || ns.page {
                        break;
                    }
                    let all = nl.iter().map(|l| l.height).sum::<f64>();
                    keep_height += ns.before
                        + if ns.next || (ns.keep && keep_height + ns.before + all <= body) {
                            all + ns.after
                        } else {
                            nl.iter()
                                .take(if ns.widow { 2 } else { 1 })
                                .map(|l| l.height)
                                .sum::<f64>()
                        };
                    if !ns.next {
                        break;
                    }
                    j += 1;
                }
            }
            if (s.keep || s.next)
                && keep_height <= body
                && y + keep_height > self.ph - self.bottom
                && y > self.top + 0.01
            {
                self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
            }
            if p.n("reflowFloatHeight") > 0. {
                let mut span = 0.;
                let mut owned_notes = Vec::new();
                'float_band: for (j, (part, part_lines, part_settings)) in
                    plans.iter().enumerate().skip(pi)
                {
                    if j > pi {
                        span = (span + part_settings.before).min(p.n("reflowFloatHeight"));
                        if span >= p.n("reflowFloatHeight") {
                            break;
                        }
                    }
                    let part_items = Self::items(part);
                    for (index, line) in part_lines.iter().enumerate() {
                        let end = part_lines
                            .get(index + 1)
                            .map(|l| l.pos)
                            .unwrap_or(usize::MAX);
                        for item in part_items
                            .iter()
                            .filter(|i| i.pos >= line.pos && i.pos < end)
                        {
                            if let ItemKind::Control(note) = &item.kind
                                && note.name == "footNote"
                            {
                                owned_notes.push(note.clone());
                            }
                        }
                        span += line.height;
                        if span >= p.n("reflowFloatHeight") {
                            break 'float_band;
                        }
                    }
                    span = (span + part_settings.after).min(p.n("reflowFloatHeight"));
                    if span >= p.n("reflowFloatHeight") {
                        break;
                    }
                }
                let notes: Vec<_> = owned_notes.iter().collect();
                let reserve =
                    self.footnote_reservation(&notes, self.left + col as f64 * (width + gap));
                if y + s.before + span > self.ph - self.bottom - reserve && y > self.top + 0.01 {
                    self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                }
                let reserve =
                    self.footnote_reservation(&notes, self.left + col as f64 * (width + gap));
                anyhow::ensure!(
                    s.before + span + reserve <= body,
                    "Floating object and anchored text exceed column capacity"
                );
            }
            y += s.before;
            let mut at = 0;
            while at < ls.len() {
                if ls[at].block {
                    let end = ls.get(at + 1).map(|l| l.pos).unwrap_or(usize::MAX);
                    let object = items
                        .iter()
                        .filter(|i| i.pos >= ls[at].pos && i.pos < end)
                        .find_map(|i| {
                            if let ItemKind::Object(n) = &i.kind {
                                Some(n)
                            } else {
                                None
                            }
                        })
                        .context("Missing reflow block")?;

                    let ox = self.left + col as f64 * (width + gap);
                    let reserve = self.footnote_reservation(&[], ox);
                    if (ls[at].height <= body || object.name != "tbl")
                        && y + ls[at].height.min(body) > self.ph - self.bottom - reserve
                        && y > self.top + 0.01
                    {
                        self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                    }
                    let ox = self.left + col as f64 * (width + gap);
                    let cap = caption_extent(object);
                    let old_page = self.pages.len();
                    let bounds_start = self.report.bounds.len();
                    if cols > 1 && object.name == "tbl" {
                        let placement = self
                            .note_spec(false)
                            .and_then(|s| s.child("placement"))
                            .map(|p| p.a("place"))
                            .unwrap_or("EACH_COLUMN");
                        anyhow::ensure!(
                            placement == "EACH_COLUMN",
                            "Mixed-column reflow requires per-column footnote placement"
                        );
                        self.reflow_table_column = Some(col);
                    }
                    let rendered = self.object(object, ox, y + cap[2], y + cap[2], ox, width);
                    let final_column = self.reflow_table_column.take();
                    rendered?;
                    if object.name == "tbl" {
                        y = self.last_table_end
                            + if self.split_caption_handled {
                                0.
                            } else {
                                cap[3]
                            };
                    } else {
                        y = self.report.bounds[bounds_start..]
                            .iter()
                            .filter(|b| {
                                b.id == object.a("id")
                                    && (b.kind == object.name || b.kind == "pic-alpha")
                            })
                            .map(|b| b.y + b.h)
                            .fold(y, f64::max)
                            + cap[3];
                    }
                    if let Some(final_column) = final_column {
                        col = final_column;
                    } else if self.pages.len() != old_page {
                        col = 0;
                    }
                    self.flow_floor = None;
                    at += 1;
                    continue;
                }
                let mut count = 0;
                let mut used = 0.;
                let mut incoming = Vec::new();
                while at + count < ls.len() && !ls[at + count].block {
                    let mut row_count = 1;
                    while at + count + row_count < ls.len()
                        && ls[at + count + row_count - 1].height == 0.
                    {
                        row_count += 1;
                    }
                    for index in at + count..at + count + row_count {
                        incoming.extend(notes_for(index));
                    }
                    let row_height = ls[at + count..at + count + row_count]
                        .iter()
                        .map(|l| l.height)
                        .sum::<f64>();
                    let reserve = self
                        .footnote_reservation(&incoming, self.left + col as f64 * (width + gap));
                    if y + used + row_height > self.ph - self.bottom - reserve + 0.001 {
                        break;
                    }
                    used += row_height;
                    count += row_count;
                }
                if count == 0 {
                    anyhow::ensure!(
                        y > self.top + 0.01,
                        "A reflowed line is taller than the printable page"
                    );
                    self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                    continue;
                }
                let unsplit_count = count;
                if s.widow
                    && !(at == 0 && (p.n("reflowFloatHeight") > 0. || p.n("reflowFloatCarry") > 0.))
                    && at + count < ls.len()
                    && ls[at].height + ls.get(at + 1).map(|l| l.height).unwrap_or(0.) <= body
                {
                    if count == 1 && y > self.top + 0.01 {
                        self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                        continue;
                    }
                    if ls.len() - at - count == 1 && count > 1 {
                        count -= 1;
                    }
                }
                while count > 0 && ls[at + count - 1].height == 0. {
                    count -= 1;
                }
                if count == 0 {
                    count = unsplit_count;
                }
                for (i, line) in ls.iter().enumerate().skip(at).take(count) {
                    self.line(
                        p,
                        i,
                        self.left + col as f64 * (width + gap),
                        y - lines(p)[i].n("vertpos") / 100.,
                        width,
                    )?;
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: "reflow-line".into(),
                        id: p.a("id").into(),
                        x: self.left + col as f64 * (width + gap) + line.left,
                        y,
                        w: line.width,
                        h: line.height,
                    });
                    y += line.height;
                }
                at += count;
                if at < ls.len() && !ls[at].block {
                    self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                }
            }
            y += s.after;
        }
        self.report.warnings.insert("Reflow uses font metrics and recursively lays out table cells and footnotes; leading left/right square pictures and rectangles wrap across following ordinary paragraphs; other floating objects become blocks; advanced wrapping and exact Hancom line breaking remain limited".into());
        Ok((y, col, width, gap))
    }
}
