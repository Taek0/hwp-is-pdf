//! Bounded HWP5 gradient and image brush payloads shared by border and drawing fills.
use super::*;
fn gradient_payload(d: &[u8], offset: usize) -> Result<(Option<Node>, usize)> {
    let flags = u32(d, offset)?;
    let mut at = offset + 4 + if flags & 1 != 0 { 12 } else { 0 };
    if flags & 4 == 0 {
        return Ok((None, at));
    }
    let kind = bytes(d, at, 1)?[0] as usize;
    let count = u32(d, at + 17)? as usize;
    ensure!((2..=256).contains(&count), "Invalid gradient color count");
    let mut g = n("gradation")
        .at(
            "type",
            ["LINEAR", "LINEAR", "RADIAL", "CONICAL", "SQUARE"]
                .get(kind)
                .copied()
                .unwrap_or("LINEAR"),
        )
        .at("angle", i32(d, at + 1)?)
        .at("centerX", i32(d, at + 5)?)
        .at("centerY", i32(d, at + 9)?)
        .at("step", u32(d, at + 13)?);
    at += 21;
    if count > 2 {
        bytes(d, at, count * 4)?;
        at += count * 4;
    }
    for _ in 0..count {
        g.children.push(n("color").at("value", rgb(u32(d, at)?)));
        at += 4;
    }
    let extra_at = at + if flags & 2 != 0 { 6 } else { 0 };
    if d.len() >= extra_at + 4 {
        let extra = u32(d, extra_at)? as usize;
        if extra > 0 {
            let extra_bytes = bytes(d, extra_at + 4, extra)?;
            g = g.at("stepCenter", extra_bytes[0]);
        }
    }
    Ok((Some(g), at))
}
pub(super) fn gradient(d: &[u8], offset: usize) -> Result<Option<Node>> {
    Ok(gradient_payload(d, offset)?.0)
}
pub(super) fn image(d: &[u8], offset: usize) -> Result<Option<Node>> {
    let flags = u32(d, offset)?;
    if flags & 2 == 0 {
        return Ok(None);
    }
    let at = gradient_payload(d, offset)?.1;
    let mode = bytes(d, at, 1)?[0] as usize;
    if mode >= 15 {
        return Ok(None);
    }
    let modes = [
        "TILE",
        "TILE_HORZ_TOP",
        "TILE_HORZ_BOTTOM",
        "TILE_VERT_LEFT",
        "TILE_VERT_RIGHT",
        "TOTAL",
        "CENTER",
        "CENTER_TOP",
        "CENTER_BOTTOM",
        "LEFT_CENTER",
        "LEFT_TOP",
        "LEFT_BOTTOM",
        "RIGHT_CENTER",
        "RIGHT_TOP",
        "RIGHT_BOTTOM",
        "NONE",
    ];
    let image = n("img")
        .at("binaryItemIDRef", format!("image{}", u16(d, at + 4)?))
        .at("bright", bytes(d, at + 1, 1)?[0] as i8)
        .at("contrast", bytes(d, at + 2, 1)?[0] as i8)
        .at("effect", bytes(d, at + 3, 1)?[0]);
    Ok(Some(
        n("imgBrush")
            .at("mode", modes.get(mode).copied().unwrap_or("NONE"))
            .with(vec![image]),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_after_solid_fill_keeps_signed_effects_and_checks_lengths() {
        let mut d = vec![0; 22];
        d[0] = 3;
        d[16] = 5;
        d[17] = 10;
        d[18] = 236;
        d[19] = 1;
        d[20] = 7;
        let b = image(&d, 0).unwrap().unwrap();
        assert_eq!(b.a("mode"), "TOTAL");
        let i = b.child("img").unwrap();
        assert_eq!(i.a("binaryItemIDRef"), "image7");
        assert_eq!(i.a("contrast"), "-20");
        assert_eq!(i.a("bright"), "10");
        assert!(image(&d[..21], 0).is_err());
    }
}

#[cfg(test)]
mod gradient_tests {
    use super::*;
    #[test]
    fn schedule_gradient_uses_byte_type_and_dword_fields() {
        let mut d = 4u32.to_le_bytes().to_vec();
        d.push(1);
        for v in [0u32, 50, 50, 2, 2, 0, 0xffffff, 1] {
            d.extend(v.to_le_bytes());
        }
        d.extend([10, 0]);
        let g = gradient(&d, 0).unwrap().unwrap();
        assert_eq!(g.a("step"), "2");
        assert_eq!(g.a("stepCenter"), "10");
        assert_eq!(g.children.len(), 2);
        assert!(gradient(&d[..29], 0).is_err());
    }
}
