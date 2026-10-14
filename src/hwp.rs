//! HWP 5 binary adapter: read-only CFB -> bounded raw DEFLATE -> records -> common layout nodes.
//! No external converter, Hancom module, or script interpreter is used.
use crate::model::{Document, Node};
use anyhow::{Context, Result, bail, ensure};
use flate2::{Decompress, FlushDecompress, Status};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
    path::Path,
};
const LIMIT: usize = 64 * 1024 * 1024;
const LANG: [&str; 7] = [
    "hangul", "latin", "hanja", "japanese", "other", "symbol", "user",
];
fn n(name: &str) -> Node {
    Node {
        name: name.into(),
        ..Default::default()
    }
}
trait Build {
    fn at(self, k: &str, v: impl ToString) -> Self;
    fn with(self, c: Vec<Node>) -> Self;
}
impl Build for Node {
    fn at(mut self, k: &str, v: impl ToString) -> Self {
        self.attrs.insert(k.into(), v.to_string());
        self
    }
    fn with(mut self, c: Vec<Node>) -> Self {
        self.children = c;
        self
    }
}
fn txt(s: impl Into<String>) -> Node {
    Node {
        name: "#text".into(),
        text: s.into(),
        ..Default::default()
    }
}
fn bytes(d: &[u8], o: usize, len: usize) -> Result<&[u8]> {
    d.get(o..o.checked_add(len).context("Field overflow")?)
        .context("Truncated HWP field")
}
fn u16(d: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes(d, o, 2)?.try_into()?))
}
fn u32(d: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes(d, o, 4)?.try_into()?))
}
fn i32(d: &[u8], o: usize) -> Result<i32> {
    Ok(u32(d, o)? as i32)
}
fn utf16(d: &[u8]) -> Result<String> {
    ensure!(d.len().is_multiple_of(2), "Odd UTF-16 length");
    Ok(String::from_utf16_lossy(
        &d.as_chunks::<2>()
            .0
            .iter()
            .map(|a| u16::from_le_bytes([a[0], a[1]]))
            .collect::<Vec<_>>(),
    ))
}
fn rgb(v: u32) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        v & 255,
        (v >> 8) & 255,
        (v >> 16) & 255
    )
}
fn margins(name: &str, d: &[u8], o: usize) -> Result<Node> {
    let mut r = n(name);
    for (i, k) in ["left", "right", "top", "bottom"].iter().enumerate() {
        r = r.at(k, u16(d, o + 2 * i)?);
    }
    Ok(r)
}
pub(crate) fn inflate(b: &[u8]) -> Result<Vec<u8>> {
    let mut dec = Decompress::new(false);
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        let mut buf = [0u8; 16384];
        let before_in = dec.total_in();
        let before_out = dec.total_out();
        let status = dec.decompress(&b[pos..], &mut buf, FlushDecompress::None)?;
        let used = (dec.total_in() - before_in) as usize;
        let produced = (dec.total_out() - before_out) as usize;
        pos += used;
        ensure!(out.len() + produced <= LIMIT, "Inflated stream too large");
        out.extend_from_slice(&buf[..produced]);
        if status == Status::StreamEnd {
            let tail = &b[pos..];
            if tail.len() == 32
                && tail[4..16].iter().all(|b| *b == 0)
                && tail[20..].iter().all(|b| *b == 0)
            {
                ensure!(
                    (u32(tail, 0)? == 0 || u32(tail, 0)? == crc32fast::hash(&out))
                        && u32(tail, 16)? as usize == out.len(),
                    "Invalid block-aligned HWP trailer"
                );
            } else if tail.len() >= 8 && !tail.iter().all(|b| *b == 0) {
                ensure!(
                    (u32(tail, 0)? == 0 || u32(tail, 0)? == crc32fast::hash(&out))
                        && u32(tail, 4)? as usize == out.len()
                        && tail[8..].iter().all(|b| *b == 0),
                    "Invalid HWP CRC/size trailer"
                );
            } else {
                ensure!(tail.iter().all(|b| *b == 0), "Unexpected DEFLATE trailer");
            }
            return Ok(out);
        }
        ensure!(used > 0 || produced > 0, "Truncated DEFLATE stream");
    }
}
#[derive(Clone, Debug)]
struct Record {
    tag: u16,
    level: u16,
    data: Vec<u8>,
    offset: usize,
    children: Vec<Record>,
}
impl Record {
    fn first(&self, t: u16) -> Option<&Record> {
        self.children.iter().find(|r| r.tag == t)
    }
    fn id(&self) -> String {
        self.data.iter().take(4).rev().map(|b| *b as char).collect()
    }
    fn all<'a>(&'a self, out: &mut Vec<&'a Record>) {
        out.push(self);
        for c in &self.children {
            c.all(out)
        }
    }
}
fn records(data: &[u8]) -> Result<Vec<Record>> {
    let mut flat = Vec::new();
    let mut p = 0;
    while p < data.len() {
        let offset = p;
        let v = u32(data, p)?;
        p += 4;
        let tag = (v & 1023) as u16;
        let level = ((v >> 10) & 1023) as u16;
        ensure!(level <= 64, "Excessive nesting");
        let mut len = (v >> 20) as usize;
        if len == 4095 {
            len = u32(data, p)? as usize;
            p += 4;
        }
        let d = bytes(data, p, len)?.to_vec();
        p += len;
        flat.push(Record {
            tag,
            level,
            data: d,
            offset,
            children: Vec::new(),
        });
        ensure!(flat.len() <= 1_000_000, "Excessive records");
    }
    fn tree(flat: &[Record], p: &mut usize, level: u16) -> Result<Vec<Record>> {
        let mut out = Vec::new();
        while *p < flat.len() && flat[*p].level == level {
            let mut r = flat[*p].clone();
            *p += 1;
            if *p < flat.len() && flat[*p].level > level {
                ensure!(flat[*p].level == level + 1, "Record level jump");
                r.children = tree(flat, p, level + 1)?;
            }
            out.push(r);
        }
        Ok(out)
    }
    if flat.is_empty() {
        return Ok(vec![]);
    }
    let mut p = 0;
    let result = tree(&flat, &mut p, flat[0].level)?;
    ensure!(p == flat.len(), "Inconsistent record tree");
    Ok(result)
}
fn groups(rs: &[Record]) -> Vec<(&Record, Vec<&Record>)> {
    let mut out: Vec<(&Record, Vec<&Record>)> = Vec::new();
    for r in rs {
        if r.tag == 72 {
            out.push((r, vec![]))
        } else if r.tag == 66
            && let Some(g) = out.last_mut()
        {
            g.1.push(r)
        }
    }
    out
}
fn list(header: &Record, paras: &[&Record]) -> Result<Node> {
    let flags = u32(&header.data, 4)?;
    let mut out = n("subList").at(
        "vertAlign",
        ["TOP", "CENTER", "BOTTOM", "TOP"][((flags >> 5) & 3) as usize],
    );
    for p in paras {
        out.children.push(paragraph(p)?);
    }
    Ok(out)
}

fn section_properties(r: &Record) -> Result<Node> {
    let page = r.first(73).context("Missing HWP page definition")?;
    let d = &page.data;
    let mut margin = n("margin");
    for (i, key) in [
        "left", "right", "top", "bottom", "header", "footer", "gutter",
    ]
    .iter()
    .enumerate()
    {
        margin = margin.at(key, u32(d, 8 + 4 * i)?);
    }
    // HWP 5 table 132 stores nominal paper dimensions separately from orientation.
    let (width, height) = (u32(d, 0)?, u32(d, 4)?);
    let landscape = u32(d, 36)? & 1 != 0;
    let page = n("pagePr")
        .at("width", if landscape { height } else { width })
        .at("height", if landscape { width } else { height })
        .with(vec![margin]);
    let mut sec = n("secPr")
        .at("binaryHwp", 1)
        .at(
            "hideFirstHeader",
            if u32(&r.data, 4)? & 1 != 0 { 1 } else { 0 },
        )
        .at(
            "hideFirstFooter",
            if u32(&r.data, 4)? & 2 != 0 { 1 } else { 0 },
        )
        .at(
            "hideFirstMasterPage",
            if u32(&r.data, 4)? & 4 != 0 { 1 } else { 0 },
        )
        .with(vec![page]);
    for (index, r) in r
        .children
        .iter()
        .filter(|c| c.tag == 74)
        .take(2)
        .enumerate()
    {
        let d = &r.data;
        let extra = usize::from(d.len() >= 28) * 2;
        let flags = u32(d, 0)?;
        let char_at = |off| -> Result<String> {
            Ok(char::from_u32(u16(d, off)? as u32)
                .filter(|c| *c != '\0')
                .map(|c| c.to_string())
                .unwrap_or_default())
        };
        sec.children.push(
            n(if index == 0 {
                "footNotePr"
            } else {
                "endNotePr"
            })
            .with(vec![
                n("autoNumFormat")
                    .at("typeCode", flags & 255)
                    .at("prefixChar", char_at(6)?)
                    .at("suffixChar", char_at(8)?),
                n("noteLine")
                    .at("length", i16::from_le_bytes(bytes(d, 12, 2)?.try_into()?))
                    .at(
                        "type",
                        if bytes(d, 20 + extra, 1)?[0] == 0 {
                            "NONE"
                        } else {
                            "SOLID"
                        },
                    )
                    .at("widthIndex", bytes(d, 21 + extra, 1)?[0])
                    .at("color", rgb(u32(d, 22 + extra)?)),
                n("noteSpacing")
                    .at("aboveLine", u16(d, 14 + extra)?)
                    .at("belowLine", u16(d, 16 + extra)?)
                    .at("betweenNotes", u16(d, 18 + extra)?),
                n("placement")
                    .at(
                        "place",
                        if index == 0 {
                            [
                                "EACH_COLUMN",
                                "MERGED_COLUMN",
                                "RIGHT_MOST_COLUMN",
                                "EACH_COLUMN",
                            ][((flags >> 8) & 3) as usize]
                        } else if flags & 256 != 0 {
                            "END_OF_SECTION"
                        } else {
                            "END_OF_DOCUMENT"
                        },
                    )
                    .at("beneathText", (flags >> 13) & 1),
            ]),
        );
    }
    Ok(sec)
}
fn common(r: &Record, name: &str) -> Result<Node> {
    let d = &r.data;
    let flags = u32(d, 4)?;
    let vr = ["PAPER", "PAGE", "PARA", "PARA"][((flags >> 3) & 3) as usize];
    let hr = ["PAPER", "PAGE", "COLUMN", "PARA"][((flags >> 8) & 3) as usize];
    let aligns = [
        "LEFT", "CENTER", "RIGHT", "LEFT", "RIGHT", "LEFT", "LEFT", "LEFT",
    ];
    let pos = n("pos")
        .at("treatAsChar", flags & 1)
        .at("vertRelTo", vr)
        .at("horzRelTo", hr)
        .at("horzAlign", aligns[((flags >> 10) & 7) as usize])
        .at(
            "vertAlign",
            [
                "TOP", "CENTER", "BOTTOM", "TOP", "BOTTOM", "TOP", "TOP", "TOP",
            ][((flags >> 5) & 7) as usize],
        )
        .at("vertOffset", i32(d, 8)?)
        .at("horzOffset", i32(d, 12)?);
    let mut object = n(name)
        .at("id", format!("hwp-{:x}", r.offset))
        .at(
            "textWrap",
            match (flags >> 21) & 7 {
                1 => "TOP_AND_BOTTOM",
                2 => "BEHIND_TEXT",
                3 => "IN_FRONT_OF_TEXT",
                _ => "SQUARE",
            },
        )
        .at("binaryHwp", 1)
        .with(vec![
            n("sz").at("width", u32(d, 16)?).at("height", u32(d, 20)?),
            pos,
            margins("outMargin", d, 28)?,
        ]);
    let marker = r
        .children
        .iter()
        .find(|c| matches!(c.tag, 76 | 77))
        .map(|c| c.offset)
        .unwrap_or(usize::MAX);
    for (header, paras) in groups(&r.children)
        .into_iter()
        .filter(|(h, _)| h.offset < marker)
    {
        let d = &header.data;
        object.children.push(
            n("caption")
                .at(
                    "side",
                    ["LEFT", "RIGHT", "TOP", "BOTTOM"][(u32(d, 8)? & 3) as usize],
                )
                .at("width", u32(d, 12)?)
                .at("gap", u16(d, 16)?)
                .at("fullWidth", u32(d, 18)?)
                .at("expandToMargin", (u32(d, 8)? >> 2) & 1)
                .with(vec![list(header, &paras)?]),
        );
    }
    Ok(object)
}
fn control(r: &Record) -> Result<Node> {
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
        "bokm" | "idxm" | "tcmt" => {
            Ok(n("ctrl").with(vec![n("nonPrinting").at("sourceControl", r.id())]))
        }
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
        "cold" => Ok(n("ctrl").with(vec![
            n("colPr")
                .at("colCount", (u16(d, 4)? >> 2) & 255)
                .at("sameGap", u16(d, 6)?),
        ])),
        "pgnp" => Ok(n("ctrl").with(vec![
            n("pageNumPos")
                .at("position", (u32(d, 4)? >> 8) & 15)
                .at("format", u32(d, 4)? & 255)
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
                    .at("hasMargin", 1)
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
                let start = if sd.len() == 25 { 1 } else { 4 };
                for (i, label) in ["center", "axis1", "axis2"].iter().enumerate() {
                    out.children.push(
                        n(label)
                            .at("x", i32(sd, start + i * 8)?)
                            .at("y", i32(sd, start + 4 + i * 8)?),
                    );
                }
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
                        .at("style", if attrs & 63 == 0 { "NONE" } else { "SOLID" }),
                );
                if cd.len() >= off + 21 && u32(cd, off + 13)? & 1 != 0 {
                    let v = u32(cd, off + 17)?;
                    if v != u32::MAX {
                        out.children
                            .push(n("fillBrush").with(vec![n("winBrush").at("faceColor", rgb(v))]));
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
        "%hlk" => Ok(n("ctrl").with(vec![n("hyperlink")])),
        field if field.starts_with('%') => {
            Ok(n("ctrl").with(vec![n("cachedField").at("sourceControl", field)]))
        }
        other => bail!("Unsupported HWP control {other}"),
    }
}
fn paragraph(r: &Record) -> Result<Node> {
    let d = &r.data;
    let mut out = n("p")
        .at("id", format!("hwp-{:x}", r.offset))
        .at("paraPrIDRef", u16(d, 8)?)
        .at(
            "pageBreak",
            if bytes(d, 11, 1)?[0] & 4 != 0 { 1 } else { 0 },
        );
    let runs = r.first(68).context("Missing character-shape runs")?;
    ensure!(runs.data.len() % 8 == 0, "Invalid shape-run length");
    let runs: Vec<(usize, u32)> = runs
        .data
        .as_chunks::<8>()
        .0
        .iter()
        .map(|d| Ok((u32(d, 0)? as usize, u32(d, 4)?)))
        .collect::<Result<_>>()?;
    ensure!(!runs.is_empty(), "Empty character runs");
    let controls: Vec<_> = r.children.iter().filter(|r| r.tag == 71).collect();
    let mut consumed = BTreeSet::new();
    let textdata = r.first(67).map(|r| r.data.as_slice()).unwrap_or(&[]);
    ensure!(textdata.len().is_multiple_of(2), "Invalid text length");
    let mut pos = 0;
    while pos < textdata.len() / 2 {
        let code = u16(textdata, 2 * pos)?;
        let style = runs
            .iter()
            .rev()
            .find(|(p, _)| *p <= pos)
            .unwrap_or(&runs[0])
            .1;
        let mut run = n("run").at("charPrIDRef", style);
        let mut advance = 1;
        if code >= 32 {
            advance = if (0xd800..=0xdbff).contains(&code) {
                2
            } else {
                1
            };
            let value = utf16(bytes(textdata, pos * 2, advance * 2)?)?;
            let mapped = match value.as_str() {
                "\u{f012b}" => Some("(인)"),
                "\u{f081c}" => Some(""),
                "\u{329e}" => Some("(인)"),
                _ => None,
            };
            run.children.push(n("t").with(vec![if let Some(s) = mapped {
                n("hwpDisplay").at("units", advance).at("value", s)
            } else {
                txt(value)
            }]));
        } else if ![0, 10, 13, 24, 25, 30, 31].contains(&code) {
            advance = 8;
            bytes(textdata, pos * 2, 16)?;
            if code == 9 {
                run.children
                    .push(n("t").with(vec![n("tab").at("width", u32(textdata, pos * 2 + 2)?)]));
            } else {
                let id: String = bytes(textdata, pos * 2 + 2, 4)?
                    .iter()
                    .rev()
                    .map(|b| *b as char)
                    .collect();
                if let Some((i, c)) = controls
                    .iter()
                    .enumerate()
                    .find(|(i, c)| !consumed.contains(i) && c.id() == id)
                {
                    consumed.insert(i);
                    run.children.push(control(c)?);
                } else {
                    run.children
                        .push(n("ctrl").with(vec![n("opaque").at("code", code)]));
                }
            }
        } else {
            run.children.push(n("t").with(vec![match code {
                10 => n("lineBreak"),
                30 | 31 => n("nbSpace"),
                24 => txt("-"),
                _ => n("lineBreak"),
            }]))
        }
        out.children.push(run);
        pos += advance;
    }
    // Empty paragraphs still carry the document's character and line-cache style.
    if out.children.is_empty() {
        out.children.push(n("run").at("charPrIDRef", runs[0].1));
    }
    ensure!(
        consumed.len() == controls.len(),
        "Unmatched control in paragraph {:x}: {:?}",
        r.offset,
        controls
            .iter()
            .enumerate()
            .filter(|(i, _)| !consumed.contains(i))
            .map(|(_, c)| c.id())
            .collect::<Vec<_>>()
    );
    let ls = r.first(69).context("Missing saved HWP line layout")?;
    ensure!(ls.data.len() % 36 == 0, "Invalid line record length");
    let mut linearray = n("linesegarray");
    for d in ls.data.as_chunks::<36>().0 {
        let mut l = n("lineseg");
        for (i, key) in [
            "textpos",
            "vertpos",
            "vertsize",
            "textheight",
            "baseline",
            "spacing",
            "horzpos",
            "horzsize",
            "flags",
        ]
        .iter()
        .enumerate()
        {
            l = l.at(
                key,
                if i == 0 || i == 8 {
                    u32(d, 4 * i)? as i64
                } else {
                    i32(d, 4 * i)? as i64
                },
            );
        }
        linearray.children.push(l);
    }
    out.children.push(linearray);
    Ok(out)
}
fn decode_pcx(d: &[u8]) -> Result<Vec<u8>> {
    ensure!(d.len() >= 128 && d[0] == 10, "Invalid PCX");
    let bits = d[3] as usize;
    let planes = d[65] as usize;
    let stride = u16(d, 66)? as usize;
    let w = (u16(d, 8)? as usize)
        .checked_sub(u16(d, 4)? as usize)
        .context("PCX width")?
        + 1;
    let h = (u16(d, 10)? as usize)
        .checked_sub(u16(d, 6)? as usize)
        .context("PCX height")?
        + 1;
    ensure!(
        w * h <= 16_000_000 && stride * planes * h <= LIMIT,
        "PCX too large"
    );
    ensure!(
        matches!((bits, planes), (1, 1..=4) | (8, 1) | (8, 3)),
        "Unsupported PCX format {bits}/{planes}"
    );
    ensure!(stride * 8 >= w * bits, "PCX stride");
    let required = stride * planes * h;
    let mut raw = Vec::with_capacity(required);
    let mut p = 128;
    while raw.len() < required {
        let v = *d.get(p).context("Truncated PCX")?;
        p += 1;
        let (count, value) = if v & 0xc0 == 0xc0 {
            let value = *d.get(p).context("PCX RLE")?;
            p += 1;
            ((v & 63) as usize, value)
        } else {
            (1, v)
        };
        ensure!(
            count > 0 && raw.len() + count <= required,
            "Invalid PCX RLE"
        );
        raw.resize(raw.len() + count, value);
    }
    let mut im = image::RgbImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let rgb = if bits == 8 && planes == 3 {
                [
                    raw[y * stride * 3 + x],
                    raw[y * stride * 3 + stride + x],
                    raw[y * stride * 3 + 2 * stride + x],
                ]
            } else {
                let index = if bits == 1 {
                    let mut v = 0;
                    for plane in 0..planes {
                        v |= (((raw[y * stride * planes + plane * stride + x / 8] >> (7 - x % 8))
                            & 1) as usize)
                            << plane;
                    }
                    v
                } else {
                    raw[y * stride + x] as usize
                };
                let palette = if bits == 8 {
                    ensure!(
                        d.len() >= 769 && d[d.len() - 769] == 12,
                        "Missing PCX palette"
                    );
                    d.len() - 768
                } else {
                    16
                };
                bytes(d, palette + index * 3, 3)?.try_into()?
            };
            im.put_pixel(x as u32, y as u32, image::Rgb(rgb));
        }
    }
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(im).write_to(&mut output, image::ImageFormat::Png)?;
    Ok(output.into_inner())
}
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
            23 => {
                let id = header.children_named("numbering").count() + 1;
                let mut number = n("numbering").at("id", id);
                let mut off = 0;
                for level in 1..=7 {
                    let props = u32(d, off)?;
                    let count = u16(d, off + 12)? as usize;
                    let fmt = utf16(bytes(d, off + 14, count * 2)?)?;
                    let head = n("paraHead")
                        .at("level", level)
                        .at("charPrIDRef", u32(d, off + 8)?)
                        .at("textOffset", u16(d, off + 6)?)
                        .at(
                            "textOffsetType",
                            if props & 16 == 0 {
                                "PERCENT"
                            } else {
                                "HWPUNIT"
                            },
                        )
                        .with(vec![txt(fmt)]);
                    number.children.push(head);
                    off += 14 + count * 2;
                }
                let start = u16(d, off)?;
                number = number.at("start", start);
                header.children.push(number);
            }
            24 => {
                let id = header.children_named("bullet").count() + 1;
                header.children.push(
                    n("bullet")
                        .at("id", id)
                        .at("charPrIDRef", u32(d, 8)?)
                        .at("textOffset", u16(d, 6)?)
                        .at(
                            "textOffsetType",
                            if u32(d, 0)? & 16 == 0 {
                                "PERCENT"
                            } else {
                                "HWPUNIT"
                            },
                        )
                        .at("char", utf16(bytes(d, 12, 2)?)?),
                );
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
                if u32(d, 46)? & 2 != 0 {
                    ch.children.push(n("bold"));
                }
                let props = u32(d, 46)?;
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
                            rgb(if d.len() >= 72 {
                                u32(d, 68)?
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
                    n("paraPr").at("id", id).with(vec![
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
                                Ok(n(key).at("value", i32(d, off)?).at("unit", "HWPUNIT"))
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
                                    "BREAK_WORD"
                                } else {
                                    "KEEP_WORD"
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
                            .at("offsetLeft", u16(d, 34)?)
                            .at("offsetRight", u16(d, 36)?)
                            .at("offsetTop", u16(d, 38)?)
                            .at("offsetBottom", u16(d, 40)?),
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
                            .at("type", if typ == 0 { "NONE" } else { "SOLID" })
                            .at("width", format!("{} mm", widths[weight.min(15)]))
                            .at("color", rgb(u32(d, 4 + 6 * i)?)),
                    );
                }
                if u32(d, 32)? & 1 != 0 {
                    let v = u32(d, 36)?;
                    if v != u32::MAX {
                        b.children
                            .push(n("fillBrush").with(vec![n("winBrush").at("faceColor", rgb(v))]));
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
    for name in &names {
        if name.starts_with("/BinData/") {
            let id = u16::from_str_radix(name.get(12..16).context("Invalid BinData name")?, 16)?;
            let raw = read(&mut cfb, name)?;
            let entry = all
                .iter()
                .filter(|r| r.tag == 18)
                .find(|r| u16(&r.data, 2).ok() == Some(id));
            let mode = entry
                .map(|r| u16(&r.data, 0).map(|v| (v >> 4) & 3))
                .transpose()?
                .unwrap_or(0);
            let raw = if mode == 1 || (mode == 0 && compressed) {
                inflate(&raw)?
            } else {
                raw
            };
            let raw = if name.to_lowercase().ends_with(".pcx") {
                decode_pcx(&raw)?
            } else {
                raw
            };
            binaries.insert(format!("image{id}"), raw);
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
    fn effects(nodes: &mut [Node], binaries: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        for node in nodes {
            if node.name == "pic"
                && let Some(img) = node.children.iter_mut().find(|n| n.name == "img")
            {
                let brightness = img.n("bright");
                let contrast = img.n("contrast");
                let gray = img.a("effect") == "1";
                if brightness != 0. || contrast != 0. || gray {
                    let id = img.a("binaryItemIDRef").to_string();
                    let derived = format!("{id}-{}-{}-{}", brightness, contrast, gray);
                    if !binaries.contains_key(&derived) {
                        let raw = binaries.get(&id).context("Missing effect image")?;
                        let image = image::load_from_memory(raw)?;
                        let mut rgb = if gray {
                            image.grayscale().to_rgb8()
                        } else {
                            image.to_rgb8()
                        };
                        for px in rgb.pixels_mut() {
                            for v in &mut px.0 {
                                let c = 255. + (*v as f64 - 255.) * (1. + contrast / 100.);
                                *v = if brightness >= 0. {
                                    c + (255. - c) * brightness / 100.
                                } else {
                                    c * (1. + brightness / 100.)
                                }
                                .clamp(0., 255.)
                                .round() as u8;
                            }
                        }
                        let mut out = Cursor::new(Vec::new());
                        image::DynamicImage::ImageRgb8(rgb)
                            .write_to(&mut out, image::ImageFormat::Png)?;
                        binaries.insert(derived.clone(), out.into_inner());
                    }
                    img.attrs.insert("binaryItemIDRef".into(), derived);
                }
            }
            effects(&mut node.children, binaries)?;
        }
        Ok(())
    }
    effects(&mut sections, &mut binaries)?;
    effects(&mut masters, &mut binaries)?;
    fn static_ole(nodes: &mut [Node], binaries: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        for node in nodes {
            if node.name == "ole" {
                let id = node
                    .child("oleRef")
                    .context("OLE reference")?
                    .a("binaryItemIDRef")
                    .to_string();
                let raw = binaries.get(&id).context("OLE binary missing")?;
                let raw =
                    if raw.get(4..12) == Some(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) {
                        &raw[4..]
                    } else {
                        raw.as_slice()
                    };
                let mut storage = cfb::CompoundFile::open(Cursor::new(raw))?;
                if storage.exists("/Contents") {
                    let mut contents = vec![];
                    storage
                        .open_stream("/Contents")?
                        .take(LIMIT as u64 + 1)
                        .read_to_end(&mut contents)?;
                    ensure!(contents.len() <= LIMIT, "Chart data too large");
                    let chart = hwp_legacy_chart::parse_ole_chart_contents(&contents)?;
                    let presentation =
                        hwp_legacy_chart::ole_chart::legacy_presentation::parse(&contents);
                    node.name = "chart".into();
                    node.attrs.insert(
                        "nativeType".into(),
                        presentation
                            .as_ref()
                            .map(|p| p.chart_type.to_string())
                            .unwrap_or_else(|| "unknown".into()),
                    );
                    node.attrs.insert(
                        "axisEvidence".into(),
                        format!("{:?}", chart.series_axis_evidence),
                    );
                    node.attrs
                        .insert("title".into(), chart.title.unwrap_or_default());
                    for category in chart.categories {
                        node.children.push(n("category").at("name", category));
                    }
                    for (i, series) in chart.series.into_iter().enumerate() {
                        let mut s = n("series").at(
                            "name",
                            series.name.unwrap_or_else(|| format!("Series {}", i + 1)),
                        );
                        if let Some(style) = presentation.as_ref().and_then(|p| p.series.get(i)) {
                            s = s.at("color", rgb(style.fill));
                        }
                        for v in series.values {
                            ensure!(v.is_finite(), "Non-finite chart value");
                            s.children.push(n("value").at("n", v));
                        }
                        node.children.push(s);
                    }
                    continue;
                }
                let stream = "/\u{1}Ole10Native";
                ensure!(
                    storage.exists(stream),
                    "OLE has no supported static raster presentation"
                );
                let mut bytes = vec![];
                storage
                    .open_stream(stream)?
                    .take(LIMIT as u64 + 1)
                    .read_to_end(&mut bytes)?;
                ensure!(bytes.len() <= LIMIT, "OLE presentation too large");
                let payload = if bytes.get(4..6) == Some(b"BM") {
                    &bytes[4..]
                } else {
                    bytes.as_slice()
                };
                let image =
                    image::load_from_memory(payload).context("Unsupported static OLE image")?;
                let mut png = Cursor::new(Vec::new());
                image.write_to(&mut png, image::ImageFormat::Png)?;
                let name = format!("{id}-ole-static");
                binaries.insert(name.clone(), png.into_inner());
                node.name = "pic".into();
                node.attrs.insert("staticOLE".into(), "1".into());
                node.children.push(n("img").at("binaryItemIDRef", name));
            }
            static_ole(&mut node.children, binaries)?;
        }
        Ok(())
    }
    static_ole(&mut sections, &mut binaries)?;
    static_ole(&mut masters, &mut binaries)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_number_position_is_normalized() {
        let mut data = b"pngp".to_vec();
        data.extend((5u32 << 8).to_le_bytes());
        data.extend([0u8; 8]);
        let record = Record {
            tag: 71,
            level: 1,
            data,
            offset: 0,
            children: vec![],
        };
        let node = control(&record).unwrap();
        assert_eq!(node.find("pageNumPos").unwrap().a("position"), "5");
    }
    #[test]
    fn zero_stream_padding_is_accepted() {
        use std::io::Write;
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"sample").unwrap();
        let mut data = encoder.finish().unwrap();
        data.extend([0u8; 8]);
        assert_eq!(inflate(&data).unwrap(), b"sample");
    }
    #[test]
    fn truncated_record_rejected() {
        assert!(records(&[67, 0, 0, 1, 0]).is_err());
    }
    #[test]
    fn extended_length_rejected() {
        assert!(records(&u32::MAX.to_le_bytes()).is_err());
    }
    #[test]
    fn truncated_deflate_rejected() {
        assert!(inflate(&[0x73]).is_err());
    }
    #[test]
    fn color_order_is_bgr_storage() {
        assert_eq!(rgb(0x0000ff), "#FF0000");
    }
    #[test]
    fn crc_and_size_trailer_is_checked() {
        use std::io::Write;
        let payload = b"HWP regression payload";
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(payload).unwrap();
        let mut encoded = encoder.finish().unwrap();
        encoded.extend(crc32fast::hash(payload).to_le_bytes());
        encoded.extend((payload.len() as u32).to_le_bytes());
        assert_eq!(inflate(&encoded).unwrap(), payload);
        let last = encoded.len() - 1;
        encoded[last] ^= 1;
        assert!(inflate(&encoded).is_err());
    }
}
