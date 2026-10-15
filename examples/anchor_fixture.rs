//! Negative, PAGE/PAPER and oversized-anchor regression examples.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut source =
        zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut entries = Vec::new();
    for i in 0..source.len() {
        let mut f = source.by_index(i)?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        entries.push((f.name().to_string(), bytes));
    }
    let picture = |id: &str,
                   w: usize,
                   h: usize,
                   hr: &str,
                   vr: &str,
                   align: &str,
                   x: i32,
                   y: i32| {
        format!(
            r#"<hp:pic id="{id}" textWrap="SQUARE"><hp:sz width="{w}" height="{h}"/><hp:pos treatAsChar="0" horzRelTo="{hr}" vertRelTo="{vr}" horzAlign="{align}" vertAlign="TOP" horzOffset="{x}" vertOffset="{y}" allowOverlap="0"/><hp:img binaryItemIDRef="mixed-image"/></hp:pic>"#
        )
    };
    for (name, cols, width) in [
        ("anchors-negative", 1, 59528),
        ("anchors-absolute", 1, 59528),
        ("anchors-absolute-columns", 2, 84188),
        ("anchors-large", 1, 59528),
        ("anchors-large-group", 1, 59528),
        ("anchors-chain", 1, 59528),
    ] {
        let page = format!(
            r#"<hp:secPr><hp:pagePr width="{width}" height="80000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:ctrl><hp:colPr colCount="{cols}" sameGap="2400"/></hp:ctrl>"#
        );
        let mut section = String::from(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">"#,
        );
        if name == "anchors-negative" {
            section += &format!(
                r#"<hp:p id="intro" paraPrIDRef="0"><hp:run charPrIDRef="0">{page}<hp:t>앞 문단의 마지막 줄입니다. PREVIOUS-END</hp:t></hp:run></hp:p>"#
            );
        }
        let objects=match name{
            "anchors-negative"=>picture("negative-picture",10000,12000,"PARA","PARA","LEFT",1500,-4000),
            "anchors-absolute"=>picture("paper-picture",10000,8000,"PAPER","PAPER","LEFT",8000,10000)+&picture("page-picture",8000,8000,"PAGE","PAGE","RIGHT",-1000,18000),
            "anchors-absolute-columns"=>picture("paper-picture",10000,8000,"PAPER","PAPER","LEFT",65000,12000),
            "anchors-large"=>picture("large-picture",30000,120000,"PARA","PARA","LEFT",0,0),
            "anchors-large-group"=>r##"<hp:group id="large-group" textWrap="SQUARE"><hp:sz width="40000" height="120000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzAlign="LEFT" vertAlign="TOP"/><hp:rect id="large-child"><hp:sz width="30000" height="100000"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzAlign="LEFT" vertAlign="TOP" horzOffset="2000" vertOffset="2000"/><hp:lineShape style="SOLID" width="50" color="#1266CC"/><hp:drawText><hp:textMargin left="500" right="500" top="500" bottom="500"/><hp:subList><hp:p id="group-label" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>BIG-GROUP: 큰 그룹의 글자도 함께 보존합니다.</hp:t></hp:run></hp:p></hp:subList></hp:drawText></hp:rect></hp:group>"##.into(),
            _=>picture("chain-first",20000,42000,"PARA","PARA","LEFT",0,0),
        };
        section += &format!(
            r#"<hp:p id="anchor" paraPrIDRef="0"><hp:run charPrIDRef="0">{}{objects}<hp:t>ANCHOR-START: 개체 위치와 페이지 경계를 확인하는 본문입니다.</hp:t></hp:run></hp:p>"#,
            if name == "anchors-negative" {
                ""
            } else {
                &page
            }
        );
        for i in 1..=8 {
            let extra = if name == "anchors-chain" && i == 2 {
                picture(
                    "chain-second",
                    20000,
                    42000,
                    "PARA",
                    "PARA",
                    "LEFT",
                    0,
                    1500,
                )
            } else {
                String::new()
            };
            let body="본문과 그림이 겹치지 않도록 페이지의 남은 영역을 확인합니다. English text follows the available area. ".repeat(3);
            section += &format!(
                r#"<hp:p id="body-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0">{extra}<hp:t>BODY-{i}: {body} END-{i}.</hp:t></hp:run></hp:p>"#
            );
        }
        section += "</hs:sec>";
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
