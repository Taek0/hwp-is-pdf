//! Visible border families and transparency-preserving image effects.
use std::io::{Cursor, Read, Write};
fn main() -> anyhow::Result<()> {
    let mut z = zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut entries = Vec::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i)?;
        let mut b = Vec::new();
        f.read_to_end(&mut b)?;
        entries.push((f.name().to_owned(), b));
    }
    let styles = [
        "DASH",
        "DOT",
        "DASH_DOT",
        "DOUBLE_SLIM",
        "SLIM_THICK",
        "WAVE",
        "DOUBLEWAVE",
    ];
    let mut borders = String::new();
    let mut rows = String::new();
    for (i, style) in styles.into_iter().enumerate() {
        let id = 30 + i;
        borders += &format!(r#"<hh:borderFill id="{id}">"#);
        for side in ["left", "right", "top", "bottom"] {
            borders +=
                &format!(r##"<hh:{side}Border type="{style}" width="0.2 mm" color="#226699"/>"##);
        }
        borders += "</hh:borderFill>";
        rows += &format!(
            r#"<hp:tr><hp:tc hasMargin="1" borderFillIDRef="{id}"><hp:cellAddr rowAddr="{i}" colAddr="0"/><hp:cellSpan rowSpan="1" colSpan="1"/><hp:cellSz width="46000" height="4500"/><hp:cellMargin left="1000" right="1000" top="500" bottom="500"/><hp:subList vertAlign="CENTER"><hp:p id="row-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{style}: 표·문단·쪽 테두리 벡터 표현</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1400" baseline="1100" horzpos="0" horzsize="44000"/></hp:linesegarray></hp:p></hp:subList></hp:tc></hp:tr>"#
        );
    }
    let mut section = format!(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="70000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:t>테두리 종류와 투명 그림 효과 검증</hp:t><hp:tbl id="styles" rowCnt="7" colCnt="1" pageBreak="NONE"><hp:sz width="46000" height="31500"/><hp:pos treatAsChar="0" horzRelTo="PAPER" vertRelTo="PAPER" horzOffset="6000" vertOffset="9000"/>{rows}</hp:tbl></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1400" baseline="1100" horzpos="0" horzsize="49528"/></hp:linesegarray></hp:p>"#
    );
    for (i, effect) in ["REAL_PIC", "GRAY_SCALE", "BLACK_WHITE"]
        .into_iter()
        .enumerate()
    {
        let x = 6000 + i * 16500;
        section += &format!(
            r##"<hp:p id="effect-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>{effect}</hp:t><hp:rect id="background-{i}" textWrap="BEHIND_TEXT"><hp:sz width="11000" height="10000"/><hp:pos treatAsChar="0" horzRelTo="PAPER" vertRelTo="PAPER" horzOffset="{x}" vertOffset="47000"/><hp:fillBrush><hp:winBrush faceColor="#CCDDFF"/></hp:fillBrush><hp:lineShape style="SOLID" width="10" color="#CCDDFF"/></hp:rect><hp:pic id="picture-{i}" textWrap="IN_FRONT_OF_TEXT"><hp:sz width="11000" height="10000"/><hp:pos treatAsChar="0" horzRelTo="PAPER" vertRelTo="PAPER" horzOffset="{x}" vertOffset="47000"/><hp:img binaryItemIDRef="mixed-image" effect="{effect}" bright="0" contrast="0"/></hp:pic></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="38000" vertsize="1400" baseline="1100" horzpos="{}" horzsize="15000"/></hp:linesegarray></hp:p>"##,
            1000 + i * 16500
        );
    }
    section += "</hs:sec>";
    let mut image = image::RgbaImage::new(100, 100);
    for (x, y, p) in image.enumerate_pixels_mut() {
        *p = image::Rgba(if (25..75).contains(&x) && (25..75).contains(&y) {
            [0, 0, 0, 0]
        } else if x < 50 {
            [255, 40, 10, 160]
        } else {
            [10, 200, 40, 160]
        });
    }
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut png, image::ImageFormat::Png)?;
    let mut out = zip::ZipWriter::new(std::fs::File::create("samples/reflow/border-effects.hwpx")?);
    for (name, mut b) in entries {
        if name == "Contents/header.xml" {
            b = String::from_utf8(b)?
                .replace("</hh:head>", &format!("{borders}</hh:head>"))
                .into_bytes();
        }
        if name.starts_with("BinData/") && name.ends_with(".png") {
            b = png.get_ref().clone();
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
