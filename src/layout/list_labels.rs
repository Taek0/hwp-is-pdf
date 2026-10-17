//! List counters, formatted labels and image bullets.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn list_auto_indent(&self, p: &Node) -> bool {
        let Some(heading) = self
            .doc
            .paras
            .get(p.a("paraPrIDRef"))
            .and_then(|s| s.child("heading"))
        else {
            return false;
        };
        if !matches!(heading.a("type"), "NUMBER" | "BULLET") {
            return false;
        }
        let bullet = heading.a("type") == "BULLET";
        let mut defs = Vec::new();
        self.doc
            .header
            .all(if bullet { "bullet" } else { "numbering" }, &mut defs);
        defs.into_iter()
            .find(|n| n.a("id") == heading.a("idRef"))
            .and_then(|n| {
                if bullet {
                    n.child("paraHead").or(Some(n))
                } else {
                    n.children_named("paraHead")
                        .find(|h| h.u("level") == heading.u("level") + 1)
                }
            })
            .is_some_and(|h| matches!(h.a("autoIndent"), "1" | "true"))
    }
    fn list_image(&self, p: &Node, g: &Glyph) -> Result<Option<(String, f64, f64)>> {
        let Some(heading) = self
            .doc
            .paras
            .get(p.a("paraPrIDRef"))
            .and_then(|s| s.child("heading"))
            .filter(|h| h.a("type") == "BULLET")
        else {
            return Ok(None);
        };
        let mut defs = Vec::new();
        self.doc.header.all("bullet", &mut defs);
        let Some(def) = defs
            .into_iter()
            .find(|d| d.a("id") == heading.a("idRef") && matches!(d.a("useImage"), "1" | "true"))
        else {
            return Ok(None);
        };
        let id = def
            .child("img")
            .context("Missing image bullet reference")?
            .a("binaryItemIDRef");
        let data = self
            .doc
            .binaries
            .get(id)
            .context("Missing image bullet binary")?;
        let image = image::load_from_memory(data)?;
        let h = g.ascent + g.descent;
        let w = h * image.width() as f64 / image.height().max(1) as f64;
        Ok(Some((id.to_string(), w, h)))
    }
    pub(super) fn draw_list_label(
        &mut self,
        p: &Node,
        label: String,
        g: Glyph,
        mut x: f64,
        base: f64,
    ) -> Result<()> {
        if p.a("reflowed") == "1" {
            let heading = self
                .doc
                .paras
                .get(p.a("paraPrIDRef"))
                .and_then(|s| s.child("heading"));
            if let Some(heading) = heading {
                let bullet = heading.a("type") == "BULLET";
                let mut defs = Vec::new();
                self.doc
                    .header
                    .all(if bullet { "bullet" } else { "numbering" }, &mut defs);
                let head = defs
                    .into_iter()
                    .find(|d| d.a("id") == heading.a("idRef"))
                    .and_then(|d| {
                        if bullet {
                            d.child("paraHead").or(Some(d))
                        } else {
                            d.children_named("paraHead")
                                .find(|h| h.u("level") == heading.u("level") + 1)
                        }
                    });
                if let Some(head) = head {
                    let adjust = head.signed("widthAdjust").max(0.) / 100.;
                    x += match head.a("align") {
                        "RIGHT" => adjust,
                        "CENTER" => adjust / 2.,
                        _ => 0.,
                    };
                }
            }
        }
        if let Some((id, w, h)) = self.list_image(p, &g)? {
            self.current.ops.push(Op::Image {
                x,
                y: base - g.ascent,
                w,
                h,
                id,
            });
            self.report.pictures += 1;
            self.report.bounds.push(Bounds {
                page: self.pages.len() + 1,
                kind: "pic".into(),
                id: format!("bullet-{}", p.a("id")),
                x,
                y: base - g.ascent,
                w,
                h,
            });
        } else {
            self.paint_generated_text(label, g, x, base)?;
        }
        Ok(())
    }
    pub(super) fn paragraph_number(
        &mut self,
        p: &Node,
        reflow: bool,
    ) -> Result<Option<(String, Glyph, f64)>> {
        if !p.a("tocEntry").is_empty() {
            return Ok(None);
        }
        let style = self.doc.paras.get(p.a("paraPrIDRef"));
        if let Some(heading) = style.and_then(|s| s.child("heading"))
            && matches!(heading.a("type"), "NUMBER" | "BULLET")
        {
            let bullet = heading.a("type") == "BULLET";
            let mut defs = Vec::new();
            self.doc
                .header
                .all(if bullet { "bullet" } else { "numbering" }, &mut defs);
            let owned_definition = defs
                .into_iter()
                .find(|d| d.a("id") == heading.a("idRef"))
                .cloned();
            let definition = owned_definition.as_ref();
            let head = if bullet {
                definition
            } else {
                definition.and_then(|n| {
                    n.children_named("paraHead")
                        .find(|h| h.u("level") == heading.u("level") + 1)
                })
            }
            .cloned();
            if let Some(mut head) = head {
                // HWPX stores bullet layout attributes in a paraHead child;
                // normalized HWP stores them on the bullet itself.
                if reflow
                    && bullet
                    && let Some(properties) = head.child("paraHead").cloned()
                {
                    head.attrs.extend(properties.attrs);
                }
                let key = (self.active_section, p.a("id").to_string());
                let label = if bullet {
                    let image = matches!(head.a("useImage"), "1" | "true");
                    anyhow::ensure!(
                        !reflow || !head.a("char").is_empty() || image,
                        "Image-only or empty bullet reflow is not supported"
                    );
                    if image {
                        String::new()
                    } else {
                        head.a("char").to_string()
                    }
                } else if let Some(label) = self.labels.get(&key) {
                    label.clone()
                } else {
                    let derived = Self::items(p).iter().find_map(|i| match &i.kind {
                        ItemKind::Control(c) if c.name == "endNote" => Some(c.u("number")),
                        _ => None,
                    });
                    let nk = (heading.a("idRef").to_string(), heading.u("level"));
                    let num = if let Some(n) = derived {
                        n
                    } else {
                        let start = if reflow && head.u("start") > 0 {
                            head.u("start")
                        } else {
                            definition.map(|d| d.u("start")).unwrap_or(1).max(1)
                        };
                        let n = self.numbering.entry(nk.clone()).or_insert(start - 1);
                        *n += 1;
                        *n
                    };
                    let mut fmt = text(&head);
                    let format_at = |level: usize| {
                        definition
                            .and_then(|d| {
                                d.children_named("paraHead").find(|h| h.u("level") == level)
                            })
                            .map(|h| h.a("numFormat"))
                            .unwrap_or("")
                    };
                    let current=crate::number_format::format(num,head.a("numFormat")).unwrap_or_else(||{self.report.warnings.insert(format!("List format {} is unsupported or out of range at {num}; decimal fallback used",head.a("numFormat")));num.to_string()});
                    if fmt.is_empty() {
                        fmt = format!("{current}.");
                    } else {
                        let path = (1..=heading.u("level") + 1)
                            .map(|level| {
                                let value = if level == heading.u("level") + 1 {
                                    num
                                } else {
                                    *self.numbering.get(&(nk.0.clone(), level - 1)).unwrap_or(&1)
                                };
                                crate::number_format::format(value, format_at(level))
                                    .unwrap_or_else(|| value.to_string())
                            })
                            .collect::<Vec<_>>()
                            .join(".");
                        fmt = fmt.replace("^N", &format!("{path}.")).replace("^n", &path);
                        for level in (1..=10).rev() {
                            let v = if level == heading.u("level") + 1 {
                                num
                            } else {
                                *self.numbering.get(&(nk.0.clone(), level - 1)).unwrap_or(&1)
                            };
                            let formatted = crate::number_format::format(v, format_at(level))
                                .unwrap_or_else(|| v.to_string());
                            fmt = fmt.replace(&format!("^{level}"), &formatted);
                        }
                    }
                    if reflow {
                        self.numbering
                            .retain(|(id, level), _| id != &nk.0 || *level <= nk.1);
                    }
                    self.labels.insert(key, fmt.clone());
                    fmt
                };
                let fallback = p
                    .children_named("run")
                    .next()
                    .map(|n| n.a("charPrIDRef"))
                    .unwrap_or("0");
                let style = if self.doc.chars.contains_key(head.a("charPrIDRef")) {
                    head.a("charPrIDRef")
                } else {
                    fallback
                };
                let first = label.chars().next().unwrap_or('1');
                let legacy_symbol = bullet
                    && !self.doc.chars.contains_key(head.a("charPrIDRef"))
                    && ('\u{f020}'..='\u{f0ff}').contains(&first)
                    && self.fonts.root.join("wingding.ttf").exists();
                if legacy_symbol {
                    self.report.warnings.insert(
                        "Unassigned legacy symbol bullet font: using Wingdings cmap".into(),
                    );
                }
                let g =
                    self.glyph_with_font(first, style, legacy_symbol.then_some("wingding.ttf"))?;
                let image = self.list_image(p, &g)?;
                let nw = image
                    .as_ref()
                    .map(|i| i.1)
                    .unwrap_or_else(|| self.fonts.width(g.font, &label, g.size));
                let adjust = if reflow {
                    head.signed("widthAdjust").max(0.) / 100.
                } else {
                    0.
                };
                let offset = if head.a("textOffsetType") == "PERCENT" {
                    g.size * head.n("textOffset") / 100.
                } else {
                    head.n("textOffset") / 100.
                };
                let nw = nw * if image.is_some() { 1. } else { g.scale }
                    + if image.is_some() {
                        0.
                    } else {
                        generated_text::tracking(&label, &g)
                    };
                let reserved = if reflow {
                    nw + offset.max(0.) + adjust
                } else {
                    nw + offset
                };
                return Ok(Some((label, g, reserved)));
            }
        }
        Ok(None)
    }
}
