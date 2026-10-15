//! Opposing paragraph-leading objects with different heights.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    for (source_name, name) in [
        ("float-flow-single", "paired-float-single"),
        ("float-flow-columns", "paired-float-columns"),
    ] {
        let mut source = zip::ZipArchive::new(std::fs::File::open(format!(
            "samples/reflow/{source_name}.hwpx"
        ))?)?;
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/{name}.hwpx"
        ))?);
        for i in 0..source.len() {
            let mut f = source.by_index(i)?;
            let mut bytes = vec![];
            f.read_to_end(&mut bytes)?;
            out.start_file(f.name(), zip::write::SimpleFileOptions::default())?;
            if f.name() == "Contents/section0.xml" {
                let mut s = String::from_utf8(bytes)?;
                let start = s.find("<hp:pic id=\"object-0\"").unwrap();
                let end = start + s[start..].find("</hp:pic>").unwrap() + "</hp:pic>".len();
                let left = s[start..end]
                    .replace("width=\"12000\"", "width=\"8000\"")
                    .replace("height=\"14000\"", "height=\"9000\"");
                let right = r##"<hp:rect id="paired-right" textWrap="SQUARE"><hp:sz width="8500" height="19000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzAlign="RIGHT" vertAlign="TOP"/><hp:outMargin left="500" right="500" top="500" bottom="500"/><hp:lineShape style="SOLID" width="35" color="#0055AA"/><hp:drawText><hp:textMargin left="400" right="400" top="400" bottom="400"/><hp:subList><hp:p id="pair-text" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>오른쪽 개체</hp:t></hp:run></hp:p></hp:subList></hp:drawText></hp:rect>"##;
                s.replace_range(start..end, &(left + right));
                out.write_all(s.as_bytes())?;
            } else {
                out.write_all(&bytes)?;
            }
        }
        out.finish()?;
    }
    Ok(())
}
