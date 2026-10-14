//! Controlled HWPX fixture: explicit source properties, not a Hancom reference render.
use std::io::Write;
fn main() -> anyhow::Result<()> {
    let root = std::path::Path::new("samples/text-style");
    std::fs::create_dir_all(root)?;
    let cases = [
        (
            "Solid underline / 실선",
            r##"<hh:underline type="BOTTOM" shape="SOLID" color="#000000"/>"##,
        ),
        (
            "Dashed underline / 긴 점선",
            r##"<hh:underline type="BOTTOM" shape="DASH" color="#000000"/>"##,
        ),
        (
            "Dotted underline / 점선",
            r##"<hh:underline type="BOTTOM" shape="DOT" color="#000000"/>"##,
        ),
        (
            "Double underline / 이중선",
            r##"<hh:underline type="BOTTOM" shape="DOUBLE_SLIM" color="#000000"/>"##,
        ),
        (
            "Wave underline / 물결선",
            r##"<hh:underline type="BOTTOM" shape="WAVE" color="#1266CC"/>"##,
        ),
        (
            "Double wave / 이중 물결선",
            r##"<hh:underline type="BOTTOM" shape="DOUBLEWAVE" color="#1266CC"/>"##,
        ),
        (
            "Top underline / 윗줄",
            r##"<hh:underline type="TOP" shape="DASH" color="#000000"/>"##,
        ),
        (
            "Disabled underline / 밑줄 없음",
            r##"<hh:underline type="NONE" shape="SOLID" color="#000000"/>"##,
        ),
        (
            "Strikeout / 취소선",
            r##"<hh:strikeout shape="SOLID" color="#CC2222"/>"##,
        ),
        (
            "Double strike / 이중 취소선",
            r##"<hh:strikeout shape="DOUBLE_SLIM" color="#CC2222"/>"##,
        ),
        (
            "Disabled strike / 취소선 없음",
            r##"<hh:strikeout shape="NONE" color="#CC2222"/>"##,
        ),
        (
            "Dormant 3D strike / 취소선 없음",
            r##"<hh:strikeout shape="3D" color="#CC2222"/>"##,
        ),
        ("Italic Latin: office affine 123", r##"<hh:italic/>"##),
        (
            "Bold italic Latin: office affine 123",
            r##"<hh:bold/><hh:italic/>"##,
        ),
    ];
    let mut chars = String::new();
    let mut paras = String::new();
    for (i, (label, style)) in cases.iter().enumerate() {
        chars += &format!(
            r##"<hh:charPr id="{i}" height="1400" textColor="#000000"><hh:fontRef hangul="0" latin="0"/>{style}</hh:charPr>"##
        );
        let sec = if i == 0 {
            r#"<hp:secPr><hp:pagePr width="59528" height="84188"><hp:margin left="5000" right="5000" top="5000" bottom="5000" header="0" footer="0"/></hp:pagePr></hp:secPr>"#
        } else {
            ""
        };
        paras += &format!(
            r#"<hp:p id="{i}" paraPrIDRef="0"><hp:run charPrIDRef="{i}">{sec}<hp:t>{label}</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="{}" vertsize="1800" textheight="1400" baseline="1400" spacing="1000" horzpos="0" horzsize="49000" flags="393216"/></hp:linesegarray></hp:p>"#,
            i * 4000
        );
    }
    let header = format!(
        r#"<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head"><hh:fontface lang="HANGUL"><hh:font id="0" face="맑은 고딕"/></hh:fontface><hh:fontface lang="LATIN"><hh:font id="0" face="Arial"/></hh:fontface><hh:charProperties>{chars}</hh:charProperties><hh:paraPr id="0"><hh:align horizontal="LEFT"/></hh:paraPr></hh:head>"#
    );
    let section = format!(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">{paras}</hs:sec>"#
    );
    let mut zip = zip::ZipWriter::new(std::fs::File::create(root.join("decorations.hwpx"))?);
    for (name, content) in [
        ("mimetype", "application/hwp+zip"),
        ("Contents/header.xml", &header),
        ("Contents/section0.xml", &section),
        ("Contents/content.hpf", r#"<package><manifest/></package>"#),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())?;
        zip.write_all(content.as_bytes())?;
    }
    zip.finish()?;
    std::fs::write(root.join("header.xml"), header)?;
    Ok(())
}
