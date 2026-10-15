//! HWP5 tab definitions (revision 1.3 table 36).
use super::*;
pub(super) fn tabs(d: &[u8], id: usize) -> Result<Node> {
    if d.len() == 4 {
        return Ok(n("tabPr").at("id", id));
    }
    let count = u32(d, 4)? as usize;
    anyhow::ensure!(count <= 10000, "Excessive tab definitions");
    bytes(d, 8, count * 8)?;
    let mut tab = n("tabPr").at("id", id);
    for i in 0..count {
        let off = 8 + i * 8;
        tab.children.push(
            n("tabItem")
                .at("pos", u32(d, off)?)
                .at(
                    "type",
                    match bytes(d, off + 4, 1)?[0] {
                        1 => "RIGHT",
                        2 => "CENTER",
                        3 => "DECIMAL",
                        _ => "LEFT",
                    },
                )
                .at(
                    "leader",
                    match bytes(d, off + 5, 1)?[0] {
                        1 => "SOLID",
                        2 => "DASH",
                        3 => "DOT",
                        _ => "NONE",
                    },
                ),
        );
    }
    Ok(tab)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_flag_only_tab_record() {
        assert!(tabs(&[0; 4], 0).unwrap().children.is_empty());
        assert!(tabs(&[0; 3], 0).is_err());
    }
}
