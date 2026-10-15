//! Geometric contour and arbitrary within-column offsets; no viewer dependency.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut source = zip::ZipArchive::new(std::fs::File::open(
        "samples/reflow/paired-float-single.hwpx",
    )?)?;
    let mut entries = Vec::new();
    for i in 0..source.len() {
        let mut f = source.by_index(i)?;
        let mut bytes = vec![];
        f.read_to_end(&mut bytes)?;
        entries.push((f.name().to_string(), bytes));
    }
    for (name, cols, width) in [("contour-single", 1, 59528), ("contour-columns", 2, 84188)] {
        let shape = |kind: &str,
                     id: &str,
                     align: &str,
                     x: i32,
                     y: i32,
                     w: usize,
                     h: usize,
                     extra: &str| {
            format!(
                r##"<hp:{kind} id="{id}" textWrap="SQUARE" textFlow="LARGEST_ONLY"><hp:sz width="{w}" height="{h}"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzAlign="{align}" vertAlign="TOP" horzOffset="{x}" vertOffset="{y}" allowOverlap="0"/><hp:outMargin left="300" right="300" top="300" bottom="300"/><hp:lineShape style="SOLID" width="50" color="#1266CC"/>{extra}</hp:{kind}>"##
            )
        };
        let objects = shape(
            "ellipse",
            "offset-ellipse",
            "LEFT",
            1000,
            500,
            9500,
            13500,
            "",
        ) + &shape(
            "polygon",
            "offset-triangle",
            "RIGHT",
            -1000,
            2500,
            9000,
            15000,
            r##"<hp:orgSz width="9000" height="15000"/><hp:pt x="9000" y="0"/><hp:pt x="0" y="7500"/><hp:pt x="9000" y="15000"/><hp:fillBrush><hp:winBrush faceColor="#FFF1B8"/></hp:fillBrush>"##,
        ) + &shape(
            "rect",
            "offset-rotation",
            "CENTER",
            0,
            17000,
            8000,
            6000,
            r#"<hp:curSz width="7000" height="4000"/><hp:rotationInfo angle="25"/>"#,
        ) + &shape(
            "pic",
            "offset-picture",
            "LEFT",
            3000,
            25000,
            6000,
            5000,
            r#"<hp:img binaryItemIDRef="mixed-image"/>"#,
        );
        let note = r#"<hp:ctrl><hp:footNote number="1" id="contour-note"><hp:subList><hp:p id="contour-note-body" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>CONTOUR-NOTE: 여러 위치의 개체를 피하는 문단의 각주입니다.</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl>"#;
        let mut section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="{width}" height="70000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:ctrl><hp:colPr colCount="{cols}" sameGap="2400"/></hp:ctrl><hp:t>윤곽선 감싸기와 여러 개체 위치 검증</hp:t></hp:run></hp:p><hp:p id="placed" paraPrIDRef="0"><hp:run charPrIDRef="0">{objects}<hp:t>ANCHOR-TEXT: 타원, 삼각형, 회전 사각형과 그림을 각각 다른 위치에 배치합니다.</hp:t>{note}</hp:run></hp:p>"#
        );
        for i in 1..=6 {
            let body="문단은 개체의 윤곽과 위치를 확인하여 빈 공간으로 이어집니다. English words follow the open area without crossing shapes. ".repeat(3);
            section += &format!(
                r#"<hp:p id="body-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>BODY-{i}: {body} END-{i}.</hp:t></hp:run></hp:p>"#
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
