//! Layout responsibility: reflow table.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn reflow_table_contents(
        &mut self,
        n: &mut Node,
        width: f64,
        nesting: usize,
    ) -> Result<()> {
        n.attrs.insert("reflowTable".into(), "1".into());
        let width = self.column_widths.iter().copied().fold(width, f64::min);
        let rows = n.u("rowCnt");
        let cols = n.u("colCnt");
        anyhow::ensure!(
            rows > 0 && cols > 0 && rows <= 1000 && cols <= 1000,
            "Invalid reflow table dimensions"
        );
        let mut cw = vec![0f64; cols];
        for cell in n.children_named("tr").flat_map(|r| r.children_named("tc")) {
            let a = cell.child("cellAddr").context("cellAddr")?;
            let span = cell.child("cellSpan").context("cellSpan")?;
            anyhow::ensure!(
                a.u("colAddr") + span.u("colSpan") <= cols
                    && a.u("rowAddr") + span.u("rowSpan") <= rows
                    && span.u("colSpan") > 0
                    && span.u("rowSpan") > 0,
                "Cell outside table"
            );
            if span.u("colSpan") == 1 {
                cw[a.u("colAddr")] = cw[a.u("colAddr")].max(val(cell.child("cellSz"), "width"));
            }
        }
        for cell in n.children_named("tr").flat_map(|r| r.children_named("tc")) {
            let col = cell.child("cellAddr").unwrap().u("colAddr");
            let count = cell.child("cellSpan").unwrap().u("colSpan");
            let missing = (col..col + count)
                .filter(|i| cw[*i] == 0.)
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                let each = (val(cell.child("cellSz"), "width")
                    - cw[col..col + count].iter().sum::<f64>())
                .max(0.)
                    / missing.len() as f64;
                for i in missing {
                    cw[i] = each;
                }
            }
        }
        for c in &mut cw {
            if *c <= 0. {
                *c = width / cols as f64;
            }
        }
        let ratio = (width / cw.iter().sum::<f64>()).min(1.);
        for c in &mut cw {
            *c *= ratio;
        }
        let m_default = margin(n, "inMargin");
        let mut rh = vec![0f64; rows];
        let mut needs = Vec::new();
        for row in n.children.iter_mut().filter(|r| r.name == "tr") {
            for cell in row.children.iter_mut().filter(|c| c.name == "tc") {
                let a = cell.child("cellAddr").unwrap();
                let col = a.u("colAddr");
                let row = a.u("rowAddr");
                let span = cell.child("cellSpan").unwrap();
                let cs = span.u("colSpan");
                let rs = span.u("rowSpan");
                let w = cw[col..col + cs].iter().sum::<f64>();
                let m = if cell.a("hasMargin") == "1" {
                    margin(cell, "cellMargin")
                } else {
                    m_default
                };
                let minimum = val(cell.child("cellSz"), "height");
                let height =
                    if let Some(list) = cell.children.iter_mut().find(|n| n.name == "subList") {
                        self.reflow_list(list, w - m[0] - m[1], nesting + 1)? + m[2] + m[3]
                    } else {
                        0.
                    };
                let h = minimum.max(height);
                if rs == 1 {
                    rh[row] = rh[row].max(h);
                }
                needs.push((row, rs, h));
                node_mut(cell, "cellSz")
                    .attrs
                    .insert("width".into(), (w * 100.).to_string());
            }
        }
        for (row, span, h) in needs {
            let extra = (h - rh[row..row + span].iter().sum::<f64>()).max(0.);
            rh[row + span - 1] += extra;
        }
        for row in n.children.iter_mut().filter(|r| r.name == "tr") {
            for cell in row.children.iter_mut().filter(|c| c.name == "tc") {
                let r = cell.child("cellAddr").unwrap().u("rowAddr");
                let span = cell.child("cellSpan").unwrap().u("rowSpan");
                node_mut(cell, "cellSz").attrs.insert(
                    "height".into(),
                    (rh[r..r + span].iter().sum::<f64>() * 100.).to_string(),
                );
            }
        }
        let sz = node_mut(n, "sz");
        sz.attrs
            .insert("width".into(), (cw.iter().sum::<f64>() * 100.).to_string());
        sz.attrs
            .insert("height".into(), (rh.iter().sum::<f64>() * 100.).to_string());
        Ok(())
    }
}
