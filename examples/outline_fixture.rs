//! Engine-only outline fixture; use hancom_format_probes for Hancom printing.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    eprintln!(
        "Engine-only fixture; not a complete Hancom document. Use hancom_format_probes for printing."
    );
    std::fs::create_dir_all("output/format-probes")?;
    let mut input =
        zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut styles = String::new();
    let mut body = String::new();
    for (i, kind) in [
        "NONE",
        "SOLID",
        "DOT",
        "BOLD",
        "DASH",
        "DASH_DOT",
        "DASH_DOT_DOT",
    ]
    .iter()
    .enumerate()
    {
        let id = 100 + i;
        styles += &format!(
            r##"<hh:charPr id="{id}" height="3200" textColor="#185890"><hh:fontRef hangul="0" latin="0"/><hh:outline type="{kind}"/></hh:charPr>"##
        );
        body += &format!(
            r#"<hp:p id="outline-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{kind} </hp:t></hp:run><hp:run charPrIDRef="{id}"><hp:t>가나다 ABC 123</hp:t></hp:run><hp:run charPrIDRef="0"><hp:t> 보통 글자</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="{}" vertsize="4000" textheight="3200" baseline="3200" spacing="2000" horzpos="0" horzsize="50000" flags="393216"/></hp:linesegarray></hp:p>"#,
            6000 + i * 8500
        );
    }
    let section = format!(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="84188"><hp:margin left="4000" right="4000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:t>글자 외곽선 · 검색 및 복사 유지</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1400" baseline="1100" horzpos="0" horzsize="51528"/></hp:linesegarray></hp:p>{body}</hs:sec>"#
    );
    let mut output = zip::ZipWriter::new(std::fs::File::create(
        "output/format-probes/text-outlines.hwpx",
    )?);
    for i in 0..input.len() {
        let mut entry = input.by_index(i)?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        if entry.name() == "Contents/header.xml" {
            bytes = String::from_utf8(bytes)?
                .replace(
                    "</hh:charProperties>",
                    &format!("{styles}</hh:charProperties>"),
                )
                .into_bytes();
        }
        output.start_file(entry.name(), zip::write::SimpleFileOptions::default())?;
        output.write_all(if entry.name() == "Contents/section0.xml" {
            section.as_bytes()
        } else {
            &bytes
        })?;
    }
    output.finish()?;
    Ok(())
}
