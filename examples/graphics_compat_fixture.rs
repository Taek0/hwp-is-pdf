//! Scoped equation decorations and page-background/arc reflow fixtures.
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
    for name in ["accents", "background-arcs"] {
        let mut body = String::new();
        if name == "accents" {
            let scripts = [
                "HAT {AB}",
                "CHECK {AB}",
                "TILDE {AB}",
                "ACUTE {x}",
                "GRAVE {x}",
                "DOT {x}",
                "DDOT {x}",
                "BAR {AB} + CD",
                "VEC {AB}",
                "DYAD {AB}",
                "UNDER {AB}",
                "{n} ATOP {k}",
                "{n} CHOOSE {k}",
                "BINOM {n}{k}",
            ];
            for (i, script) in scripts.iter().enumerate() {
                body += &format!(
                    r#"<hp:p id="math-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>EQ-{i:02}: </hp:t><hp:equation id="eq-{i}" baseUnit="1600"><hp:pos treatAsChar="1"/><hp:script>{script}</hp:script></hp:equation></hp:run></hp:p>"#
                );
            }
        } else {
            body += r##"<hp:p id="background" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:rect id="background-box" textWrap="BEHIND_TEXT"><hp:sz width="49528" height="65000"/><hp:pos treatAsChar="0" horzRelTo="PAPER" vertRelTo="PAPER" horzOffset="5000" vertOffset="5000"/><hp:lineShape style="NONE"/><hc:fillBrush><hc:winBrush faceColor="#EDF4FC"/></hc:fillBrush></hp:rect><hp:t>BACKGROUND: 배경은 본문 공간을 밀어내지 않습니다.</hp:t></hp:run></hp:p>"##;
            body += r##"<hp:p id="line-p" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:line id="rule"><hp:sz width="30000" height="0"/><hp:pos treatAsChar="0" horzRelTo="PARA" vertRelTo="PARA"/><hp:lineShape style="SOLID" width="80" color="#336699"/></hp:line></hp:run></hp:p>"##;
            for (i, kind) in ["NORMAL", "PIE", "CHORD"].iter().enumerate() {
                body += &format!(
                    r##"<hp:p id="arc-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>ARC-{kind}</hp:t><hp:arc id="arc-object-{i}" type="{kind}"><hp:sz width="10000" height="10000"/><hp:orgSz width="10000" height="10000"/><hp:pos treatAsChar="1"/><hp:lineShape style="SOLID" width="80" color="#336699"/><hc:center x="0" y="0"/><hc:ax1 x="10000" y="0"/><hc:ax2 x="0" y="10000"/>{}</hp:arc></hp:run></hp:p>"##,
                    if i == 0 {
                        ""
                    } else {
                        r##"<hc:fillBrush><hc:winBrush faceColor="#CDE2F5"/></hc:fillBrush>"##
                    }
                );
            }
            body += r#"<hp:p id="after" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>AFTER: 선과 호 다음의 본문입니다.</hp:t></hp:run></hp:p>"#;
        }
        let section = format!(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core"><hp:p id="title" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr width="59528" height="80000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr></hp:secPr><hp:t>도형과 수식 호환성</hp:t></hp:run></hp:p>{body}</hs:sec>"#
        );
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/graphics-{name}.hwpx"
        ))?);
        for (path, b) in &entries {
            out.start_file(path, zip::write::SimpleFileOptions::default())?;
            out.write_all(if path == "Contents/section0.xml" {
                section.as_bytes()
            } else {
                b
            })?;
        }
        out.finish()?;
    }
    Ok(())
}
