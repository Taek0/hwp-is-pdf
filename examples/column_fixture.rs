//! Unequal-width, reading-order, definition-transition and shared-note fixtures.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut entries = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        entries.push((f.name().to_string(), bytes));
    }
    for name in ["unequal", "transition", "right", "merged", "balanced"] {
        let layout = if name == "right" { "RIGHT" } else { "LEFT" };
        let kind = if name == "balanced" {
            "BALANCED_NEWSPAPER"
        } else {
            "NEWSPAPER"
        };
        let note = if name == "merged" {
            r#"<hp:footNotePr><hp:placement place="MERGED_COLUMN"/></hp:footNotePr>"#
        } else {
            ""
        };
        let def = format!(
            r##"<hp:ctrl><hp:colPr colCount="2" sameSz="0" type="{kind}" layout="{layout}"><hp:colSz width="15000" gap="2400"/><hp:colSz width="32128" gap="0"/><hp:colLine type="SOLID" color="#999999"/></hp:colPr></hp:ctrl>"##
        );
        let page = format!(
            r#"<hp:secPr><hp:pagePr width="59528" height="60000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr>{note}</hp:secPr>{def}"#
        );
        let mut section = String::from(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">"#,
        );
        for i in 0..if name == "balanced" { 3 } else { 10 } {
            let control = if i == 0 {
                page.clone()
            } else if name == "transition" && i == 4 {
                r#"<hp:ctrl><hp:colPr colCount="1" sameGap="0"/></hp:ctrl>"#.into()
            } else {
                String::new()
            };
            let body="서로 다른 단의 너비에 맞추어 남은 문장을 다시 배치합니다. All characters must remain in source order. ".repeat(4);
            let foot = if name == "merged" && i == 5 {
                r#"<hp:ctrl><hp:footNote number="1"><hp:subList><hp:p id="note" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>NOTE-1: 뒤 단의 각주도 앞 단 본문과 겹치지 않아야 합니다. 공통 각주 영역을 확인합니다.</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl>"#
            } else {
                ""
            };
            section += &format!(
                r#"<hp:p id="body-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0">{control}<hp:t>BEGIN-{i} {body} END-{i}</hp:t>{foot}</hp:run></hp:p>"#
            );
        }
        section += "</hs:sec>";
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/columns-{name}.hwpx"
        ))?);
        for (path, bytes) in &entries {
            out.start_file(path, zip::write::SimpleFileOptions::default())?;
            out.write_all(if path == "Contents/section0.xml" {
                section.as_bytes()
            } else {
                bytes
            })?;
        }
        out.finish()?;
    }
    Ok(())
}
