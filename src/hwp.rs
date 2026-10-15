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

mod columns;
mod controls;
mod document;
mod images;
mod list_records;
mod object_properties;
mod paragraphs;
mod section;
mod tab_records;
use controls::control;
pub use document::open;
use images::decode_pcx;
use object_properties::common;
use paragraphs::paragraph;
use section::section_properties;

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
