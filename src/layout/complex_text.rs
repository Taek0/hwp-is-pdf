//! Bidirectional run ordering and OpenType glyph shaping.
use super::*;
struct Character {
    start: usize,
    end: usize,
    pos: usize,
    style: String,
    g: Glyph,
    script: u8,
}
struct Run {
    text: String,
    style: String,
    g: Glyph,
    glyphs: Vec<crate::pdf::PositionedGlyph>,
    width: f64,
    metrics: Vec<(usize, f64, f64, f64)>,
    unicode: BTreeMap<u16, String>,
}
fn complex(c: char) -> bool {
    matches!(c as u32,0x0300..=0x036f|0x0590..=0x08ff|0xfb1d..=0xfdff|0xfe70..=0xfeff)
}
fn mark(c: char) -> bool {
    matches!(c as u32,0x0300..=0x036f|0x0591..=0x05bd|0x064b..=0x065f)
}
fn script(c: char) -> u8 {
    match c as u32 {
        0x0590..=0x05ff | 0xfb1d..=0xfb4f => 1,
        0x0600..=0x08ff | 0xfb50..=0xfdff | 0xfe70..=0xfeff => 2,
        _ => 0,
    }
}
impl<'a> Engine<'a> {
    fn needs_complex(&self, p: &Node) -> bool {
        self.doc
            .paras
            .get(p.a("paraPrIDRef"))
            .is_some_and(|s| s.a("textDir") == "RTL")
            || Self::items(p)
                .iter()
                .any(|i| matches!(i.kind,ItemKind::Char(c) if complex(c)))
    }
    fn shaped_runs(&mut self, p: &Node, items: &[Item]) -> Result<Vec<Run>> {
        let mut text = String::new();
        let mut chars: Vec<Character> = Vec::new();
        for item in items {
            let c = match &item.kind {
                ItemKind::Char(c) => *c,
                ItemKind::Object(n) if n.a("reflowBehind") == "1" => continue,
                ItemKind::Control(n)
                    if matches!(
                        n.name.as_str(),
                        "colPr"
                            | "header"
                            | "footer"
                            | "pageHiding"
                            | "pageNumPos"
                            | "newNumber"
                            | "bookmark"
                            | "fieldBegin"
                            | "fieldEnd"
                            | "hyperlink"
                            | "cachedField"
                            | "nonPrinting"
                    ) =>
                {
                    continue;
                }
                _ => bail!(
                    "Complex text with inline objects, tabs or display controls requires additional bidi integration"
                ),
            };
            let start = text.len();
            text.push(c);
            let mut g = if script(c) > 0 {
                let s = self.doc.chars.get(&item.style).context("Character style")?;
                let file = match (s.child("bold").is_some(), s.child("italic").is_some()) {
                    (true, true) => "arialbi.ttf",
                    (false, true) => "ariali.ttf",
                    (true, false) => "arialbd.ttf",
                    _ => "arial.ttf",
                };
                self.glyph_with_font(c, &item.style, Some(file))?
            } else if mark(c) && chars.last().is_some_and(|last| last.style == item.style) {
                let file = self.fonts.list[chars.last().unwrap().g.font].name.clone();
                self.glyph_with_font(c, &item.style, Some(&file))?
            } else {
                self.glyph(c, &item.style)?
            };
            if script(c) > 0 {
                g.shear = 0.;
            }
            let sc = if c.is_whitespace() || mark(c) {
                chars.last().map(|c| c.script).unwrap_or(0)
            } else {
                script(c)
            };
            chars.push(Character {
                start,
                end: text.len(),
                pos: item.pos,
                style: item.style.clone(),
                g,
                script: sc,
            });
        }
        let direction = self
            .doc
            .paras
            .get(p.a("paraPrIDRef"))
            .filter(|s| s.a("textDir") == "RTL")
            .map(|_| unicode_bidi::Level::rtl());
        let bidi = unicode_bidi::BidiInfo::new(&text, direction);
        let mut out = Vec::new();
        for paragraph in &bidi.paragraphs {
            let (levels, ranges) = bidi.visual_runs(paragraph, paragraph.range.clone());
            for range in ranges {
                let rtl = levels[range.start].is_rtl();
                let selected: Vec<_> = chars
                    .iter()
                    .filter(|c| {
                        c.start >= range.start
                            && c.start < range.end
                            && !matches!(&text[c.start..c.end], "\n" | "\r")
                    })
                    .collect();
                let mut groups: Vec<Vec<&Character>> = Vec::new();
                for c in selected {
                    if groups.last().and_then(|g| g.last()).is_some_and(|last| {
                        last.style == c.style
                            && last.g.font == c.g.font
                            && last.script == c.script
                            && last.g.size == c.g.size
                            && last.g.scale == c.g.scale
                            && last.g.shift == c.g.shift
                    }) {
                        groups.last_mut().unwrap().push(c);
                    } else {
                        groups.push(vec![c]);
                    }
                }
                if rtl {
                    groups.reverse();
                }
                for group in groups {
                    let first = group[0];
                    let content = &text[first.start..group.last().unwrap().end];
                    let g = first.g.clone();
                    let face = rustybuzz::Face::from_slice(&self.fonts.list[g.font].bytes, 0)
                        .context("Invalid shaping font")?;
                    let mut buffer = rustybuzz::UnicodeBuffer::new();
                    buffer.push_str(content);
                    buffer.set_direction(if rtl {
                        rustybuzz::Direction::RightToLeft
                    } else {
                        rustybuzz::Direction::LeftToRight
                    });
                    let shaped = rustybuzz::shape(&face, &[], buffer);
                    let unicode = complex_unicode::mappings(
                        &face,
                        content,
                        &shaped
                            .glyph_infos()
                            .iter()
                            .map(|g| (g.glyph_id as u16, g.cluster as usize))
                            .collect::<Vec<_>>(),
                    );
                    let unit = g.size / face.units_per_em() as f64;
                    let mut cursor = 0.;
                    let mut glyphs = Vec::new();
                    let mut metrics = Vec::new();
                    for (info, position) in
                        shaped.glyph_infos().iter().zip(shaped.glyph_positions())
                    {
                        let owner = group
                            .iter()
                            .rev()
                            .find(|c| c.start - first.start <= info.cluster as usize)
                            .unwrap_or(&first);
                        let advance = position.x_advance.unsigned_abs() as f64 * unit * g.scale;
                        let x = cursor + position.x_offset as f64 * unit * g.scale;
                        let y = -(position.y_offset as f64) * unit;
                        let id = info.glyph_id as u16;
                        let bb = face.glyph_bounding_box(ttf_parser::GlyphId(id));
                        let ascent = bb
                            .map(|b| b.y_max as f64 * unit - y - g.shift)
                            .unwrap_or(0.)
                            .max(0.);
                        let descent = bb
                            .map(|b| y - b.y_min as f64 * unit + g.shift)
                            .unwrap_or(0.)
                            .max(0.);
                        metrics.push((owner.pos, advance, ascent, descent));
                        glyphs.push(crate::pdf::PositionedGlyph { id, x, y });
                        cursor += advance;
                    }
                    out.push(Run {
                        text: content.into(),
                        style: first.style.clone(),
                        g,
                        glyphs,
                        width: cursor,
                        metrics,
                        unicode,
                    });
                }
            }
        }
        Ok(out)
    }
    pub(super) fn measure_complex_units(
        &mut self,
        p: &Node,
        items: &[Item],
        units: &mut [crate::reflow::Unit],
    ) -> Result<()> {
        if !self.needs_complex(p) {
            return Ok(());
        }
        let runs = self.shaped_runs(p, items)?;
        for (item, unit) in items.iter().zip(units.iter_mut()) {
            if matches!(item.kind, ItemKind::Char(_)) {
                unit.width = 0.;
                unit.trailing = 0.;
            }
        }
        for run in runs {
            for (pos, width, ascent, descent) in run.metrics {
                if let Some(u) = units.iter_mut().find(|u| u.pos == pos) {
                    u.width += width;
                    u.ascent = u.ascent.max(ascent);
                    u.descent = u.descent.max(descent);
                }
            }
        }
        self.report.warnings.insert("Complex text uses Unicode bidi ordering and OpenType shaping; inline objects/tabs/display controls in shaped paragraphs remain limited".into());
        Ok(())
    }
    pub(super) fn paint_complex_line(
        &mut self,
        p: &Node,
        li: usize,
        ox: f64,
        oy: f64,
        width: f64,
    ) -> Result<bool> {
        if !self.needs_complex(p) {
            return Ok(false);
        }
        let segs = lines(p);
        let seg = segs[li];
        let start = seg.u("textpos");
        let end = segs
            .get(li + 1)
            .map(|s| s.u("textpos"))
            .unwrap_or(usize::MAX);
        let items: Vec<_> = Self::items(p)
            .into_iter()
            .filter(|i| i.pos >= start && i.pos < end)
            .collect();
        let runs = self.shaped_runs(p, &items)?;
        let total = runs.iter().map(|r| r.width).sum::<f64>();
        let available = if seg.n("horzsize") > 0. {
            seg.n("horzsize") / 100.
        } else {
            width
        };
        let factor = (available / total.max(0.01)).min(1.);
        let line_y = oy + seg.n("vertpos") / 100.;
        let base = line_y + seg.n("baseline") / 100.;
        let mut x = ox + seg.n("horzpos") / 100.;
        let align = self
            .doc
            .paras
            .get(p.a("paraPrIDRef"))
            .and_then(|s| s.child("align"))
            .map(|s| s.a("horizontal"))
            .unwrap_or("LEFT");
        x += match align {
            "RIGHT" => (available - total).max(0.),
            "CENTER" => (available - total).max(0.) / 2.,
            _ => 0.,
        };
        let origin = x;
        let field_targets = self.line_field_targets(
            p,
            &items,
            x,
            line_y,
            base,
            li == 0 && p.u("reflowStart") == 0,
        );
        self.current.ops.push(Op::TransformBegin {
            matrix: crate::graphics::IDENTITY,
        });
        self.current.ops.push(Op::ActualTextBegin {
            text: items
                .iter()
                .filter_map(|i| {
                    if let ItemKind::Char(c) = i.kind {
                        (!matches!(c, '\n' | '\r')).then_some(c)
                    } else {
                        None
                    }
                })
                .collect(),
        });
        for mut run in runs {
            let g = &run.g;
            for (id, text) in run.unicode {
                self.fonts.list[g.font].used.entry(id).or_insert(text);
            }
            for glyph in &mut run.glyphs {
                glyph.x *= factor;
            }
            for (metric, glyph) in run.metrics.iter().zip(&run.glyphs) {
                if let Some(i) = items.iter().position(|i| i.pos == metric.0) {
                    self.link_ink(
                        &field_targets[i],
                        x + glyph.x,
                        line_y,
                        metric.1 * factor,
                        seg.n("vertsize") / 100.,
                        base,
                    );
                }
            }
            if g.shear != 0. {
                self.current.ops.push(Op::TransformBegin {
                    matrix: [1., 0., -g.shear, 1., g.shear * (base + g.shift), 0.],
                });
            }
            let emphasis = self
                .doc
                .chars
                .get(&run.style)
                .and_then(|s| text_marks::Emphasis::named(s.a("symMark")));
            if emphasis.is_some() {
                for (metric, glyph) in run.metrics.iter().zip(&run.glyphs) {
                    if metric.1>0. && items.iter().any(|i| i.pos==metric.0
                        && matches!(i.kind,ItemKind::Char(c) if !c.is_whitespace() && !c.is_control())) {
                        let mut decorated=g.clone(); decorated.emphasis=emphasis;
                        decorated.raw_ascent=(metric.2+glyph.y+g.shift).max(0.);
                        decorated.raw_descent=(metric.3-glyph.y-g.shift).max(0.);
                        self.paint_emphasis(&decorated,x+glyph.x,base+g.shift+glyph.y,metric.1*factor);
                    }
                }
            }
            self.current.ops.push(Op::Shaped {
                x,
                y: base + g.shift,
                text: run.text,
                font: g.font,
                size: g.size,
                scale: g.scale * factor,
                color: g.color,
                glyphs: run.glyphs,
            });
            if g.shear != 0. {
                self.current.ops.push(Op::TransformEnd);
            }
            if let Some(style) = self.doc.chars.get(&run.style) {
                for kind in ["underline", "strikeout"] {
                    if let Some(def) = style
                        .child(kind)
                        .filter(|d| kind == "strikeout" || matches!(d.a("type"), "BOTTOM" | "TOP"))
                        && let Some(shape) = crate::decoration::style(def)
                    {
                        let y = base
                            + g.shift
                            + if kind == "strikeout" {
                                -g.size * 0.3
                            } else if def.a("type") == "TOP" {
                                -g.size
                            } else {
                                run.metrics.iter().map(|m| m.3).fold(0f64, f64::max) - g.shift + 0.9
                            };
                        self.current.ops.extend(crate::decoration::underline(
                            x,
                            x + run.width * factor,
                            y,
                            shape,
                            color(def.a("color")),
                        ));
                    }
                }
            }
            x += run.width * factor;
        }
        self.current.ops.push(Op::ActualTextEnd);
        self.current.ops.push(Op::TransformEnd);
        if li == 0
            && p.u("reflowStart") == 0
            && let Some((label, g, _)) = self.paragraph_number(p, p.a("reflowed") == "1")?
        {
            self.draw_list_label(p, label, g, ox + p.n("reflowNumberX"), base)?;
        }
        self.report.bounds.push(Bounds {
            page: self.pages.len() + 1,
            kind: "line".into(),
            id: p.a("id").into(),
            x: origin,
            y: line_y,
            w: x - origin,
            h: seg.n("vertsize") / 100.,
        });
        Ok(true)
    }
}
