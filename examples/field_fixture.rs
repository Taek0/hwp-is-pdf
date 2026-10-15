//! Closed empty fields, outline TOC, and preserved cached field results.
use std::io::{Read, Write};
fn field(id: &str, kind: &str, command: &str, cached: &str) -> String {
    format!(
        r#"<hp:ctrl><hp:fieldBegin id="{id}" type="{kind}"><hp:parameters><hp:stringParam name="Command">{command}</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl><hp:t>{cached}</hp:t><hp:ctrl><hp:fieldEnd beginIDRef="{id}"/></hp:ctrl>"#
    )
}
fn p(id: &str, style: usize, body: &str) -> String {
    format!(
        r#"<hp:p id="{id}" paraPrIDRef="{style}"><hp:run charPrIDRef="0">{body}</hp:run></hp:p>"#
    )
}
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
    for cached in [false, true] {
        let mut body = p(
            "title",
            0,
            r#"<hp:secPr><hp:pagePr width="59528" height="50000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:ctrl><hp:bookmark name="목차"/></hp:ctrl><hp:t>필드와 목차 · PDF 탐색 검증</hp:t>"#,
        );
        body += &p(
            "toc",
            0,
            &field(
                "toc",
                "TABLEOFCONTENTS",
                "",
                if cached {
                    "보존된 사용자 목차 ........ 99"
                } else {
                    ""
                },
            ),
        );
        body += &p(
            "total",
            0,
            &format!(
                "<hp:t>TOTAL-PAGES: </hp:t>{}",
                field(
                    "total",
                    "SUMMERY",
                    "NUMPAGES",
                    if cached { "99" } else { "" }
                )
            ),
        );
        body += &p(
            "reference",
            0,
            &format!(
                "<hp:t>SECOND-PAGE: </hp:t>{}",
                field("reference", "CROSSREF", "PAGEREF chapter-two", "")
            ),
        );
        body += &p(
            "external",
            0,
            &format!(
                "<hp:t>외부 링크: </hp:t>{}<hp:t> (여기는 링크 아님)</hp:t>",
                field(
                    "external",
                    "HYPERLINK",
                    "https://www.hancom.com",
                    "한컴 홈페이지"
                )
            ),
        );
        for chapter in 0..3 {
            let bookmark = if chapter == 1 {
                r#"<hp:ctrl><hp:bookmark name="chapter-two"/></hp:ctrl>"#
            } else {
                ""
            };
            body += &p(
                &format!("heading-{chapter}"),
                40,
                &format!(
                    "{bookmark}<hp:t>{}. 장 제목과 페이지 참조</hp:t>",
                    chapter + 1
                ),
            );
            body += &p(
                &format!("subheading-{chapter}"),
                41,
                &format!("<hp:t>{}.1. 하위 제목</hp:t>", chapter + 1),
            );
            for row in 0..13 {
                body += &p(
                    &format!("body-{chapter}-{row}"),
                    0,
                    &format!(
                        "<hp:t>본문 {}-{row:02}: 목차의 페이지 번호는 최종 배치 이후 결정됩니다.</hp:t>",
                        chapter + 1
                    ),
                );
            }
        }
        body += &p(
            "back",
            0,
            &field("back", "HYPERLINK", "#목차", "목차로 돌아가기"),
        );
        let section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">{body}</hs:sec>"#
        );
        let additions = r#"<hh:paraPr id="40"><hh:heading type="OUTLINE" level="0"/><hh:align horizontal="LEFT"/><hh:lineSpacing type="PERCENT" value="200"/><hh:breakSetting keepWithNext="1"/></hh:paraPr><hh:paraPr id="41"><hh:heading type="OUTLINE" level="1"/><hh:align horizontal="LEFT"/><hh:lineSpacing type="PERCENT" value="160"/><hh:breakSetting keepWithNext="1"/></hh:paraPr>"#;
        let name = if cached { "cached" } else { "toc" };
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/fields-{name}.hwpx"
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
