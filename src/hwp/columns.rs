//! HWP5 column-control decoding (format revision 1.3, tables 138–139).
use super::*;
pub(super) fn columns(d: &[u8]) -> Result<Node> {
    let flags = u16(d, 4)?;
    let count = ((flags >> 2) & 255).max(1);
    let gap = u16(d, 6)?;
    let mut column = n("colPr")
        .at("colCount", count)
        .at("sameGap", gap)
        .at(
            "type",
            match flags & 3 {
                1 => "BALANCED_NEWSPAPER",
                2 => "PARALLEL",
                _ => "NEWSPAPER",
            },
        )
        .at(
            "layout",
            match (flags >> 10) & 3 {
                1 => "RIGHT",
                2 => "MIRROR",
                _ => "LEFT",
            },
        )
        .at("sameSz", usize::from(flags & (1 << 12) != 0));
    if count > 1 && flags & (1 << 12) == 0 {
        for i in 0..count as usize {
            column.children.push(
                n("colSz")
                    .at("width", u16(d, 8 + i * 2)?)
                    .at("gap", if i + 1 < count as usize { gap } else { 0 }),
            );
        }
    }
    Ok(n("ctrl").with(vec![column]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unequal_columns_keep_binary_widths_and_direction() {
        let mut d = vec![0; 12];
        let flags: u16 = (2 << 2) | (1 << 10) | 1;
        d[4..6].copy_from_slice(&flags.to_le_bytes());
        d[6..8].copy_from_slice(&2400u16.to_le_bytes());
        d[8..10].copy_from_slice(&15000u16.to_le_bytes());
        d[10..12].copy_from_slice(&32000u16.to_le_bytes());
        let ctrl = columns(&d).unwrap();
        let c = ctrl.child("colPr").unwrap();
        assert_eq!(c.a("type"), "BALANCED_NEWSPAPER");
        assert_eq!(c.a("layout"), "RIGHT");
        assert_eq!(
            c.children_named("colSz")
                .map(|n| n.u("width"))
                .collect::<Vec<_>>(),
            vec![15000, 32000]
        );
        assert!(columns(&d[..10]).is_err());
    }
}
