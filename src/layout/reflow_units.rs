//! Convert styled text, controls and objects into measured wrapping units.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn paragraph_units(&mut self, p: &Node) -> Result<Vec<crate::reflow::Unit>> {
        let items = Self::items(p);
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
                            descent: g.descent,
                            em: g.size,
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
                            ascent: g.ascent,
                            descent: g.descent,
                            em: g.size,
                            block: false,
                        },
                        Some(g),
                    )
                }
                ItemKind::Tab(w) => (
                    crate::reflow::Unit {
                        pos: item.pos,
                        width: *w,
                        ch: Some(' '),
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
                            block,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Object(n)
                    if matches!(
                        n.name.as_str(),
                        "tbl" | "pic" | "rect" | "ellipse" | "polygon" | "curve" | "group"
                    ) =>
                {
                    let block =
                        n.name == "tbl" || n.child("pos").is_none_or(|p| p.a("treatAsChar") != "1");
                    let m = margin(n, "outMargin");
                    let cap = caption_extent(n);
                    let h = effective_height(n) + m[2] + m[3] + cap[2] + cap[3];
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some(' '),
                            width: if block {
                                0.
                            } else {
                                val(n.child("sz"), "width") + m[0] + m[1]
                            },
                            ascent: if block { h } else { h * 0.85 },
                            descent: if block { 0. } else { h * 0.15 },
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
                        g.size * 0.6,
                    ) * g.scale;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            width: w,
                            ascent: g.size,
                            em: g.size,
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
                            descent: g.descent,
                            em: g.size,
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
                            | "pageHiding"
                            | "newNumber"
                            | "header"
                            | "footer"
                            | "cachedField"
                            | "bookmark"
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
                units[i].width = units[i].width.max(a.ink_right - b.ink_left + 0.02);
            }
        }
        for i in 1..items.len() {
            if matches!(&items[i].kind,ItemKind::Control(n) if n.name=="footNote") {
                units[i - 1].ch = None;
            }
        }
        let mut grouped: Vec<crate::reflow::Unit> = Vec::new();
        for unit in units {
            if let Some(last) = grouped.last_mut().filter(|last| last.pos == unit.pos) {
                last.width += unit.width;
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
