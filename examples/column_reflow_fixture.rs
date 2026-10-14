//! Mixed tables, pictures and notes flowing through two/three equal columns.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut source =
        zip::ZipArchive::new(std::fs::File::open("samples/reflow/mixed-no-cache.hwpx")?)?;
    let mut entries = Vec::new();
    for i in 0..source.len() {
        let mut f = source.by_index(i)?;
        let mut b = Vec::new();
        f.read_to_end(&mut b)?;
        entries.push((f.name().to_string(), b));
    }
    for (name, cols, width) in [
        ("mixed-two-column", 2, 84188),
        ("mixed-three-column", 3, 119056),
    ] {
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/{name}.hwpx"
        ))?);
        for (path, bytes) in &entries {
            out.start_file(path, zip::write::SimpleFileOptions::default())?;
            if path == "Contents/section0.xml" {
                let s=String::from_utf8(bytes.clone())?.replace("width=\"59528\"",&format!("width=\"{width}\""))
                    .replace("</hp:secPr>",&format!("</hp:secPr><hp:ctrl><hp:colPr colCount=\"{cols}\" sameGap=\"2400\"/></hp:ctrl>"));
                out.write_all(s.as_bytes())?;
            } else {
                out.write_all(bytes)?;
            }
        }
        out.finish()?;
    }
    Ok(())
}
