//! Installed-language faces and explicitly enabled/disabled Latin kerning.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut entries = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let mut b = Vec::new();
        f.read_to_end(&mut b)?;
        entries.push((f.name().to_owned(), b));
    }
    let mut section = String::from(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">"#,
    );
    for (i, (style, text)) in [
        ("20", "Kerning ON: AVATAR To Wa"),
        ("21", "Kerning OFF: AVATAR To Wa"),
        ("20", "한글 / 漢字 / 日本語かな / ★☆ →"),
    ]
    .into_iter()
    .enumerate()
    {
        let page = if i == 0 {
            r#"<hp:secPr><hp:pagePr width="59528" height="40000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr>"#
        } else {
            ""
        };
        section += &format!(
            r#"<hp:p id="line-{i}" paraPrIDRef="0"><hp:run charPrIDRef="{style}">{page}<hp:t>{text}</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="{}" vertsize="2400" baseline="2000" horzpos="0" horzsize="49528"/></hp:linesegarray></hp:p>"#,
            i * 6000
        );
    }
    section += "</hs:sec>";
    let mut out = zip::ZipWriter::new(std::fs::File::create(
        "samples/reflow/language-kerning.hwpx",
    )?);
    for (name, mut b) in entries {
        if name == "Contents/header.xml" {
            let mut extra = String::new();
            for (lang, face) in [
                ("HANJA", "SimSun"),
                ("JAPANESE", "MS Gothic"),
                ("SYMBOL", "Segoe UI Symbol"),
            ] {
                extra += &format!(
                    r#"<hh:fontface lang="{lang}"><hh:font id="0" face="{face}"/></hh:fontface>"#
                );
            }
            for (id, kern) in [(20, 1), (21, 0)] {
                extra += &format!(
                    r##"<hh:charPr id="{id}" height="2000" textColor="#000000" useKerning="{kern}"><hh:fontRef hangul="0" latin="0" hanja="0" japanese="0" symbol="0"/><hh:relSz hangul="100" latin="100" hanja="100" japanese="100" symbol="100"/><hh:ratio hangul="100" latin="100" hanja="120" japanese="90" symbol="100"/></hh:charPr>"##
                );
            }
            b = String::from_utf8(b)?
                .replace("</hh:head>", &format!("{extra}</hh:head>"))
                .into_bytes();
        }
        out.start_file(&name, zip::write::SimpleFileOptions::default())?;
        out.write_all(if name == "Contents/section0.xml" {
            section.as_bytes()
        } else {
            &b
        })?;
    }
    out.finish()?;
    Ok(())
}
