//! HWP binary adapter: section.
use super::*;

pub(super) fn section_properties(r: &Record) -> Result<Node> {
    let page = r.first(73).context("Missing HWP page definition")?;
    let d = &page.data;
    let mut margin = n("margin");
    for (i, key) in [
        "left", "right", "top", "bottom", "header", "footer", "gutter",
    ]
    .iter()
    .enumerate()
    {
        margin = margin.at(key, u32(d, 8 + 4 * i)?);
    }
    // HWP 5 table 132 stores nominal paper dimensions separately from orientation.
    let (width, height) = (u32(d, 0)?, u32(d, 4)?);
    let landscape = u32(d, 36)? & 1 != 0;
    let page = n("pagePr")
        .at("width", if landscape { height } else { width })
        .at("height", if landscape { width } else { height })
        .with(vec![margin]);
    let mut sec = n("secPr")
        .at("binaryHwp", 1)
        .at(
            "hideFirstHeader",
            if u32(&r.data, 4)? & 1 != 0 { 1 } else { 0 },
        )
        .at(
            "hideFirstFooter",
            if u32(&r.data, 4)? & 2 != 0 { 1 } else { 0 },
        )
        .at(
            "hideFirstMasterPage",
            if u32(&r.data, 4)? & 4 != 0 { 1 } else { 0 },
        )
        .with(vec![page]);
    for (index, r) in r
        .children
        .iter()
        .filter(|c| c.tag == 74)
        .take(2)
        .enumerate()
    {
        let d = &r.data;
        let extra = usize::from(d.len() >= 28) * 2;
        let flags = u32(d, 0)?;
        let char_at = |off| -> Result<String> {
            Ok(char::from_u32(u16(d, off)? as u32)
                .filter(|c| *c != '\0')
                .map(|c| c.to_string())
                .unwrap_or_default())
        };
        sec.children.push(
            n(if index == 0 {
                "footNotePr"
            } else {
                "endNotePr"
            })
            .with(vec![
                n("autoNumFormat")
                    .at("typeCode", flags & 255)
                    .at("prefixChar", char_at(6)?)
                    .at("suffixChar", char_at(8)?),
                n("noteLine")
                    .at("length", i16::from_le_bytes(bytes(d, 12, 2)?.try_into()?))
                    .at(
                        "type",
                        if bytes(d, 20 + extra, 1)?[0] == 0 {
                            "NONE"
                        } else {
                            "SOLID"
                        },
                    )
                    .at("widthIndex", bytes(d, 21 + extra, 1)?[0])
                    .at("color", rgb(u32(d, 22 + extra)?)),
                n("noteSpacing")
                    .at("aboveLine", u16(d, 14 + extra)?)
                    .at("belowLine", u16(d, 16 + extra)?)
                    .at("betweenNotes", u16(d, 18 + extra)?),
                n("placement")
                    .at(
                        "place",
                        if index == 0 {
                            [
                                "EACH_COLUMN",
                                "MERGED_COLUMN",
                                "RIGHT_MOST_COLUMN",
                                "EACH_COLUMN",
                            ][((flags >> 8) & 3) as usize]
                        } else if flags & 256 != 0 {
                            "END_OF_SECTION"
                        } else {
                            "END_OF_DOCUMENT"
                        },
                    )
                    .at("beneathText", (flags >> 13) & 1),
            ]),
        );
    }
    let flags = u32(&r.data, 4)?;
    let visibility = |hide, only| {
        if flags & hide != 0 {
            "HIDE_FIRST"
        } else if flags & only != 0 {
            "SHOW_FIRST"
        } else {
            "SHOW_ALL"
        }
    };
    sec.children.push(
        n("visibility")
            .at("border", visibility(8, 256))
            .at("fill", visibility(16, 512)),
    );
    for (index, border) in r
        .children
        .iter()
        .filter(|r| r.tag == 75)
        .take(3)
        .enumerate()
    {
        sec.children
            .push(page_border(&border.data, ["BOTH", "EVEN", "ODD"][index])?);
    }
    Ok(sec)
}

fn page_border(d: &[u8], kind: &str) -> Result<Node> {
    let flags = u32(d, 0)?;
    // Table 135's fields total 14 bytes, despite its printed total of 12.
    Ok(n("pageBorderFill")
        .at("type", kind)
        .at(
            "borderReference",
            if flags & 1 != 0 { "PAPER" } else { "CONTENT" },
        )
        .at("headerInside", (flags >> 1) & 1)
        .at("footerInside", (flags >> 2) & 1)
        .at(
            "fillArea",
            ["PAPER", "PAGE", "BORDER", "BORDER"][((flags >> 3) & 3) as usize],
        )
        .at("borderFillIDRef", u16(d, 12)?)
        .with(vec![margins("offset", d, 4)?]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_border_record_preserves_flags_offsets_and_rejects_truncation() {
        let d = [23, 0, 0, 0, 100, 0, 200, 0, 44, 1, 144, 1, 7, 0];
        let n = page_border(&d, "ODD").unwrap();
        assert_eq!(n.a("borderReference"), "PAPER");
        assert_eq!(n.a("headerInside"), "1");
        assert_eq!(n.a("footerInside"), "1");
        assert_eq!(n.a("fillArea"), "BORDER");
        assert_eq!(n.a("borderFillIDRef"), "7");
        assert_eq!(n.child("offset").unwrap().a("bottom"), "400");
        assert!(page_border(&d[..12], "BOTH").is_err());
    }
}
