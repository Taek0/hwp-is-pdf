//! Copy a known-good HWP and vary only one embedded picture's effect parameters.
//! Diagnostic fixture creation only; the conversion library remains read-only.
use anyhow::{Context, Result, ensure};
use std::{
    io::{Read, Write},
    path::Path,
};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    ensure!(a.len() == 4, "INPUT.hwp OUTPUT_DIRECTORY IMAGE_ID");
    let input = Path::new(&a[1]);
    hwp_engine::model::Document::open(input)?;
    let out = Path::new(&a[2]);
    let image_id: u16 = a[3].parse()?;
    let original = std::fs::read(input)?;
    std::fs::create_dir_all(out.join("pdf"))?;
    let mut entries = Vec::new();
    for (bright, contrast) in [(0i8, 0i8), (70, 0), (0, -50)] {
        let name = format!("brightness-{bright}_contrast-{contrast}");
        let target = out.join(format!("{name}.hwp"));
        ensure!(
            !target.exists(),
            "Probe already exists: {}",
            target.display()
        );
        std::fs::copy(input, &target)?;
        let mut file = cfb::open_rw(&target)?;
        let mut header = Vec::new();
        file.open_stream("/FileHeader")?.read_to_end(&mut header)?;
        ensure!(
            header.len() >= 40 && header[36] & 6 == 0,
            "Encrypted/distribution probes are excluded"
        );
        let compressed = header[36] & 1 != 0;
        let paths: Vec<_> = file
            .walk()
            .filter(|e| {
                e.is_stream() && e.path().to_string_lossy().starts_with("/BodyText/Section")
            })
            .map(|e| e.path().to_path_buf())
            .collect();
        let mut changed = 0;
        for path in paths {
            let mut raw = Vec::new();
            file.open_stream(&path)?.read_to_end(&mut raw)?;
            let (mut data, mut trailer) = if compressed {
                let mut decoder = flate2::read::DeflateDecoder::new(raw.as_slice());
                let mut data = Vec::new();
                decoder.read_to_end(&mut data)?;
                let tail = &raw[decoder.total_in() as usize..];
                ensure!(
                    tail.iter().all(|v| *v == 0) || tail.len() >= 8,
                    "Unexpected stream tail"
                );
                (data, tail.to_vec())
            } else {
                (raw, Vec::new())
            };
            let mut pos = 0;
            let mut touched = false;
            while pos + 4 <= data.len() {
                let h = u32::from_le_bytes(data[pos..pos + 4].try_into()?);
                pos += 4;
                let mut len = (h >> 20) as usize;
                if len == 4095 {
                    let extended = data.get(pos..pos + 4).context("Truncated size")?;
                    len = u32::from_le_bytes(extended.try_into()?) as usize;
                    pos += 4;
                }
                ensure!(pos + len <= data.len(), "Truncated record");
                if h & 1023 == 85
                    && len >= 73
                    && u16::from_le_bytes(data[pos + 71..pos + 73].try_into()?) == image_id
                {
                    data[pos + 68] = contrast as u8;
                    data[pos + 69] = bright as u8;
                    touched = true;
                    changed += 1;
                }
                pos += len;
            }
            if touched {
                let bytes = if compressed {
                    let mut e = flate2::write::DeflateEncoder::new(
                        Vec::new(),
                        flate2::Compression::default(),
                    );
                    e.write_all(&data)?;
                    let mut b = e.finish()?;
                    if !trailer.iter().all(|v| *v == 0) {
                        let size_at =
                            if trailer.len() == 32 && trailer[4..16].iter().all(|v| *v == 0) {
                                16
                            } else {
                                4
                            };
                        ensure!(
                            trailer[size_at + 4..].iter().all(|v| *v == 0),
                            "Unsupported stream trailer"
                        );
                        trailer[..4].copy_from_slice(&crc32fast::hash(&data).to_le_bytes());
                        trailer[size_at..size_at + 4]
                            .copy_from_slice(&(data.len() as u32).to_le_bytes());
                    }
                    b.extend_from_slice(&trailer);
                    b
                } else {
                    data
                };
                let mut stream = file.open_stream(&path)?;
                stream.set_len(0)?;
                stream.write_all(&bytes)?;
            }
        }
        ensure!(changed > 0, "Image ID not found");
        drop(file);
        hwp_engine::model::Document::open(&target)?;
        entries.push(serde_json::json!({"name":name,"brightness":bright,"contrast":contrast,"changedPictures":changed,"input":target,"referencePdf":out.join("pdf").join(format!("{name}.pdf"))}));
    }
    ensure!(std::fs::read(input)? == original, "Source changed");
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_vec_pretty(&entries)?,
    )?;
    Ok(())
}
