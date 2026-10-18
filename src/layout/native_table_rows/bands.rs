//! Closed row bands: no vertically merged cell crosses a band's boundary.
use super::*;
pub(super) fn ranges(cells: &[&Node], first: usize, count: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = first;
    while start < count {
        let mut end = start + 1;
        loop {
            let next = cells
                .iter()
                .filter(|c| {
                    let row = c.child("cellAddr").unwrap().u("rowAddr");
                    row >= start && row < end
                })
                .map(|c| {
                    c.child("cellAddr").unwrap().u("rowAddr")
                        + c.child("cellSpan").unwrap().u("rowSpan")
                })
                .fold(end, usize::max);
            if next == end {
                break;
            }
            end = next;
        }
        out.push((start, end));
        start = end;
    }
    out
}
impl<'a> Engine<'a> {
    pub(super) fn paint_native_row_band(
        &mut self,
        n: &Node,
        band: (usize, usize),
        x: f64,
        y: f64,
        cw: &[f64],
        rh: &[f64],
    ) -> Result<()> {
        for c in n.children_named("tr").flat_map(|r| r.children_named("tc")) {
            let a = c.child("cellAddr").unwrap();
            let row = a.u("rowAddr");
            if row < band.0 || row >= band.1 {
                continue;
            }
            let col = a.u("colAddr");
            let span = c.child("cellSpan").unwrap();
            let xx = x + cw[..col].iter().sum::<f64>();
            let yy = y + rh[band.0..row].iter().sum::<f64>();
            let width = cw[col..col + span.u("colSpan")].iter().sum();
            let height = rh[row..row + span.u("rowSpan")].iter().sum();
            self.border(c.a("borderFillIDRef"), xx, yy, width, height);
            self.paint_whole_cell_content(n, c, xx, yy, width, height)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_row_spans_form_one_closed_band() {
        let n=crate::model::xml(br#"<tbl><tr><tc><cellAddr rowAddr="0"/><cellSpan rowSpan="2"/></tc></tr><tr><tc><cellAddr rowAddr="1"/><cellSpan rowSpan="3"/></tc></tr><tr><tc><cellAddr rowAddr="4"/><cellSpan rowSpan="1"/></tc></tr></tbl>"#).unwrap();
        let cells: Vec<_> = n
            .children_named("tr")
            .flat_map(|r| r.children_named("tc"))
            .collect();
        assert_eq!(ranges(&cells, 0, 5), vec![(0, 4), (4, 5)]);
    }
}
