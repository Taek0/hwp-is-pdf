//! A repeatable header is a complete merged band followed by body rows.
use super::*;
pub(super) fn band_rows(n: &Node) -> Option<usize> {
    let rows = n.u("rowCnt");
    if !matches!(n.a("repeatHeader"), "1" | "true") || rows < 2 {
        return None;
    }
    let cells: Vec<_> = n
        .children_named("tr")
        .flat_map(|r| r.children_named("tc"))
        .collect();
    let start = |c: &Node| c.child("cellAddr").map(|a| a.u("rowAddr")).unwrap_or(0);
    let end = |c: &Node| start(c) + c.child("cellSpan").map(|s| s.u("rowSpan")).unwrap_or(1);
    if cells.iter().any(|c| c.attrs.contains_key("header")) {
        let mut band = 0;
        for row in 0..rows {
            let covering: Vec<_> = cells
                .iter()
                .filter(|c| start(c) <= row && end(c) > row)
                .collect();
            if covering.is_empty()
                || covering
                    .iter()
                    .any(|c| !matches!(c.a("header"), "1" | "true"))
            {
                break;
            }
            band = row + 1;
        }
        if band == 0 || band == rows || cells.iter().any(|c| start(c) < band && end(c) > band) {
            return None;
        }
        Some(band)
    } else {
        // Without explicit cell flags, preserve the old first-row fallback.
        // A vertical merge alone must not turn body rows into repeated headers.
        let first: Vec<_> = cells.iter().filter(|c| start(c) == 0).collect();
        (!first.is_empty() && first.iter().all(|c| end(c) == 1)).then_some(1)
    }
}
