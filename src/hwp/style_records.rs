//! Named paragraph/character styles from HWP DocInfo (table 47).
use super::*;
pub(super) fn style(data: &[u8], id: usize) -> Result<Node> {
    let local_len = u16(data, 0)? as usize;
    let local = utf16(bytes(data, 2, local_len * 2)?)?;
    let english_at = 2 + local_len * 2;
    let english_len = u16(data, english_at)? as usize;
    let english = utf16(bytes(data, english_at + 2, english_len * 2)?)?;
    let fields = english_at + 2 + english_len * 2;
    Ok(n("style")
        .at("id", id)
        .at("name", local)
        .at("engName", english)
        .at(
            "type",
            if bytes(data, fields, 1)?[0] & 7 == 1 {
                "CHAR"
            } else {
                "PARA"
            },
        )
        .at("nextStyleIDRef", bytes(data, fields + 1, 1)?[0])
        .at("langID", u16(data, fields + 2)?)
        .at("paraPrIDRef", u16(data, fields + 4)?)
        .at("charPrIDRef", u16(data, fields + 6)?))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn style_names_and_shape_references_survive_and_truncation_fails() {
        let mut data = Vec::new();
        for name in ["쪽 번호", "Page Number"] {
            let words: Vec<_> = name.encode_utf16().collect();
            data.extend((words.len() as u16).to_le_bytes());
            for c in words {
                data.extend(c.to_le_bytes());
            }
        }
        data.extend([1, 0]);
        data.extend(0x0412u16.to_le_bytes());
        data.extend(3u16.to_le_bytes());
        data.extend(4u16.to_le_bytes());
        let n = style(&data, 13).unwrap();
        assert_eq!(n.a("name"), "쪽 번호");
        assert_eq!(n.a("engName"), "Page Number");
        assert_eq!(n.a("type"), "CHAR");
        assert_eq!(n.a("charPrIDRef"), "4");
        assert_eq!(n.a("paraPrIDRef"), "3");
        assert!(style(&data[..data.len() - 1], 13).is_err());
    }
}
