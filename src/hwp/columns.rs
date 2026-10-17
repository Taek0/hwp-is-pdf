//! HWP5 column-control decoding (format revision 1.3, tables 138–139).
use super::*;

/// A saved nested text list uses column-local coordinates just like body text.
/// Normalize those coordinates before height measurement and cell pagination.
pub(super) fn position_list(list: &mut Node) {
    let definitions: Vec<_> = list
        .children_named("p")
        .flat_map(|p| p.children_named("run"))
        .flat_map(|r| r.children_named("ctrl"))
        .filter_map(|c| c.child("colPr"))
        .collect();
    if definitions.len() != 1
        || definitions[0].u("colCount") < 2
        || definitions[0].a("sameSz") != "1"
        || definitions[0].a("layout") != "LEFT"
    {
        return;
    }
    let count = definitions[0].u("colCount");
    let gap = definitions[0].n("sameGap");
    let width = list
        .children_named("p")
        .filter_map(|p| p.child("linesegarray"))
        .flat_map(|a| &a.children)
        .map(|l| l.n("horzpos") + l.n("horzsize"))
        .fold(0., f64::max);
    if width <= 0. {
        return;
    }
    let mut last = None;
    let mut column = 0;
    // Validate first; never partially normalize an unrecognized multi-band list.
    for (p, l) in list.children_named("p").flat_map(|p| {
        p.child("linesegarray")
            .into_iter()
            .flat_map(move |a| a.children.iter().map(move |l| (p, l)))
    }) {
        let visible = !crate::model::text(p).trim().is_empty();
        if last.is_some_and(|(y, v)| l.n("vertpos") < y || (l.n("vertpos") == y && visible && v)) {
            column += 1;
        }
        if column >= count {
            return;
        }
        last = Some((l.n("vertpos"), visible));
    }
    last = None;
    column = 0;
    for p in &mut list.children {
        if p.name != "p" {
            continue;
        }
        let visible = !crate::model::text(p).trim().is_empty();
        if let Some(a) = p.children.iter_mut().find(|n| n.name == "linesegarray") {
            for l in &mut a.children {
                if last.is_some_and(|(y, v)| {
                    l.n("vertpos") < y || (l.n("vertpos") == y && visible && v)
                }) {
                    column += 1;
                }
                last = Some((l.n("vertpos"), visible));
                l.attrs.insert(
                    "horzpos".into(),
                    (l.n("horzpos") + column as f64 * (width + gap)).to_string(),
                );
                l.attrs.insert("savedColumn".into(), column.to_string());
            }
        }
    }
    list.attrs.insert("savedColumns".into(), "1".into());
}
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
    fn nested_columns_keep_local_y_and_add_horizontal_origin() {
        let mut list = crate::model::xml(br#"<subList><p><run><ctrl><colPr colCount="2" sameGap="1000" sameSz="1" layout="LEFT"/></ctrl><t>A</t></run><linesegarray><lineseg vertpos="0" horzsize="5000"/><lineseg vertpos="1600" horzsize="5000"/></linesegarray></p><p><run><t>B</t></run><linesegarray><lineseg vertpos="0" horzsize="5000"/><lineseg vertpos="1600" horzsize="5000"/></linesegarray></p></subList>"#).unwrap();
        position_list(&mut list);
        assert_eq!(list.a("savedColumns"), "1");
        let mut ls = vec![];
        list.all("lineseg", &mut ls);
        assert_eq!(
            ls.iter().map(|l| l.u("horzpos")).collect::<Vec<_>>(),
            vec![0, 0, 6000, 6000]
        );
        assert_eq!(
            ls.iter().map(|l| l.u("vertpos")).collect::<Vec<_>>(),
            vec![0, 1600, 0, 1600]
        );
    }
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
