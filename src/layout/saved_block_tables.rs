//! Native cell caches can place the anchor line after a floating block table.
use super::*;
pub(super) fn mark(table: &mut Node) -> bool {
    let mut changed = false;
    for row in table.children.iter_mut().filter(|n| n.name == "tr") {
        for cell in row.children.iter_mut().filter(|n| n.name == "tc") {
            let Some(list) = cell
                .children
                .iter_mut()
                .find(|n| n.name == "subList" && n.a("savedColumns") != "1")
            else {
                continue;
            };
            let mut previous_end: Option<f64> = None;
            for p in list.children.iter_mut().filter(|n| n.name == "p") {
                let ls = lines(p);
                let first = ls.first().map(|l| l.n("vertpos") / 100.);
                let next_end = ls
                    .last()
                    .map(|l| (l.n("vertpos") + l.n("vertsize") + l.n("spacing")) / 100.);
                let items = Engine::items(p);
                let objects = items
                    .iter()
                    .filter(|i| matches!(i.kind, ItemKind::Object(_)))
                    .count();
                let pure = items.iter().all(|i| {
                    matches!(i.kind, ItemKind::Object(_))
                        || matches!(i.kind,ItemKind::Char(c) if c.is_whitespace())
                });
                if let (Some(before), Some(first)) = (previous_end, first)
                    && objects == 1
                    && pure
                    && p.a("reflowed") != "1"
                    && p.a("pageBreak") != "1"
                    && p.a("columnBreak") != "1"
                {
                    for run in p.children.iter_mut().filter(|n| n.name == "run") {
                        for object in run.children.iter_mut().filter(|n| n.name == "tbl") {
                            let eligible = object.a("binaryHwp") == "1"
                                && object.a("textWrap") == "TOP_AND_BOTTOM"
                                && object.child("caption").is_none()
                                && object.child("pos").is_some_and(|pos| {
                                    pos.a("treatAsChar") == "0"
                                        && pos.a("vertRelTo") == "PARA"
                                        && pos.a("vertAlign") == "TOP"
                                        && pos.signed("vertOffset").abs() < 0.01
                                });
                            let margins = margin(object, "outMargin");
                            let extent = effective_height(object) + margins[2] + margins[3];
                            if eligible && extent > 0. && ((first - before) - extent).abs() <= 0.5 {
                                object
                                    .attrs
                                    .insert("savedBeforeLine".into(), extent.to_string());
                                changed = true;
                            }
                        }
                    }
                }
                previous_end = next_end;
            }
        }
    }
    changed
}

pub(super) fn mark_fragments(table: &mut Node) -> bool {
    let mut changed = false;
    for row in table.children.iter_mut().filter(|n| n.name == "tr") {
        for cell in row.children.iter_mut().filter(|n| n.name == "tc") {
            let Some(list) = cell
                .children
                .iter_mut()
                .find(|n| n.name == "subList" && n.a("savedColumns") != "1")
            else {
                continue;
            };
            let mut previous = None;
            let mut end = 0.;
            let mut first = None;
            let mut page = 0;
            let mut pages = Vec::new();
            let mut ambiguous = false;
            for p in list.children_named("p") {
                for (i, l) in lines(p).into_iter().enumerate() {
                    let y = l.n("vertpos") / 100.;
                    if previous.is_some_and(|old| y < old) {
                        if i != 0 {
                            ambiguous = true;
                            break;
                        }
                        first.get_or_insert(end);
                        page += 1;
                    }
                    previous = Some(y);
                    end = y + l.n("vertsize") / 100.;
                }
                pages.push(page);
            }
            if let Some(height) = first
                && !ambiguous
            {
                list.attrs
                    .insert("savedFirstFragmentHeight".into(), height.to_string());
                for (p, page) in list
                    .children
                    .iter_mut()
                    .filter(|n| n.name == "p")
                    .zip(pages)
                {
                    p.attrs.insert("savedCellPage".into(), page.to_string());
                }
                changed = true;
            }
        }
    }
    changed
}

pub(super) fn reserve_continuation(list: &Node, delta: f64) -> Node {
    let positions: Vec<_> = list
        .children_named("p")
        .map(|p| {
            flowing_lines(list)
                .into_iter()
                .filter(|(node, _, _)| std::ptr::eq(*node, p))
                .map(|(_, _, y)| y)
                .collect::<Vec<_>>()
        })
        .collect();
    let mut copy = list.clone();
    for (p, ys) in copy
        .children
        .iter_mut()
        .filter(|n| n.name == "p")
        .zip(positions)
    {
        let shift = if p.u("savedCellPage") > 0 { delta } else { 0. };
        if let Some(cache) = p.children.iter_mut().find(|n| n.name == "linesegarray") {
            for (line, y) in cache
                .children
                .iter_mut()
                .filter(|n| n.name == "lineseg")
                .zip(ys)
            {
                line.attrs
                    .insert("vertpos".into(), ((y + shift) * 100.).to_string());
            }
        }
    }
    copy
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_reserved_gap_is_required_before_back_positioning_a_table() {
        for (y, expected) in [(12200, true), (12000, false), (2000, false)] {
            let mut n=crate::model::xml(format!(r#"<tbl><tr><tc><subList><p><run><t>Before</t></run><linesegarray><lineseg vertpos="0" vertsize="1500" spacing="500"/></linesegarray></p><p><run><tbl binaryHwp="1" textWrap="TOP_AND_BOTTOM"><sz height="10000"/><outMargin top="100" bottom="100"/><pos treatAsChar="0" vertRelTo="PARA" vertAlign="TOP" vertOffset="0"/></tbl></run><linesegarray><lineseg vertpos="{y}" vertsize="1000" spacing="0"/></linesegarray></p></subList></tc></tr></tbl>"#).as_bytes()).unwrap();
            assert_eq!(mark(&mut n), expected);
            let mut tables = Vec::new();
            n.all("tbl", &mut tables);
            assert_eq!(
                tables[1].n("savedBeforeLine"),
                if expected { 102. } else { 0. }
            );
        }
    }
}
