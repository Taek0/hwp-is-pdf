//! BinData table indices remain stable when physical stream IDs have gaps.
use hwp_engine::model::Document;
use std::io::{Read, Write};

#[test]
fn storage_ids_are_not_picture_reference_indices() -> anyhow::Result<()> {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/additional/sample-5017-pics.hwp");
    let original = Document::open(&source)?;
    assert!(!original.binaries.is_empty());
    let path = std::env::temp_dir().join(format!("hwp-bindata-{}.hwp", std::process::id()));
    std::fs::copy(source, &path)?;
    {
        let mut f = cfb::open_rw(&path)?;
        let names: Vec<_> = f
            .walk()
            .filter(|e| e.is_stream())
            .map(|e| e.path().to_string_lossy().to_string())
            .filter(|s| s.starts_with("/BinData/"))
            .collect();
        for name in names {
            let id = u16::from_str_radix(&name[12..16], 16)?;
            let mut raw = vec![];
            f.open_stream(&name)?.read_to_end(&mut raw)?;
            let target = format!("{}{:04X}{}", &name[..12], id + 100, &name[16..]);
            f.create_stream(target)?.write_all(&raw)?;
            f.remove_stream(name)?;
        }
        let mut compressed = vec![];
        f.open_stream("/DocInfo")?.read_to_end(&mut compressed)?;
        let mut data = vec![];
        flate2::read::DeflateDecoder::new(compressed.as_slice()).read_to_end(&mut data)?;
        let mut pos = 0;
        let mut changed = 0;
        while pos + 4 <= data.len() {
            let h = u32::from_le_bytes(data[pos..pos + 4].try_into()?);
            pos += 4;
            let mut len = (h >> 20) as usize;
            if len == 4095 {
                len = u32::from_le_bytes(data[pos..pos + 4].try_into()?) as usize;
                pos += 4;
            }
            if h & 1023 == 18 && data[pos] & 0xf != 0 {
                let id = u16::from_le_bytes(data[pos + 2..pos + 4].try_into()?);
                data[pos + 2..pos + 4].copy_from_slice(&(id + 100).to_le_bytes());
                changed += 1;
            }
            pos += len;
        }
        assert!(changed > 0);
        let mut encoder =
            flate2::write::DeflateEncoder::new(vec![], flate2::Compression::default());
        encoder.write_all(&data)?;
        let encoded = encoder.finish()?;
        let mut stream = f.open_stream("/DocInfo")?;
        stream.set_len(0)?;
        stream.write_all(&encoded)?;
    }
    let modified = Document::open(&path)?;
    std::fs::remove_file(path)?;
    assert_eq!(original.binaries, modified.binaries);
    assert_eq!(original.sections, modified.sections);
    Ok(())
}
