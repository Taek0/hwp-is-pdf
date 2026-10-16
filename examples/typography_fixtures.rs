//! Controlled input fixtures; these are not viewer-exported visual references.
use std::{io::Write, path::Path};
fn save(name: &str, chars: &str, paragraphs: &str) -> anyhow::Result<()> {
    let root = Path::new("samples/text-style");
    std::fs::create_dir_all(root)?;
    let header = format!(
        r#"<head><fontface lang="HANGUL"><font id="0" face="맑은 고딕"/></fontface><fontface lang="LATIN"><font id="0" face="Arial"/></fontface><charProperties>{chars}</charProperties><paraPr id="0"><align horizontal="LEFT"/><lineSpacing type="PERCENT" value="160"/></paraPr></head>"#
    );
    let section = format!("<sec>{paragraphs}</sec>");
    let mut z = zip::ZipWriter::new(std::fs::File::create(root.join(format!("{name}.hwpx")))?);
    for (path, value) in [
        ("mimetype", "application/hwp+zip"),
        ("Contents/header.xml", header.as_str()),
        ("Contents/section0.xml", section.as_str()),
        ("Contents/content.hpf", "<package><manifest/></package>"),
    ] {
        z.start_file(path, zip::write::SimpleFileOptions::default())?;
        z.write_all(value.as_bytes())?;
    }
    z.finish()?;
    Ok(())
}
fn main() -> anyhow::Result<()> {
    let sec = r#"<secPr><pagePr width="59528" height="84188"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr>"#;
    let mut chars = String::new();
    for (id, size) in [(0, 1200), (1, 800), (2, 2000)] {
        chars += &format!(
            r##"<charPr id="{id}" height="{size}" textColor="#000000"><fontRef hangul="0" latin="0"/></charPr>"##
        );
    }
    save(
        "empty-paragraphs",
        &chars,
        &format!(
            r#"<p id="before" paraPrIDRef="0"><run charPrIDRef="0">{sec}<t>Declared empty paragraph size / 빈 문단 크기</t></run></p><p paraPrIDRef="0"><run charPrIDRef="1"><t/></run></p><p id="after-small" paraPrIDRef="0"><run charPrIDRef="0"><t>After 8pt empty paragraph: advance 12.8pt</t></run></p><p paraPrIDRef="0"><run charPrIDRef="2"><t/></run></p><p id="after-large" paraPrIDRef="0"><run charPrIDRef="0"><t>After 20pt empty paragraph: advance 32pt</t></run></p>"#
        ),
    )?;
    let marks = [
        "NONE",
        "DOT_ABOVE",
        "RING_ABOVE",
        "TILDE",
        "CARON",
        "SIDE",
        "COLON",
        "GRAVE_ACCENT",
        "ACUTE_ACCENT",
        "CIRCUMFLEX",
        "MACRON",
        "HOOK_ABOVE",
        "DOT_BELOW",
    ];
    let mut chars=r##"<charPr id="label" height="1100" textColor="#666666"><fontRef hangul="0" latin="0"/></charPr>"##.to_string();
    let mut rows = String::new();
    for (i, mark) in marks.iter().enumerate() {
        chars += &format!(
            r##"<charPr id="{i}" height="2000" textColor="#123A60" symMark="{mark}"><fontRef hangul="0" latin="0"/></charPr>"##
        );
        rows += &format!(
            r#"<p id="row-{i}" paraPrIDRef="0"><run charPrIDRef="label">{}<t>{mark}   </t></run><run charPrIDRef="{i}"><t>한글 Aa</t></run><linesegarray><lineseg textpos="0" vertpos="{}" vertsize="2600" baseline="2100" horzpos="0" horzsize="49000" spacing="1600"/></linesegarray></p>"#,
            if i == 0 { sec } else { "" },
            i * 5000
        );
    }
    save("emphasis-marks", &chars, &rows)
}
