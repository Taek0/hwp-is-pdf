//! Non-executing stream/record diagnostic for format compatibility work.
use anyhow::Result;
use std::io::Read;
fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("input.hwp");
    let tags: Vec<u32> = std::env::args()
        .nth(2)
        .map(|s| s.split(',').filter_map(|s| s.parse().ok()).collect())
        .unwrap_or_else(|| vec![71, 72, 74, 77, 81, 82, 83, 87, 88]);
    let limit = std::env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(64usize);
    let mut f = cfb::open(path)?;
    let names: Vec<_> = f
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_path_buf())
        .collect();
    let mut header = vec![];
    f.open_stream("/FileHeader")?.read_to_end(&mut header)?;
    let compressed = header[36] & 1 != 0;
    println!("version {:?}", &header[32..36]);
    for name in names {
        if !name.to_string_lossy().contains("DocInfo")
            && !name.to_string_lossy().contains("Section")
        {
            continue;
        }
        let mut raw = vec![];
        f.open_stream(&name)?.read_to_end(&mut raw)?;
        let raw = if name.to_string_lossy().starts_with("/ViewText/") {
            hwp_engine::distribution::decode(&raw)?
        } else {
            raw
        };
        let b = if compressed {
            let mut dec = flate2::read::DeflateDecoder::new(raw.as_slice());
            let mut b = vec![];
            dec.read_to_end(&mut b)?;
            let used = dec.total_in() as usize;
            println!(
                "{} raw={} decoded={} CRC={:08x} tail={:02x?}",
                name.display(),
                raw.len(),
                b.len(),
                crc32fast::hash(&b),
                &raw[used..]
            );
            b
        } else {
            raw
        };
        let mut p = 0;
        while p + 4 <= b.len() {
            let v = u32::from_le_bytes(b[p..p + 4].try_into()?);
            p += 4;
            let tag = v & 1023;
            let lev = (v >> 10) & 1023;
            let mut len = (v >> 20) as usize;
            if len == 4095 {
                len = u32::from_le_bytes(b[p..p + 4].try_into()?) as usize;
                p += 4;
            }
            let d = &b[p..p + len];
            if tags.contains(&tag) {
                println!(
                    "{:x} {} {} len={} data={:02x?}",
                    p - 4,
                    tag,
                    lev,
                    len,
                    &d[..len.min(limit)]
                );
            }
            p += len;
        }
    }
    Ok(())
}
