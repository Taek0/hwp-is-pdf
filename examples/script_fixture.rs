//! Source-controlled combinations for slant, scripts and per-language offsets.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut base =
        zip::ZipArchive::new(std::fs::File::open("samples/text-style/decorations.hwpx")?)?;
    let mut header = String::new();
    base.by_name("Contents/header.xml")?
        .read_to_string(&mut header)?;
    let styles=["".to_string(),"<hh:italic/>".into(),"<hh:supscript/>".into(),"<hh:subscript/>".into(),
 r##"<hh:offset hangul="-30" latin="-30"/><hh:underline type="BOTTOM" shape="SOLID" color="#1266CC"/>"##.into(),
 r##"<hh:offset hangul="30" latin="30"/><hh:strikeout shape="SOLID" color="#CC2222"/>"##.into(),
 r##"<hh:italic/><hh:supscript/><hh:underline type="BOTTOM" shape="DOUBLE_SLIM" color="#1266CC"/>"##.into(),
 r##"<hh:offset hangul="-20" latin="10" hanja="30" japanese="40"/>"##.into()];
    let mut chars = String::from("<hh:charProperties>");
    for (i, style) in styles.iter().enumerate() {
        chars += &format!(
            r##"<hh:charPr id="{i}" height="1800" textColor="#000000"><hh:fontRef hangul="0" latin="0"/>{style}</hh:charPr>"##
        );
    }
    chars += "</hh:charProperties>";
    let start = header.find("<hh:charProperties>").unwrap();
    let end = header.find("</hh:charProperties>").unwrap() + "</hh:charProperties>".len();
    header.replace_range(start..end, &chars);
    let rows: Vec<Vec<(usize, &str)>> = vec![
        vec![(0, "기본: 한글 가나다 ABC 123")],
        vec![(1, "기울임: 한글 가나다 ABC 123")],
        vec![(0, "위첨자: 면적 x"), (2, "2 가"), (0, " + y"), (2, "2 나")],
        vec![(0, "아래첨자: H"), (3, "2 가"), (0, "O + CO"), (3, "2 나")],
        vec![(0, "위로 이동: 기준 "), (4, "한글 ABC"), (0, " 기준")],
        vec![(0, "아래로 이동: 기준 "), (5, "한글 ABC"), (0, " 기준")],
        vec![(0, "조합: 기본 "), (6, "기울임 위첨자"), (0, " 기본")],
        vec![(0, "언어별 이동: "), (7, "가 A 漢 あ")],
    ];
    let mut section = String::from(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">"#,
    );
    for (i, runs) in rows.iter().enumerate() {
        section += &format!(r#"<hp:p id="{i}" paraPrIDRef="0">"#);
        for (j, (style, text)) in runs.iter().enumerate() {
            section += &format!(r#"<hp:run charPrIDRef="{style}">"#);
            if i == 0 && j == 0 {
                section += r#"<hp:secPr><hp:pagePr width="59528" height="84188"><hp:margin left="5000" right="5000" top="5000" bottom="5000" header="0" footer="0"/></hp:pagePr></hp:secPr>"#;
            }
            section += &format!("<hp:t>{text}</hp:t></hp:run>");
        }
        section += &format!(
            r#"<hp:linesegarray><hp:lineseg textpos="0" vertpos="{}" vertsize="3000" textheight="1800" baseline="2400" spacing="1200" horzpos="0" horzsize="49000" flags="393216"/></hp:linesegarray></hp:p>"#,
            i * 6500
        );
    }
    section += "</hs:sec>";
    let mut zip = zip::ZipWriter::new(std::fs::File::create(
        "samples/text-style/scripts-offsets.hwpx",
    )?);
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
    Ok(())
}
