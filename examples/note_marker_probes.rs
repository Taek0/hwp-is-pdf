//! Prepare a complete note-marker reference batch without changing the source.
use anyhow::{Context, Result, ensure};
use std::{
    io::{Read, Write},
    path::Path,
};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        a.len() == 4,
        "INPUT.hwpx OUTPUT_DIR MARKER_CHAR_ID NUMBERED_PARA_ID"
    );
    let marker_id = a[2].parse::<u32>()?.to_string();
    let para_id = a[3].parse::<u32>()?.to_string();
    let original = std::fs::read(&a[0])?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&original))?;
    let mut files = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        if f.is_dir() || f.name().starts_with("Preview/") {
            continue;
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        files.push((f.name().to_owned(), data));
    }
    let header = std::str::from_utf8(
        &files
            .iter()
            .find(|(n, _)| n == "Contents/header.xml")
            .context("Header")?
            .1,
    )?;
    let tree = roxmltree::Document::parse(header)?;
    let marker = tree
        .descendants()
        .find(|n| n.has_tag_name("charPr") && n.attribute("id") == Some(marker_id.as_str()))
        .context("Marker style")?;
    let para = tree
        .descendants()
        .find(|n| n.has_tag_name("paraPr") && n.attribute("id") == Some(para_id.as_str()))
        .context("Numbered paragraph style")?;
    let marker_xml = &header[marker.range()];
    let para_xml = &header[para.range()];
    let out = Path::new(&a[1]);
    std::fs::create_dir_all(out)?;
    for (name, large, superscript, no_number) in [
        ("01_math_black_markers", false, false, false),
        ("02_math_black_markers_14pt", true, false, false),
        ("03_math_black_markers_superscript", false, true, false),
        ("04_math_black_markers_no_list_numbers", false, false, true),
    ] {
        let mut m = marker_xml.replace(
            &format!(
                "textColor=\"{}\"",
                marker.attribute("textColor").context("Marker color")?
            ),
            "textColor=\"#000000\"",
        );
        if large {
            m = m.replace(
                &format!(
                    "height=\"{}\"",
                    marker.attribute("height").context("Marker height")?
                ),
                "height=\"1400\"",
            );
        }
        let mut h = header.replace(marker_xml, &m);
        if no_number {
            ensure!(
                para_xml.contains("type=\"NUMBER\""),
                "Expected numbered paragraph"
            );
            h = h.replace(
                para_xml,
                &para_xml.replace("type=\"NUMBER\"", "type=\"NONE\""),
            );
        }
        let target = out.join(format!("{name}.hwpx"));
        ensure!(!target.exists(), "Output already exists");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&target)?);
        let mut changed = 0;
        for (path, data) in &files {
            let mut content = if path == "Contents/header.xml" {
                h.as_bytes().to_vec()
            } else {
                data.clone()
            };
            if superscript && path.starts_with("Contents/section") && path.ends_with(".xml") {
                let text = std::str::from_utf8(&content)?;
                let doc = roxmltree::Document::parse(text)?;
                let mut ranges = doc
                    .descendants()
                    .filter(|n| n.has_tag_name("endNotePr"))
                    .filter_map(|n| n.children().find(|c| c.has_tag_name("autoNumFormat")))
                    .map(|n| n.range())
                    .collect::<Vec<_>>();
                let mut result = text.to_owned();
                ranges.sort_by_key(|r| std::cmp::Reverse(r.start));
                for range in ranges {
                    let replacement =
                        text[range.clone()].replace("supscript=\"0\"", "supscript=\"1\"");
                    ensure!(
                        replacement != text[range.clone()],
                        "Expected normal endnote marker"
                    );
                    result.replace_range(range, &replacement);
                    changed += 1;
                }
                content = result.into_bytes();
            }
            writer.start_file(path, zip::write::SimpleFileOptions::default())?;
            writer.write_all(&content)?;
        }
        ensure!(
            !superscript || changed > 0,
            "Endnote configuration not found"
        );
        writer.finish()?;
        hwp_engine::model::Document::open(&target)?;
        println!("{}", target.display());
    }
    ensure!(std::fs::read(&a[0])? == original, "Source changed");
    Ok(())
}
