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
        let (script, script_key) = text_metrics::language(c);
        let fallback = if c.is_ascii() { "latin" } else { "hangul" };
        let specified = ch
            .child("fontRef")
            .is_some_and(|r| r.attrs.contains_key(script));
        let lang = if specified { script } else { fallback };
        let langkey = if specified {
            script_key
        } else if c.is_ascii() {
            "LATIN"
        } else {
            "HANGUL"
        };
        let fontref = ch.child("fontRef").map(|n| n.a(lang)).unwrap_or("0");
        let name = self
            .doc
            .fonts
            .get(&(langkey.into(), fontref.into()))
            .map(String::as_str)
            .unwrap_or("unknown");
        let bold = ch.child("bold").is_some();
        let italic = ch.child("italic").is_some();
        let mut font = self.document_character_font(name, script, bold, italic, override_font)?;
        if !self.fonts.has(font, c)
            && ('\u{f020}'..='\u{f0ff}').contains(&c)
            && self.fonts.root.join("wingding.ttf").exists()
        {
            let symbol = self.fonts.get("wingding.ttf")?;
            if self.fonts.has(symbol, c) {
                font = symbol;
                self.report.warnings.insert(format!("Legacy private-use symbol U+{:04X} resolved with Wingdings because the requested face has no glyph",c as u32));
            }
        }
        // Native references use HCR Batang for the one-dot leader when
        // an older requested face has no glyph. Keep the source size/style.
        if c == '\u{2024}' && !self.fonts.has(font, c) {
            let file = if bold {
                "HANBatangB.ttf"
            } else {
                "HANBatang.ttf"
            };
            if self.fonts.root.join(file).exists() {
                let fallback = self.fonts.get(file)?;
                if self.fonts.has(fallback, c) {
                    font = fallback;
                }
            }
        }
        if !self.fonts.has(font, c) {
            for fallback in [
                "malgun.ttf",
                "simsun.ttc",
                "msgothic.ttc",
                "seguisym.ttf",
                "arial.ttf",
                if bold {
                    "HANBatangB.ttf"
                } else {
                    "HANBatang.ttf"
                },
                if bold {
                    "HANDotumB.ttf"
                } else {
                    "HANDotum.ttf"
                },
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
        let nominal_size = ch.n("height") / 100.
            * text_metrics::property(ch, "relSz", script, fallback, 100.)
            / 100.;
        let superscript = ch.child("supscript").is_some();
        let subscript = ch.child("subscript").is_some();
        let script_scale = if superscript || subscript { 0.64 } else { 1. };
        let size = nominal_size * script_scale;
        let offset = text_metrics::property(
            ch,
            "offset",
            script,
            if script == "symbol" {
                "other"
            } else {
                fallback
            },
            0.,
        );
        let shift = nominal_size
            * ((if self.vertical_mode > 0 {
                -offset
            } else {
                offset
            }) / 100.
                + if superscript {
                    if self.vertical_mode > 0 { -0.32 } else { -0.44 }
                } else if subscript {
                    if self.vertical_mode > 0 { 0.24 } else { 0.12 }
                } else {
                    0.
                });
        if superscript || subscript {
            self.report.warnings.insert("Superscript/subscript use 64% size and nominal-font baseline shifts; exact Hancom metrics unverified".into());
        }
        let scale = text_metrics::property(ch, "ratio", script, fallback, 100.) / 100.;
        // Latin spacing uses the half-em character cell, including spaces;
        // the adjustment follows horizontal character scaling as well.
        let spacing = text_metrics::property(ch, "spacing", script, fallback, 0.) / 100.
            * size
            * scale
            * if script == "latin" { 0.5 } else { 1. };
        let advance = if c == ' ' && ch.a("useFontSpace") == "0" {
            size * 0.5
        } else {
            self.fonts.width(font, &c.to_string(), size)
        };
        let width = if self.vertical_mode > 0 && c == ' ' {
            nominal_size * 0.5 * scale + spacing
        } else if vertical_text::upright(self.vertical_mode, c) && !c.is_whitespace() {
            size + spacing
        } else {
            advance * scale + spacing
        };
        let face = ttf_parser::Face::parse(&self.fonts.list[font].bytes, 0).unwrap();
        let shear = if italic && !face.is_italic() {
            // The supplied Hancom print uses c/a = 0.25651/0.75.
            // Preserve native italic faces; only synthesize the missing style.
            20f64.to_radians().sin() * scale
        } else {
            0.
        };
        if shear != 0. {
            self.report.warnings.insert("Italic without native italic face uses Hancom-calibrated shear scaled with character width".into());
        }
        let bb = crate::pdf::glyph_id(&face, c).and_then(|id| face.glyph_bounding_box(id));
        let unit = size / face.units_per_em() as f64;
        let pad = self.fonts.ink_padding(font, size);
        let emphasis = (!c.is_whitespace() && !c.is_control())
            .then(|| text_marks::Emphasis::named(ch.a("symMark")))
            .flatten();
        let (mark_above, mark_below) = emphasis.map(|m| m.extent(size)).unwrap_or((0., 0.));
        let character_border = self.character_border_width(ch);
        let outline = text_outlines::read(ch, size);
        if let Some(o) = ch
            .child("outline")
            .filter(|o| !matches!(o.a("type"), "" | "NONE"))
            && outline.is_none()
        {
            self.report.warnings.insert(format!(
                "Unknown character outline type {}; ordinary glyph retained",
                o.a("type")
            ));
        }
        if outline.is_some() {
            self.report.warnings.insert("Character outline uses vector PDF text with nominal stroke/dash metrics; exact Hancom calibration pending".into());
        }
        let glyph = Glyph {
            nominal_size,
            vertical_origin: crate::pdf::glyph_id(&face, c)
                .and_then(|id| {
                    face.glyph_y_origin(id).or_else(|| {
                        face.glyph_bounding_box(id).and_then(|b| {
                            face.glyph_ver_side_bearing(id)
                                .map(|t| b.y_max.saturating_add(t))
                        })
                    })
                })
                .unwrap_or(face.ascender()) as f64
                * unit,
            native_advance: advance * scale,
            shadow: text_shadows::read(ch, size),
            outline,
            emphasis,
            font,
            size,
            scale,
            width,
            tracking: spacing,
            color: color(ch.a("textColor")),
            ascent: (bb.map(|b| b.y_max as f64 * unit + pad).unwrap_or(0.).max(
                if character_border > 0. {
                    size * 0.85
                } else {
                    f64::NEG_INFINITY
                },
            ) + character_border
                - shift
                + mark_above)
                .max(0.),
            descent: (bb.map(|b| -b.y_min as f64 * unit + pad).unwrap_or(0.).max(
                if character_border > 0. {
                    size * 0.15
                } else {
                    f64::NEG_INFINITY
                },
            ) + character_border
                + shift
                + mark_below)
                .max(0.),
            ink_left: bb
                .map(|b| (b.x_min as f64 * scale + shear * b.y_min as f64) * unit - pad)
                .unwrap_or(0.),
            ink_right: bb
                .map(|b| (b.x_max as f64 * scale + shear * b.y_max as f64) * unit + pad)
                .unwrap_or(0.),
            shift,
            shear,
            raw_ascent: bb.map(|b| b.y_max as f64 * unit + pad).unwrap_or(0.),
            raw_descent: bb
                .map(|b| (-b.y_min as f64 * unit + pad).max(0.))
                .unwrap_or(0.),
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
                                "lineBreak" | "hwpParagraphEnd" => pos += 1,
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
                    | "curve" | "arc" | "group" | "chart" | "checkBtn" => {
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
                    "fieldBegin" | "fieldEnd" | "bookmark" => {
                        items.push(Item {
                            pos,
                            style: style.clone(),
                            kind: ItemKind::Control(n.clone()),
                        });
                        pos += 8;
                    }
                    "secPr" => pos += 8,
                    _ => {}
                }
            }
        }
        Self::normalize_field_items(&mut items);
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_italic_matches_reference_transform_without_replacing_native_italic() {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let mut style = d.chars["0"].clone();
        style
            .children
            .retain(|c| !matches!(c.name.as_str(), "bold" | "italic"));
        style.children.push(Node {
            name: "italic".into(),
            ..Default::default()
        });
        node_mut(&mut style, "fontRef")
            .attrs
            .insert("hangul".into(), "probe-ko".into());
        node_mut(&mut style, "fontRef")
            .attrs
            .insert("latin".into(), "probe-en".into());
        d.fonts
            .insert(("HANGUL".into(), "probe-ko".into()), "함초롬바탕".into());
        d.fonts.insert(
            ("LATIN".into(), "probe-en".into()),
            "Times New Roman".into(),
        );
        let mut narrow = style.clone();
        node_mut(&mut narrow, "ratio")
            .attrs
            .insert("hangul".into(), "75".into());
        d.chars.insert("narrow".into(), narrow);
        let mut upright = style.clone();
        upright.children.retain(|c| c.name != "italic");
        upright.children.push(Node {
            name: "bold".into(),
            ..Default::default()
        });
        d.chars.insert("upright".into(), upright);
        d.chars.insert("probe".into(), style);
        let mut e = Engine::new(
            &d,
            Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        let ko = e.glyph('가', "probe").unwrap();
        assert!((ko.shear - 0.25651 / 0.75).abs() < 0.00001);
        assert!(
            (ko.width - e.fonts.width(ko.font, "가", ko.size) * ko.scale - ko.tracking).abs()
                < 0.00001
        );
        let narrow = e.glyph('가', "narrow").unwrap();
        assert!((narrow.shear - ko.shear * 0.75).abs() < 0.00001);
        let last = e.glyph('임', "probe").unwrap();
        let next = e.glyph('진', "upright").unwrap();
        assert_eq!(
            text_metrics::compatible_advance(last.width, &last, &next),
            last.width
        );
        assert!(text_metrics::compatible_advance(0.1, &last, &next) > 0.1);
        let en = e.glyph('A', "probe").unwrap();
        assert_eq!(en.shear, 0.);
        assert!(
            ttf_parser::Face::parse(&e.fonts.list[en.font].bytes, 0)
                .unwrap()
                .is_italic()
        );
    }
}
