use std::io::{Read, Write};
fn para(id: usize, y: usize, text: &str, object: &str) -> String {
    format!(
        r#"<hp:p id="{id}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{text}</hp:t>{object}</hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="{y}" vertsize="1600" textheight="1200" baseline="1200" spacing="0" horzpos="0" horzsize="45000" flags="393216"/></hp:linesegarray></hp:p>"#
    )
}
fn table(id: &str, body: &str, height: usize) -> String {
    format!(
        r#"<hp:tbl id="{id}" rowCnt="1" colCnt="1" pageBreak="CELL"><hp:sz width="35000" height="{height}"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzAlign="LEFT" vertAlign="TOP"/><hp:tr><hp:tc hasMargin="1" borderFillIDRef="1"><hp:cellAddr rowAddr="0" colAddr="0"/><hp:cellSpan rowSpan="1" colSpan="1"/><hp:cellSz width="35000" height="{height}"/><hp:cellMargin left="500" right="500" top="500" bottom="500"/><hp:subList vertAlign="TOP">{body}</hp:subList></hp:tc></hp:tr></hp:tbl>"#
    )
}
fn main() -> anyhow::Result<()> {
    let mut z = zip::ZipArchive::new(std::fs::File::open("samples/text-style/decorations.hwpx")?)?;
    let mut header = String::new();
    z.by_name("Contents/header.xml")?
        .read_to_string(&mut header)?;
    header = header.replace("height=\"1400\"", "height=\"1200\"");
    // Use a plain style; the source decoration fixture's first style is underlined.
    header = header.replace(
        r##"<hh:underline type="BOTTOM" shape="SOLID" color="#000000"/>"##,
        "",
    );
    let borders = r##"<hh:borderFill id="1"><hh:leftBorder type="SOLID" width="0.1 mm" color="#000000"/><hh:rightBorder type="SOLID" width="0.1 mm" color="#000000"/><hh:topBorder type="SOLID" width="0.1 mm" color="#000000"/><hh:bottomBorder type="SOLID" width="0.1 mm" color="#000000"/></hh:borderFill>"##;
    header = header.replace("</hh:head>", &format!("{borders}</hh:head>"));
    let sec = r#"<hp:secPr><hp:pagePr width="59528" height="84188"><hp:margin left="5000" right="5000" top="5000" bottom="5000" header="0" footer="0"/></hp:pagePr></hp:secPr>"#;
    let note = (1..=90)
        .map(|i| {
            para(
                i,
                (i - 1) * 1600,
                &format!("각주 {i:03}: 다음 페이지에도 이 줄이 보존되어야 합니다."),
                "",
            )
        })
        .collect::<String>();
    let ctrl = format!(
        r#"<hp:ctrl><hp:footNote number="1" id="long-note"><hp:subList>{note}</hp:subList></hp:footNote></hp:ctrl>"#
    );
    let next = para(200, 0, "각주와 함께 이어지는 본문입니다.", "").replacen(
        "paraPrIDRef=\"0\"",
        "paraPrIDRef=\"0\" pageBreak=\"1\"",
        1,
    );
    let notes = format!(
        "{}{}",
        para(0, 0, "긴 각주의 본문 참조", &format!("{sec}{ctrl}")),
        next
    );
    let inner = table(
        "inner",
        &format!(
            "{}{}",
            para(2, 0, "중첩 표 첫 줄", ""),
            para(3, 96000, "중첩 표 마지막 줄", "")
        ),
        98000,
    )
    .replace("width=\"35000\"", "width=\"33000\"");
    let outer = table("outer", &para(1, 0, "", &inner), 100000);
    let nested = format!(
        "{}{}",
        para(0, 0, "", &format!("{sec}{outer}")),
        para(4, 2000, "큰 표 다음 본문입니다.", "")
    );
    let rect = format!(
        r##"<hp:rect id="oversized-rect"><hp:sz width="30000" height="100000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><hp:lineShape style="SOLID" width="30" color="#1266CC"/><hp:drawText><hp:subList>{}{}</hp:subList></hp:drawText></hp:rect>"##,
        para(1, 0, "큰 개체의 첫 줄", ""),
        para(2, 98000, "큰 개체의 마지막 줄", "")
    );
    let object = format!(
        "{}{}",
        para(0, 0, "", &format!("{sec}{rect}")),
        para(3, 2000, "큰 개체 다음 본문입니다.", "")
    );
    std::fs::create_dir_all("samples/overflow")?;
    let mut rows = String::new();
    for row in 0..60 {
        rows += "<hp:tr>";
        for col in 0..2 {
            if col == 0 && row % 2 == 1 {
                continue;
            }
            let span = if col == 0 { 2 } else { 1 };
            let label = if col == 0 {
                format!("병합 M{:03}", row / 2 + 1)
            } else {
                format!("행 R{:03}", row + 1)
            };
            rows += &format!(
                r#"<hp:tc hasMargin="1" borderFillIDRef="1"><hp:cellAddr rowAddr="{row}" colAddr="{col}"/><hp:cellSpan rowSpan="{span}" colSpan="1"/><hp:cellSz width="16500" height="{}"/><hp:cellMargin left="400" right="400" top="300" bottom="300"/><hp:subList vertAlign="TOP">{}</hp:subList></hp:tc>"#,
                span * 2200,
                para(row + col * 100, 0, &label, "")
            );
        }
        rows += "</hp:tr>";
    }
    let inner_rows = format!(
        r#"<hp:tbl id="nested-rows" rowCnt="60" colCnt="2" pageBreak="CELL"><hp:sz width="33000" height="132000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/>{rows}</hp:tbl>"#
    );
    let outer_rows = table("outer-rows", &para(1, 0, "", &inner_rows), 134000);
    let merged = format!(
        "{}{}",
        para(0, 0, "", &format!("{sec}{outer_rows}")),
        para(500, 2000, "분할 표 다음 본문입니다.", "")
    );
    let mut shifted = rows.clone();
    for row in (0..60).rev() {
        shifted = shifted.replace(
            &format!("rowAddr=\"{row}\""),
            &format!("rowAddr=\"{}\"", row + 1),
        );
    }
    let heading = format!(
        r#"<hp:tr><hp:tc hasMargin="1" borderFillIDRef="1"><hp:cellAddr rowAddr="0" colAddr="0"/><hp:cellSpan rowSpan="1" colSpan="2"/><hp:cellSz width="33000" height="2200"/><hp:cellMargin left="400" right="400" top="300" bottom="300"/><hp:subList>{}</hp:subList></hp:tc></hp:tr>"#,
        para(900, 0, "반복 머리행: 항목 안내", "")
    );
    let repeat = format!(
        r#"<hp:tbl id="repeat-inner" repeatHeader="1" rowCnt="61" colCnt="2" pageBreak="CELL"><hp:sz width="33000" height="134200"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/>{heading}{shifted}</hp:tbl>"#
    );
    let captions = format!(
        r#"<hp:caption side="TOP" gap="600"><hp:subList>{}</hp:subList></hp:caption><hp:caption side="BOTTOM" gap="600"><hp:subList>{}</hp:subList></hp:caption>"#,
        para(901, 0, "위 캡션: 중첩 표 분할 검증", ""),
        para(902, 0, "아래 캡션: 마지막 행 다음에 한 번만 표시", "")
    );
    let caption_table = table("caption-outer", &para(1, 0, "", &repeat), 138000).replacen(
        "<hp:tr>",
        &format!("{captions}<hp:tr>"),
        1,
    );
    let repeat_caption = format!(
        "{}{}",
        para(0, 0, "", &format!("{sec}{caption_table}")),
        para(500, 2000, "캡션까지 끝난 뒤 이어지는 본문입니다.", "")
    );
    let picture_object = r#"<hp:pic id="mixed-picture"><hp:sz width="10000" height="10000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzOffset="400" vertOffset="400"/><hp:img binaryItemIDRef="mixed-image"/></hp:pic>"#;
    let rotation_object = format!(
        r##"<hp:rect id="mixed-rotation"><hp:sz width="9000" height="7000"/><hp:curSz width="8000" height="4000"/><hp:rotationInfo angle="25"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzOffset="400" vertOffset="400"/><hp:lineShape style="SOLID" width="40" color="#0055AA"/><hp:drawText><hp:textMargin left="500" right="500" top="500" bottom="500"/><hp:subList>{}</hp:subList></hp:drawText></hp:rect>"##,
        para(2000, 0, "회전 개체", "")
    );
    let note_body = (1..=12)
        .map(|i| {
            para(
                3000 + i,
                (i - 1) * 1600,
                &format!("각주 N{i:03}: 표 안의 기준 행과 같은 페이지에 표시합니다."),
                "",
            )
        })
        .collect::<String>();
    let note_control = format!(
        r#"<hp:ctrl><hp:footNote number="1" id="mixed-note"><hp:subList>{note_body}</hp:subList></hp:footNote></hp:ctrl>"#
    );
    let mut mixed_rows = heading.clone();
    for row in 1..=60 {
        mixed_rows += "<hp:tr>";
        for col in 0..2 {
            if col == 0 && ((29..=33).contains(&row) || (43..=45).contains(&row)) {
                continue;
            }
            let span = if col == 0 && row == 28 {
                6
            } else if col == 0 && row == 42 {
                4
            } else {
                1
            };
            let content = if col == 0 && row == 28 {
                para(1000 + row, 0, "", picture_object)
            } else if col == 0 && row == 42 {
                para(1000 + row, 0, "", &rotation_object)
            } else {
                para(
                    1000 + row + col * 100,
                    0,
                    &format!("{} {:03}", if col == 0 { "셀 L" } else { "행 R" }, row),
                    if col == 1 && row == 31 {
                        &note_control
                    } else {
                        ""
                    },
                )
            };
            mixed_rows += &format!(
                r#"<hp:tc hasMargin="1" borderFillIDRef="1"><hp:cellAddr rowAddr="{row}" colAddr="{col}"/><hp:cellSpan rowSpan="{span}" colSpan="1"/><hp:cellSz width="16500" height="{}"/><hp:cellMargin left="400" right="400" top="300" bottom="300"/><hp:subList vertAlign="TOP">{content}</hp:subList></hp:tc>"#,
                span * 2200
            );
        }
        mixed_rows += "</hp:tr>";
    }
    let mixed_inner = format!(
        r#"<hp:tbl id="mixed-inner" repeatHeader="1" rowCnt="61" colCnt="2" pageBreak="CELL"><hp:sz width="33000" height="134200"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/>{mixed_rows}</hp:tbl>"#
    );
    let mixed_outer = table("mixed-outer", &para(1, 0, "", &mixed_inner), 138000).replacen(
        "<hp:tr>",
        &format!("{captions}<hp:tr>"),
        1,
    );
    let mixed = format!(
        "{}{}",
        para(0, 0, "", &format!("{sec}{mixed_outer}")),
        para(500, 2000, "그림 회전 각주 표 다음 본문입니다.", "")
    );
    let colors = [[220, 45, 45], [35, 165, 75], [30, 95, 210], [240, 195, 35]];
    let bitmap = image::RgbImage::from_fn(100, 100, |x, y| {
        image::Rgb(colors[(x / 50 + y / 50 * 2) as usize])
    });
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(bitmap).write_to(&mut png, image::ImageFormat::Png)?;
    for (name, body) in [
        ("long-footnote", notes),
        ("oversized-nested-table", nested),
        ("oversized-object", object),
        ("nested-merged-rows", merged),
        ("nested-repeat-caption", repeat_caption),
        ("nested-mixed", mixed),
    ] {
        let section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">{body}</hs:sec>"#
        );
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/overflow/{name}.hwpx"
        ))?);
        let manifest = if name == "nested-mixed" {
            r#"<package><manifest><item id="mixed-image" href="BinData/mixed.png"/></manifest></package>"#
        } else {
            "<package><manifest/></package>"
        };
        for (p, s) in [
            ("mimetype", "application/hwp+zip"),
            ("Contents/header.xml", header.as_str()),
            ("Contents/section0.xml", section.as_str()),
            ("Contents/content.hpf", manifest),
        ] {
            out.start_file(p, zip::write::SimpleFileOptions::default())?;
            out.write_all(s.as_bytes())?;
        }
        if name == "nested-mixed" {
            out.start_file(
                "BinData/mixed.png",
                zip::write::SimpleFileOptions::default(),
            )?;
            out.write_all(png.get_ref())?;
        }
        out.finish()?;
    }
    Ok(())
}
