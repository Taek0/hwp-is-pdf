//! Paragraph-leading square wrapping on either side, including rotated text.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut source =
        zip::ZipArchive::new(std::fs::File::open("samples/reflow/mixed-no-cache.hwpx")?)?;
    let mut entries = Vec::new();
    for i in 0..source.len() {
        let mut f = source.by_index(i)?;
        let mut b = vec![];
        f.read_to_end(&mut b)?;
        entries.push((f.name().to_string(), b));
    }
    for (name, cols, width) in [("float-single", 1, 59528), ("float-columns", 2, 84188)] {
        let mut section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="intro" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="{width}" height="60000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:ctrl><hp:colPr colCount="{cols}" sameGap="2400"/></hp:ctrl><hp:t>그림과 도형 주위 글 감싸기 검증</hp:t></hp:run></hp:p>"#
        );
        for (i, side, kind, short) in [
            (0, "LEFT", "pic", false),
            (1, "RIGHT", "rect", false),
            (2, "LEFT", "rect", false),
            (3, "RIGHT", "pic", true),
        ] {
            let decoration = if kind == "pic" {
                r#"<hp:img binaryItemIDRef="mixed-image"/>"#.to_string()
            } else {
                format!(
                    r##"{}<hp:lineShape style="SOLID" width="35" color="#0055AA"/><hp:drawText><hp:textMargin left="500" right="500" top="500" bottom="500"/><hp:subList><hp:p id="shape-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>도형 {i}</hp:t></hp:run></hp:p></hp:subList></hp:drawText>"##,
                    if i == 2 {
                        r#"<hp:curSz width="9000" height="7000"/><hp:rotationInfo angle="25"/>"#
                    } else {
                        ""
                    }
                )
            };
            let content="그림 옆의 본문은 남은 너비에 맞춰 줄바꿈합니다. 개체 아래에서는 원래 문단 너비로 돌아옵니다. English words wrap around the object. ".repeat(if short{1}else{9});
            section += &format!(
                r#"<hp:p id="float-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:{kind} id="object-{i}" textWrap="SQUARE"><hp:sz width="12000" height="14000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzAlign="{side}" vertAlign="TOP"/><hp:outMargin left="500" right="500" top="500" bottom="500"/>{decoration}</hp:{kind}><hp:t>START-{i} {content} END-{i}</hp:t></hp:run></hp:p>"#
            );
        }
        section += r#"<hp:p id="after" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>짧은 마지막 문단 뒤의 본문입니다. AFTER-END</hp:t></hp:run></hp:p></hs:sec>"#;
        section=section.replace("</hp:pic><hp:t>START-0",r#"</hp:pic><hp:ctrl><hp:footNote number="1" id="float-note"><hp:subList><hp:p id="float-note-body" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>FLOAT-NOTE: 그림 옆 본문의 각주도 같은 페이지에 배치합니다.</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl><hp:t>START-0"#);
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/{name}.hwpx"
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
