//! Derive reproducible cached/cacheless mixed reflow fixtures from the mixed-table sample.
use std::io::{Read, Write};
fn p(id: &str, text: &str, extra: &str) -> String {
    format!(
        r#"<hp:p id="{id}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{text}</hp:t>{extra}</hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1600" textheight="1200" baseline="1200" spacing="0" horzpos="0" horzsize="49500"/></hp:linesegarray></hp:p>"#
    )
}
fn main() -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open("samples/overflow/nested-mixed.hwpx")?)?;
    let mut entries = std::collections::BTreeMap::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let mut bytes = vec![];
        f.read_to_end(&mut bytes)?;
        entries.insert(f.name().to_string(), bytes);
    }
    let mut section = String::from_utf8(entries["Contents/section0.xml"].clone())?;
    section=section.replace("<hp:t>셀 L 001</hp:t>",&format!("<hp:t>셀 L 001: CELL-START {} CELL-END</hp:t>","표 셀도 저장된 줄 배치 없이 너비에 맞추어 다시 줄바꿈합니다. English words remain readable. ".repeat(4)));
    section = section.replace(
        "<hp:footNote number=\"1\" id=\"mixed-note\">",
        "<hp:footNote number=\"2\" id=\"mixed-note\">",
    );
    let body_note = (1..=4)
        .map(|i| {
            p(
                &format!("bn{i}"),
                &format!(
                    "본문 각주 B{i:03}: {}",
                    "각주 본문도 사용 가능한 너비에 맞춰 자동으로 줄바꿈합니다. ".repeat(3)
                ),
                "",
            )
        })
        .collect::<String>();
    let control = format!(
        r#"<hp:ctrl><hp:footNote number="1" id="body-note"><hp:subList>{body_note}</hp:subList></hp:footNote></hp:ctrl>"#
    );
    let intro=p("intro",&format!("INTRO-START {}","본문과 표, 그림과 각주가 함께 있는 문서의 재배치 예제입니다. 긴 본문은 글꼴과 용지 너비를 기준으로 다시 계산합니다. ".repeat(3)),&control);
    let inline = r#"<hp:pic id="inline-body-picture"><hp:sz width="5000" height="3500"/><hp:pos treatAsChar="1" horzRelTo="PARA" vertRelTo="PARA"/><hp:outMargin left="200" right="200" top="100" bottom="100"/><hp:img binaryItemIDRef="mixed-image"/></hp:pic>"#;
    let inline_p = p(
        "inline",
        "인라인 그림 앞의 본문입니다. ",
        &format!("{inline}<hp:t> 그림 뒤의 본문도 같은 흐름에서 줄바꿈합니다. INLINE-END</hp:t>"),
    );
    let start = section.find('>').unwrap() + 1;
    section.insert_str(start, &format!("{intro}{inline_p}"));
    section = section.replace(
        "그림 회전 각주 표 다음 본문입니다.",
        &format!(
            "{} BODY-END",
            "표와 캡션 뒤의 본문입니다. 마지막 문단도 앞선 표의 실제 높이를 반영해 이어집니다. "
                .repeat(4)
        ),
    );
    for (name, cache) in [("mixed-cached", true), ("mixed-no-cache", false)] {
        let mut content = section.clone();
        if !cache {
            while let Some(start) = content.find("<hp:linesegarray>") {
                let end = start
                    + content[start..].find("</hp:linesegarray>").unwrap()
                    + "</hp:linesegarray>".len();
                content.replace_range(start..end, "");
            }
        }
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/{name}.hwpx"
        ))?);
        for (path, data) in &entries {
            out.start_file(path, zip::write::SimpleFileOptions::default())?;
            out.write_all(if path == "Contents/section0.xml" {
                content.as_bytes()
            } else {
                data
            })?;
        }
        out.finish()?;
    }
    Ok(())
}
