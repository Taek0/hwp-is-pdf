//! HWP binary adapter: controls.
use super::*;

pub(super) fn control(r: &Record) -> Result<Node> {
    let d = &r.data;
    match r.id().as_str() {
        "fn  " | "en  " => {
            let mut node = n(if r.id() == "fn  " {
                "footNote"
            } else {
                "endNote"
            });
            node = node.at("id", format!("hwp-{:x}", r.offset));
            for (h, ps) in groups(&r.children) {
                node.children.push(list(h, &ps)?);
            }
            let number = node
                .find("autoNum")
                .map(|a| a.a("num").to_string())
                .unwrap_or_else(|| "1".into());
            node = node.at("number", number);
            Ok(n("ctrl").with(vec![node]))
        }
        "tdut" => {
            let main_len = u16(d, 4)? as usize;
            let main = utf16(bytes(d, 6, main_len * 2)?)?;
            let off = 6 + main_len * 2;
            let sub_len = u16(d, off)? as usize;
            let sub = utf16(bytes(d, off + 2, sub_len * 2)?)?;
            let off = off + 2 + sub_len * 2;
            Ok(n("ctrl").with(vec![
                n("ruby")
                    .at("main", main)
                    .at("sub", sub)
                    .at("position", u32(d, off)?)
                    .at("ratio", u32(d, off + 4)?),
            ]))
        }
        "nwno" => Ok(n("ctrl").with(vec![
            n("newNumber")
                .at("type", u32(d, 4)? & 15)
                .at("number", u16(d, 8)?),
        ])),
        "pgct" => Ok(n("ctrl").with(vec![n("pageNumCtrl").at(
            "pageStartsOn",
            match u32(d, 4)? & 3 {
                1 => "EVEN",
                2 => "ODD",
                _ => "BOTH",
            },
        )])),
        "pghd" => {
            let f = u16(d, 4)?;
            let mut h = n("pageHiding");
            for (i, key) in [
                "hideHeader",
                "hideFooter",
                "hideMasterPage",
                "hideBorder",
                "hideFill",
                "hidePageNum",
            ]
            .iter()
            .enumerate()
            {
                h = h.at(key, (f >> i) & 1);
            }
            Ok(n("ctrl").with(vec![h]))
        }
        "idxm" | "tcmt" => Ok(n("ctrl").with(vec![n("nonPrinting").at("sourceControl", r.id())])),
        "eqed" => {
            let eq = r.first(88).context("Missing equation payload")?;
            let ed = &eq.data;
            let len = u16(ed, 4)? as usize;
            let end = 6 + 2 * len;
            let mut object = common(r, "equation")?
                .at("baseUnit", u32(ed, end)?)
                .at("textColor", rgb(u32(ed, end + 4)?))
                .at("baseLine", u16(ed, end + 8)?);
            object
                .children
                .push(n("script").with(vec![txt(utf16(bytes(ed, 6, len * 2)?)?)]));
            Ok(object)
        }
        "head" | "foot" => {
            let name = if r.id() == "head" { "header" } else { "footer" };
            let mut node = n(name).at(
                "applyPageType",
                ["BOTH", "EVEN", "ODD", "BOTH"][(u32(d, 4)? & 3) as usize],
            );
            for (h, ps) in groups(&r.children) {
                node.children.push(list(h, &ps)?);
            }
            Ok(n("ctrl").with(vec![node]))
        }
        "tcps" => {
            let len = u16(d, 4)? as usize;
            let off = 6 + 2 * len;
            let mut overlay = n("charOverlay")
                .at("text", utf16(bytes(d, 6, len * 2)?)?)
                .at("borderType", bytes(d, off, 1)?[0])
                .at("innerSize", bytes(d, off + 1, 1)?[0] as i8)
                .at("spread", bytes(d, off + 2, 1)?[0]);
            let count = bytes(d, off + 3, 1)?[0] as usize;
            for i in 0..count {
                overlay
                    .children
                    .push(n("charRef").at("id", u32(d, off + 4 + i * 4)?));
            }
            Ok(n("ctrl").with(vec![overlay]))
        }
        "secd" => section_properties(r),
        "cold" => columns::columns(d),
        "pgnp" => Ok(n("ctrl").with(vec![
            n("pageNumPos")
                .at("position", (u32(d, 4)? >> 8) & 15)
                .at("format", u32(d, 4)? & 255)
                .at(
                    "sideChar",
                    if d.len() >= 16 {
                        char::from_u32(u16(d, 14)? as u32)
                            .filter(|c| *c != '\0')
                            .map(|c| c.to_string())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    },
                )
                .at(
                    "prefix",
                    char::from_u32(u16(d, 10)? as u32)
                        .filter(|c| *c != '\0')
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                )
                .at(
                    "suffix",
                    char::from_u32(u16(d, 12)? as u32)
                        .filter(|c| *c != '\0')
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                ),
        ])),
        "atno" => Ok(n("ctrl").with(vec![
            n("autoNum")
                .at(
                    "numType",
                    match u32(d, 4)? & 15 {
                        0 => "PAGE",
                        1 => "FOOTNOTE",
                        2 => "ENDNOTE",
                        _ => "OTHER",
                    },
                )
                .at("num", u16(d, 8)?)
                .with(if d.len() >= 16 {
                    vec![
                        n("autoNumFormat")
                            .at(
                                "prefixChar",
                                char::from_u32(u16(d, 12)? as u32)
                                    .filter(|c| *c != '\0')
                                    .map(|c| c.to_string())
                                    .unwrap_or_default(),
                            )
                            .at(
                                "suffixChar",
                                char::from_u32(u16(d, 14)? as u32)
                                    .filter(|c| *c != '\0')
                                    .map(|c| c.to_string())
                                    .unwrap_or_default(),
                            ),
                    ]
                } else {
                    vec![]
                }),
        ])),
        "tbl " => {
            let mut out = common(r, "tbl")?;
            let table = r.first(77).context("Missing table record")?;
            let td = &table.data;
            let rows = u16(td, 4)? as usize;
            out = out
                .at("rowCnt", rows)
                .at("colCnt", u16(td, 6)?)
                .at(
                    "pageBreak",
                    match u32(td, 0)? & 3 {
                        0 => "NONE",
                        1 => "CELL",
                        _ => "TABLE",
                    },
                )
                .at("repeatHeader", (u32(td, 0)? >> 2) & 1)
                .at("borderFillIDRef", u16(td, 18 + 2 * rows)?);
            out.children.push(margins("inMargin", td, 10)?);
            let mut tr: Vec<Node> = (0..rows).map(|_| n("tr")).collect();
            for (hdr, ps) in groups(&r.children)
                .into_iter()
                .filter(|(h, _)| h.offset > table.offset)
            {
                let d = &hdr.data;
                let row = u16(d, 10)? as usize;
                ensure!(row < rows, "Invalid table row");
                let mut cell_margin = margins("cellMargin", d, 24)?;
                for (i, key) in ["left", "right", "top", "bottom"].iter().enumerate() {
                    if u16(d, 24 + 2 * i)? == u16::MAX {
                        cell_margin = cell_margin.at(key, u16(td, 10 + 2 * i)?);
                    }
                }
                let cell = n("tc")
                    .at("hasMargin", (u32(d, 4)? >> 16) & 1)
                    .at("header", (u32(d, 4)? >> 18) & 1)
                    .at("borderFillIDRef", u16(d, 32)?)
                    .with(vec![
                        list(hdr, &ps)?,
                        n("cellAddr").at("colAddr", u16(d, 8)?).at("rowAddr", row),
                        n("cellSpan")
                            .at("colSpan", u16(d, 12)?)
                            .at("rowSpan", u16(d, 14)?),
                        n("cellSz")
                            .at("width", u32(d, 16)?)
                            .at("height", u32(d, 20)?),
                        cell_margin,
                    ]);
                tr[row].children.push(cell);
            }
            out.children.extend(tr);
            Ok(out)
        }
        "gso " => {
            let comp = r.first(76).context("Missing shape component")?;
            let cd = &comp.data;
            let kind = comp.id();
            if kind == "$con" {
                let mut group = common(r, "group")?;
                group.children.push(
                    n("curSz")
                        .at("width", u32(cd, 28)?)
                        .at("height", u32(cd, 32)?),
                );
                group.children.push(
                    n("rotationInfo")
                        .at("angle", u16(cd, 40)?)
                        .at("centerX", i32(cd, 42)?)
                        .at("centerY", i32(cd, 46)?),
                );
                group.children.push(
                    n("flip")
                        .at("horizontal", u32(cd, 36)? & 1)
                        .at("vertical", (u32(cd, 36)? >> 1) & 1),
                );
                let ow = u32(cd, 20)?.max(1) as f64;
                let oh = u32(cd, 24)?.max(1) as f64;
                let sx = u32(cd, 28)? as f64 / ow;
                let sy = u32(cd, 32)? as f64 / oh;
                for child in comp.children.iter().filter(|c| c.tag == 76) {
                    let mut child = child.clone();
                    if child.data.get(..4) != child.data.get(4..8) {
                        let mut b = bytes(&child.data, 0, 4)?.to_vec();
                        b.extend_from_slice(&child.data);
                        child.data = b;
                    }
                    let mut d = vec![0u8; 46];
                    d[..4].copy_from_slice(b" osg");
                    d[4..8].copy_from_slice(&0x310u32.to_le_bytes());
                    for (offset, value) in [
                        (8, (i32(&child.data, 12)? as f64 * sy) as i32),
                        (12, (i32(&child.data, 8)? as f64 * sx) as i32),
                        (16, (u32(&child.data, 28)? as f64 * sx) as i32),
                        (20, (u32(&child.data, 32)? as f64 * sy) as i32),
                    ] {
                        d[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                    }
                    let fake = Record {
                        tag: 71,
                        level: r.level + 1,
                        data: d,
                        offset: child.offset,
                        children: vec![child],
                    };
                    group.children.push(control(&fake)?);
                }
                return Ok(group);
            }
            let name = match kind.as_str() {
                "$pic" => "pic",
                "$rec" => "rect",
                "$lin" => "line",
                "$ell" => "ellipse",
                "$pol" => "polygon",
                "$arc" => "arc",
                "$cur" => "curve",
                "$ole" => "ole",
                _ => bail!("Unsupported HWP shape {kind}"),
            };
            let mut out = common(r, name)?;
            if name == "ole" {
                let ole = comp.first(84).context("Missing OLE record")?;
                out.children.push(
                    n("oleRef").at("binaryItemIDRef", format!("image{}", u16(&ole.data, 12)?)),
                );
                return Ok(out);
            }
            out = out
                .at("originalWidth", u32(cd, 20)?)
                .at("originalHeight", u32(cd, 24)?);
            out.children.push(
                n("curSz")
                    .at(
                        "width",
                        if name == "line" {
                            i32(cd, 28)?.unsigned_abs()
                        } else {
                            u32(cd, 28)?
                        },
                    )
                    .at(
                        "height",
                        if name == "line" {
                            i32(cd, 32)?.unsigned_abs()
                        } else {
                            u32(cd, 32)?
                        },
                    ),
            );
            out.children.push(
                n("rotationInfo")
                    .at("angle", u16(cd, 40)?)
                    .at("centerX", i32(cd, 42)?)
                    .at("centerY", i32(cd, 46)?),
            );
            out.children.push(
                n("flip")
                    .at(
                        "horizontal",
                        (u32(cd, 36)? & 1) ^ u32::from(name == "line" && i32(cd, 28)? < 0),
                    )
                    .at(
                        "vertical",
                        ((u32(cd, 36)? >> 1) & 1) ^ u32::from(name == "line" && i32(cd, 32)? < 0),
                    ),
            );
            if matches!(name, "polygon" | "curve") {
                let shape = comp
                    .first(if name == "polygon" { 82 } else { 83 })
                    .context("Missing path shape record")?;
                let sd = &shape.data;
                // Public spec describes INT16; actual modern records use a 32-bit count.
                let count = u32(sd, 0)? as usize;
                ensure!(count <= 100000, "Excessive path points");
                for i in 0..count {
                    out.children.push(
                        n("point")
                            .at("x", i32(sd, 4 + i * 8)?)
                            .at("y", i32(sd, 8 + i * 8)?),
                    );
                }
                if name == "curve" {
                    for &kind in bytes(sd, 4 + count * 8, count.saturating_sub(1))? {
                        ensure!(kind <= 1, "Unknown curve segment type {kind}");
                        out.children.push(n("segment").at("type", kind));
                    }
                }
            }
            if name == "arc" {
                let arc = comp.first(81).context("Missing arc record")?;
                let sd = &arc.data;
                out = out.at(
                    "type",
                    match bytes(sd, 0, 1)?[0] {
                        1 => "PIE",
                        2 => "CHORD",
                        _ => "NORMAL",
                    },
                );
                let start = if sd.len() == 25 { 1 } else { 4 };
                for (i, label) in ["center", "axis1", "axis2"].iter().enumerate() {
                    out.children.push(
                        n(label)
                            .at("x", i32(sd, start + i * 8)?)
                            .at("y", i32(sd, start + 4 + i * 8)?),
                    );
                }
            }
            if name == "rect"
                && let Some(rect) = comp.first(79)
            {
                out = out.at("cornerRatio", bytes(&rect.data, 0, 1)?[0]);
            }
            if name == "pic" {
                let pic = comp.first(85).context("Missing picture record")?;
                let pd = &pic.data;
                out.children.push(
                    n("imgClip")
                        .at("left", i32(pd, 44)?)
                        .at("top", i32(pd, 48)?)
                        .at("right", i32(pd, 52)?)
                        .at("bottom", i32(pd, 56)?),
                );
                if pd.len() >= 90 && u32(pd, 78)? == 0 {
                    out.children.push(
                        n("imgDim")
                            .at("dimwidth", u32(pd, 82)?)
                            .at("dimheight", u32(pd, 86)?),
                    );
                }
                out.children.push(
                    n("img")
                        .at("binaryItemIDRef", format!("image{}", u16(pd, 71)?))
                        .at("contrast", bytes(pd, 68, 1)?[0] as i8)
                        .at("bright", bytes(pd, 69, 1)?[0] as i8)
                        .at("effect", bytes(pd, 70, 1)?[0]),
                );
            } else {
                let count = u16(cd, 50)? as usize;
                ensure!(count < 128, "Excessive transform matrices");
                let off = 100 + 96 * count;
                let attrs = u32(cd, off + 8)?;
                out.children.push(
                    n("lineShape")
                        .at("color", rgb(u32(cd, off)?))
                        .at("width", i32(cd, off + 4)?)
                        .at(
                            "style",
                            match attrs & 63 {
                                0 => "NONE",
                                1 => "SOLID",
                                2 => "DASH",
                                3 => "DOT",
                                4 => "DASH_DOT",
                                5 => "DASH_DOT_DOT",
                                6 => "LONG_DASH",
                                _ => "SOLID",
                            },
                        ),
                );
                if cd.len() >= off + 21 && u32(cd, off + 13)? & 1 != 0 {
                    let v = u32(cd, off + 17)?;
                    if v != u32::MAX {
                        let mut brush = n("winBrush").at("faceColor", rgb(v));
                        if cd.len() >= off + 29 {
                            brush = brush.at("hatchColor", rgb(u32(cd, off + 21)?)).at(
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
                                .get(u32(cd, off + 25)? as usize)
                                .copied()
                                .unwrap_or("NONE"),
                            );
                        }
                        out.children.push(n("fillBrush").with(vec![brush]));
                    }
                }
                if cd.len() >= off + 17
                    && let Some(gradient) = super::fill_records::gradient(cd, off + 13)?
                {
                    if let Some(fill) = out.children.iter_mut().find(|c| c.name == "fillBrush") {
                        fill.children.push(gradient);
                    } else {
                        out.children.push(n("fillBrush").with(vec![gradient]));
                    }
                }
                if cd.len() >= off + 17
                    && let Some(image) = super::fill_records::image(cd, off + 13)?
                {
                    if let Some(fill) = out.children.iter_mut().find(|c| c.name == "fillBrush") {
                        fill.children.push(image);
                    } else {
                        out.children.push(n("fillBrush").with(vec![image]));
                    }
                }
                if let Some((hdr, ps)) = groups(&comp.children).first() {
                    out.children.push(
                        n("drawText")
                            .with(vec![list(hdr, ps)?, margins("textMargin", &hdr.data, 8)?]),
                    );
                }
            }
            Ok(out)
        }
        "bokm" => field_records::bookmark(r),
        field if field.starts_with('%') => field_records::field(r),
        other => bail!("Unsupported HWP control {other}"),
    }
}
