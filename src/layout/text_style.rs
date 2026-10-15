//! Layout responsibility: text style.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn glyph(&mut self, c: char, style: &str) -> Result<Glyph> {
        self.glyph_with_font(c, style, None)
    }
    pub(super) fn glyph_with_font(
        &mut self,
        c: char,
        style: &str,
        override_font: Option<&str>,
    ) -> Result<Glyph> {
        let key = (c, format!("{style}\0{}", override_font.unwrap_or("")));
        if let Some(g) = self.glyph_cache.get(&key) {
            return Ok(g.clone());
        }
        let ch = self.doc.chars.get(style).context("Invalid charPrIDRef")?;
        let latin = c.is_ascii();
        let lang = if latin { "latin" } else { "hangul" };
        let langkey = if latin { "LATIN" } else { "HANGUL" };
        let fontref = ch.child("fontRef").map(|n| n.a(lang)).unwrap_or("0");
        let name = self
            .doc
            .fonts
            .get(&(langkey.into(), fontref.into()))
            .map(String::as_str)
            .unwrap_or("unknown");
        let bold = ch.child("bold").is_some();
        let italic = ch.child("italic").is_some();
        let serif = name.contains("명조") || name.contains("바탕") || name.contains("Times");
        let file = if name.eq_ignore_ascii_case("Wingdings") {
            "wingding.ttf"
        } else if name.eq_ignore_ascii_case("Symbol") {
            "symbol.ttf"
        } else if latin && serif {
            match (bold, italic) {
                (true, true) => "timesbi.ttf",
                (false, true) => "timesi.ttf",
                (true, false) => "timesbd.ttf",
                _ => "times.ttf",
            }
        } else if latin {
            match (bold, italic) {
                (true, true) => "arialbi.ttf",
                (false, true) => "ariali.ttf",
                (true, false) => "arialbd.ttf",
                _ => "arial.ttf",
            }
        } else if serif {
            "batang.ttc"
        } else if bold {
            "malgunbd.ttf"
        } else {
            "malgun.ttf"
        };
        let file = override_font.unwrap_or(file);
        self.report
            .warnings
            .insert(format!("Font substitution: {name} -> {file}"));
        let mut font = self.fonts.get(file)?;
        if !self.fonts.has(font, c) {
            for fallback in [
                "malgun.ttf",
                "simsun.ttc",
                "msgothic.ttc",
                "seguisym.ttf",
                "arial.ttf",
            ] {
                if !self.fonts.root.join(fallback).exists() {
                    continue;
                }
                let other = self.fonts.get(fallback)?;
                if self.fonts.has(other, c) {
                    font = other;
                    break;
                }
            }
            if !self.fonts.has(font, c) {
                self.report
                    .warnings
                    .insert(format!("Missing glyph U+{:04X}", c as u32));
            }
        }
        let nominal_size =
            ch.n("height") / 100. * ch.child("relSz").map(|n| n.n(lang) / 100.).unwrap_or(1.);
        let superscript = ch.child("supscript").is_some();
        let subscript = ch.child("subscript").is_some();
        let script_scale = if superscript || subscript { 0.65 } else { 1. };
        let size = nominal_size * script_scale;
        let offset_lang = match c as u32 {
            0x3040..=0x30ff | 0x31f0..=0x31ff => "japanese",
            0x3400..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x323af => "hanja",
            0x1100..=0x11ff | 0x3130..=0x318f | 0xa960..=0xa97f | 0xac00..=0xd7ff => "hangul",
            0..=0x024f => "latin",
            _ => "other",
        };
        let offset = ch.child("offset").map(|n| n.n(offset_lang)).unwrap_or(0.);
        let shift = nominal_size
            * (offset / 100.
                + if superscript {
                    -0.35
                } else if subscript {
                    0.15
                } else {
                    0.
                });
        if superscript || subscript {
            self.report.warnings.insert("Superscript/subscript use 65% size and nominal-font baseline shifts; exact Hancom metrics unverified".into());
        }
        let scale = ch.child("ratio").map(|n| n.n(lang) / 100.).unwrap_or(1.);
        let spacing = ch
            .child("spacing")
            .map(|n| n.n(lang) / 100. * size)
            .unwrap_or(0.);
        let width = self.fonts.width(font, &c.to_string(), size) * scale + spacing;
        let face = ttf_parser::Face::parse(&self.fonts.list[font].bytes, 0).unwrap();
        let shear = if italic && !face.is_italic() {
            12f64.to_radians().tan()
        } else {
            0.
        };
        if shear != 0. {
            self.report.warnings.insert("Italic without native italic face uses 12-degree synthetic slant; exact Hancom angle unverified".into());
        }
        let bb = crate::pdf::glyph_id(&face, c).and_then(|id| face.glyph_bounding_box(id));
        let unit = size / face.units_per_em() as f64;
        let glyph = Glyph {
            font,
            size,
            scale,
            width,
            color: color(ch.a("textColor")),
            ascent: (bb.map(|b| b.y_max as f64 * unit).unwrap_or(0.) - shift).max(0.),
            descent: (bb.map(|b| -b.y_min as f64 * unit).unwrap_or(0.) + shift).max(0.),
            ink_left: bb
                .map(|b| (b.x_min as f64 * scale + shear * b.y_min as f64) * unit)
                .unwrap_or(0.),
            ink_right: bb
                .map(|b| (b.x_max as f64 * scale + shear * b.y_max as f64) * unit)
                .unwrap_or(0.),
            shift,
            shear,
            raw_ascent: bb.map(|b| b.y_max as f64 * unit).unwrap_or(0.),
            raw_descent: bb.map(|b| (-b.y_min as f64 * unit).max(0.)).unwrap_or(0.),
        };
        self.glyph_cache.insert(key, glyph.clone());
        Ok(glyph)
    }
    pub(super) fn items(p: &Node) -> Vec<Item> {
        let mut items = Vec::new();
        let mut pos = 0;
        for run in p.children_named("run") {
            let style = run.a("charPrIDRef").to_string();
            for n in &run.children {
                match n.name.as_str() {
                    "t" => {
                        for child in &n.children {
                            match child.name.as_str() {
                                "#text" => {
                                    for c in child.text.chars() {
                                        items.push(Item {
                                            pos,
                                            style: style.clone(),
                                            kind: ItemKind::Char(c),
                                        });
                                        pos += c.len_utf16()
                                    }
                                }
                                "tab" => {
                                    items.push(Item {
                                        pos,
                                        style: style.clone(),
                                        kind: ItemKind::Tab(child.n("width") / 100.),
                                    });
                                    pos += 8;
                                }
                                "lineBreak" => pos += 1,
                                "hwpDisplay" => {
                                    for c in child.a("value").chars() {
                                        items.push(Item {
                                            pos,
                                            style: style.clone(),
                                            kind: ItemKind::Char(c),
                                        });
                                    }
                                    pos += child.u("units");
                                }
                                "nbSpace" | "fwSpace" => {
                                    items.push(Item {
                                        pos,
                                        style: style.clone(),
                                        kind: ItemKind::Char(' '),
                                    });
                                    pos += 1;
                                }
                                _ => {}
                            }
                        }
                    }
                    "tbl" | "pic" | "equation" | "rect" | "line" | "ellipse" | "polygon"
                    | "curve" | "arc" | "group" | "chart" => {
                        items.push(Item {
                            pos,
                            style: style.clone(),
                            kind: ItemKind::Object(n.clone()),
                        });
                        pos += 8;
                    }
                    "ctrl" => {
                        for c in n.children.iter().filter(|c| c.name != "#text") {
                            items.push(Item {
                                pos,
                                style: style.clone(),
                                kind: ItemKind::Control(c.clone()),
                            });
                            pos += 8;
                        }
                    }
                    "secPr" => pos += 8,
                    _ => {}
                }
            }
        }
        items
    }
}
