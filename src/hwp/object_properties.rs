//! HWP binary adapter: object properties.
use super::*;

pub(super) fn common(r: &Record, name: &str) -> Result<Node> {
    let d = &r.data;
    let flags = u32(d, 4)?;
    let vr = ["PAPER", "PAGE", "PARA", "PARA"][((flags >> 3) & 3) as usize];
    let hr = ["PAPER", "PAGE", "COLUMN", "PARA"][((flags >> 8) & 3) as usize];
    let aligns = [
        "LEFT", "CENTER", "RIGHT", "LEFT", "RIGHT", "LEFT", "LEFT", "LEFT",
    ];
    let pos = n("pos")
        .at("treatAsChar", flags & 1)
        .at("vertRelTo", vr)
        .at("horzRelTo", hr)
        .at("horzAlign", aligns[((flags >> 10) & 7) as usize])
        .at(
            "vertAlign",
            [
                "TOP", "CENTER", "BOTTOM", "TOP", "BOTTOM", "TOP", "TOP", "TOP",
            ][((flags >> 5) & 7) as usize],
        )
        .at("vertOffset", i32(d, 8)?)
        .at("horzOffset", i32(d, 12)?);
    let mut object = n(name)
        .at("id", format!("hwp-{:x}", r.offset))
        .at(
            "textWrap",
            match (flags >> 21) & 7 {
                1 => "TOP_AND_BOTTOM",
                2 => "BEHIND_TEXT",
                3 => "IN_FRONT_OF_TEXT",
                _ => "SQUARE",
            },
        )
        .at("binaryHwp", 1)
        .with(vec![
            n("sz").at("width", u32(d, 16)?).at("height", u32(d, 20)?),
            pos,
            margins("outMargin", d, 28)?,
        ]);
    let marker = r
        .children
        .iter()
        .find(|c| matches!(c.tag, 76 | 77))
        .map(|c| c.offset)
        .unwrap_or(usize::MAX);
    for (header, paras) in groups(&r.children)
        .into_iter()
        .filter(|(h, _)| h.offset < marker)
    {
        let d = &header.data;
        object.children.push(
            n("caption")
                .at(
                    "side",
                    ["LEFT", "RIGHT", "TOP", "BOTTOM"][(u32(d, 8)? & 3) as usize],
                )
                .at("width", u32(d, 12)?)
                .at("gap", u16(d, 16)?)
                .at("fullWidth", u32(d, 18)?)
                .at("expandToMargin", (u32(d, 8)? >> 2) & 1)
                .with(vec![list(header, &paras)?]),
        );
    }
    Ok(object)
}
