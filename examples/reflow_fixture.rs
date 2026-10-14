//! Controlled HWPX paragraphs with and without stored line caches.
use std::io::Write;
fn main() -> anyhow::Result<()> {
    let header = r##"<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core"><hh:fontface lang="HANGUL"><hh:font id="0" face="맑은 고딕"/></hh:fontface><hh:fontface lang="LATIN"><hh:font id="0" face="Arial"/></hh:fontface><hh:charPr id="0" height="1200" textColor="#20252B"><hh:fontRef hangul="0" latin="0"/></hh:charPr><hh:charPr id="1" height="2200" textColor="#174F82"><hh:fontRef hangul="0" latin="0"/><hh:bold/></hh:charPr><hh:charPr id="2" height="1500" textColor="#B23C28"><hh:fontRef hangul="0" latin="0"/><hh:bold/></hh:charPr><hh:paraPr id="0"><hh:align horizontal="JUSTIFY"/><hh:breakSetting widowOrphan="1" breakLatinWord="KEEP_WORD" breakNonLatinWord="BREAK_WORD"/><hh:margin><hc:left value="800"/><hc:right value="800"/><hc:intent value="1200"/><hc:prev value="0"/><hc:next value="800"/></hh:margin><hh:lineSpacing type="PERCENT" value="160"/></hh:paraPr><hh:paraPr id="1"><hh:align horizontal="LEFT"/><hh:breakSetting keepWithNext="1"/><hh:margin><hc:prev value="1000"/><hc:next value="800"/></hh:margin><hh:lineSpacing type="PERCENT" value="130"/></hh:paraPr><hh:paraPr id="2"><hh:align horizontal="LEFT"/><hh:breakSetting keepLines="1" widowOrphan="1"/><hh:margin><hc:next value="1000"/></hh:margin><hh:lineSpacing type="FIXED" value="2100"/></hh:paraPr><hh:paraPr id="3"><hh:align horizontal="LEFT"/><hh:breakSetting pageBreakBefore="1" keepWithNext="1"/><hh:lineSpacing type="PERCENT" value="130"/></hh:paraPr></hh:head>"##;
    std::fs::create_dir_all("samples/reflow")?;
    for (name, cache, columns) in [
        ("plain-no-cache", false, 1),
        ("plain-cached", true, 1),
        ("two-column-no-cache", false, 2),
    ] {
        let mut section = String::from(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">"#,
        );
        for i in 0..18 {
            let heading = i == 0 || i == 6 || i == 12;
            let style = if i == 12 {
                3
            } else if heading {
                1
            } else if i == 8 {
                2
            } else {
                0
            };
            let mut content = if heading {
                format!(r#"<hp:run charPrIDRef="1"><hp:t>문단 재배치 검증 {i:02}</hp:t></hp:run>"#)
            } else {
                let sentence = "한글 문장은 글자 경계에서 줄을 나누고 English words stay together when space allows. 글자 크기와 문단 여백을 반영해 다음 줄과 페이지로 이어집니다. ";
                format!(
                    r#"<hp:run charPrIDRef="0"><hp:t>본문 P{i:03}: {}</hp:t></hp:run><hp:run charPrIDRef="2"><hp:t>강조 구간</hp:t></hp:run><hp:run charPrIDRef="0"><hp:t> 끝.</hp:t></hp:run>"#,
                    sentence.repeat(if i == 8 { 2 } else { 3 })
                )
            };
            if i == 0 {
                content = format!(
                    r#"<hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="{}" height="55000"><hp:margin left="4000" right="4000" top="4000" bottom="4000" header="0" footer="0"/></hp:pagePr></hp:secPr><hp:ctrl><hp:colPr colCount="{columns}" sameGap="1800"/></hp:ctrl></hp:run>{content}"#,
                    if columns == 1 { 42000 } else { 59528 }
                );
            }
            if i == 4 {
                content=r#"<hp:run charPrIDRef="0"><hp:t>명시적 줄바꿈 전.<hp:lineBreak/>명시적 줄바꿈 후.<hp:lineBreak/><hp:lineBreak/>빈 줄 다음 문장. 긴 단어: SUPERCALIFRAGILISTICEXPIALIDOCIOUSSUPERCALIFRAGILISTICEXPIALIDOCIOUS</hp:t></hp:run>"#.into();
            }
            section += &format!(r#"<hp:p id="{i}" paraPrIDRef="{style}">{content}"#);
            if cache {
                section += &format!(
                    r#"<hp:linesegarray><hp:lineseg textpos="0" vertpos="{}" vertsize="1600" textheight="1200" baseline="1200" spacing="0" horzpos="0" horzsize="34000"/></hp:linesegarray>"#,
                    i * 2200
                );
            }
            section += "</hp:p>";
        }
        section += "</hs:sec>";
        let mut zip = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/{name}.hwpx"
        ))?);
        for (path, data) in [
            ("mimetype", "application/hwp+zip"),
            ("Contents/header.xml", header),
            ("Contents/section0.xml", section.as_str()),
            ("Contents/content.hpf", "<package><manifest/></package>"),
        ] {
            zip.start_file(path, zip::write::SimpleFileOptions::default())?;
            zip.write_all(data.as_bytes())?;
        }
        zip.finish()?;
    }
    Ok(())
}
