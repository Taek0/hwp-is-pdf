//! Header/footer parity and one missing cache deep inside a table cell.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    for name in ["parity", "hidden-first", "partial-nested"] {
        let source = if name == "partial-nested" {
            "samples/reflow/captions-table-nested.hwpx"
        } else {
            "samples/reflow/contour-single.hwpx"
        };
        let mut input = zip::ZipArchive::new(std::fs::File::open(source)?)?;
        let mut entries = Vec::new();
        for i in 0..input.len() {
            let mut f = input.by_index(i)?;
            let mut b = Vec::new();
            f.read_to_end(&mut b)?;
            entries.push((f.name().to_string(), b));
        }
        let section = if name == "partial-nested" {
            let original = String::from_utf8(
                entries
                    .iter()
                    .find(|(p, _)| p == "Contents/section0.xml")
                    .unwrap()
                    .1
                    .clone(),
            )?;
            let cache = r#"<hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1200" textheight="1200" baseline="1000" spacing="600" horzpos="0" horzsize="35000"/></hp:linesegarray>"#;
            let mut s = original.replace("</hp:p>", &format!("{cache}</hp:p>"));
            let begin = s.find("<hp:p id=\"inner-text\"").unwrap();
            let end = begin + s[begin..].find("</hp:p>").unwrap();
            let without = s[begin..end].replace(cache, "");
            s.replace_range(begin..end, &without);
            s
        } else {
            let mut body = String::new();
            for i in 0..28 {
                body += &format!(
                    r#"<hp:p id="body-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>BODY-{i:02}: 머리말과 꼬리말의 실제 높이를 제외한 공간에 본문을 배치합니다. 뒤 페이지에서도 안전한 간격을 유지합니다.</hp:t></hp:run></hp:p>"#
                );
            }
            let hiding = if name == "hidden-first" {
                r#"<hp:ctrl><hp:pageHiding hideHeader="1" hideFooter="1"/></hp:ctrl>"#
            } else {
                ""
            };
            format!(
                r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="45000"><hp:margin left="4000" right="4000" top="3000" bottom="3000" header="500" footer="500"/></hp:pagePr></hp:secPr>{hiding}<hp:ctrl><hp:header applyPageType="ODD"><hp:subList><hp:p id="odd" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>ODD-HEADER: 홀수 페이지 머리말입니다. 원래 예약 높이보다 길어지는 머리말도 본문과 겹치지 않아야 합니다.</hp:t></hp:run></hp:p></hp:subList></hp:header><hp:header applyPageType="EVEN"><hp:subList><hp:p id="even" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>EVEN-HEADER: 짝수 페이지 머리말입니다.</hp:t></hp:run></hp:p></hp:subList></hp:header><hp:footer applyPageType="BOTH"><hp:subList><hp:p id="footer" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>FOOTER-PAGE: </hp:t><hp:ctrl><hp:autoNum numType="PAGE" num="1"/></hp:ctrl></hp:run></hp:p></hp:subList></hp:footer></hp:ctrl><hp:t>본문 시작</hp:t></hp:run></hp:p>{body}</hs:sec>"#
            )
        };
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/furniture-{name}.hwpx"
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
