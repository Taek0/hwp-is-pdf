//! Side captions, caption notes and textboxes in reflowed table cells.
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
    let para = |id: &str, value: &str| {
        format!(
            r#"<hp:p id="{id}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{value}</hp:t></hp:run></hp:p>"#
        )
    };
    for name in [
        "picture-left",
        "textbox-right",
        "table-side",
        "table-note",
        "table-nested",
    ] {
        let cols = if name == "table-side" { 2 } else { 1 };
        let page = format!(
            r#"<hp:secPr><hp:pagePr width="59528" height="60000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:ctrl><hp:colPr colCount="{cols}" sameGap="2400"/></hp:ctrl>"#
        );
        let side = match name {
            "picture-left" => "LEFT",
            "textbox-right" | "table-side" => "RIGHT",
            _ => "BOTTOM",
        };
        let foot = if name == "table-note" {
            format!(
                r#"<hp:ctrl><hp:footNote number="1"><hp:subList>{}</hp:subList></hp:footNote></hp:ctrl>"#,
                para(
                    "caption-note",
                    "CAPTION-NOTE: 표 아래 캡션의 각주가 본문이나 캡션과 겹치지 않아야 합니다."
                )
            )
        } else {
            String::new()
        };
        let cap = format!(
            r#"<hp:caption side="{side}" width="9000" gap="600"><hp:subList><hp:p id="caption" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>CAPTION-START: {} CAPTION-END</hp:t>{foot}</hp:run></hp:p></hp:subList></hp:caption>"#,
            "옆 캡션과 본문의 영역을 나누어 배치합니다. ".repeat(if name == "table-side" {
                10
            } else {
                3
            })
        );
        let object = if name == "picture-left" {
            format!(
                r#"<hp:pic id="object"><hp:sz width="18000" height="9000"/><hp:pos treatAsChar="1"/><hp:img binaryItemIDRef="mixed-image"/>{cap}</hp:pic>"#
            )
        } else if name == "textbox-right" {
            format!(
                r##"<hp:rect id="object"><hp:sz width="30000" height="5000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><hp:lineShape style="SOLID" width="50" color="#1266CC"/><hp:drawText><hp:textMargin left="600" right="600" top="600" bottom="600"/><hp:subList>{}</hp:subList></hp:drawText>{cap}</hp:rect>"##,
                para(
                    "textbox",
                    "TEXTBOX: 글상자는 내용에 맞추어 높이를 늘립니다. 글상자 뒤 문단을 덮지 않아야 합니다."
                )
            )
        } else {
            let mut rows = String::new();
            for i in 0..42 {
                let nested = if name == "table-nested" && i == 5 {
                    format!(
                        r##"<hp:p id="nested" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:rect id="inner-box"><hp:sz width="18000" height="1800"/><hp:pos treatAsChar="1"/><hp:lineShape style="SOLID" width="40" color="#1266CC"/><hp:drawText><hp:textMargin left="500" right="500" top="500" bottom="500"/><hp:subList>{}</hp:subList></hp:drawText><hp:caption side="RIGHT" width="8000" gap="500"><hp:subList>{}</hp:subList></hp:caption></hp:rect></hp:run></hp:p>"##,
                        para(
                            "inner-text",
                            "INNER-BOX: 셀 내부의 글상자가 원래 높이보다 늘어나도 다음 행을 덮지 않습니다."
                        ),
                        para(
                            "inner-cap",
                            "INNER-CAP: 셀 안의 옆 캡션도 표와 함께 이어집니다."
                        )
                    )
                } else {
                    String::new()
                };
                rows += &format!(
                    r#"<hp:tr><hp:tc borderFillIDRef="0"><hp:cellAddr colAddr="0" rowAddr="{i}"/><hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="35000" height="2200"/><hp:subList>{}{nested}</hp:subList></hp:tc></hp:tr>"#,
                    para(
                        &format!("cell-{i}"),
                        &format!("ROW-{i:02}: 셀 안의 문장을 다시 줄바꿈합니다.")
                    )
                );
            }
            format!(
                r#"<hp:tbl id="object" rowCnt="42" colCnt="1"><hp:sz width="35000" height="92400"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><hp:inMargin left="500" right="500" top="400" bottom="400"/>{rows}{cap}</hp:tbl>"#
            )
        };
        let section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="intro" paraPrIDRef="0"><hp:run charPrIDRef="0">{page}<hp:t>INTRO: 캡션과 셀 내부 배치를 검증합니다.</hp:t></hp:run></hp:p><hp:p id="object-p" paraPrIDRef="0"><hp:run charPrIDRef="0">{object}</hp:run></hp:p>{}</hs:sec>"#,
            para(
                "after",
                "AFTER: 개체와 캡션이 끝난 뒤에 이 문단을 출력합니다."
            )
        );
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/captions-{name}.hwpx"
        ))?);
        for (path, b) in &entries {
            out.start_file(path, zip::write::SimpleFileOptions::default())?;
            out.write_all(if path == "Contents/section0.xml" {
                section.as_bytes()
            } else {
                b
            })?;
        }
        out.finish()?;
    }
    Ok(())
}
