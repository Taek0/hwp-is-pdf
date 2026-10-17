//! Inspect decoded embedded assets without executing OLE or native programs.
use anyhow::Result;
use hwp_engine::model::Document;
use std::{
    io::{Cursor, Read},
    path::Path,
};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let d = Document::open(Path::new(&args[0]))?;
    let output = Path::new(&args[1]);
    std::fs::create_dir_all(output)?;
    for (index, (id, bytes)) in d.binaries.iter().enumerate() {
        println!(
            "{id}: {} bytes, magic {:02x?}",
            bytes.len(),
            &bytes[..bytes.len().min(16)]
        );
        std::fs::write(output.join(format!("asset-{index}.bin")), bytes)?;
        let magic = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
        let raw = if bytes.starts_with(&magic) {
            Some(bytes.as_slice())
        } else if bytes.get(4..12) == Some(&magic) {
            Some(&bytes[4..])
        } else {
            None
        };
        if let Some(raw) = raw {
            let mut c = cfb::CompoundFile::open(Cursor::new(raw))?;
            let names: Vec<_> = c
                .walk()
                .filter(|e| e.is_stream())
                .map(|e| e.path().to_path_buf())
                .collect();
            for (i, name) in names.iter().enumerate() {
                let mut b = vec![];
                c.open_stream(name)?
                    .take(64 * 1024 * 1024)
                    .read_to_end(&mut b)?;
                println!(
                    "  stream-{index}-{i}: {} ({} bytes)",
                    name.display(),
                    b.len()
                );
                if name.to_string_lossy() == "/Contents"
                    && let Some(p) = hwp_legacy_chart::ole_chart::legacy_presentation::parse(&b)
                {
                    println!(
                        "  PRESENTATION type={} extent={:?} plot={:?}",
                        p.chart_type, p.extent, p.plot
                    );
                    println!("  FONTS {:?}", p.axis_fonts);
                    for (i, a) in p.axes.iter().enumerate() {
                        if let Some(a) = a {
                            println!(
                                "  AXIS {i}: {}..{}, divisions {}",
                                a.min, a.max, a.divisions
                            );
                        }
                    }
                    for s in p.series {
                        println!(
                            "  SERIES {}: color={:06x} fill={:06x} width={} secondary={} selectors={:?}",
                            s.index, s.color, s.fill, s.width, s.secondary, s.selectors
                        );
                    }
                }
                std::fs::write(output.join(format!("stream-{index}-{i}.bin")), b)?;
            }
        }
    }
    for section in &d.sections {
        let mut charts = vec![];
        section.all("chart", &mut charts);
        for c in charts {
            println!(
                "CHART {:?}, categories {}, series {}",
                c.attrs,
                c.children_named("category").count(),
                c.children_named("series").count()
            );
            for s in c.children_named("series") {
                println!(
                    "  DATA {} {:?}",
                    s.a("name"),
                    s.children_named("value")
                        .take(8)
                        .map(|v| v.n("n"))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    Ok(())
}
