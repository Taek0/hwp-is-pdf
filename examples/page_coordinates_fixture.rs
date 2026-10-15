//! Reproducible saved-layout examples for border origins and paragraph anchors.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut source =
        zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut entries = Vec::new();
    for i in 0..source.len() {
        let mut f = source.by_index(i)?;
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        entries.push((f.name().to_owned(), data));
    }
    let mut section = String::from(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">"#,
    );
    for i in 0..3 {
        let sec = if i == 0 {
            r#"<hp:secPr><hp:pagePr width="59528" height="84188"><hp:margin left="6000" right="6000" top="6000" bottom="6000" header="1500" footer="1500"/></hp:pagePr><hp:visibility border="SHOW_ALL" fill="SHOW_ALL"/><hp:pageBorderFill type="BOTH" borderFillIDRef="1" textBorder="PAPER" headerInside="0" footerInside="0" fillArea="PAPER"><hp:offset left="1000" right="1000" top="1000" bottom="1000"/></hp:pageBorderFill></hp:secPr>"#
        } else {
            ""
        };
        section += &format!(
            r#"<hp:p id="title-{i}" paraPrIDRef="0" pageBreak="{}"><hp:run charPrIDRef="0">{sec}<hp:t>좌표 검증 {}: 본문 기준 쪽 테두리 / 문단 기준 개체</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1400" baseline="1100" horzpos="0" horzsize="47528"/></hp:linesegarray></hp:p>"#,
            usize::from(i > 0),
            i + 1
        );
        for (j, align) in ["TOP", "CENTER", "BOTTOM"].into_iter().enumerate() {
            section += &format!(
                r##"<hp:p id="anchor-{i}-{j}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:rect id="marker-{i}-{j}" textWrap="IN_FRONT_OF_TEXT"><hp:sz width="5000" height="2500"/><hp:pos treatAsChar="0" horzRelTo="PAPER" horzAlign="LEFT" horzOffset="46000" vertRelTo="PARA" vertAlign="{align}" vertOffset="0"/><hp:lineShape style="SOLID" width="30" color="#007755"/></hp:rect><hp:t>{align}: 개체의 세로 기준은 이 문단의 시작 줄입니다.</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="{}" vertsize="1400" baseline="1100" horzpos="0" horzsize="39000"/></hp:linesegarray></hp:p>"##,
                6000 + j * 6000
            );
        }
    }
    section += "</hs:sec>";
    let mut out = zip::ZipWriter::new(std::fs::File::create(
        "samples/reflow/page-coordinates.hwpx",
    )?);
    for (name, data) in entries {
        out.start_file(&name, zip::write::SimpleFileOptions::default())?;
        out.write_all(if name == "Contents/section0.xml" {
            section.as_bytes()
        } else {
            &data
        })?;
    }
    out.finish()?;
    Ok(())
}
