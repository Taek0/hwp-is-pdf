//! Create two logical-identical HWPX copies with different saved line caches.
//! Use a complete source package so the probes can also be opened in Hancom.
use anyhow::{Context, Result, ensure};
use std::{
    io::{Read, Write},
    path::Path,
};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        a.len() == 4 || (a.len() == 5 && a[4] == "--bottom-margin-probe"),
        "INPUT.hwpx OUTPUT_DIR PARA_ID CHAR_ID [--bottom-margin-probe]"
    );
    let vary_bottom = a.len() == 5;
    let original = std::fs::read(&a[0])?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&original))?;
    let mut files = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        if f.is_dir() {
            continue;
        }
        let mut b = Vec::new();
        f.read_to_end(&mut b)?;
        files.push((f.name().to_owned(), b));
    }
    let bytes = &files
        .iter()
        .find(|(n, _)| n == "Contents/section0.xml")
        .context("Section0")?
        .1;
    let source = std::str::from_utf8(bytes)?;
    let xml = roxmltree::Document::parse(source)?;
    let root = xml.root_element();
    let root_text = &source[root.range()];
    let root_open = &root_text[..root_text.find('>').context("Root start")? + 1];
    let sec = root
        .descendants()
        .find(|n| n.has_tag_name("secPr"))
        .context("Section properties")?;
    let page = sec
        .children()
        .find(|n| n.has_tag_name("pagePr"))
        .context("Page properties")?;
    let mut page_text = source[page.range()].to_owned();
    for (name, new) in [("width", "40000"), ("height", "30000")] {
        page_text = page_text.replace(
            &format!(
                "{name}=\"{}\"",
                page.attribute(name).context("Page dimension")?
            ),
            &format!("{name}=\"{new}\""),
        );
    }
    let margin = page
        .children()
        .find(|n| n.has_tag_name("margin"))
        .context("Margin")?;
    let mut margin_text = source[margin.range()].to_owned();
    for (name, new) in [
        ("left", "4000"),
        ("right", "4000"),
        ("top", "4000"),
        ("bottom", if vary_bottom { "8000" } else { "4000" }),
        ("header", "0"),
        ("footer", "0"),
    ] {
        margin_text = margin_text.replace(
            &format!(
                "{name}=\"{}\"",
                margin.attribute(name).context("Margin attribute")?
            ),
            &format!("{name}=\"{new}\""),
        );
    }
    page_text = page_text.replace(&source[margin.range()], &margin_text);
    let properties = source[sec.range()].replace(&source[page.range()], &page_text);
    let dir = Path::new(&a[1]);
    std::fs::create_dir_all(dir)?;
    for stalled in [false, true] {
        if vary_bottom && stalled {
            continue;
        }
        let name = if vary_bottom {
            "bottom-margin-80"
        } else if stalled {
            "stalled-cache"
        } else {
            "normal-cache"
        };
        let path = dir.join(format!("{name}.hwpx"));
        ensure!(!path.exists(), "Probe exists");
        let mut section = root_open.to_owned();
        for i in 0..32 {
            let y = if stalled {
                (i % 17).min(15) * 1600
            } else {
                (i % 14) * 1600
            };
            let controls = if i == 0 {
                format!(
                    r#"{properties}<hp:ctrl><hp:colPr id="" type="NEWSPAPER" layout="LEFT" colCount="2" sameSz="1" sameGap="2000"/></hp:ctrl>"#
                )
            } else {
                String::new()
            };
            let text = match i {
                0 => "A START",
                18 => "B AFTER BLANKS",
                30 => "C NEXT PAGE",
                _ => "",
            };
            section += &format!(
                r#"<hp:p id="{}" paraPrIDRef="{}" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="{}">{controls}<hp:t>{text}</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="{y}" vertsize="1000" textheight="1000" baseline="850" spacing="600" horzpos="0" horzsize="15000" flags="393216"/></hp:linesegarray></hp:p>"#,
                i + 1000,
                a[2],
                a[3]
            );
        }
        section += "</hs:sec>";
        let mut out = zip::ZipWriter::new(std::fs::File::create(&path)?);
        for (filename, data) in &files {
            if filename.starts_with("Preview/") {
                continue;
            }
            out.start_file(filename, zip::write::SimpleFileOptions::default())?;
            out.write_all(if filename == "Contents/section0.xml" {
                section.as_bytes()
            } else {
                data
            })?;
        }
        out.finish()?;
        hwp_engine::model::Document::open(&path)?;
        println!("{}", path.display());
    }
    ensure!(std::fs::read(&a[0])? == original, "Source changed");
    Ok(())
}
