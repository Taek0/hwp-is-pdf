//! Balance a plain-text column set by simulating source continuations.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn balanced_column_bottom(&mut self, paragraphs: &[&Node]) -> Result<f64> {
        let full = self.ph - self.bottom;
        if self
            .column_definition
            .as_ref()
            .is_none_or(|d| d.a("type") != "BALANCED_NEWSPAPER")
        {
            return Ok(full);
        }
        if paragraphs.iter().any(|p| {
            p.a("pageBreak") == "1"
                || Self::items(p).iter().any(|i| {
                    matches!(&i.kind, ItemKind::Object(_))
                        || matches!(&i.kind, ItemKind::Control(n) if n.name!="colPr")
                })
        }) {
            self.report.warnings.insert(
                "Balanced columns with objects, notes or explicit page breaks use sequential flow"
                    .into(),
            );
            return Ok(full);
        }
        let numbering = self.numbering.clone();
        let labels = self.labels.clone();
        let result = (|| -> Result<f64> {
            let available = full - self.column_band_top;
            if !self.balance_fits(paragraphs, available)? {
                return Ok(full);
            }
            let mut lo = 0.;
            let mut hi = available;
            for _ in 0..20 {
                let mid = (lo + hi) / 2.;
                if self.balance_fits(paragraphs, mid)? {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            Ok(self.column_band_top + hi + 0.01)
        })();
        self.numbering = numbering;
        self.labels = labels;
        result
    }
    fn balance_fits(&mut self, paragraphs: &[&Node], height: f64) -> Result<bool> {
        let mut col = 0;
        let mut used = 0.;
        for original in paragraphs {
            let mut source = (*original).clone();
            let mut start = 0;
            loop {
                if col >= self.active_columns {
                    return Ok(false);
                }
                source.attrs.insert("reflowStart".into(), start.to_string());
                let (_, ls, s) = self.reflow_paragraph(&source, self.column_width(col), 0)?;
                used += s.before;
                let mut next = None;
                for row in &ls {
                    if used + row.height > height + 0.001 {
                        next = Some(row.pos);
                        break;
                    }
                    used += row.height;
                }
                if let Some(pos) = next {
                    col += 1;
                    used = 0.;
                    start = pos;
                } else {
                    used += s.after;
                    break;
                }
            }
        }
        Ok(true)
    }
}
