//! Bounded, non-executing Windows metafile playback into existing PDF operations.
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
mod complex;
mod paint;
pub(crate) mod presentation;
mod raster;
mod text;
pub(crate) use paint::draw;

pub(crate) fn is_wmf(b: &[u8]) -> bool {
    b.starts_with(&[0xd7, 0xcd, 0xc6, 0x9a])
        || (b.len() >= 18
            && matches!(u16(b, 0), Ok(1 | 2))
            && u16(b, 2).ok() == Some(9)
            && matches!(u16(b, 4), Ok(0x100 | 0x300)))
}
fn u16(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        b.get(o..o + 2).context("Truncated WMF word")?.try_into()?,
    ))
}
fn i16(b: &[u8], o: usize) -> Result<i16> {
    Ok(u16(b, o)? as i16)
}
fn u32(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(o..o + 4).context("Truncated WMF dword")?.try_into()?,
    ))
}
struct Record<'a> {
    offset: usize,
    kind: u16,
    data: &'a [u8],
}
fn records(b: &[u8]) -> Result<Vec<Record<'_>>> {
    ensure!(
        b.len() <= 64 * 1024 * 1024 && is_wmf(b),
        "Invalid or oversized WMF"
    );
    let header = if b.starts_with(&[0xd7, 0xcd, 0xc6, 0x9a]) {
        22
    } else {
        0
    };
    ensure!(u16(b, header + 2)? == 9, "Invalid WMF header");
    let words = u32(b, header + 6)? as usize;
    // Some writers emit an empty placeable metafile with a zero mtSize.
    // Accept only the complete header + EOF form, never an unbounded stream.
    let empty =
        words == 0 && b.len() == header + 24 && b.get(header + 18..) == Some(&[3, 0, 0, 0, 0, 0]);
    let end = header
        + (if empty { 12 } else { words })
            .checked_mul(2)
            .context("WMF size overflow")?;
    ensure!(end <= b.len() && end >= header + 18, "Truncated WMF file");
    let mut at = header + 18;
    let mut out = vec![];
    let mut eof = false;
    while at < end {
        ensure!(out.len() < 100000, "Too many WMF records");
        let len = (u32(b, at)? as usize)
            .checked_mul(2)
            .context("WMF record overflow")?;
        ensure!(len >= 6 && len <= end - at, "Invalid WMF record size");
        let kind = u16(b, at + 4)?;
        out.push(Record {
            offset: at,
            kind,
            data: &b[at + 6..at + len],
        });
        at += len;
        if kind == 0 {
            eof = true;
            break;
        }
    }
    ensure!(eof, "Missing WMF EOF");
    Ok(out)
}
pub(crate) fn prepare(images: &mut BTreeMap<String, Vec<u8>>) -> Result<Vec<String>> {
    let mut extra = vec![];
    let mut software = vec![];
    for (id, b) in images.iter().filter(|(_, b)| is_wmf(b)) {
        let records = records(b)?;
        if complex::needed(&records) {
            extra.push((
                id.clone(),
                complex::render(&records).with_context(|| format!("Complex WMF {id}"))?,
            ));
            software.push(id.clone());
            continue;
        }
        for r in records {
            if r.kind == 0xf43 {
                extra.push((format!("{id}-wmf-{}", r.offset), raster::bitmap(r.data)?));
            }
        }
    }
    for (id, b) in extra {
        images.insert(id, b);
    }
    Ok(software)
}
pub(crate) fn mark_software(nodes: &mut [crate::model::Node], ids: &[String]) {
    for node in nodes {
        if node.name == "pic"
            && node
                .child("img")
                .is_some_and(|img| ids.iter().any(|id| id == img.a("binaryItemIDRef")))
        {
            node.attrs.insert("softwareWMF".into(), "1".into());
            // The intermediate bitmap resolution is not the original picture's
            // crop coordinate system. Retain vector playback's target rectangle.
            for child in &mut node.children {
                if child.name == "imgClip" {
                    child.name = "wmfSourceClip".into();
                }
            }
        }
        mark_software(&mut node.children, ids);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_size_is_allowed_only_for_a_complete_empty_metafile() {
        let mut b = vec![0; 24];
        b[0] = 1;
        b[2] = 9;
        b[5] = 3;
        b[18] = 3;
        assert_eq!(records(&b).unwrap().len(), 1);
        b[22] = 1;
        assert!(records(&b).is_err());
        b[22] = 0;
        b.push(0);
        assert!(records(&b).is_err());
    }
    #[test]
    fn malformed_lengths_fail_without_playback() {
        let mut b = vec![0; 24];
        b[0] = 1;
        b[2] = 9;
        b[5] = 3;
        b[6] = 12;
        assert!(records(&b).is_err());
        b[18] = 255;
        assert!(records(&b).is_err());
        b[18] = 3;
        assert_eq!(records(&b).unwrap().len(), 1);
    }
}
