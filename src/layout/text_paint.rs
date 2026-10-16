//! Layout responsibility: text paint.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn line(&mut self, p: &Node, li: usize, ox: f64, oy: f64, width: f64) -> Result<()> {
        let segs = lines(p);
        let seg = segs[li];
        if li == 0 {
            for item in Self::items(p) {
                if let ItemKind::Object(n) = item.kind
                    && (n.a("reflowGeneral") == "1" || n.a("reflowBehind") == "1")
                    && (p.u("reflowStart") == 0 || n.a("reflowBehind") != "1")
                {
                    let anchor_y = oy + seg.n("vertpos") / 100.;
                    self.object(&n, ox, anchor_y, anchor_y, ox, width)?;
                }
            }
        }
        self.paint_paragraph_border(p, li, ox, oy, width);
        if self.paint_complex_line(p, li, ox, oy, width)? {
            return Ok(());
        }
        let start = seg.u("textpos");
        let end = segs
            .get(li + 1)
            .map(|n| n.u("textpos"))
            .unwrap_or(usize::MAX);
        let all_items = Self::items(p);
        let deferred = if self.depth == 0 {
            saved_float_order::trailing_table(p, &all_items)
        } else {
            None
        };
        let mut its: Vec<_> = all_items
            .into_iter()
            .filter(|i|deferred.as_ref().is_none_or(|(pos,_)| i.pos!=*pos || !matches!(i.kind,ItemKind::Object(_))))
            .filter(|i| i.pos >= start && i.pos < end)
            .filter(|i| !matches!(&i.kind,ItemKind::Object(n) if n.a("reflowGeneral")=="1" || n.a("reflowBehind")=="1"))
            .filter(|i| p.a("reflowed") != "1" || !matches!(i.kind, ItemKind::Char('\n' | '\r')))
            .collect();
        if p.a("reflowed") == "1" && li + 1 < segs.len() && seg.a("hardBreak") != "1" {
            while its
                .last()
                .is_some_and(|i| matches!(i.kind, ItemKind::Char(' ')))
            {
                its.pop();
            }
        }
        let mut line_y = oy + seg.n("vertpos") / 100.;
        let mut base = line_y + seg.n("baseline") / 100.;
        let mut x = ox + seg.n("horzpos") / 100.;
        let mut avail = if seg.n("horzsize") > 0. {
            seg.n("horzsize") / 100.
        } else {
            width
        };
        if self.depth > 0 {
            avail = avail.min((width - seg.n("horzpos") / 100.).max(0.));
        }
        let style = self.doc.paras.get(p.a("paraPrIDRef"));
        let align = style
            .and_then(|n| n.child("align"))
            .map(|n| n.a("horizontal"))
            .unwrap_or("LEFT");
        let align = if p.a("tocEntry").is_empty() {
            align
        } else {
            "LEFT"
        };
        let number_draw = if li == 0 && p.u("reflowStart") == 0 {
            self.paragraph_number(p, p.a("reflowed") == "1")?
        } else {
            None
        };
        if p.a("reflowed") != "1"
            && let Some((_, _, reserved)) = &number_draw
        {
            x += reserved;
            avail -= reserved;
        }
        let mut widths = Vec::new();
        let mut gs = Vec::new();
        for i in &its {
            let (g, w) = match &i.kind {
                ItemKind::Char(c) => {
                    let g = self.glyph(*c, &i.style)?;
                    let w = g.width;
                    (Some(g), w)
                }
                ItemKind::Tab(w) => (None, *w),
                ItemKind::Object(n) => {
                    let m = margin(n, "outMargin");
                    let inline = n
                        .child("pos")
                        .map(|p| p.a("treatAsChar") == "1")
                        .unwrap_or(false);
                    (
                        None,
                        if inline {
                            val(n.child("sz"), "width")
                                + m[0]
                                + m[1]
                                + if n.a("reflowCaptions") == "1" {
                                    let c = caption_extent(n);
                                    c[0] + c[1]
                                } else {
                                    0.
                                }
                        } else {
                            0.
                        },
                    )
                }
                ItemKind::Control(n) if n.name == "charOverlay" => {
                    let g = self.glyph('가', &i.style)?;
                    let width = g.size;
                    (Some(g), width)
                }
                ItemKind::Control(n) if n.name == "ruby" => {
                    let g = self.glyph(n.a("main").chars().next().unwrap_or(' '), &i.style)?;
                    let width = self.fonts.width(g.font, n.a("main"), g.size);
                    (Some(g), width)
                }
                ItemKind::Control(n)
                    if n.name == "footNote" || (n.name == "endNote" && number_draw.is_none()) =>
                {
                    let g = self.glyph('1', &i.style)?;
                    let label = self.note_label(n, n.name == "endNote");
                    let w = self.fonts.width(g.font, &label, g.size * 0.6);
                    (Some(g), w)
                }
                ItemKind::Control(n) if n.name == "autoNum" => {
                    let g = self.glyph('1', &i.style)?;
                    let w = self.fonts.width(g.font, &self.auto_label(n), g.size) * g.scale;
                    (Some(g), w)
                }
                ItemKind::Control(n) if n.name == "pageField" => {
                    let g = self.glyph('8', &i.style)?;
                    let w = self.fonts.width(g.font, "8888", g.size) * g.scale;
                    (Some(g), w)
                }
                ItemKind::Control(_) => (None, 0.),
            };
            gs.push(g);
            widths.push(w);
        }
        // Source negative tracking can exceed a narrow replacement glyph's advance.
        // Preserve distinct glyph ink before fitting the whole line to its cached width.
        for i in 0..its.len().saturating_sub(1) {
            if matches!(its[i].kind, ItemKind::Char(_))
                && matches!(its[i + 1].kind, ItemKind::Char(_))
                && let (Some(left), Some(right)) = (&gs[i], &gs[i + 1])
            {
                widths[i] =
                    if let Some(adjust) = self.pair_kerning(&its[i], &its[i + 1], left, right) {
                        (widths[i] + adjust).max(0.)
                    } else {
                        widths[i].max(left.ink_right - right.ink_left + 0.02)
                    };
            }
        }
        text_metrics::trim_trailing_tracking(&its, &mut gs, &mut widths);
        if p.a("reflowed") == "1" {
            let (stops, step) = self.paragraph_tabs(p);
            let mut at = seg.n("horzpos") / 100.;
            for i in 0..its.len() {
                if matches!(its[i].kind, ItemKind::Tab(_)) {
                    widths[i] = crate::tabs::advance(
                        &stops,
                        step,
                        at,
                        its[i + 1..].iter().zip(&widths[i + 1..]).map(|(item, w)| {
                            (
                                match item.kind {
                                    ItemKind::Char(c) => Some(c),
                                    ItemKind::Tab(_) => Some('\t'),
                                    _ => None,
                                },
                                *w,
                            )
                        }),
                    );
                }
                at += widths[i];
            }
        }
        let mut total: f64 = widths.iter().sum();
        if self.doc.header.a("binaryHwp") == "1" {
            base = base.max(line_y + gs.iter().flatten().map(|g| g.raw_ascent).fold(0., f64::max));
        }
        if total > avail && !its.iter().any(|i| matches!(i.kind, ItemKind::Tab(_))) {
            let fixed: f64 = widths
                .iter()
                .zip(&gs)
                .filter(|(_, g)| g.is_none())
                .map(|(w, _)| *w)
                .sum();
            let text_width = total - fixed;
            if text_width > 0. && avail > fixed {
                let factor = (avail - fixed) / text_width;
                for (i, g) in gs.iter_mut().enumerate() {
                    if let Some(g) = g {
                        g.scale *= factor;
                        g.width *= factor;
                        widths[i] *= factor;
                    }
                }
                total = widths.iter().sum();
            }
        }
        let mut gap = 0.;
        if align == "CENTER" {
            x += (avail - total).max(0.) / 2.
        } else if align == "RIGHT" {
            x += (avail - total).max(0.);
        }
        // Never justify through tabs or into the measured boxes of equations/images.
        let atomic_display = its.windows(2).any(|p| p[0].pos == p[1].pos);
        let stretch = if atomic_display {
            its.windows(2)
                .filter(|p| {
                    p[0].pos != p[1].pos
                        && matches!(p[0].kind, ItemKind::Char(_))
                        && matches!(p[1].kind, ItemKind::Char(_))
                })
                .count()
        } else {
            its.iter()
                .filter(|i| matches!(i.kind, ItemKind::Char(_)))
                .count()
                .saturating_sub(1)
        };
        if !its.iter().any(|i| matches!(i.kind, ItemKind::Tab(_)))
            && stretch > 0
            && ((align == "JUSTIFY" && li + 1 < segs.len() && seg.a("hardBreak") != "1")
                || align == "DISTRIBUTE"
                || total > avail)
        {
            gap = (avail - total) / stretch as f64;
        }
        let origin = x;
        let field_targets =
            self.line_field_targets(p, &its, x, line_y, base, li == 0 && p.u("reflowStart") == 0);
        let mut caption_cursor = line_y;
        let mut underlines: Vec<(String, f64, f64, f64, f64, f64)> = Vec::new();
        let mut strikes: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        for (idx, it) in its.iter().enumerate() {
            self.link_ink(
                &field_targets[idx],
                x,
                line_y,
                widths[idx],
                seg.n("vertsize") / 100.,
                base,
            );
            match &it.kind {
                ItemKind::Char(c) => {
                    let g = gs[idx].as_ref().unwrap();
                    if self
                        .doc
                        .chars
                        .get(&it.style)
                        .and_then(|s| s.child("strikeout"))
                        .and_then(crate::decoration::style)
                        .is_some()
                    {
                        let end = x + widths[idx] + gap;
                        if let Some(span) = strikes.last_mut().filter(|s| {
                            s.0 == it.style
                                && (s.2 - x).abs() < 0.01
                                && (s.4 - g.shift).abs() < 0.01
                        }) {
                            span.2 = end;
                        } else {
                            strikes.push((it.style.clone(), x, end, g.size, g.shift));
                        }
                    }
                    let grouped = g.shear != 0. || g.emphasis.is_some();
                    if grouped {
                        self.current.ops.push(Op::TransformBegin {
                            matrix: [1., 0., -g.shear, 1., g.shear * (base + g.shift), 0.],
                        });
                    }
                    self.current.ops.push(Op::Text {
                        x,
                        y: base + g.shift,
                        text: c.to_string(),
                        font: g.font,
                        size: g.size,
                        scale: g.scale,
                        color: g.color,
                    });
                    if g.emphasis.is_some() {
                        let advance = self.fonts.width(g.font, &c.to_string(), g.size) * g.scale;
                        self.paint_emphasis(g, x, base + g.shift, advance);
                    }
                    if grouped {
                        self.current.ops.push(Op::TransformEnd);
                    }
                    if self
                        .doc
                        .chars
                        .get(&it.style)
                        .and_then(|s| s.child("underline"))
                        .filter(|n| matches!(n.a("type"), "BOTTOM" | "TOP"))
                        .is_some()
                    {
                        let end = x + widths[idx] + gap;
                        if let Some(span) = underlines.last_mut().filter(|s| {
                            s.0 == it.style
                                && (s.2 - x).abs() < 0.01
                                && (s.5 - g.shift).abs() < 0.01
                        }) {
                            span.2 = end;
                            span.4 = span.4.max(g.raw_descent);
                        } else {
                            underlines.push((
                                it.style.clone(),
                                x,
                                end,
                                g.size,
                                g.raw_descent,
                                g.shift,
                            ));
                        }
                    }
                    x += widths[idx]
                        + if atomic_display
                            && its.get(idx + 1).is_some_and(|next| next.pos == it.pos)
                        {
                            0.
                        } else {
                            gap
                        };
                }
                ItemKind::Tab(w) => {
                    let advance = if p.a("reflowed") == "1" {
                        widths[idx]
                    } else {
                        *w
                    };
                    if p.a("reflowed") == "1" {
                        self.paint_tab_leader(p, x, base, advance, ox, &it.style)?;
                    }
                    x += advance;
                }
                ItemKind::Object(n) => {
                    let pos = n.child("pos");
                    let object_start_page = self.pages.len();
                    let block = n.a("textWrap") == "TOP_AND_BOTTOM"
                        && pos.is_some_and(|p| {
                            p.a("treatAsChar") != "1" && p.a("vertRelTo") == "PARA"
                        });
                    let mut object_y = line_y;
                    if block && n.child("caption").is_some() {
                        let e = caption_extent(n);
                        let m = margin(n, "outMargin");
                        let extent = effective_height(n) + e[2] + e[3] + m[2] + m[3];
                        if self.depth == 0
                            && !(n.name == "tbl"
                                && n.a("binaryHwp") == "1"
                                && n.a("pageBreak") == "TABLE"
                                && n.children_named("tr")
                                    .flat_map(|r| r.children_named("tc"))
                                    .filter_map(|c| c.child("subList"))
                                    .any(|l| content_height(l) > effective_height(n)))
                            && caption_cursor + extent > self.ph - self.bottom
                            && extent < self.ph - self.top - self.bottom
                        {
                            self.finish()?;
                            caption_cursor = self.top;
                        }
                        object_y = caption_cursor;
                        caption_cursor += extent;
                    } else if block
                        && n.name == "pic"
                        && segs[0].n("vertpos") / 100. + 0.01
                            >= effective_height(n) + margin(n, "outMargin")[3]
                        && its
                            .iter()
                            .any(|i| matches!(i.kind, ItemKind::Char(c) if !c.is_whitespace()))
                    {
                        // Block pictures precede their paragraph's first text line in the cache.
                        object_y -= effective_height(n) + margin(n, "outMargin")[3];
                    }
                    self.object(n, ox, object_y, base, x, width)?;
                    if n.name == "tbl"
                        && self.pages.len() > object_start_page
                        && its[idx + 1..]
                            .iter()
                            .any(|i| matches!(i.kind,ItemKind::Char(c) if !c.is_whitespace()))
                    {
                        line_y = self.last_table_end;
                        let reserve = self.footnote_reservation(&[], ox);
                        if line_y + seg.n("vertsize") / 100. > self.ph - self.bottom - reserve {
                            self.finish()?;
                            let s = &self.doc.sections[self.active_section];
                            self.decorations(s, false, self.active_columns, self.active_gap)?;
                            line_y = self.top;
                        }
                        base = line_y + seg.n("baseline") / 100.;
                        x = ox + seg.n("horzpos") / 100.;
                        self.flow_floor =
                            Some((self.pages.len(), line_y + seg.n("vertsize") / 100., true));
                    }
                    x += widths[idx];
                }
                ItemKind::Control(n) => match n.name.as_str() {
                    "pageField" => {
                        let g = gs[idx].as_ref().unwrap();
                        self.paint_page_field(n, g, x, base, widths[idx]);
                        x += widths[idx];
                    }
                    "charOverlay" => {
                        let g = gs[idx].as_ref().unwrap();
                        let w = widths[idx];
                        let y = base - g.size * 0.85;
                        self.report.bounds.push(Bounds {
                            page: self.pages.len() + 1,
                            kind: "intentional-overlay".into(),
                            id: format!("{}-{}", p.a("id"), it.pos),
                            x,
                            y,
                            w,
                            h: g.size,
                        });
                        let border = n.u("borderType");
                        if border == 1 {
                            for i in 0..48 {
                                let a = i as f64 * std::f64::consts::TAU / 48.;
                                let b = (i + 1) as f64 * std::f64::consts::TAU / 48.;
                                self.current.ops.push(Op::Line {
                                    x: x + w / 2. + w / 2. * a.cos(),
                                    y: y + w / 2. + w / 2. * a.sin(),
                                    x2: x + w / 2. + w / 2. * b.cos(),
                                    y2: y + w / 2. + w / 2. * b.sin(),
                                    width: 0.4,
                                    color: g.color,
                                });
                            }
                        } else if border != 0 {
                            self.box_lines(x, y, w, g.size, 0.4, g.color);
                            if border != 2 {
                                self.report.warnings.insert(format!(
                                    "Overlay border type {border} approximated by rectangle"
                                ));
                            }
                        }
                        let text = n.a("text");
                        let factor = ((g.size + n.n("innerSize")) / g.size).clamp(0.2, 1.);
                        let chars: Vec<String> = if n.a("spread") == "1" {
                            vec![text.into()]
                        } else {
                            text.chars().map(|c| c.to_string()).collect()
                        };
                        for (i, t) in chars.iter().enumerate() {
                            let style = n
                                .children
                                .get(i)
                                .map(|c| c.a("id"))
                                .filter(|s| self.doc.chars.contains_key(*s))
                                .unwrap_or(&it.style);
                            let gg = self.glyph(t.chars().next().unwrap_or(' '), style)?;
                            let size = g.size * factor;
                            let tw = self.fonts.width(gg.font, t, size);
                            let scale = if tw > w { w / tw } else { 1. };
                            self.current.ops.push(Op::Text {
                                x: x + (w - tw * scale) / 2.,
                                y: y + (g.size - size) / 2. + size * 0.85,
                                text: t.clone(),
                                font: gg.font,
                                size,
                                scale,
                                color: gg.color,
                            });
                        }
                        x += w;
                    }
                    "ruby" => {
                        let g = gs[idx].as_ref().unwrap();
                        let w = widths[idx];
                        self.current.ops.push(Op::Text {
                            x,
                            y: base,
                            text: n.a("main").into(),
                            font: g.font,
                            size: g.size,
                            scale: g.scale,
                            color: g.color,
                        });
                        let size = g.size * 0.5;
                        let tw = self.fonts.width(g.font, n.a("sub"), size);
                        self.current.ops.push(Op::Text {
                            x: x + (w - tw) / 2.,
                            y: if n.a("position") == "1" {
                                base + size
                            } else {
                                base - g.size
                            },
                            text: n.a("sub").into(),
                            font: g.font,
                            size,
                            scale: 1.,
                            color: g.color,
                        });
                        x += w;
                    }
                    "footNote" | "endNote" => {
                        let end = n.name == "endNote";
                        let mut note_top = base;
                        if !end || number_draw.is_none() {
                            let g = gs[idx].as_ref().unwrap();
                            note_top = base - g.size * 1.2;
                            self.current.ops.push(Op::Text {
                                x,
                                y: base - g.size * 0.35,
                                text: self.note_label(n, end),
                                font: g.font,
                                size: g.size * 0.6,
                                scale: 1.,
                                color: g.color,
                            });
                            x += widths[idx];
                        }
                        if !self.note_rendering {
                            if end {
                                let mut note = n.clone();
                                note.attrs.insert(
                                    "sourceSection".into(),
                                    self.active_section.to_string(),
                                );
                                self.notes.push(note);
                            } else {
                                let (nx, nw) = self.footnote_area(ox);
                                if self.capture_table_headers {
                                    self.table_note_anchors.push(PlannedTableNote {
                                        node: n.clone(),
                                        x: nx,
                                        width: nw,
                                        y: base,
                                        top: note_top,
                                        op_index: self.current.ops.len() - 1,
                                    });
                                } else {
                                    self.footnotes.push((n.clone(), nx, nw));
                                }
                            }
                        }
                    }
                    "autoNum" => {
                        let num = self.auto_label(n);
                        let g = gs[idx].as_ref().unwrap();
                        self.current.ops.push(Op::Text {
                            x,
                            y: base,
                            text: num,
                            font: g.font,
                            size: g.size,
                            scale: g.scale,
                            color: g.color,
                        });
                        x += widths[idx];
                    }
                    _ => {}
                },
            }
        }
        self.paint_line_decorations(base, underlines, strikes);
        if let Some((label, g, _)) = number_draw {
            let label_x = ox
                + if p.a("reflowed") == "1" {
                    p.n("reflowNumberX")
                } else {
                    0.
                };
            self.draw_list_label(p, label, g, label_x, base)?;
        }
        self.report.bounds.push(Bounds {
            page: self.pages.len() + 1,
            kind: "line".into(),
            id: p.a("id").into(),
            x: origin,
            y: line_y,
            w: (x - origin).max(0.),
            h: seg.n("vertsize") / 100.,
        });
        if li + 1 == segs.len()
            && let Some((_, n)) = deferred
        {
            self.paint_trailing_saved_table(&n, p, ox, oy, width)?;
        }
        Ok(())
    }
}
