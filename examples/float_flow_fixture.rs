//! Multiple paragraph styles flowing beside a single leading float.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    for (source_name, name) in [
        ("float-single", "float-flow-single"),
        ("float-columns", "float-flow-columns"),
    ] {
        let mut source = zip::ZipArchive::new(std::fs::File::open(format!(
            "samples/reflow/{source_name}.hwpx"
        ))?)?;
        let mut out = zip::ZipWriter::new(std::fs::File::create(format!(
            "samples/reflow/{name}.hwpx"
        ))?);
        for i in 0..source.len() {
            let mut f = source.by_index(i)?;
            let mut bytes = vec![];
            f.read_to_end(&mut bytes)?;
            out.start_file(f.name(), zip::write::SimpleFileOptions::default())?;
            if f.name() == "Contents/section0.xml" {
                let mut s = String::from_utf8(bytes)?;
                let start = s.find("<hp:t>START-0").unwrap();
                let end = start + s[start..].find("</hp:t>").unwrap();
                s.replace_range(start..end, "<hp:t>START-0 첫 번째 짧은 문단입니다. END-0");
                let mut following = String::new();
                for j in 1..=5 {
                    let content = if j < 4 {
                        "별도 문단도 앞선 그림을 피해서 배치합니다. ".to_string()
                    } else {
                        "여러 문단의 간격과 글꼴을 유지하면서 그림 아래에서 전체 너비를 되찾습니다. English words follow the remaining space. ".repeat(3)
                    };
                    let note = if j == 2 {
                        r#"<hp:ctrl><hp:footNote number="2" id="follow-note"><hp:subList><hp:p id="follow-note-body" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>FOLLOW-NOTE: 후속 문단 각주도 그림과 같은 페이지에 배치합니다.</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl>"#
                    } else {
                        ""
                    };
                    following += &format!(
                        r#"<hp:p id="follow-{j}" paraPrIDRef="flow-{j}"><hp:run charPrIDRef="0"><hp:t>문단 F{j}: {content}</hp:t>{note}</hp:run></hp:p>"#
                    );
                }
                s = s.replace(
                    "<hp:p id=\"float-1\"",
                    &(following + "<hp:p id=\"float-1\""),
                );
                out.write_all(s.as_bytes())?;
            } else if f.name() == "Contents/header.xml" {
                let mut s = String::from_utf8(bytes)?.replacen(
                    "<hh:head",
                    r#"<hh:head xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core""#,
                    1,
                );
                let mut definitions = String::new();
                for j in 1..=5 {
                    definitions += &format!(
                        r#"<hh:paraPr id="flow-{j}"><hh:align horizontal="LEFT"/><hh:margin><hc:left value="{}"/><hc:prev value="300"/><hc:next value="400"/></hh:margin><hh:lineSpacing type="PERCENT" value="160"/><hh:breakSetting widowOrphan="1"/></hh:paraPr>"#,
                        if j == 3 { 800 } else { 0 }
                    );
                }
                s = s.replace("</hh:head>", &(definitions + "</hh:head>"));
                out.write_all(s.as_bytes())?;
            } else {
                out.write_all(&bytes)?;
            }
        }
        out.finish()?;
    }
    Ok(())
}
