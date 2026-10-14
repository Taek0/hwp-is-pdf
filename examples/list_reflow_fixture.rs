//! Controlled list fixture: decimal transitions, hierarchy and long bullet items.
use std::io::{Read, Write};
fn main() -> anyhow::Result<()> {
    let mut source =
        zip::ZipArchive::new(std::fs::File::open("samples/reflow/plain-no-cache.hwpx")?)?;
    let mut header = String::new();
    source
        .by_name("Contents/header.xml")?
        .read_to_string(&mut header)?;
    let mut additions = String::from(
        r#"<hh:numbering id="20" start="98"><hh:paraHead level="1" charPrIDRef="2" textOffsetType="HWPUNIT" textOffset="600">^1.</hh:paraHead><hh:paraHead level="2" start="1" charPrIDRef="0" textOffsetType="PERCENT" textOffset="50">^1.^2)</hh:paraHead></hh:numbering><hh:bullet id="21" char="●" charPrIDRef="2" textOffsetType="HWPUNIT" textOffset="600"/>"#,
    );
    for (id, kind, reference, level, left) in [
        (20, "NUMBER", 20, 0, 3000),
        (21, "NUMBER", 20, 1, 5500),
        (22, "BULLET", 21, 0, 3000),
    ] {
        additions += &format!(
            r#"<hh:paraPr id="{id}"><hh:align horizontal="LEFT"/><hh:heading type="{kind}" idRef="{reference}" level="{level}"/><hh:margin><hc:left value="{left}"/><hc:intent value="-2500"/><hc:next value="900"/></hh:margin><hh:lineSpacing type="PERCENT" value="160"/><hh:breakSetting widowOrphan="1"/></hh:paraPr>"#
        );
    }
    header = header.replace("</hh:head>", &(additions + "</hh:head>"));
    let mut section = String::from(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"><hp:p id="title" paraPrIDRef="1"><hp:run charPrIDRef="1"><hp:secPr><hp:pagePr width="42000" height="55000"><hp:margin left="4000" right="4000" top="4000" bottom="4000"/></hp:pagePr></hp:secPr><hp:t>번호와 글머리표 재배치</hp:t></hp:run></hp:p>"#,
    );
    for (i, style) in [20, 21, 21, 20, 21, 20, 22, 22, 20].into_iter().enumerate() {
        let content = "항목의 본문이 여러 줄로 이어져도 내어쓰기와 번호 간격을 유지합니다. English words stay together. ".repeat(if i==6 {24} else {3});
        section += &format!(
            r#"<hp:p id="list-{i}" paraPrIDRef="{style}"><hp:run charPrIDRef="0"><hp:t>항목 L{i:02}: {content}끝 L{i:02}.</hp:t></hp:run></hp:p>"#
        );
    }
    section += "</hs:sec>";
    let mut zip = zip::ZipWriter::new(std::fs::File::create("samples/reflow/lists-no-cache.hwpx")?);
    for (name, data) in [
        ("mimetype", "application/hwp+zip"),
        ("Contents/header.xml", header.as_str()),
        ("Contents/section0.xml", section.as_str()),
        ("Contents/content.hpf", "<package><manifest/></package>"),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())?;
        zip.write_all(data.as_bytes())?;
    }
    zip.finish()?;
    Ok(())
}
