//! Recover column boundaries from mutually consistent merged-cell widths.
use super::*;
pub(super) fn reconcile(n: &Node, prior: &[f64]) -> Option<Vec<f64>> {
    let cols = prior.len();
    if cols == 0 || cols > 1000 || cols != n.u("colCnt") {
        return None;
    }
    let mut edges = Vec::new();
    let mut inconsistent_prior = false;
    for row in n.children_named("tr") {
        let start_index = edges.len();
        for cell in row.children_named("tc") {
            let start = cell.child("cellAddr")?.u("colAddr");
            let span = cell.child("cellSpan")?.u("colSpan");
            let end = start.checked_add(span)?;
            let width = val(cell.child("cellSz"), "width");
            if span == 0 || end > cols || !width.is_finite() || width <= 0. {
                return None;
            }
            inconsistent_prior |= (prior[start..end].iter().sum::<f64>() - width).abs() > 0.1;
            edges.push((start, end, width));
        }
        // Older native rows can retain a slightly different total width after a
        // table resize. Reconcile complete rows to the declared extent before
        // solving crossing merges; incomplete rowspan rows are not rescaled.
        let declared = val(n.child("sz"), "width");
        let row_edges = &mut edges[start_index..];
        row_edges.sort_by_key(|e| e.0);
        let complete = row_edges.first().is_some_and(|e| e.0 == 0)
            && row_edges.last().is_some_and(|e| e.1 == cols)
            && row_edges.windows(2).all(|w| w[0].1 == w[1].0);
        let total = row_edges.iter().map(|e| e.2).sum::<f64>();
        if n.a("binaryHwp") == "1"
            && complete
            && declared > 0.
            && (total - declared).abs() / declared <= 0.01
        {
            for edge in row_edges {
                edge.2 *= declared / total;
            }
        }
    }
    if !inconsistent_prior {
        return None;
    }
    let mut adjacent = vec![Vec::new(); cols + 1];
    for &(a, b, w) in &edges {
        adjacent[a].push((b, w));
        adjacent[b].push((a, -w));
    }
    let mut positions = vec![None; cols + 1];
    positions[0] = Some(0.);
    let mut queue = std::collections::VecDeque::from([0]);
    while let Some(a) = queue.pop_front() {
        let origin = positions[a]?;
        for &(b, width) in &adjacent[a] {
            let target = origin + width;
            if let Some(current) = positions[b] {
                if (current - target).abs() > 0.05 {
                    return None;
                }
            } else {
                positions[b] = Some(target);
                queue.push_back(b);
            }
        }
    }
    // A disconnected authored boundary has no trustworthy absolute position.
    if edges
        .iter()
        .any(|&(a, b, _)| positions[a].is_none() || positions[b].is_none())
    {
        return None;
    }
    positions[cols]?;
    let anchors: Vec<_> = positions
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.map(|v| (i, v)))
        .collect();
    for pair in anchors.windows(2) {
        let ((a, x), (b, y)) = (pair[0], pair[1]);
        if y <= x {
            return None;
        }
        for (i, p) in positions.iter_mut().enumerate().take(b).skip(a + 1) {
            *p = Some(x + (y - x) * (i - a) as f64 / (b - a) as f64);
        }
    }
    let boundaries: Vec<_> = positions.into_iter().collect::<Option<_>>()?;
    let widths: Vec<_> = boundaries.windows(2).map(|p| p[1] - p[0]).collect();
    widths
        .iter()
        .all(|w| w.is_finite() && *w > 0.)
        .then_some(widths)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(conflict: bool) -> Node {
        let rows = if conflict {
            vec![
                vec![(0, 2, 8000), (2, 3, 22000)],
                vec![(0, 2, 9000), (2, 3, 21000)],
            ]
        } else {
            vec![
                vec![(0, 2, 8000), (2, 3, 22000)],
                vec![(0, 3, 14000), (3, 2, 16000)],
                vec![(0, 4, 22000), (4, 1, 8000)],
            ]
        };
        let mut s = String::from("<tbl colCnt='5'>");
        for row in rows {
            s.push_str("<tr>");
            for (col, span, w) in row {
                s.push_str(&format!("<tc><cellAddr colAddr='{col}'/><cellSpan colSpan='{span}'/><cellSz width='{w}'/></tc>"));
            }
            s.push_str("</tr>");
        }
        s.push_str("</tbl>");
        crate::model::xml(s.as_bytes()).unwrap()
    }
    #[test]
    fn crossing_merges_solve_shared_boundaries_instead_of_evenly_guessing() {
        let n = fixture(false);
        let actual = reconcile(&n, &[40., 40., 70., 70., 80.]).unwrap();
        assert_eq!(actual, vec![40., 40., 60., 80., 80.]);
        let mut reversed = n.clone();
        reversed.children.reverse();
        assert_eq!(
            reconcile(&reversed, &[40., 40., 70., 70., 80.]).unwrap(),
            actual
        );
    }
    #[test]
    fn conflicting_widths_and_already_valid_grids_keep_the_existing_path() {
        assert!(reconcile(&fixture(true), &[40., 40., 70., 70., 80.]).is_none());
        assert!(reconcile(&fixture(false), &[40., 40., 60., 80., 80.]).is_none());
    }
    #[test]
    fn small_complete_native_row_resize_is_normalized_before_crossing_merges() {
        let n=crate::model::xml(br#"<tbl binaryHwp="1" colCnt="3"><sz width="10000"/><tr><tc><cellAddr colAddr="0"/><cellSpan colSpan="1"/><cellSz width="1980"/></tc><tc><cellAddr colAddr="1"/><cellSpan colSpan="2"/><cellSz width="7920"/></tc></tr><tr><tc><cellAddr colAddr="0"/><cellSpan colSpan="2"/><cellSz width="6000"/></tc><tc><cellAddr colAddr="2"/><cellSpan colSpan="1"/><cellSz width="4000"/></tc></tr></tbl>"#).unwrap();
        let widths=reconcile(&n, &[19.8, 40.2, 40.]).unwrap();
        assert!(widths.iter().zip([20.,40.,40.]).all(|(a,b)|(a-b).abs()<1e-9));
    }
}
