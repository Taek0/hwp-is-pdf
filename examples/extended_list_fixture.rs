//! Number formats, child-level restarts and image bullets with wrapped body text.
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
    for name in ["formats", "hierarchy", "image"] {
        let mut additions = String::new();
        let mut body = String::new();
        let style = |id: usize, kind: &str, reference: usize, level: usize| {
            format!(
                r#"<hh:paraPr id="{id}"><hh:align horizontal="LEFT"/><hh:heading type="{kind}" idRef="{reference}" level="{level}"/><hh:margin><hc:left value="3000"/><hc:intent value="-2500"/><hc:next value="700"/></hh:margin><hh:lineSpacing type="PERCENT" value="160"/></hh:paraPr>"#
            )
        };
        let para = |id: usize, pr: usize, value: &str| {
            format!(
                r#"<hp:p id="item-{id}" paraPrIDRef="{pr}"><hp:run charPrIDRef="0"><hp:t>{value}</hp:t></hp:run></hp:p>"#
            )
        };
        if name == "formats" {
            for (i, kind) in [
                "DIGIT",
                "CIRCLED_DIGIT",
                "ROMAN_CAPITAL",
                "ROMAN_SMALL",
                "LATIN_CAPITAL",
                "LATIN_SMALL",
                "CIRCLED_LATIN_CAPITAL",
                "CIRCLED_LATIN_SMALL",
                "HANGUL_SYLLABLE",
                "CIRCLED_HANGUL_SYLLABLE",
                "HANGUL_JAMO",
                "CIRCLED_HANGUL_JAMO",
                "HANGUL_PHONETIC",
                "IDEOGRAPH",
                "CIRCLED_IDEOGRAPH",
            ]
            .iter()
            .enumerate()
            {
                let id = 40 + i;
                additions += &format!(
                    r#"<hh:numbering id="{id}" start="3"><hh:paraHead level="1" numFormat="{kind}" charPrIDRef="0" textOffsetType="HWPUNIT" textOffset="800" widthAdjust="1000" align="RIGHT">^1.</hh:paraHead></hh:numbering>"#
                );
                additions += &style(id, "NUMBER", id, 0);
                body += &para(
                    i,
                    id,
                    &format!("FORMAT-{i:02}: {kind} 번호와 본문 사이 간격을 확인합니다."),
                );
            }
        } else if name == "hierarchy" {
            additions += r#"<hh:numbering id="40" start="3"><hh:paraHead level="1" numFormat="ROMAN_CAPITAL" charPrIDRef="0" textOffsetType="HWPUNIT" textOffset="800">^1.</hh:paraHead><hh:paraHead level="2" start="5" numFormat="LATIN_SMALL" charPrIDRef="0" textOffsetType="HWPUNIT" textOffset="800">^N</hh:paraHead></hh:numbering>"#;
            additions += &style(40, "NUMBER", 40, 0);
            additions += &style(41, "NUMBER", 40, 1);
            for (i, pr) in [40, 41, 41, 40, 41].into_iter().enumerate() {
                body += &para(
                    i,
                    pr,
                    &format!("LEVEL-{i}: 상위 번호가 바뀌면 하위 번호를 시작값으로 되돌립니다."),
                );
            }
        } else {
            additions += r#"<hh:bullet id="40" char="" useImage="1"><hh:paraHead charPrIDRef="0" textOffsetType="HWPUNIT" textOffset="800"/><hc:img binaryItemIDRef="mixed-image"/></hh:bullet>"#;
            additions += &style(40, "BULLET", 40, 0);
            for i in 0..4 {
                body+=&para(i,40,&format!("IMAGE-{i}: {} END-{i}","그림 글머리표는 첫 줄에 한 번만 출력하고 이어지는 본문은 같은 들여쓰기를 유지합니다. ".repeat(if i==1{20}else{3})));
            }
        }
        let section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="60000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:t>번호 형식과 그림 글머리표</hp:t></hp:run></hp:p>{body}</hs:sec>"#
        );
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/list-extended-{name}.hwpx"
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
