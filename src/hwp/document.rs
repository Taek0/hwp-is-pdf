//! HWP binary adapter: document.
use super::*;

pub fn open(path: &Path) -> Result<Document> {
    ensure!(
        std::fs::metadata(path)?.len() <= LIMIT as u64,
        "Input exceeds 64 MiB"
    );
    let mut cfb = cfb::open(path)?;
    fn read(cfb: &mut cfb::CompoundFile<std::fs::File>, path: &str) -> Result<Vec<u8>> {
        let mut b = Vec::new();
        cfb.open_stream(path)?
            .take(LIMIT as u64 + 1)
            .read_to_end(&mut b)?;
        ensure!(b.len() <= LIMIT, "Stream exceeds 64 MiB");
        Ok(b)
    }
    let fh = read(&mut cfb, "/FileHeader")?;
    ensure!(fh.starts_with(b"HWP Document File"), "Not HWP 5");
    ensure!(bytes(&fh, 35, 1)?[0] == 5, "Unsupported HWP version");
    let flags = u32(&fh, 36)?;
    ensure!(
        flags & 2 == 0,
        "Password-encrypted HWP requires a documented password decoder"
    );
    let distribution = flags & 4 != 0;
    let compressed = flags & 1 != 0;
    let di = read(&mut cfb, "/DocInfo")?;
    let di = if compressed { inflate(&di)? } else { di };
    let info = records(&di)?;
    let mut all = Vec::new();
    for r in &info {
        r.all(&mut all);
    }
    let mapping = all
        .iter()
        .find(|r| r.tag == 17)
        .context("Missing ID mapping")?;
    let faces: Vec<_> = all.iter().filter(|r| r.tag == 19).collect();
    let mut fonts = BTreeMap::new();
    let mut face_index = 0;
    for (lang_index, lang) in LANG.iter().enumerate() {
        let count = u32(&mapping.data, 4 + lang_index * 4)? as usize;
        for id in 0..count {
            let f = faces.get(face_index).context("Face index overflow")?;
            let name = utf16(bytes(&f.data, 3, 2 * u16(&f.data, 1)? as usize)?)?;
            fonts.insert((lang.to_uppercase(), id.to_string()), name);
            face_index += 1;
        }
    }
    let mut chars = BTreeMap::new();
    let mut paras = BTreeMap::new();
    let mut borders = BTreeMap::new();
    let mut header = n("head").at("binaryHwp", 1);
    for r in &all {
        let d = &r.data;
        match r.tag {
            22 => {
                let id = header.children_named("tabPr").count();
                header.children.push(tab_records::tabs(d, id)?);
            }
            23 => {
                let id = header.children_named("numbering").count() + 1;
                header.children.push(list_records::numbering(d, id)?);
            }
            24 => {
                let id = header.children_named("bullet").count() + 1;
                header.children.push(list_records::bullet(d, id)?);
            }
            26 => {
                let id = header.children_named("style").count();
                header.children.push(style_records::style(d, id)?);
            }
            21 => {
                let id = chars.len().to_string();
                let mut ch = n("charPr")
                    .at("id", &id)
                    .at("height", u32(d, 42)?)
                    .at("textColor", rgb(u32(d, 52)?));
                for (name, off, wide, signed) in [
                    ("fontRef", 0, true, false),
                    ("ratio", 14, false, false),
                    ("spacing", 21, false, true),
                    ("relSz", 28, false, false),
                    ("offset", 35, false, true),
                ] {
                    let mut child = n(name);
                    for (i, lang) in LANG.iter().enumerate() {
                        let value = if wide {
                            u16(d, off + i * 2)? as i32
                        } else if signed {
                            bytes(d, off + i, 1)?[0] as i8 as i32
                        } else {
                            bytes(d, off + i, 1)?[0] as i32
                        };
                        child = child.at(lang, value);
                    }
                    ch.children.push(child);
                }
                if d.len() >= 70 {
                    ch = ch.at("borderFillIDRef", u16(d, 68)?);
                }
                if u32(d, 60)? != 0xffff_ffff {
                    ch = ch.at("shadeColor", rgb(u32(d, 60)?));
                }
                if u32(d, 46)? & 2 != 0 {
                    ch.children.push(n("bold"));
                }
                let props = u32(d, 46)?;
                if props & (1 << 13) != 0 {
                    ch.children.push(n("emboss"));
                }
                if props & (1 << 14) != 0 {
                    ch.children.push(n("engrave"));
                }
                if matches!((props >> 11) & 3, 1 | 2) {
                    ch.children.push(
                        n("shadow")
                            .at(
                                "type",
                                if (props >> 11) & 3 == 1 {
                                    "DROP"
                                } else {
                                    "CONTINUOUS"
                                },
                            )
                            .at("color", rgb(u32(d, 64)?))
                            .at("offsetX", d[50] as i8)
                            .at("offsetY", d[51] as i8),
                    );
                }
                if let Some(kind) = [
                    "NONE",
                    "SOLID",
                    "DOT",
                    "THICK",
                    "DASH",
                    "DASH_DOT",
                    "DASH_DOT_DOT",
                ]
                .get(((props >> 8) & 7) as usize)
                .filter(|&&s| s != "NONE")
                {
                    ch.children.push(n("outline").at("type", kind));
                }
                // HWP table 35 uses CARON/TILDE order, unlike the OWPML enum.
                if let Some(name) = [
                    "NONE",
                    "DOT_ABOVE",
                    "RING_ABOVE",
                    "CARON",
                    "TILDE",
                    "SIDE",
                    "COLON",
                ]
                .get(((props >> 21) & 15) as usize)
                .filter(|&&name| name != "NONE")
                {
                    ch = ch.at("symMark", name);
                }
                ch = ch
                    .at("useFontSpace", (props >> 25) & 1)
                    .at("useKerning", (props >> 30) & 1);
                if props & 1 != 0 {
                    ch.children.push(n("italic"));
                }
                if props & (1 << 15) != 0 {
                    ch.children.push(n("supscript"));
                }
                if props & (1 << 16) != 0 {
                    ch.children.push(n("subscript"));
                }
                // Shape 15 is a frequent dormant/default strike value. Only
                // explicit supported line shapes can produce a strike stroke.
                let strike_style = (props >> 26) & 15;
                if (props >> 18) & 7 != 0 && strike_style <= 12 {
                    ch.children
                        .push(n("strikeout").at("style", strike_style).at(
                            "color",
                            rgb(if d.len() >= 74 {
                                u32(d, 70)?
                            } else {
                                u32(d, 52)?
                            }),
                        ));
                }
                // Only 1 (bottom) and 3 (top) denote visible underlines.
                // Reserved value 2 occurs in real documents with style 15;
                // treating every nonzero value as bottom underlines all text.
                if matches!((props >> 2) & 3, 1 | 3) {
                    ch.children.push(
                        n("underline")
                            .at(
                                "type",
                                if (props >> 2) & 3 == 3 {
                                    "TOP"
                                } else {
                                    "BOTTOM"
                                },
                            )
                            .at("style", (props >> 4) & 15)
                            .at("color", rgb(u32(d, 56)?)),
                    );
                }
                chars.insert(id, ch);
            }
            25 => {
                let id = paras.len().to_string();
                let align = [
                    "JUSTIFY",
                    "LEFT",
                    "RIGHT",
                    "CENTER",
                    "DISTRIBUTE",
                    "DISTRIBUTE",
                    "LEFT",
                    "LEFT",
                ][((u32(d, 0)? >> 2) & 7) as usize];
                paras.insert(
                    id.clone(),
                    n("paraPr")
                        .at("id", id)
                        .at("binaryHwp", 1)
                        .at("tabPrIDRef", u16(d, 28)?)
                        .with(vec![
                            n("align").at("horizontal", align),
                            n("margin").with(
                                [
                                    ("left", 4),
                                    ("right", 8),
                                    ("intent", 12),
                                    ("prev", 16),
                                    ("next", 20),
                                ]
                                .into_iter()
                                .map(|(key, off)| {
                                    Ok(n(key)
                                        .at("value", f64::from(i32(d, off)?) / 2.)
                                        .at("unit", "HWPUNIT"))
                                })
                                .collect::<Result<Vec<_>>>()?,
                            ),
                            n("lineSpacing")
                                .at(
                                    "type",
                                    match if d.len() >= 54 {
                                        u32(d, 46)? & 31
                                    } else {
                                        u32(d, 0)? & 3
                                    } {
                                        1 => "FIXED",
                                        2 => "BETWEEN_LINES",
                                        3 => "AT_LEAST",
                                        _ => "PERCENT",
                                    },
                                )
                                .at(
                                    "value",
                                    if d.len() >= 54 {
                                        u32(d, 50)?
                                    } else {
                                        u32(d, 24)?
                                    },
                                ),
                            n("breakSetting")
                                .at("widowOrphan", (u32(d, 0)? >> 16) & 1)
                                .at("keepWithNext", (u32(d, 0)? >> 17) & 1)
                                .at("keepLines", (u32(d, 0)? >> 18) & 1)
                                .at("pageBreakBefore", (u32(d, 0)? >> 19) & 1)
                                .at(
                                    "breakLatinWord",
                                    if (u32(d, 0)? >> 5) & 3 == 2 {
                                        "BREAK_WORD"
                                    } else {
                                        "KEEP_WORD"
                                    },
                                )
                                .at(
                                    "breakNonLatinWord",
                                    if u32(d, 0)? & 128 != 0 {
                                        "KEEP_WORD"
                                    } else {
                                        "BREAK_WORD"
                                    },
                                ),
                            n("heading")
                                .at(
                                    "type",
                                    ["NONE", "OUTLINE", "NUMBER", "BULLET"]
                                        [((u32(d, 0)? >> 23) & 3) as usize],
                                )
                                .at("level", (u32(d, 0)? >> 25) & 7)
                                .at("idRef", u16(d, 30)?),
                            n("border")
                                .at("borderFillIDRef", u16(d, 32)?)
                                .at("connect", (u32(d, 0)? >> 28) & 1)
                                .at("ignoreMargin", (u32(d, 0)? >> 29) & 1)
                                .at("offsetLeft", u16(d, 34)? as i16)
                                .at("offsetRight", u16(d, 36)? as i16)
                                .at("offsetTop", u16(d, 38)? as i16)
                                .at("offsetBottom", u16(d, 40)? as i16),
                        ]),
                );
            }
            20 => {
                let id = (borders.len() + 1).to_string();
                let mut b = n("borderFill").at("id", &id);
                let widths = [
                    0.1, 0.12, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 1., 1.5, 2., 3., 4., 5.,
                ];
                for (i, key) in ["leftBorder", "rightBorder", "topBorder", "bottomBorder"]
                    .iter()
                    .enumerate()
                {
                    let typ = bytes(d, 2 + 6 * i, 1)?[0];
                    let weight = bytes(d, 3 + 6 * i, 1)?[0] as usize;
                    b.children.push(
                        n(key)
                            .at(
                                "type",
                                [
                                    "NONE",
                                    "SOLID",
                                    "DASH",
                                    "DOT",
                                    "DASH_DOT",
                                    "DASH_DOT_DOT",
                                    "LONG_DASH",
                                    "CIRCLE",
                                    "DOUBLE_SLIM",
                                    "SLIM_THICK",
                                    "THICK_SLIM",
                                    "SLIM_THICK_SLIM",
                                    "WAVE",
                                    "DOUBLEWAVE",
                                    "THICK3D",
                                    "THICKREV3D",
                                    "3D",
                                    "REV3D",
                                ]
                                .get(typ as usize)
                                .copied()
                                .unwrap_or("SOLID"),
                            )
                            .at("width", format!("{} mm", widths[weight.min(15)]))
                            .at("color", rgb(u32(d, 4 + 6 * i)?)),
                    );
                }
                if u32(d, 32)? & 1 != 0 {
                    let v = u32(d, 36)?;
                    if v != u32::MAX {
                        let mut brush = n("winBrush").at("faceColor", rgb(v));
                        if d.len() >= 48 {
                            brush = brush.at("hatchColor", rgb(u32(d, 40)?)).at(
                                "hatchStyle",
                                [
                                    "NONE",
                                    "HORIZONTAL",
                                    "VERTICAL",
                                    "BACK_SLASH",
                                    "SLASH",
                                    "CROSS",
                                    "CROSS_DIAGONAL",
                                ]
                                .get(u32(d, 44)? as usize)
                                .copied()
                                .unwrap_or("NONE"),
                            );
                        }
                        b.children.push(n("fillBrush").with(vec![brush]));
                    }
                }
                if let Some(gradient) = super::fill_records::gradient(d, 32)? {
                    if let Some(fill) = b.children.iter_mut().find(|c| c.name == "fillBrush") {
                        fill.children.push(gradient);
                    } else {
                        b.children.push(n("fillBrush").with(vec![gradient]));
                    }
                }
                if let Some(image) = super::fill_records::image(d, 32)? {
                    if let Some(fill) = b.children.iter_mut().find(|c| c.name == "fillBrush") {
                        fill.children.push(image);
                    } else {
                        b.children.push(n("fillBrush").with(vec![image]));
                    }
                }
                borders.insert(id, b);
            }
            _ => {}
        }
    }
    let names: Vec<String> = cfb
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    let mut binaries = BTreeMap::new();
    let mut pcx_ids = BTreeSet::new();
    for name in &names {
        if name.starts_with("/BinData/") {
            let id = u16::from_str_radix(name.get(12..16).context("Invalid BinData name")?, 16)?;
            let raw = read(&mut cfb, name)?;
            // Drawing references are one-based BinData record indices, not
            // the storage IDs embedded in those records (BINxxxx streams).
            let entries: Vec<_> = all
                .iter()
                .filter(|r| r.tag == 18)
                .enumerate()
                .filter(|(_, r)| {
                    u16(&r.data, 0).is_ok_and(|v| v & 0xf != 0) && u16(&r.data, 2).ok() == Some(id)
                })
                .collect();
            let mode = entries
                .first()
                .map(|(_, r)| u16(&r.data, 0).map(|v| (v >> 4) & 3))
                .transpose()?
                .unwrap_or(0);
            let raw = if mode == 1 || (mode == 0 && compressed) {
                inflate(&raw)?
            } else {
                raw
            };
            let raw = if name.to_lowercase().ends_with(".pcx") {
                for (index, _) in &entries {
                    pcx_ids.insert(format!("image{}", index + 1));
                }
                decode_pcx(&raw)?
            } else {
                raw
            };
            for (index, _) in entries {
                binaries.insert(format!("image{}", index + 1), raw.clone());
            }
        }
    }
    let section_prefix = if distribution {
        "/ViewText/Section"
    } else {
        "/BodyText/Section"
    };
    let mut section_names: Vec<_> = names
        .iter()
        .filter(|n| n.starts_with(section_prefix))
        .collect();
    section_names.sort_by_key(|s| {
        s.trim_start_matches(section_prefix)
            .parse::<usize>()
            .unwrap_or(0)
    });
    let mut sections = Vec::new();
    let mut masters = Vec::new();
    for name in section_names {
        let raw = read(&mut cfb, name)?;
        let raw = if distribution {
            crate::distribution::decode(&raw)?
        } else {
            raw
        };
        let raw = if compressed { inflate(&raw)? } else { raw };
        let roots = records(&raw)?;
        let mut section = n("sec").at("binaryHwp", 1);
        for r in roots.iter().filter(|r| r.tag == 66) {
            section.children.push(paragraph(r)?);
            for sec in r
                .children
                .iter()
                .filter(|c| c.tag == 71 && c.id() == "secd")
            {
                let gs = groups(&sec.children);
                let count = gs.len();
                for (i, (h, ps)) in gs.iter().enumerate() {
                    masters.push(
                        n("masterPage")
                            .at("binaryHwp", 1)
                            .at(
                                "type",
                                if count == 2 {
                                    if i == 0 { "EVEN" } else { "ODD" }
                                } else {
                                    "BOTH"
                                },
                            )
                            .with(vec![list(h, ps)?]),
                    );
                }
            }
        }
        sections.push(section);
    }
    ensure!(!sections.is_empty(), "Missing BodyText");
    super::images::mark_pcx(&mut sections, &pcx_ids);
    super::images::mark_pcx(&mut masters, &pcx_ids);
    crate::metafile::prepare(&mut binaries)?;
    crate::image_effects::apply(&mut sections, &mut binaries)?;
    crate::image_effects::apply(&mut masters, &mut binaries)?;
    crate::image_effects::apply(std::slice::from_mut(&mut header), &mut binaries)?;
    for border in borders.values_mut() {
        crate::image_effects::apply(std::slice::from_mut(border), &mut binaries)?;
    }
    super::ole_objects::normalize(&mut sections, &mut binaries)?;
    super::ole_objects::normalize(&mut masters, &mut binaries)?;
    Ok(Document {
        header,
        sections,
        masters,
        binaries,
        chars,
        paras,
        borders,
        fonts,
    })
}
