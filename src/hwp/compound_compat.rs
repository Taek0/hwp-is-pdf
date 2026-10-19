//! Recover only unreachable zero-filled FAT slots; active stream bytes stay untouched.
use std::{collections::BTreeSet, io::Cursor};
fn word(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}
fn dword(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
fn walk(start: u32, fat: &[u32], sectors: usize, used: &mut BTreeSet<usize>) -> Option<Vec<usize>> {
    let mut chain = Vec::new();
    let mut local = BTreeSet::new();
    let mut at = start;
    while at != 0xffff_fffe {
        let i = at as usize;
        if i >= sectors || i >= fat.len() || !local.insert(i) {
            return None;
        }
        if used.contains(&i) {
            break;
        }
        used.insert(i);
        chain.push(i);
        at = fat[i];
    }
    Some(chain)
}
pub(super) fn repair(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 512
        || !data.len().is_multiple_of(512)
        || data.get(..8) != Some(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1])
        || word(data, 26)? != 3
        || word(data, 30)? != 9
        || word(data, 32)? != 6
        || dword(data, 72)? != 0
    {
        return None;
    }
    let sectors = data.len() / 512 - 1;
    let count = dword(data, 44)? as usize;
    if count == 0 || count > 109 {
        return None;
    }
    let mut ids = Vec::new();
    let mut fat = Vec::new();
    let mut used = BTreeSet::new();
    for i in 0..count {
        let sector = dword(data, 76 + i * 4)? as usize;
        if sector >= sectors || !used.insert(sector) {
            return None;
        }
        ids.push(sector);
        for j in 0..128 {
            fat.push(dword(data, (sector + 1) * 512 + j * 4)?);
        }
    }
    // A strict container is never rewritten, even if it has unusual unused data.
    if cfb::CompoundFile::open(Cursor::new(data)).is_ok() {
        return None;
    }
    let directory = walk(dword(data, 48)?, &fat, sectors, &mut used)?;
    let mut streams = Vec::new();
    let mut mini_streams = Vec::new();
    let mut root_size = 0usize;
    for sector in directory {
        let bytes = data.get((sector + 1) * 512..(sector + 2) * 512)?;
        for entry in bytes.as_chunks::<128>().0 {
            let kind = entry[66];
            let size = u64::from_le_bytes(entry[120..128].try_into().ok()?);
            if size > 0 && (kind == 5 || (kind == 2 && size >= u64::from(dword(data, 56)?))) {
                streams.push((dword(entry, 116)?, kind == 5));
                if kind == 5 {
                    root_size = usize::try_from(size).ok()?;
                }
            } else if kind == 2 && size > 0 {
                mini_streams.push(dword(entry, 116)?);
            }
        }
    }
    let mini_fat_ids = if dword(data, 64)? > 0 {
        walk(dword(data, 60)?, &fat, sectors, &mut used)?
    } else {
        Vec::new()
    };
    if mini_fat_ids.len() != dword(data, 64)? as usize {
        return None;
    }
    let mut root_chain = Vec::new();
    for (start, root) in streams {
        let chain = walk(start, &fat, sectors, &mut used)?;
        if root {
            root_chain = chain;
        }
    }
    let candidates: Vec<_> = fat
        .iter()
        .enumerate()
        .filter_map(|(i, next)| {
            (*next == 0
                && !used.contains(&i)
                && (i >= sectors
                    || data
                        .get((i + 1) * 512..(i + 2) * 512)
                        .is_some_and(|b| b.iter().all(|v| *v == 0))))
            .then_some(i)
        })
        .collect();
    let mut mini_candidates = Vec::new();
    if !mini_fat_ids.is_empty() {
        if root_size > data.len() || root_chain.len() * 512 < root_size {
            return None;
        }
        let mut mini_fat = Vec::new();
        for sector in &mini_fat_ids {
            for j in 0..128 {
                mini_fat.push(dword(data, (sector + 1) * 512 + j * 4)?);
            }
        }
        let mut root_data = Vec::new();
        for sector in root_chain {
            root_data.extend_from_slice(data.get((sector + 1) * 512..(sector + 2) * 512)?);
        }
        root_data.truncate(root_size);
        let mini_count = root_size.div_ceil(64);
        let mut mini_used = BTreeSet::new();
        for start in mini_streams {
            walk(start, &mini_fat, mini_count, &mut mini_used)?;
        }
        mini_candidates = mini_fat
            .iter()
            .enumerate()
            .filter_map(|(i, next)| {
                (*next == 0
                    && !mini_used.contains(&i)
                    && (i >= mini_count
                        || root_data
                            .get(i * 64..(i + 1) * 64)
                            .is_some_and(|b| b.iter().all(|v| *v == 0))))
                .then_some(i)
            })
            .collect();
    }
    if candidates.is_empty() && mini_candidates.is_empty() {
        return None;
    }
    let mut out = data.to_vec();
    for index in candidates {
        let at = (ids[index / 128] + 1) * 512 + (index % 128) * 4;
        out[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    }
    for index in mini_candidates {
        let at = (mini_fat_ids[index / 128] + 1) * 512 + (index % 128) * 4;
        out[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    }
    // Let the normal parser validate allocation ownership and directory structure.
    cfb::CompoundFile::open(Cursor::new(out.as_slice())).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    #[test]
    fn unused_zero_slots_recover_without_changing_stream_data() {
        let mut f =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))
                .unwrap();
        f.create_stream("/value")
            .unwrap()
            .write_all(b"preserved")
            .unwrap();
        let mut bytes = f.into_inner().into_inner();
        let original = bytes.clone();
        assert!(repair(&bytes).is_none());
        let sectors = bytes.len() / 512 - 1;
        let fat = dword(&bytes, 76).unwrap() as usize;
        bytes.resize(bytes.len() + 3 * 512, 0);
        for i in sectors..sectors + 3 {
            let at = (fat + 1) * 512 + i * 4;
            bytes[at..at + 4].fill(0);
        }
        let unchanged = bytes.clone();
        assert!(cfb::CompoundFile::open(Cursor::new(&bytes)).is_err());
        let repaired = repair(&bytes).unwrap();
        assert_eq!(bytes, unchanged);
        let mut f = cfb::CompoundFile::open(Cursor::new(repaired)).unwrap();
        let mut value = Vec::new();
        f.open_stream("/value")
            .unwrap()
            .read_to_end(&mut value)
            .unwrap();
        assert_eq!(value, b"preserved");
        assert_eq!(&bytes[..512], &original[..512]);
    }
    #[test]
    fn an_active_zero_link_is_not_repaired() {
        let mut f =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))
                .unwrap();
        f.create_stream("/large")
            .unwrap()
            .write_all(&[0; 4096])
            .unwrap();
        let mut bytes = f.into_inner().into_inner();
        let fat_id = dword(&bytes, 76).unwrap() as usize;
        let directory = dword(&bytes, 48).unwrap() as usize;
        let at = (directory + 1) * 512;
        let stream = (0..4)
            .find_map(|i| {
                let e = &bytes[at + i * 128..at + (i + 1) * 128];
                (e[66] == 2).then(|| dword(e, 116).unwrap() as usize)
            })
            .unwrap();
        bytes[(fat_id + 1) * 512 + stream * 4..(fat_id + 1) * 512 + stream * 4 + 4].fill(0);
        assert!(repair(&bytes).is_none());
    }
    #[test]
    fn unused_minifat_zero_links_recover_but_active_cycles_do_not() {
        let mut f =
            cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new()))
                .unwrap();
        f.create_stream("/value")
            .unwrap()
            .write_all(&[7; 128])
            .unwrap();
        f.create_stream("/unused1")
            .unwrap()
            .write_all(&[0; 64])
            .unwrap();
        f.create_stream("/unused2")
            .unwrap()
            .write_all(&[0; 64])
            .unwrap();
        f.create_stream("/tail")
            .unwrap()
            .write_all(&[9; 64])
            .unwrap();
        f.remove_stream("/unused1").unwrap();
        f.remove_stream("/unused2").unwrap();
        let mut bytes = f.into_inner().into_inner();
        let mini = dword(&bytes, 60).unwrap() as usize;
        for i in [2, 3] {
            bytes[(mini + 1) * 512 + i * 4..(mini + 1) * 512 + i * 4 + 4].fill(0);
        }
        let repaired = repair(&bytes).unwrap();
        let mut f = cfb::CompoundFile::open(Cursor::new(repaired)).unwrap();
        let mut value = Vec::new();
        f.open_stream("/value")
            .unwrap()
            .read_to_end(&mut value)
            .unwrap();
        assert_eq!(value, vec![7; 128]);
        bytes[(mini + 1) * 512 + 4..(mini + 1) * 512 + 8].fill(0);
        assert!(repair(&bytes).is_none());
    }
}
