//! Reflow equations and long endnotes through ordinary and variable columns.
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
    for name in ["single", "columns", "unequal"] {
        let columns = if name == "single" {
            r#"<hp:colPr colCount="1"/>"#
        } else if name == "columns" {
            r#"<hp:colPr colCount="2" sameGap="2400"/>"#
        } else {
            r#"<hp:colPr colCount="2"><hp:colSz width="18000" gap="2400"/><hp:colSz width="29128" gap="0"/></hp:colPr>"#
        };
        let page = format!(
            r#"<hp:secPr><hp:pagePr width="59528" height="50000"><hp:margin left="5000" right="5000" top="5000" bottom="5000"/></hp:pagePr><hp:endNotePr><hp:placement place="END_OF_SECTION"/><hp:noteSpacing aboveLine="300" belowLine="300" betweenNotes="500"/></hp:endNotePr></hp:secPr><hp:ctrl>{columns}</hp:ctrl>"#
        );
        let equation = |id: &str, natural: bool| {
            format!(
                r#"<hp:equation id="{id}" baseUnit="1600" baseLine="70"><hp:pos treatAsChar="1"/>{}<hp:script>{{x^2+1}} over {{y+2}}</hp:script></hp:equation>"#,
                if natural {
                    ""
                } else {
                    r#"<hp:sz width="6000" height="3500"/>"#
                }
            )
        };
        let mut section = String::from(
            r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">"#,
        );
        for i in 0..9 {
            let mut note = String::new();
            if i % 3 == 0 {
                let number = i / 3 + 1;
                note = format!(r#"<hp:ctrl><hp:endNote number="{number}"><hp:subList>"#);
                for j in 0..18 {
                    note += &format!(
                        r#"<hp:p id="note-{number}-{j}" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>NOTE-{number}-{j:02}: 미주가 다음 단과 페이지로 이어져도 빠짐없이 남아야 합니다.</hp:t>{}</hp:run></hp:p>"#,
                        if j == 5 {
                            equation(&format!("note-eq-{number}"), true)
                        } else {
                            String::new()
                        }
                    );
                }
                note += "</hp:subList></hp:endNote></hp:ctrl>";
            }
            section += &format!(
                r#"<hp:p id="body-{i}" paraPrIDRef="0"><hp:run charPrIDRef="0">{}<hp:t>BODY-{i}: 본문 수식 앞 </hp:t>{}<hp:t> 수식 뒤의 글자를 같은 줄 또는 다음 줄에 안전하게 배치합니다.</hp:t>{note}</hp:run></hp:p>"#,
                if i == 0 { page.as_str() } else { "" },
                equation(&format!("body-eq-{i}"), i % 2 == 0)
            );
        }
        section += "</hs:sec>";
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/equation-notes-{name}.hwpx"
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
