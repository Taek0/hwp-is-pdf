//! Shared table grid constraints for painting and saved-cell cache repair.
use super::*;
pub(super) fn native_width(n: &Node) -> Option<f64> {
    let count = n.u("colCnt");
    if count == 0 || count > 1000 {
        return None;
    }
    let cells: Vec<_> = n
        .children_named("tr")
        .flat_map(|r| r.children_named("tc"))
        .collect();
    let mut widths = vec![0f64; count];
    for c in &cells {
        let start = c.child("cellAddr")?.u("colAddr");
        let span = c.child("cellSpan")?.u("colSpan");
        if span == 0 || start.checked_add(span)? > count {
            return None;
        }
        if span == 1 {
            widths[start] = widths[start].max(val(c.child("cellSz"), "width"));
        }
    }
    for _ in 0..2 {
        for c in &cells {
            let start = c.child("cellAddr")?.u("colAddr");
            let span = c.child("cellSpan")?.u("colSpan");
            let missing: Vec<_> = (start..start + span).filter(|i| widths[*i] == 0.).collect();
            if !missing.is_empty() {
                let value = (val(c.child("cellSz"), "width")
                    - widths[start..start + span].iter().sum::<f64>())
                .max(0.)
                    / missing.len() as f64;
                for i in missing {
                    widths[i] = value;
                }
            }
        }
    }
    if let Some(solved) = table_columns::reconcile(n, &widths) {
        widths = solved;
    }
    widths
        .iter()
        .all(|w| *w > 0. && w.is_finite())
        .then(|| widths.iter().sum())
}
impl<'a> Engine<'a> {
    pub(super) fn measure_table_grid(
        &mut self,
        n: &Node,
        h: f64,
    ) -> Result<(Vec<f64>, Vec<f64>, f64)> {
        let rows = n.u("rowCnt");
        let cols = n.u("colCnt");
        if rows == 0 || cols == 0 || rows > 1000 || cols > 1000 {
            bail!("Invalid table dimensions")
        }
        let cells: Vec<_> = n
            .children_named("tr")
            .flat_map(|r| r.children_named("tc"))
            .collect();
        let mut cw = vec![0f64; cols];
        let mut rh = vec![0f64; rows];
        let mut minimum = vec![0f64; rows];
        for c in &cells {
            let a = c.child("cellAddr").context("cellAddr")?;
            let span = c.child("cellSpan").context("cellSpan")?;
            let sz = c.child("cellSz").context("cellSz")?;
            let col = a.u("colAddr");
            let row = a.u("rowAddr");
            let cs = span.u("colSpan");
            let rs = span.u("rowSpan");
            if cs == 0 || rs == 0 || col + cs > cols || row + rs > rows {
                bail!("Cell outside table")
            }
            if cs == 1 {
                cw[col] = cw[col].max(sz.n("width") / 100.)
            }
            if rs == 1 {
                rh[row] = rh[row].max(sz.n("height") / 100.)
            }
        }
        for _ in 0..2 {
            for c in &cells {
                let a = c.child("cellAddr").unwrap();
                let span = c.child("cellSpan").unwrap();
                let sz = c.child("cellSz").unwrap();
                for (v, start, count, size) in [
                    (
                        &mut cw,
                        a.u("colAddr"),
                        span.u("colSpan"),
                        sz.n("width") / 100.,
                    ),
                    (
                        &mut rh,
                        a.u("rowAddr"),
                        span.u("rowSpan"),
                        sz.n("height") / 100.,
                    ),
                ] {
                    let missing: Vec<_> = (start..start + count).filter(|&i| v[i] == 0.).collect();
                    if !missing.is_empty() {
                        let value = (size - v[start..start + count].iter().sum::<f64>()).max(0.)
                            / missing.len() as f64;
                        for i in missing {
                            v[i] = value;
                        }
                    }
                }
            }
        }
        if let Some(solved) = table_columns::reconcile(n, &cw) {
            cw = solved;
        }
        let automatic: Vec<_> = rh
            .iter()
            .enumerate()
            .filter_map(|(i, h)| (*h == 0.).then_some(i))
            .collect();
        let mut measured_cells = cells.clone();
        measured_cells.sort_by_key(|c| c.child("cellSpan").unwrap().u("rowSpan"));
        for c in &measured_cells {
            let a = c.child("cellAddr").unwrap();
            let span = c.child("cellSpan").unwrap();
            let row = a.u("rowAddr");
            let rs = span.u("rowSpan");
            let m = if c.a("hasMargin") == "1" {
                margin(c, "cellMargin")
            } else {
                margin(n, "inMargin")
            };
            let content = c
                .child("subList")
                .map(|l| {
                    let (a, b) = floating_extent(l);
                    b - a
                })
                .unwrap_or(0.);
            let typed = if let Some(list) = c.child("subList") {
                self.measured_height(list)?
            } else {
                0.
            };
            let needed = content.max(typed) + m[2] + m[3];
            if rs == 1 {
                minimum[row] = minimum[row].max(needed);
            } else {
                let missing = (needed - minimum[row..row + rs].iter().sum::<f64>()).max(0.);
                minimum[row + rs - 1] += missing;
            }
            let current: f64 = rh[row..row + rs].iter().sum();
            if needed > current {
                rh[row + rs - 1] += needed - current;
            }
        }
        let mut total: f64 = rh.iter().sum();
        let min_total: f64 = minimum.iter().sum();
        if n.a("binaryHwp") == "1" && total > h && h >= min_total && total > min_total {
            let ratio = (h - min_total) / (total - min_total);
            for (i, height) in rh.iter_mut().enumerate() {
                *height = minimum[i] + (*height - minimum[i]) * ratio;
            }
            total = h;
        }
        if total < h && !rh.is_empty() {
            // A single zero-height native row is auto-sized (commonly a merged
            // heading). It owns the remaining declared height, not the last
            // fixed-height row. Multiple ambiguous auto rows retain the fallback.
            let row = if n.a("binaryHwp") == "1" && automatic.len() == 1 {
                automatic[0]
            } else {
                rh.len() - 1
            };
            rh[row] += h - total;
            total = h;
        }
        Ok((cw, rh, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allocated_table_height_matches_returned_pagination_extent() {
        let d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let n=crate::model::xml(br#"<tbl rowCnt="1" colCnt="1"><tr><tc><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="9000" height="1000"/></tc></tr></tbl>"#).unwrap();
        let mut e = Engine::new(&d, Path::new("."));
        let (_, rows, total) = e.measure_table_grid(&n, 300.).unwrap();
        assert_eq!(rows, vec![300.]);
        assert_eq!(total, rows.iter().sum::<f64>());
    }

    #[test]
    fn native_auto_header_receives_spare_height_without_stretching_fixed_body_row() {
        let d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let n=crate::model::xml(br#"<tbl binaryHwp="1" rowCnt="2" colCnt="1"><tr><tc><cellAddr rowAddr="0" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="9000" height="0"/></tc></tr><tr><tc><cellAddr rowAddr="1" colAddr="0"/><cellSpan rowSpan="1" colSpan="1"/><cellSz width="9000" height="10000"/></tc></tr></tbl>"#).unwrap();
        let mut e = Engine::new(&d, Path::new("."));
        let (_, rows, total) = e.measure_table_grid(&n, 200.).unwrap();
        assert_eq!(rows, vec![100., 100.]);
        assert_eq!(total, 200.);
    }
}
