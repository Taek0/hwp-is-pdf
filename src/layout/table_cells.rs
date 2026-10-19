//! Shared complete-cell contents for ordinary grids and unbroken merged bands.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn paint_whole_cell_content(
        &mut self,
        table: &Node,
        cell: &Node,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Result<()> {
        if let Some(list) = cell.child("subList") {
            let m = if cell.a("hasMargin") == "1" {
                margin(cell, "cellMargin")
            } else {
                margin(table, "inMargin")
            };
            let (min, mut max) = floating_extent(list);
            max = max.max(self.measured_height(list)?);
            let first = table.n("firstFragmentCapacity");
            let alignment_height = if first > y && y + h > first {
                first - y
            } else {
                h
            };
            let first_content = list.n("savedFirstFragmentHeight");
            let content = if alignment_height < h && first_content > 0. {
                first_content.min(max - min)
            } else {
                max - min
            };
            let spare = (alignment_height - m[2] - m[3] - content).max(0.);
            let shift = match list.a("vertAlign") {
                "CENTER" => spare / 2.,
                "BOTTOM" => spare,
                _ => 0.,
            };
            let continuation = if alignment_height < h && first_content > 0. {
                let last_fragment = list.children_named("p").all(|p| p.u("savedCellPage") <= 1);
                let next_spare = if last_fragment {
                    (h - alignment_height - m[2] - m[3] - (max - min - first_content)).max(0.)
                } else {
                    0.
                };
                let next_shift = match list.a("vertAlign") {
                    "CENTER" => next_spare / 2.,
                    "BOTTOM" => next_spare,
                    _ => 0.,
                };
                let delta = (first - y - shift - first_content).max(0.) + next_shift;
                (max - min + delta + m[2] + m[3] <= h + 0.01)
                    .then(|| saved_block_tables::reserve_continuation(list, delta))
            } else {
                None
            };
            self.local(
                continuation.as_ref().unwrap_or(list),
                x + m[0],
                y + m[2] + shift - min,
                w - m[0] - m[1],
            )?;
        }
        Ok(())
    }
}
