//! HWP binary adapter: paragraphs.
use super::*;

pub(super) fn paragraph(r: &Record) -> Result<Node> {
    let d = &r.data;
    let mut out = n("p")
        .at("id", format!("hwp-{:x}", r.offset))
        .at("paraPrIDRef", u16(d, 8)?)
        .at(
            "pageBreak",
            if bytes(d, 11, 1)?[0] & 4 != 0 { 1 } else { 0 },
        );
    let runs = r.first(68).context("Missing character-shape runs")?;
    ensure!(runs.data.len() % 8 == 0, "Invalid shape-run length");
    let runs: Vec<(usize, u32)> = runs
        .data
        .as_chunks::<8>()
        .0
        .iter()
        .map(|d| Ok((u32(d, 0)? as usize, u32(d, 4)?)))
        .collect::<Result<_>>()?;
    ensure!(!runs.is_empty(), "Empty character runs");
    let controls: Vec<_> = r.children.iter().filter(|r| r.tag == 71).collect();
    let mut consumed = BTreeSet::new();
    let textdata = r.first(67).map(|r| r.data.as_slice()).unwrap_or(&[]);
    ensure!(textdata.len().is_multiple_of(2), "Invalid text length");
    let mut pos = 0;
    while pos < textdata.len() / 2 {
        let code = u16(textdata, 2 * pos)?;
        let style = runs
            .iter()
            .rev()
            .find(|(p, _)| *p <= pos)
            .unwrap_or(&runs[0])
            .1;
        let mut run = n("run").at("charPrIDRef", style);
        let mut advance = 1;
        if code >= 32 {
            advance = if (0xd800..=0xdbff).contains(&code) {
                2
            } else {
                1
            };
            let value = utf16(bytes(textdata, pos * 2, advance * 2)?)?;
            let mapped = match value.as_str() {
                "\u{f012b}" => Some("(인)"),
                "\u{f081c}" => Some(""),
                "\u{329e}" => Some("(인)"),
                _ => None,
            };
            run.children.push(n("t").with(vec![if let Some(s) = mapped {
                n("hwpDisplay").at("units", advance).at("value", s)
            } else {
                txt(value)
            }]));
        } else if ![0, 10, 13, 24, 25, 30, 31].contains(&code) {
            advance = 8;
            bytes(textdata, pos * 2, 16)?;
            if code == 4 {
                run.children.push(n("ctrl").with(vec![n("fieldEnd")]));
            } else if code == 9 {
                run.children
                    .push(n("t").with(vec![n("tab").at("width", u32(textdata, pos * 2 + 2)?)]));
            } else {
                let id: String = bytes(textdata, pos * 2 + 2, 4)?
                    .iter()
                    .rev()
                    .map(|b| *b as char)
                    .collect();
                if let Some((i, c)) = controls
                    .iter()
                    .enumerate()
                    .find(|(i, c)| !consumed.contains(i) && c.id() == id)
                {
                    consumed.insert(i);
                    run.children.push(control(c)?);
                } else {
                    run.children
                        .push(n("ctrl").with(vec![n("opaque").at("code", code)]));
                }
            }
        } else {
            run.children.push(n("t").with(vec![match code {
                10 => n("lineBreak"),
                30 | 31 => n("nbSpace"),
                24 => txt("-"),
                _ => n("lineBreak"),
            }]))
        }
        out.children.push(run);
        pos += advance;
    }
    // Empty paragraphs still carry the document's character and line-cache style.
    if out.children.is_empty() {
        out.children.push(n("run").at("charPrIDRef", runs[0].1));
    }
    ensure!(
        consumed.len() == controls.len(),
        "Unmatched control in paragraph {:x}: {:?}",
        r.offset,
        controls
            .iter()
            .enumerate()
            .filter(|(i, _)| !consumed.contains(i))
            .map(|(_, c)| c.id())
            .collect::<Vec<_>>()
    );
    let ls = r.first(69).context("Missing saved HWP line layout")?;
    ensure!(ls.data.len() % 36 == 0, "Invalid line record length");
    let mut linearray = n("linesegarray");
    for d in ls.data.as_chunks::<36>().0 {
        let mut l = n("lineseg");
        for (i, key) in [
            "textpos",
            "vertpos",
            "vertsize",
            "textheight",
            "baseline",
            "spacing",
            "horzpos",
            "horzsize",
            "flags",
        ]
        .iter()
        .enumerate()
        {
            l = l.at(
                key,
                if i == 0 || i == 8 {
                    u32(d, 4 * i)? as i64
                } else {
                    i32(d, 4 * i)? as i64
                },
            );
        }
        linearray.children.push(l);
    }
    out.children.push(linearray);
    Ok(out)
}
