//! Public-viewer probes must retain a complete, previously printable package.
use anyhow::{Context, Result, ensure};
use std::io::{Read, Write};

fn attrs(s: &str, tag: &str, values: &[(&str, String)]) -> Result<String> {
    let start = s
        .find(&format!("<{tag} "))
        .context(format!("Missing {tag}"))?;
    let end = start + s[start..].find('>').context("Tag end")?;
    let mut head = s[start..end].to_string();
    for (key, value) in values {
        let needle = format!(" {key}=\"");
        if let Some(at) = head.find(&needle) {
            let a = at + needle.len();
            let b = a + head[a..].find('"').context("Attribute end")?;
            head.replace_range(a..b, value);
        } else {
            let at = head.len() - usize::from(head.ends_with('/'));
            head.insert_str(at, &format!(" {key}=\"{value}\""));
        }
    }
    Ok(format!("{}{head}{}", &s[..start], &s[end..]))
}
fn set(s: &str, tag: &str, values: &[(&str, &str)]) -> Result<String> {
    attrs(
        s,
        tag,
        &values
            .iter()
            .map(|(k, v)| (*k, v.to_string()))
            .collect::<Vec<_>>(),
    )
}
fn element(s: &str, tag: &str) -> Result<String> {
    let tree = roxmltree::Document::parse(s)?;
    let node = tree
        .descendants()
        .find(|n| n.has_tag_name(tag))
        .context(format!("Missing {tag}"))?;
    Ok(s[node.range()].into())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(args.len() == 2, "INPUT_COMPLETE.hwpx OUTPUT_DIR");
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&args[0])?)?;
    let mut files = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        if f.is_dir() {
            continue;
        }
        let mut b = Vec::new();
        f.read_to_end(&mut b)?;
        files.push((f.name().to_string(), b));
    }
    for required in [
        "mimetype",
        "version.xml",
        "META-INF/container.xml",
        "Contents/content.hpf",
        "Contents/header.xml",
        "Contents/section0.xml",
    ] {
        ensure!(
            files.iter().any(|(n, _)| n == required),
            "Incomplete source package: {required}"
        );
    }
    let read = |name: &str| -> Result<String> {
        Ok(String::from_utf8(
            files
                .iter()
                .find(|(n, _)| n == name)
                .context("Package entry")?
                .1
                .clone(),
        )?)
    };
    let header = read("Contents/header.xml")?;
    let section = read("Contents/section0.xml")?;
    let tree = roxmltree::Document::parse(&section)?;
    let root = &section[tree.root_element().range()];
    let opening = &root[..root.find('>').context("Section opening")? + 1];
    let mut sec = element(&section, "secPr")?;
    sec = set(
        &sec,
        "hp:pagePr",
        &[
            ("width", "59528"),
            ("height", "84188"),
            ("landscape", "WIDELY"),
        ],
    )?;
    sec = set(
        &sec,
        "hp:margin",
        &[
            ("left", "4000"),
            ("right", "4000"),
            ("top", "5000"),
            ("bottom", "5000"),
            ("header", "0"),
            ("footer", "0"),
        ],
    )?;
    sec = set(
        &sec,
        "hp:visibility",
        &[
            ("hideFirstMasterPage", "1"),
            ("hideFirstHeader", "1"),
            ("hideFirstFooter", "1"),
        ],
    )?;
    let mut col = element(&section, "colPr")?;
    // Preserve complete attributes but remove the source's two-column separator.
    if let Some(i) = col.find('>') {
        col = format!("{}/>", col[..i].trim_end_matches('/'));
    }
    col = set(&col, "hp:colPr", &[("colCount", "1"), ("sameGap", "0")])?;
    let ht = roxmltree::Document::parse(&header)?;
    let base = ht
        .descendants()
        .find(|n| n.has_tag_name("charPr") && n.attribute("id") == Some("62"))
        .context("Source character style 62")?;
    let mut base = header[base.range()].to_string();
    base = set(
        &base,
        "hh:charPr",
        &[("height", "1200"), ("textColor", "#000000")],
    )?;
    let chars = ht
        .descendants()
        .find(|n| n.has_tag_name("charProperties"))
        .context("Character properties")?;
    let first_id = chars
        .children()
        .filter_map(|n| n.attribute("id").and_then(|s| s.parse::<usize>().ok()))
        .max()
        .unwrap_or(0)
        + 1;
    std::fs::create_dir_all(&args[1])?;
    for mode in ["drawing-styles", "text-outlines"] {
        let styles = if mode == "drawing-styles" {
            vec![
                "SOLID",
                "DASH",
                "DOT",
                "DASH_DOT",
                "DASH_DOT_DOT",
                "LONG_DASH",
                "NONE",
            ]
        } else {
            vec![
                "NONE",
                "SOLID",
                "DOT",
                "BOLD",
                "DASH",
                "DASH_DOT",
                "DASH_DOT_DOT",
            ]
        };
        let plain = first_id;
        let mut extra = attrs(&base, "hh:charPr", &[("id", plain.to_string())])?;
        let mut body = format!(
            r#"<hp:p id="900000" paraPrIDRef="5" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="{plain}">{sec}<hp:ctrl>{col}</hp:ctrl><hp:t>{mode} — repaired complete package</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="0" vertsize="1400" textheight="1200" baseline="1100" spacing="600" horzpos="0" horzsize="51528" flags="393216"/></hp:linesegarray></hp:p>"#
        );
        for (i, style) in styles.iter().enumerate() {
            let y = 6000 + i * 8500;
            let pid = 900001 + i;
            let content = if mode == "text-outlines" {
                let id = first_id + i + 1;
                let ch = attrs(
                    &base,
                    "hh:charPr",
                    &[
                        ("id", id.to_string()),
                        ("height", "3000".into()),
                        ("textColor", "#185890".into()),
                    ],
                )?;
                extra += &set(&ch, "hh:outline", &[("type", style)])?;
                format!(
                    r#"<hp:run charPrIDRef="{plain}"><hp:t>{style} </hp:t></hp:run><hp:run charPrIDRef="{id}"><hp:t>가나다 ABC 123</hp:t></hp:run><hp:run charPrIDRef="{plain}"><hp:t> 보통 글자</hp:t></hp:run>"#
                )
            } else {
                let mut shapes = String::new();
                for (j, kind) in ["line", "rect"].iter().enumerate() {
                    let mut object = element(&section, kind)?;
                    if let Some(a) = object.find("<hp:drawText ") {
                        let b = a
                            + object[a..]
                                .find("</hp:drawText>")
                                .context("Draw text end")?
                            + 14;
                        object.replace_range(a..b, "");
                    }
                    object = attrs(
                        &object,
                        &format!("hp:{kind}"),
                        &[
                            ("id", (910000 + i * 2 + j).to_string()),
                            ("instid", (920000 + i * 2 + j).to_string()),
                            ("zOrder", (i * 2 + j).to_string()),
                            ("textWrap", "IN_FRONT_OF_TEXT".into()),
                        ],
                    )?;
                    for tag in ["hp:orgSz", "hp:curSz", "hp:sz"] {
                        object = set(&object, tag, &[("width", "14000"), ("height", "4500")])?;
                    }
                    object = set(
                        &object,
                        "hp:rotationInfo",
                        &[("centerX", "7000"), ("centerY", "2250")],
                    )?;
                    object = set(&object, "hc:scaMatrix", &[("e1", "1"), ("e5", "1")])?;
                    object = set(
                        &object,
                        "hp:lineShape",
                        &[("style", style), ("width", "100"), ("color", "#185890")],
                    )?;
                    object = attrs(
                        &object,
                        "hp:pos",
                        &[
                            ("treatAsChar", "0".into()),
                            ("vertRelTo", "PAPER".into()),
                            ("horzRelTo", "PAPER".into()),
                            ("vertOffset", (y + 5000).to_string()),
                            ("horzOffset", (18000 + j * 18000).to_string()),
                            ("flowWithText", "0".into()),
                            ("allowOverlap", "1".into()),
                        ],
                    )?;
                    if *kind == "rect" {
                        for (tag, x, y) in [
                            ("hc:pt0", "0", "0"),
                            ("hc:pt1", "14000", "0"),
                            ("hc:pt2", "14000", "4500"),
                            ("hc:pt3", "0", "4500"),
                        ] {
                            object = set(&object, tag, &[("x", x), ("y", y)])?;
                        }
                        object = set(&object, "hc:winBrush", &[("faceColor", "#E0EDF8")])?;
                    } else {
                        object = set(&object, "hc:endPt", &[("x", "14000"), ("y", "4500")])?;
                    }
                    shapes += &object;
                }
                format!(r#"<hp:run charPrIDRef="{plain}"><hp:t>{style}</hp:t>{shapes}</hp:run>"#)
            };
            body += &format!(
                r#"<hp:p id="{pid}" paraPrIDRef="5" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0">{content}<hp:linesegarray><hp:lineseg textpos="0" vertpos="{y}" vertsize="4000" textheight="3000" baseline="3000" spacing="2000" horzpos="0" horzsize="51528" flags="393216"/></hp:linesegarray></hp:p>"#
            );
        }
        let insert = chars.range().end - "</hh:charProperties>".len();
        let mut h = header.clone();
        h.insert_str(insert, &extra);
        h = attrs(
            &h,
            "hh:charProperties",
            &[(
                "itemCnt",
                (chars.children().filter(|n| n.is_element()).count()
                    + if mode == "text-outlines" { 8 } else { 1 })
                .to_string(),
            )],
        )?;
        let s = format!("{opening}{body}</hs:sec>");
        roxmltree::Document::parse(&h)?;
        roxmltree::Document::parse(&s)?;
        let mut out = zip::ZipWriter::new(std::fs::File::create(
            std::path::Path::new(&args[1]).join(format!("{mode}.hwpx")),
        )?);
        for (name, bytes) in &files {
            out.start_file(
                name,
                zip::write::SimpleFileOptions::default().compression_method(
                    if name == "mimetype" {
                        zip::CompressionMethod::Stored
                    } else {
                        zip::CompressionMethod::Deflated
                    },
                ),
            )?;
            out.write_all(match name.as_str() {
                "Contents/header.xml" => h.as_bytes(),
                "Contents/section0.xml" => s.as_bytes(),
                _ => bytes,
            })?;
        }
        out.finish()?;
    }
    Ok(())
}
