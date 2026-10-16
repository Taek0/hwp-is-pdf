//! Space for a note's first line and safe continuation below actual page content.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn saved_line_note_reservation(
        &self,
        extra: &[&Node],
        x: f64,
        line_bottom: f64,
    ) -> f64 {
        let full = self.footnote_reservation(extra, x);
        let available = self.ph - self.bottom - line_bottom;
        if self.active_columns != 1
            || extra.is_empty()
            || full <= available
            || !self.continued_notes.is_empty()
        {
            return full;
        }
        let (nx, _) = self.footnote_area(x);
        let mut count = 0usize;
        let mut minimum = 0.;
        let mut complete = 0.;
        for (n, xx, _) in &self.footnotes {
            if (xx - nx).abs() < 0.1
                && let Some(l) = n.child("subList")
            {
                minimum += content_height(l);
                complete += content_height(l);
                count += 1;
            }
        }
        for n in extra {
            if let Some(l) = n.child("subList") {
                complete += content_height(l);
                let flow = flowing_lines(l);
                if !flow.is_empty() {
                    minimum += notes::note_row_advance(&flow, 0);
                    count += 1;
                }
            }
        }
        let overhead = self.note_gap(false, "betweenNotes", 4.) * count.saturating_sub(1) as f64
            + self.note_gap(false, "aboveLine", 6.)
            + self.note_gap(false, "belowLine", 4.)
            + 1.;
        // Oversized groups retain their established bounded-area reservation.
        if complete + overhead > full + 0.01 {
            return full;
        }
        minimum += overhead;
        if available >= minimum {
            available
        } else {
            full
        }
    }

    pub(super) fn body_content_bottom(&self) -> f64 {
        self.body_content_bottom_in_area(self.left, self.pw - self.left - self.right)
    }

    pub(super) fn body_content_bottom_in_area(&self, x: f64, width: f64) -> f64 {
        let limit = self.ph - self.bottom;
        let mut bottom = self.top;
        let mut matrix = crate::graphics::IDENTITY;
        let mut stack = Vec::new();
        for op in &self.current.ops {
            match op {
                Op::TransformBegin { matrix: m } => {
                    stack.push(matrix);
                    matrix = crate::graphics::compose(matrix, *m);
                }
                Op::TransformEnd => matrix = stack.pop().unwrap_or(crate::graphics::IDENTITY),
                Op::Text { .. } | Op::Shaped { .. } => {
                    if let Some(b) = crate::pagination::bounds(op, &self.fonts) {
                        let b = crate::graphics::rect(matrix, b);
                        if b[1] >= self.top
                            && b[3] <= limit + 0.01
                            && b[2] > x + 0.01
                            && b[0] < x + width - 0.01
                        {
                            bottom = bottom.max(b[3]);
                        }
                    }
                }
                _ => {}
            }
        }
        for b in self
            .report
            .bounds
            .iter()
            .filter(|b| b.page == self.pages.len() + 1 && b.kind != "watermark")
        {
            if b.y >= self.top
                && b.y + b.h <= limit + 0.01
                && b.x + b.w > x + 0.01
                && b.x < x + width - 0.01
            {
                bottom = bottom.max(b.y + b.h);
            }
        }
        bottom
    }

    pub(super) fn notes_exceed_free_area(&self) -> bool {
        if self.active_columns != 1 || self.footnotes.is_empty() {
            return false;
        }
        let free = self.ph - self.bottom - self.body_content_bottom();
        self.footnotes
            .iter()
            .any(|(_, x, _)| self.footnote_reservation(&[], *x) > free + 0.01)
    }
}
