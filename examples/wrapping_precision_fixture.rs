//! Alpha holes, closed cubic contours and separated group members.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open("samples/reflow/contour-single.hwpx")?)?;
    let mut header = String::new();
    zip.by_name("Contents/header.xml")?
        .read_to_string(&mut header)?;
    let rgba = image::RgbaImage::from_fn(128, 128, |x, y| {
        let r = (x as f64 - 63.5).hypot(y as f64 - 63.5);
        if (34.0..55.0).contains(&r) {
            image::Rgba([20, 130, 210, 255])
        } else {
            image::Rgba([0, 0, 0, 0])
        }
    });
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(rgba).write_to(&mut png, image::ImageFormat::Png)?;
    let pos = |align: &str, x: usize, y: usize| {
        format!(
            r#"<hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA" horzAlign="{align}" vertAlign="TOP" horzOffset="{x}" vertOffset="{y}" allowOverlap="0"/>"#
        )
    };
    for (name, cols, width) in [
        ("precision-single", 1, 59528),
        ("precision-columns", 2, 100000),
    ] {
        let picture = format!(
            r#"<hp:pic id="alpha-ring" textWrap="SQUARE" textFlow="BOTH_SIDES"><hp:sz width="16000" height="16000"/>{}<hp:img binaryItemIDRef="ring"/></hp:pic>"#,
            pos("CENTER", 0, 0)
        );
        let curve = format!(
            r##"<hp:curve id="cubic-contour" textWrap="SQUARE" textFlow="BOTH_SIDES"><hp:sz width="9000" height="9000"/><hp:orgSz width="9000" height="9000"/>{}<hp:lineShape style="SOLID" width="50" color="#CC5533"/><hp:pt x="4500" y="0"/><hp:pt x="9000" y="4500"/><hp:pt x="4500" y="9000"/><hp:pt x="0" y="4500"/><hp:pt x="4500" y="0"/><hp:segment type="1"/><hp:segment type="1"/><hp:segment type="1"/><hp:segment type="1"/></hp:curve>"##,
            pos("LEFT", 1200, 19000)
        );
        let mut group = format!(
            r#"<hp:group id="separated-group" textWrap="SQUARE" textFlow="BOTH_SIDES"><hp:sz width="15000" height="9500"/>{}"#,
            pos("RIGHT", 0, 22000)
        );
        for (i, x, h) in [(0, 0, 9500), (1, 11000, 7500)] {
            group += &format!(
                r##"<hp:rect id="group-child-{i}"><hp:sz width="4000" height="{h}"/>{}<hp:lineShape style="SOLID" width="40" color="#228855"/></hp:rect>"##,
                pos("LEFT", x, 0)
            );
        }
        group += "</hp:group>";
        let mut section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="{width}" height="80000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:ctrl><hp:colPr colCount="{cols}" sameGap="2400"/></hp:ctrl><hp:t>1단계: 투명 영역·곡선·그룹·양쪽 줄 사용</hp:t></hp:run></hp:p><hp:p id="precision-anchor" paraPrIDRef="0"><hp:run charPrIDRef="0">{picture}{curve}{group}<hp:t>PRECISION-START 투명 그림과 곡선 주위에 본문을 배치합니다.</hp:t></hp:run></hp:p>"#
        );
        for i in 1..=8 {
            let text="왼쪽 공간부터 읽고 같은 줄의 다음 빈 공간으로 이어집니다. 투명한 구멍과 그룹 안의 빈 곳도 사용할 수 있습니다. English words stay readable. ".repeat(3);
            section += &format!(
                r#"<hp:p id="precision-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>P{i}: {text} END-P{i}</hp:t></hp:run></hp:p>"#
            );
        }
        section += "</hs:sec>";
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/{name}.hwpx"
        ))?);
        for(path,data)in[("mimetype",b"application/hwp+zip".as_slice()),("Contents/header.xml",header.as_bytes()),("Contents/section0.xml",section.as_bytes()),("Contents/content.hpf",br#"<package><manifest><item id="ring" href="BinData/ring.png" media-type="image/png"/></manifest></package>"#.as_slice()),("BinData/ring.png",png.get_ref().as_slice())]{out.start_file(path,zip::write::SimpleFileOptions::default())?;out.write_all(data)?;}
        out.finish()?;
    }
    Ok(())
}
