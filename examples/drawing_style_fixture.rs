//! Compact source fixture for independent Hancom printing and engine regression.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let destination = std::env::args()
        .nth(1)
        .unwrap_or("output/format-probes/drawing-styles.hwpx".into());
    let path = std::path::Path::new(&destination);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut source =
        zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut body = String::new();
    for (i, style) in [
        "SOLID",
        "DASH",
        "DOT",
        "DASH_DOT",
        "DASH_DOT_DOT",
        "LONG_DASH",
        "NONE",
    ]
    .iter()
    .enumerate()
    {
        let y = 9000 + i * 8500;
        let mut objects = String::new();
        for (j, kind) in ["line", "rect", "ellipse", "polygon"].iter().enumerate() {
            let x = 14000 + j * 10000;
            let geometry = if *kind == "polygon" {
                r#"<hc:pt x="0" y="5000"/><hc:pt x="4000" y="0"/><hc:pt x="8000" y="5000"/>"#
            } else {
                ""
            };
            let fill = if *kind == "line" {
                ""
            } else {
                r##"<hc:fillBrush><hc:winBrush faceColor="#E0EDF8" hatchColor="#000000" hatchStyle="NONE" alpha="0"/></hc:fillBrush>"##
            };
            objects += &format!(
                r##"<hp:{kind} id="shape-{i}-{j}" textWrap="IN_FRONT_OF_TEXT"><hp:sz width="8000" height="5000"/><hp:orgSz width="8000" height="5000"/><hp:curSz width="8000" height="5000"/><hp:pos treatAsChar="0" horzRelTo="PAPER" vertRelTo="PAPER" horzOffset="{x}" vertOffset="{y}"/><hp:lineShape style="{style}" width="100" color="#185890"/>{fill}{geometry}</hp:{kind}>"##
            );
        }
        body += &format!(
            r#"<hp:p id="label-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{style}</hp:t>{objects}</hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="{}" vertsize="1200" baseline="1000" horzpos="0" horzsize="50000"/></hp:linesegarray></hp:p>"#,
            y - 5000
        );
    }
    let section = format!(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="84188"><hp:margin left="4000" right="4000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:t>도형 선 종류 · 채움 · 선 없음</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1400" baseline="1100" horzpos="0" horzsize="51528"/></hp:linesegarray></hp:p>{body}</hs:sec>"#
    );
    let mut output = zip::ZipWriter::new(std::fs::File::create(path)?);
    for i in 0..source.len() {
        let mut entry = source.by_index(i)?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
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
