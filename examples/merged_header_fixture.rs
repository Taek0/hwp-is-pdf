//! Reproducible merged-header fixture, not a Hancom reference export.
use std::io::Write;
fn main() -> anyhow::Result<()> {
    let cell = |row: usize, col: usize, span: usize, text: String| {
        format!(
            r#"<tc borderFillIDRef="1" header="{}"><cellAddr rowAddr="{row}" colAddr="{col}"/><cellSpan rowSpan="{span}" colSpan="1"/><cellSz width="15000" height="{}"/><subList><p paraPrIDRef="0"><run charPrIDRef="0"><t>{text}</t></run></p></subList></tc>"#,
            usize::from(row < 2),
            span * 2200
        )
    };
    let mut rows = format!(
        "<tr>{}{}</tr><tr>{}</tr>",
        cell(0, 0, 2, "MERGED".into()),
        cell(0, 1, 1, "TOP".into()),
        cell(1, 1, 1, "LOW".into())
    );
    for i in 2..32 {
        rows += &format!(
            "<tr>{}{}</tr>",
            cell(i, 0, 1, format!("L{i:02}")),
            cell(i, 1, 1, format!("R{i:02}"))
        );
    }
    let section = format!(
        r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="30000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><tbl id="merged-header" rowCnt="32" colCnt="2" repeatHeader="1" pageBreak="TABLE"><sz width="30000" height="70400"/><pos treatAsChar="1" horzRelTo="PARA" vertRelTo="PARA"/><inMargin left="100" right="100" top="100" bottom="100"/>{rows}</tbl></run></p></sec>"#
    );
    let borders = ["leftBorder", "rightBorder", "topBorder", "bottomBorder"]
        .map(|name| format!(r##"<{name} type="SOLID" width="0.1 mm" color="#334455"/>"##))
        .concat();
    let header = format!(
        r##"<head><fontface lang="LATIN"><font id="0" face="Arial"/></fontface><charPr id="0" height="1200" textColor="#123A60"><fontRef latin="0"/></charPr><paraPr id="0"><align horizontal="LEFT"/><lineSpacing type="PERCENT" value="160"/></paraPr><borderFill id="1">{borders}</borderFill></head>"##
    );
    std::fs::create_dir_all("samples/tables-notes")?;
    let mut z = zip::ZipWriter::new(std::fs::File::create(
        "samples/tables-notes/merged-header.hwpx",
    )?);
    for (name, value) in [
        ("mimetype", "application/hwp+zip"),
        ("Contents/header.xml", header.as_str()),
        ("Contents/section0.xml", section.as_str()),
        ("Contents/content.hpf", "<package><manifest/></package>"),
    ] {
        z.start_file(name, zip::write::SimpleFileOptions::default())?;
        z.write_all(value.as_bytes())?;
    }
    z.finish()?;
    Ok(())
}
