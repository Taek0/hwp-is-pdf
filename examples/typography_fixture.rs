//! Dynamic tab alignment, Arabic/Hebrew shaping and mixed-cell bidi text.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut input =
        zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut entries = Vec::new();
    for i in 0..input.len() {
        let mut f = input.by_index(i)?;
        let mut b = Vec::new();
        f.read_to_end(&mut b)?;
        entries.push((f.name().to_string(), b));
    }
    for name in ["tabs", "rtl", "rtl-table"] {
        let mut additions = String::from(
            r##"<hh:charPr id="30" height="1400" textColor="#000000"><hh:fontRef latin="0" hangul="0"/><hh:underline type="BOTTOM" shape="SOLID" color="#000000"/></hh:charPr><hh:paraPr id="30" textDir="RTL"><hh:align horizontal="RIGHT"/><hh:lineSpacing type="PERCENT" value="170"/></hh:paraPr>"##,
        );
        let mut body = String::new();
        if name == "tabs" {
            for (i, kind) in ["LEFT", "RIGHT", "CENTER", "DECIMAL"].iter().enumerate() {
                let id = 40 + i;
                additions += &format!(
                    r#"<hh:tabPr id="{id}"><hh:tabItem pos="20000" type="{kind}" leader="DOT"/></hh:tabPr><hh:paraPr id="{id}" tabPrIDRef="{id}"><hh:align horizontal="LEFT"/><hh:lineSpacing type="PERCENT" value="180"/></hh:paraPr>"#
                );
                body += &format!(
                    r#"<hp:p id="tab-{kind}" paraPrIDRef="{id}"><hp:run charPrIDRef="0"><hp:t>{kind}<hp:tab width="1000"/>12.50</hp:t></hp:run></hp:p>"#
                );
            }
        } else if name == "rtl" {
            for i in 0..12 {
                let char_style = if i == 0 { 30 } else { 0 };
                body += &format!(
                    r#"<hp:p id="arabic-{i}" paraPrIDRef="30"><hp:run charPrIDRef="{char_style}"><hp:t>RTL-{i}: مرحبا بالعالم لا السلام عليكم 123 English مرحبا بالعالم لا السلام عليكم</hp:t></hp:run></hp:p>"#
                );
            }
            body += r#"<hp:p id="hebrew" paraPrIDRef="30"><hp:run charPrIDRef="0"><hp:t>שָׁלוֹם עולם 456 English</hp:t></hp:run></hp:p><hp:p id="combining" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>Combining: café côté é é é</hp:t></hp:run></hp:p>"#;
        } else {
            let mut rows = String::new();
            for i in 0..30 {
                rows += &format!(
                    r#"<hp:tr><hp:tc borderFillIDRef="0"><hp:cellAddr rowAddr="{i}" colAddr="0"/><hp:cellSpan rowSpan="1" colSpan="1"/><hp:cellSz width="22000" height="2200"/><hp:subList><hp:p id="cell-{i}" paraPrIDRef="30"><hp:run charPrIDRef="0"><hp:t>ROW-{i:02}: مرحبا بالعالم لا السلام عليكم</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr>"#
                );
            }
            body = format!(
                r#"<hp:p id="table" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:tbl id="rtl-table" rowCnt="30" colCnt="1"><hp:sz width="22000" height="66000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><hp:inMargin left="500" right="500" top="400" bottom="400"/>{rows}</hp:tbl></hp:run></hp:p>"#
            );
        }
        let cols = if name == "rtl-table" { 2 } else { 1 };
        let height = if name == "rtl-table" { 45000 } else { 60000 };
        let section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="{height}"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:ctrl><hp:colPr colCount="{cols}" sameGap="2400"/></hp:ctrl><hp:t>문자 조형과 탭 정렬</hp:t></hp:run></hp:p>{body}</hs:sec>"#
        );
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/typography-{name}.hwpx"
        ))?);
        for (path, b) in &entries {
            out.start_file(path, zip::write::SimpleFileOptions::default())?;
            if path == "Contents/section0.xml" {
                out.write_all(section.as_bytes())?;
            } else if path == "Contents/header.xml" {
                out.write_all(
                    String::from_utf8(b.clone())?
                        .replace("</hh:head>", &format!("{additions}</hh:head>"))
                        .as_bytes(),
                )?;
            } else {
                out.write_all(b)?;
            }
        }
        out.finish()?;
    }
    Ok(())
}
