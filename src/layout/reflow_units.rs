//! Convert styled text, controls and objects into measured wrapping units.
use super::*;
impl<'a> Engine<'a> {
    fn inline_em(&self, style: &str) -> f64 {
        self.doc
            .chars
            .get(style)
            .map(|n| {
                n.n("height") / 100. * text_metrics::property(n, "relSz", "latin", "latin", 100.)
                    / 100.
                    * if n.child("supscript").is_some() || n.child("subscript").is_some() {
                        0.64
                    } else {
                        1.
                    }
            })
            .filter(|v| *v > 0.)
            .unwrap_or(12.)
    }
    pub(super) fn paragraph_units(&mut self, p: &Node) -> Result<Vec<crate::reflow::Unit>> {
        let items = Self::items(p);
        if items.is_empty()
            && let Some(run) = p.child("run")
            && self.doc.chars.contains_key(run.a("charPrIDRef"))
        {
            let g = self.glyph(' ', run.a("charPrIDRef"))?;
            return Ok(vec![crate::reflow::Unit {
                ascent: g.ascent,
                baseline_ascent: (g.ascent + g.shift).max(0.),
                descent: g.descent,
                em: g.nominal_size,
                ..Default::default()
            }]);
        }
        let mut units = Vec::new();
        let mut glyphs = Vec::new();
        for item in &items {
            let (unit, glyph) = match &item.kind {
                ItemKind::Char(c) if *c == '\n' || *c == '\r' => {
                    let g = self.glyph(' ', &item.style)?;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some('\n'),
                            ascent: g.ascent,
                            baseline_ascent: (g.ascent + g.shift).max(0.),
                            descent: g.descent,
                            em: g.nominal_size,
                            block: false,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Char(c) => {
                    let g = self.glyph(*c, &item.style)?;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some(*c),
                            width: g.width.max(0.),
                            trailing: g.tracking,
                            ascent: g.ascent,
                            baseline_ascent: (g.ascent + g.shift).max(0.),
                            descent: g.descent,
                            em: g.nominal_size,
                            block: false,
                        },
                        Some(g),
                    )
                }
                ItemKind::Tab(w) => (
                    crate::reflow::Unit {
                        pos: item.pos,
                        width: *w,
                        ch: Some('\t'),
                        ..Default::default()
                    },
                    None,
                ),
                ItemKind::Object(n) if n.a("reflowFloat") == "1" => (
                    crate::reflow::Unit {
                        pos: item.pos,
                        ..Default::default()
                    },
                    None,
                ),
                ItemKind::Object(n) if n.name == "equation" => {
                    let m = margin(n, "outMargin");
                    let h = val(n.child("sz"), "height");
                    let b = n.n("baseLine") / 100.;
                    let block = n.child("pos").is_none_or(|p| p.a("treatAsChar") != "1");
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some(' '),
                            width: if block {
                                0.
                            } else {
                                val(n.child("sz"), "width") + m[0] + m[1]
                            },
                            ascent: if block { h + m[2] + m[3] } else { h * b + m[2] },
                            descent: if block {
                                0.
                            } else {
                                h * (1. - b) + m[2] + m[3]
                            },
                            em: if !block && self.doc.header.a("binaryHwp") != "1" {
                                self.inline_em(&item.style)
                            } else {
                                0.
                            },
                            block,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Object(n)
                    if matches!(
                        n.name.as_str(),
                        "tbl"
                            | "pic"
                            | "rect"
                            | "ellipse"
                            | "polygon"
                            | "curve"
                            | "group"
                            | "line"
                            | "arc"
                            | "chart"
                            | "checkBtn"
                    ) =>
                {
                    let block =
                        n.name == "tbl" || n.child("pos").is_none_or(|p| p.a("treatAsChar") != "1");
                    let m = margin(n, "outMargin");
                    let cap = caption_extent(n);
                    let native = effective_height(n).max(if n.name == "line" {
                        val(n.child("lineShape"), "width").max(1.)
                    } else {
                        0.
                    });
                    let side = side_caption_height(n).max(native);
                    let h = side + m[2] + m[3] + cap[2] + cap[3];
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some(' '),
                            width: if block {
                                0.
                            } else {
                                val(n.child("sz"), "width") + m[0] + m[1] + cap[0] + cap[1]
                            },
                            ascent: if block {
                                h
                            } else if n.a("reflowCaptions") == "1" {
                                native * 0.85 + cap[2] + m[2]
                            } else {
                                h * 0.85
                            },
                            descent: if block {
                                0.
                            } else if n.a("reflowCaptions") == "1" {
                                native * 0.15 + (side - native) + cap[3] + m[2] + m[3]
                            } else {
                                h * 0.15
                            },
                            em: if !block && self.doc.header.a("binaryHwp") != "1" {
                                self.inline_em(&item.style)
                            } else {
                                0.
                            },
                            block,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Control(n) if matches!(n.name.as_str(), "footNote" | "endNote") => {
                    let g = self.glyph('1', &item.style)?;
                    let w = self.fonts.width(
                        g.font,
                        &self.note_label(n, n.name == "endNote"),
                        g.size * notes::ANCHOR_SIZE,
                    ) * g.scale;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            width: w,
                            ascent: g.size,
                            em: g.nominal_size,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Control(n) if n.name == "autoNum" => {
                    let g = self.glyph('1', &item.style)?;
                    let w = self.fonts.width(g.font, &self.auto_label(n), g.size) * g.scale;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            width: w,
                            ascent: g.ascent,
                            baseline_ascent: (g.ascent + g.shift).max(0.),
                            descent: g.descent,
                            em: g.nominal_size,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Control(n) if n.name == "pageField" => {
                    let g = self.glyph('8', &item.style)?;
                    let width = self.fonts.width(g.font, "8888", g.size) * g.scale;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some(' '),
                            width,
                            ascent: g.ascent,
                            baseline_ascent: (g.ascent + g.shift).max(0.),
                            descent: g.descent,
                            em: g.nominal_size,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Control(c)
                    if matches!(
                        c.name.as_str(),
                        "colPr"
                            | "pageNumPos"
                            | "pageNumCtrl"
                            | "pageHiding"
                            | "newNumber"
                            | "header"
                            | "footer"
                            | "cachedField"
                            | "bookmark"
                            | "fieldBegin"
                            | "fieldEnd"
                            | "hyperlink"
                            | "nonPrinting"
                    ) =>
                {
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Control(n)
                    if n.name == "opaque" && matches!(n.u("code"), 4..=8 | 19 | 20) =>
                {
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ..Default::default()
                        },
                        None,
                    )
                }
                _ => bail!(
                    "Plain-text reflow cannot lay out objects/notes/control in paragraph {}",
                    p.a("id")
                ),
            };
            units.push(unit);
            glyphs.push(glyph);
        }
        for i in 0..units.len().saturating_sub(1) {
            if let (Some(a), Some(b)) = (&glyphs[i], &glyphs[i + 1]) {
                units[i].width =
                    if let Some(adjust) = self.pair_kerning(&items[i], &items[i + 1], a, b) {
                        (units[i].width + adjust).max(0.)
                    } else {
                        if self.doc.header.a("binaryHwp") == "1" || a.shear != 0. || b.shear != 0. {
                            text_metrics::compatible_advance(units[i].width, a, b)
                        } else {
                            units[i].width.max(a.ink_right - b.ink_left + 0.02)
                        }
                    };
            }
        }
        for i in 0..items.len().saturating_sub(1) {
            if let (Some(a), Some(b)) = (&glyphs[i], &glyphs[i + 1]) {
                units[i].width =
                    self.character_boundary_advance(units[i].width, &items[i], &items[i + 1], a, b);
            }
        }
        for i in 1..items.len() {
            if matches!(&items[i].kind,ItemKind::Control(n) if n.name=="footNote") {
                units[i - 1].ch = None;
            }
        }
        for (unit, glyph) in units.iter_mut().zip(&glyphs) {
            if let Some(g) = glyph {
                unit.trailing = unit.width - (g.width - g.tracking);
            }
        }
        let mut grouped: Vec<crate::reflow::Unit> = Vec::new();
        self.measure_complex_units(p, &items, &mut units)?;
        for unit in units {
            if let Some(last) = grouped.last_mut().filter(|last| last.pos == unit.pos) {
                last.width += unit.width;
                last.trailing = unit.trailing;
                last.ascent = last.ascent.max(unit.ascent);
                last.descent = last.descent.max(unit.descent);
                last.em = last.em.max(unit.em);
                last.ch = None;
            } else {
                grouped.push(unit);
            }
        }
        Ok(grouped)
    }
}
