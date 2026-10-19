use anyhow::Result;
use std::io::{Cursor, Read};
fn main() -> Result<()> {
    let path = std::env::args().nth(1).unwrap();
    let mut f = cfb::open(path)?;
    let names: Vec<_> = f
        .walk()
        .filter(|e| e.is_stream() && e.path().to_string_lossy().contains("BinData"))
        .map(|e| e.path().to_path_buf())
        .collect();
    for name in names {
        let mut b = vec![];
        f.open_stream(&name)?.read_to_end(&mut b)?;
        let mut decoded = vec![];
        if flate2::read::DeflateDecoder::new(b.as_slice())
            .read_to_end(&mut decoded)
            .is_ok()
        {
            b = decoded;
        }
        println!(
            "{} len={} prefix={:02x?}",
            name.display(),
            b.len(),
            &b[..b.len().min(48)]
        );
        if b.get(4..12) == Some(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) {
            b.drain(..4);
        }
        if let Ok(mut ole) = cfb::CompoundFile::open(Cursor::new(b)) {
            let ns: Vec<_> = ole
                .walk()
                .filter(|e| e.is_stream())
                .map(|e| e.path().to_path_buf())
                .collect();
            for n in ns {
                let mut d = vec![];
                ole.open_stream(&n)?.read_to_end(&mut d)?;
                if n.to_string_lossy() == "/OOXMLChartContents"
                    && let Some(target) = std::env::args().nth(2)
                {
                    std::fs::write(target, &d)?;
                }
                if n.to_string_lossy() == "/\u{2}OlePres000"
                    && let Some(target) = std::env::args().nth(3)
                {
                    std::fs::write(target, &d)?;
                }
                println!(
                    "  {} len={} prefix={:02x?}",
                    n.display(),
                    d.len(),
                    &d[..d.len().min(80)]
                );
                if n.to_string_lossy() == "/Contents" {
                    println!(
                        "GRID {:?} bytes300={:02x?}",
                        hwp_legacy_chart::ole_chart::grid::scan_legacy_grid(&d),
                        &d[300.min(d.len())..400.min(d.len())]
                    );
                }
                for sig in [
                    b"VtPicture".as_slice(),
                    b"VtDataGrid".as_slice(),
                    b" EMF".as_slice(),
                    &[1u8, 0, 9, 0, 0, 3],
                ] {
                    for (i, _) in d.windows(sig.len()).enumerate().filter(|(_, v)| *v == sig) {
                        println!(
                            "SIGNATURE {:?} at {} next {:02x?}",
                            sig,
                            i,
                            &d[i..(i + 240).min(d.len())]
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
