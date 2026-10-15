//! TrueType outline subsetting with stable glyph IDs for PDF Identity-H fonts.
//! Layout/shaping uses the original face; only the embedded program is reduced.
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
fn bytes(b: &[u8], at: usize, len: usize) -> Result<&[u8]> {
    b.get(at..at.checked_add(len).context("Font range overflow")?)
        .context("Truncated font table")
}
fn u16be(b: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_be_bytes(bytes(b, at, 2)?.try_into()?))
}
fn u32be(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(bytes(b, at, 4)?.try_into()?))
}
fn checksum(b: &[u8]) -> u32 {
    b.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}
fn tables(src: &[u8]) -> Result<BTreeMap<[u8; 4], Vec<u8>>> {
    let count = u16be(src, 4)? as usize;
    ensure!(count > 0 && count < 4096, "Invalid font table count");
    let mut out = BTreeMap::new();
    for i in 0..count {
        let record = 12 + 16 * i;
        let tag = bytes(src, record, 4)?.try_into()?;
        let offset = u32be(src, record + 8)? as usize;
        let length = u32be(src, record + 12)? as usize;
        ensure!(
            out.insert(tag, bytes(src, offset, length)?.to_vec())
                .is_none(),
            "Duplicate font table"
        );
    }
    Ok(out)
}
pub(super) fn subset(src: &[u8], used: impl IntoIterator<Item = u16>) -> Result<Option<Vec<u8>>> {
    let mut t = tables(src)?;
    // Variable/color/CFF fonts need a different dependency closure. Honor the
    // font's no-subsetting flag by retaining its complete original program.
    if !t.contains_key(b"glyf")
        || !t.contains_key(b"loca")
        || [b"fvar", b"COLR", b"CBDT", b"sbix", b"SVG "]
            .iter()
            .any(|k| t.contains_key(*k))
        || t.get(b"OS/2")
            .is_some_and(|b| u16be(b, 8).is_ok_and(|f| f & 0x100 != 0))
    {
        return Ok(None);
    }
    let head = t.get(b"head").context("Missing head")?;
    let format = u16be(head, 50)?;
    ensure!(format <= 1, "Invalid loca format");
    let count = u16be(t.get(b"maxp").context("Missing maxp")?, 4)? as usize;
    ensure!(count > 0, "Empty font");
    let loca = t.get(b"loca").unwrap();
    let glyf = t.get(b"glyf").unwrap();
    let offsets = (0..=count)
        .map(|i| {
            if format == 0 {
                u16be(loca, i * 2).map(|v| v as usize * 2)
            } else {
                u32be(loca, i * 4).map(|v| v as usize)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        offsets.windows(2).all(|p| p[0] <= p[1]) && offsets[count] <= glyf.len(),
        "Invalid glyph offsets"
    );
    let mut keep: BTreeSet<u16> = used.into_iter().collect();
    keep.insert(0);
    ensure!(
        keep.iter().all(|&g| (g as usize) < count),
        "Glyph ID outside font"
    );
    let mut queue: Vec<_> = keep.iter().copied().collect();
    while let Some(gid) = queue.pop() {
        let data = &glyf[offsets[gid as usize]..offsets[gid as usize + 1]];
        if data.is_empty() {
            continue;
        }
        ensure!(data.len() >= 10, "Truncated glyph header");
        if u16be(data, 0)? as i16 >= 0 {
            continue;
        }
        let mut at = 10;
        loop {
            let flags = u16be(data, at)?;
            let child = u16be(data, at + 2)?;
            ensure!((child as usize) < count, "Composite glyph outside font");
            if keep.insert(child) {
                queue.push(child);
            }
            at += 4 + if flags & 1 != 0 { 4 } else { 2 };
            at += if flags & 8 != 0 {
                2
            } else if flags & 64 != 0 {
                4
            } else if flags & 128 != 0 {
                8
            } else {
                0
            };
            bytes(data, 0, at)?;
            if flags & 32 == 0 {
                break;
            }
        }
    }
    let mut new_glyf = Vec::new();
    let mut new_loca = Vec::with_capacity((count + 1) * 4);
    for i in 0..count {
        new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
        if keep.contains(&(i as u16)) {
            new_glyf.extend_from_slice(&glyf[offsets[i]..offsets[i + 1]]);
            while !new_glyf.len().is_multiple_of(4) {
                new_glyf.push(0);
            }
        }
    }
    new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
    t.insert(*b"glyf", new_glyf);
    t.insert(*b"loca", new_loca);
    let head = t.get_mut(b"head").unwrap();
    head[8..12].fill(0);
    head[50..52].copy_from_slice(&1u16.to_be_bytes());
    // Keep hinting, cmap and metrics. PDF supplies shaped glyph positions and
    // Unicode mappings, so layout tables and invalidated signatures are omitted.
    let retained = [
        *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"loca", *b"glyf", *b"cmap", *b"name", *b"post",
        *b"OS/2", *b"cvt ", *b"fpgm", *b"prep", *b"gasp",
    ];
    t.retain(|tag, _| retained.contains(tag));
    let count = t.len();
    let power = 1usize << count.ilog2();
    let mut out = vec![0; 12 + count * 16];
    out[..4].copy_from_slice(bytes(src, 0, 4)?);
    for (at, value) in [
        (4, count),
        (6, power * 16),
        (8, power.ilog2() as usize),
        (10, count * 16 - power * 16),
    ] {
        out[at..at + 2].copy_from_slice(&(value as u16).to_be_bytes());
    }
    let mut head_offset = 0;
    for (i, (tag, data)) in t.into_iter().enumerate() {
        let at = 12 + i * 16;
        let offset = out.len();
        out[at..at + 4].copy_from_slice(&tag);
        out[at + 4..at + 8].copy_from_slice(&checksum(&data).to_be_bytes());
        out[at + 8..at + 12].copy_from_slice(&(offset as u32).to_be_bytes());
        out[at + 12..at + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        if &tag == b"head" {
            head_offset = offset;
        }
        out.extend(data);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
    }
    let adjustment = 0xb1b0afbau32.wrapping_sub(checksum(&out));
    out[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
    Ok((out.len() < src.len()).then_some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::Fonts;
    #[derive(Default, PartialEq, Debug)]
    struct Outline(Vec<(u8, Vec<f32>)>);
    impl ttf_parser::OutlineBuilder for Outline {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push((0, vec![x, y]));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push((1, vec![x, y]));
        }
        fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
            self.0.push((2, vec![a, b, x, y]));
        }
        fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
            self.0.push((3, vec![a, b, c, d, x, y]));
        }
        fn close(&mut self) {
            self.0.push((4, vec![]));
        }
    }
    #[test]
    fn subsets_keep_composite_outlines_metrics_ids_and_checksum() {
        let mut fonts = Fonts::new(std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }));
        for (name, text) in [
            ("arial.ttf", "éÅAV"),
            ("HANBatang.ttf", "한글漢字\u{f081c}\u{f012b}"),
        ] {
            let id = fonts.get(name).unwrap();
            let src = &fonts.list[id].bytes;
            let face = ttf_parser::Face::parse(src, 0).unwrap();
            let ids: Vec<_> = text
                .chars()
                .map(|c| face.glyph_index(c).unwrap().0)
                .collect();
            let out = subset(src, ids.iter().copied()).unwrap().unwrap();
            assert!(out.len() < src.len() / 2);
            assert_eq!(checksum(&out), 0xb1b0afba);
            let reduced = ttf_parser::Face::parse(&out, 0).unwrap();
            for id in ids {
                let id = ttf_parser::GlyphId(id);
                let mut a = Outline::default();
                let mut b = Outline::default();
                face.outline_glyph(id, &mut a);
                reduced.outline_glyph(id, &mut b);
                assert_eq!(a, b);
                assert_eq!(face.glyph_hor_advance(id), reduced.glyph_hor_advance(id));
            }
        }
    }
    #[test]
    fn no_subsetting_flag_retains_the_original_font_program() {
        let mut fonts = Fonts::new(std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }));
        let id = fonts.get("arial.ttf").unwrap();
        let mut src = fonts.list[id].bytes.clone();
        let count = u16be(&src, 4).unwrap() as usize;
        let at = (0..count)
            .map(|i| 12 + 16 * i)
            .find(|&at| &src[at..at + 4] == b"OS/2")
            .unwrap();
        let offset = u32be(&src, at + 8).unwrap() as usize;
        src[offset + 8] |= 1;
        assert!(subset(&src, [0]).unwrap().is_none());
    }
    #[test]
    fn truncated_fonts_are_rejected_without_panics() {
        for data in [&[][..], &[0; 12][..], &[0, 1, 0, 0, 0, 2][..]] {
            assert!(subset(data, [0]).is_err());
        }
    }
}
