//! Conservative look-ahead reservation for merged/rightmost footnote areas.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn refresh_shared_notes(
        &mut self,
        paragraphs: &[&Node],
        start: usize,
    ) -> Result<()> {
        let placement = self
            .note_spec(false)
            .and_then(|n| n.child("placement"))
            .map(|n| n.a("place"))
            .unwrap_or("EACH_COLUMN")
            .to_string();
        self.shared_note_reserve = 0.;
        if placement == "EACH_COLUMN" {
            return Ok(());
        }
        let max_width = self.column_widths.iter().copied().fold(0f64, f64::max);
        let capacity = (self.ph - self.top - self.bottom) * self.active_columns as f64;
        let numbering = self.numbering.clone();
        let labels = self.labels.clone();
        let mut estimate = 0.;
        let mut note_height = 0.;
        let mut count = 0usize;
        let (_, note_width) = self.footnote_area(self.column_x(self.active_columns - 1));
        for (offset, p) in paragraphs.iter().enumerate().skip(start) {
            let mut notes = Vec::new();
            p.all("footNote", &mut notes);
            for note in notes {
                if let Some(list) = note.child("subList") {
                    let mut list = list.clone();
                    note_height += self.reflow_list(&mut list, note_width, 1)?;
                    count += 1;
                }
            }
            if offset > start {
                let settings = crate::reflow::settings(self.doc.paras.get(p.a("paraPrIDRef")));
                let available =
                    max_width + settings.left.abs() + settings.right.abs() + settings.indent.abs();
                let mut advance = 0.;
                let mut min_size = f64::INFINITY;
                for item in Self::items(p) {
                    if let ItemKind::Char(c) = item.kind
                        && !matches!(c, '\n' | '\r')
                    {
                        let g = self.glyph(c, &item.style)?;
                        advance += g.width.max(0.);
                        min_size = min_size.min(g.size);
                    }
                }
                if min_size.is_finite() {
                    estimate += (advance / available.max(1.)).floor() * min_size;
                }
            }
            if estimate > capacity {
                break;
            }
        }
        self.numbering = numbering;
        self.labels = labels;
        if count > 0 {
            let full = note_height
                + self.note_gap(false, "betweenNotes", 4.) * (count - 1) as f64
                + self.note_gap(false, "aboveLine", 6.)
                + self.note_gap(false, "belowLine", 4.)
                + 1.;
            let body = self.ph - self.top - self.bottom;
            self.shared_note_reserve = if full > body - 48. { body * 0.5 } else { full };
        }
        Ok(())
    }
}
