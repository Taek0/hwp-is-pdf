//! Numbering records, including optional level starts and levels eight to ten.
use super::*;
pub(super) fn bullet(d: &[u8], id: usize) -> Result<Node> {
    let props = u32(d, 0)?;
    let mut bullet = n("bullet")
        .at("id", id)
        .at("useInstWidth", (props >> 2) & 1)
        .at("autoIndent", (props >> 3) & 1)
        .at("charPrIDRef", u32(d, 8)?)
        .at("textOffset", u16(d, 6)?)
        .at("widthAdjust", u16(d, 4)? as i16)
        .at(
            "align",
            match props & 3 {
                1 => "CENTER",
                2 => "RIGHT",
                _ => "LEFT",
            },
        )
        .at(
            "textOffsetType",
            if props & 16 == 0 {
                "PERCENT"
            } else {
                "HWPUNIT"
            },
        )
        .at("char", utf16(bytes(d, 12, 2)?)?);
    if d.len() >= 18 && u32(d, 14)? != 0 {
        let image = u16(d, 21)?;
        anyhow::ensure!(image > 0, "Missing binary image bullet ID");
        bullet = bullet.at("useImage", 1).with(vec![
            n("img")
                .at("binaryItemIDRef", format!("image{image}"))
                .at("contrast", bytes(d, 18, 1)?[0] as i8)
                .at("bright", bytes(d, 19, 1)?[0] as i8)
                .at("effect", bytes(d, 20, 1)?[0]),
        ]);
    }
    if d.len() >= 25 {
        bullet = bullet.at("checkedChar", utf16(bytes(d, 23, 2)?)?);
    }
    Ok(bullet)
}
pub(super) fn numbering(d: &[u8], id: usize) -> Result<Node> {
    let mut number = n("numbering").at("id", id);
    let mut off = 0;
    for level in 1..=7 {
        let props = u32(d, off)?;
        let count = u16(d, off + 12)? as usize;
        let fmt = utf16(bytes(d, off + 14, count * 2)?)?;
        number.children.push(
            n("paraHead")
                .at("level", level)
                .at("useInstWidth", (props >> 2) & 1)
                .at("autoIndent", (props >> 3) & 1)
                .at("charPrIDRef", u32(d, off + 8)?)
                .at("widthAdjust", u16(d, off + 4)? as i16)
                .at(
                    "align",
                    match props & 3 {
                        1 => "CENTER",
                        2 => "RIGHT",
                        _ => "LEFT",
                    },
                )
                .at("textOffset", u16(d, off + 6)?)
                .at(
                    "textOffsetType",
                    if props & 16 == 0 {
                        "PERCENT"
                    } else {
                        "HWPUNIT"
                    },
                )
                .with(vec![txt(fmt)]),
        );
        off += 14 + count * 2;
    }
    number = number.at("start", u16(d, off)?);
    off += 2;
    if d.len() >= off + 28 {
        for i in 0..7 {
            number.children[i]
                .attrs
                .insert("start".into(), u32(d, off + i * 4)?.to_string());
        }
        off += 28;
        if d.len() > off {
            // Real 5.1 records also carry the complete 12-byte heading fields
            // for levels 8–10; the public table lists only their format strings.
            let full = (|| -> Result<(Vec<Node>, usize)> {
                let mut at = off;
                let mut heads = Vec::new();
                for level in 8..=10 {
                    let props = u32(d, at)?;
                    let count = u16(d, at + 12)? as usize;
                    let value = utf16(bytes(d, at + 14, count * 2)?)?;
                    heads.push(
                        n("paraHead")
                            .at("level", level)
                            .at("useInstWidth", (props >> 2) & 1)
                            .at("autoIndent", (props >> 3) & 1)
                            .at("charPrIDRef", u32(d, at + 8)?)
                            .at("widthAdjust", u16(d, at + 4)? as i16)
                            .at("textOffset", u16(d, at + 6)?)
                            .at(
                                "textOffsetType",
                                if props & 16 == 0 {
                                    "PERCENT"
                                } else {
                                    "HWPUNIT"
                                },
                            )
                            .at(
                                "align",
                                match props & 3 {
                                    1 => "CENTER",
                                    2 => "RIGHT",
                                    _ => "LEFT",
                                },
                            )
                            .with(vec![txt(value)]),
                    );
                    at += 14 + count * 2;
                }
                anyhow::ensure!(
                    matches!(d.len() - at, 0 | 12),
                    "Unexpected extended heading tail"
                );
                Ok((heads, at))
            })();
            if let Ok((heads, end)) = full {
                number.children.extend(heads);
                off = end;
            } else {
                for level in 8..=10 {
                    let count = u16(d, off)? as usize;
                    off += 2;
                    let value = utf16(bytes(d, off, count * 2)?)?;
                    off += count * 2;
                    let mut head = number.children[6].clone();
                    head.attrs.insert("level".into(), level.to_string());
                    head.attrs.remove("start");
                    head.children = vec![txt(value)];
                    number.children.push(head);
                }
            }
            if d.len() >= off + 12 {
                for i in 0..3 {
                    number.children[i + 7]
                        .attrs
                        .insert("start".into(), u32(d, off + i * 4)?.to_string());
                }
            }
        }
    }
    Ok(number)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bullet_width_and_hanging_flags_are_independent() {
        for props in [0u32, 4, 8, 12] {
            let mut data = vec![0; 14];
            data[..4].copy_from_slice(&props.to_le_bytes());
            let n = bullet(&data, 1).unwrap();
            assert_eq!(n.u("useInstWidth"), (props as usize >> 2) & 1);
            assert_eq!(n.u("autoIndent"), (props as usize >> 3) & 1);
        }
    }
    #[test]
    fn image_bullet_record_keeps_binary_reference() {
        let mut d = vec![0; 25];
        d[14..18].copy_from_slice(&1u32.to_le_bytes());
        d[21..23].copy_from_slice(&3u16.to_le_bytes());
        let b = bullet(&d, 1).unwrap();
        assert_eq!(b.a("useImage"), "1");
        assert_eq!(b.child("img").unwrap().a("binaryItemIDRef"), "image3");
        assert!(bullet(&d[..22], 1).is_err());
    }
    #[test]
    fn extended_starts_and_formats() {
        let mut d = Vec::new();
        for _ in 0..7 {
            d.extend([0u8; 12]);
            d.extend(2u16.to_le_bytes());
            for c in "^1".encode_utf16() {
                d.extend(c.to_le_bytes());
            }
        }
        d.extend(1u16.to_le_bytes());
        for i in 1..=7u32 {
            d.extend(i.to_le_bytes());
        }
        for _ in 0..3 {
            d.extend(2u16.to_le_bytes());
            for c in "^N".encode_utf16() {
                d.extend(c.to_le_bytes());
            }
        }
        for i in 8..=10u32 {
            d.extend(i.to_le_bytes());
        }
        let n = numbering(&d, 1).unwrap();
        assert_eq!(n.children.len(), 10);
        assert_eq!(n.children[9].u("start"), 10);
        assert_eq!(crate::model::text(&n.children[9]), "^N");
    }
}
