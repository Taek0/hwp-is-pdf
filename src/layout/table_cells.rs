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
            let spare = (h - m[2] - m[3] - (max - min)).max(0.);
            let shift = match list.a("vertAlign") {
                "CENTER" => spare / 2.,
                "BOTTOM" => spare,
                _ => 0.,
            };
            self.local(list, x + m[0], y + m[2] + shift - min, w - m[0] - m[1])?;
        }
        Ok(())
    }
}
