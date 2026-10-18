//! Remaining-cell grids for fragments of a closed vertical-merge band.
use super::*;

#[derive(Clone)]
pub(super) struct Band<'b> {
    pub cells: Vec<BandCell<'b>>,
    heights: Vec<f64>,
}
#[derive(Clone)]
pub(super) struct BandCell<'b> {
    pub cell: Cell<'b>,
    row: usize,
    end: usize,
    minimum: f64,
}
pub(super) struct Fragment {
    pub takes: Vec<usize>,
    pub height: f64,
    row: usize,
    full: bool,
}
impl<'b> Band<'b> {
    pub fn new(
        engine: &mut Engine<'_>,
        nodes: &[&'b Node],
        defaults: [f64; 4],
        columns: &[f64],
        heights: &[f64],
        first: usize,
        last: usize,
    ) -> Result<Self> {
        let mut cells = Vec::new();
        for node in nodes.iter().filter(|c| {
            let row = c.child("cellAddr").unwrap().u("rowAddr");
            row >= first && row < last
        }) {
            let addr = node.child("cellAddr").unwrap();
            let span = node.child("cellSpan").unwrap();
            let col = addr.u("colAddr");
            let row = addr.u("rowAddr") - first;
            let flow = if let Some(list) = node.child("subList") {
                engine.measured_flow(list)?
            } else {
                Vec::new()
            };
            cells.push(BandCell {
                row,
                end: row + span.u("rowSpan"),
                minimum: val(node.child("cellSz"), "height"),
                cell: Cell {
                    node,
                    flow,
                    at: 0,
                    x: columns[..col].iter().sum(),
                    width: columns[col..col + span.u("colSpan")].iter().sum(),
                    margin: if node.a("hasMargin") == "1" {
                        margin(node, "cellMargin")
                    } else {
                        defaults
                    },
                },
            });
        }
        Ok(Self {
            cells,
            heights: heights[first..last].to_vec(),
        })
    }
    pub fn height(&self) -> f64 {
        self.heights.iter().sum()
    }
    pub fn done(&self) -> bool {
        self.heights.is_empty()
    }
    pub fn extent(&self, cell: &BandCell<'_>) -> (f64, f64) {
        (
            self.heights[..cell.row].iter().sum(),
            self.heights[..cell.end].iter().sum(),
        )
    }
    pub fn fit(&self, room: f64) -> Option<Fragment> {
        if room <= 0.1 {
            return None;
        }
        let full = self.height() <= room + 0.01;
        let mut before = 0.;
        let row = if full {
            self.heights.len()
        } else {
            self.heights.iter().position(|h| {
                if before + h > room + 0.01 {
                    true
                } else {
                    before += h;
                    false
                }
            })?
        };
        let mut takes: Vec<_> = self
            .cells
            .iter()
            .map(|c| {
                let (top, bottom) = self.extent(c);
                if full || bottom <= room + 0.01 {
                    c.cell.flow.len() - c.cell.at
                } else {
                    c.cell.fit((room - top).max(0.))
                }
            })
            .collect();
        if !full
            && self
                .cells
                .iter()
                .zip(&takes)
                .all(|(c, t)| c.cell.at + t == c.cell.flow.len())
            && self.cells.iter().any(|c| {
                let (top, bottom) = self.extent(c);
                c.minimum > (bottom.min(room) - top).max(0.) + 0.01
            })
            && let Some(i) = takes.iter().rposition(|n| *n > 0)
        {
            takes[i] -= 1;
        }
        let height = if full {
            self.height()
        } else {
            self.cells
                .iter()
                .zip(&takes)
                .map(|(c, t)| {
                    let (top, bottom) = self.extent(c);
                    if top >= room {
                        return 0.;
                    }
                    if bottom <= before + 0.01 {
                        return bottom;
                    }
                    top + c.cell.required(*t).max(c.minimum.min((room - top).max(0.)))
                })
                .fold(before, f64::max)
                .min(room)
        };
        if height <= 0.1 || (!full && takes.iter().all(|t| *t == 0)) {
            return None;
        }
        Some(Fragment {
            takes,
            height,
            row,
            full,
        })
    }
    pub fn consume(&mut self, part: &Fragment) {
        if part.full {
            self.cells.clear();
            self.heights.clear();
            return;
        }
        for i in 0..self.cells.len() {
            let (top, bottom) = self.extent(&self.cells[i]);
            let c = &mut self.cells[i];
            c.cell.at += part.takes[i];
            c.minimum = (c.minimum - (bottom.min(part.height) - top).max(0.)).max(0.);
        }
        if self
            .cells
            .iter()
            .all(|c| c.cell.at == c.cell.flow.len() && c.minimum <= 0.01)
        {
            self.cells.clear();
            self.heights.clear();
            return;
        }
        let base = part.row
            + usize::from(
                self.cells
                    .iter()
                    .filter(|c| c.end == part.row + 1)
                    .all(|c| c.cell.at == c.cell.flow.len() && c.minimum <= 0.01),
            );
        self.cells.retain(|c| c.end > base);
        for c in &mut self.cells {
            c.row = c.row.saturating_sub(base);
            c.end -= base;
        }
        let count = self.heights.len() - base;
        self.heights = vec![0.; count];
        for c in &self.cells {
            if c.end == c.row + 1 {
                self.heights[c.row] = self.heights[c.row].max(c.minimum);
            }
        }
        let mut order: Vec<_> = (0..self.cells.len()).collect();
        order.sort_by_key(|i| self.cells[*i].end - self.cells[*i].row);
        for i in order {
            let c = &self.cells[i];
            let needed = c
                .minimum
                .max(c.cell.required(c.cell.flow.len() - c.cell.at));
            let current: f64 = self.heights[c.row..c.end].iter().sum();
            self.heights[c.end - 1] += (needed - current).max(0.);
        }
    }
}

impl Engine<'_> {
    pub(super) fn paint_merged_fragment(
        &mut self,
        band: &Band<'_>,
        part: &Fragment,
        x: f64,
        y: f64,
    ) -> Result<()> {
        for (entry, take) in band.cells.iter().zip(&part.takes) {
            let (top, bottom) = band.extent(entry);
            let height = bottom.min(part.height) - top;
            if height <= 0.01 {
                continue;
            }
            self.paint_native_fragment_cell(&entry.cell, *take, x, y + top, height)?;
        }
        Ok(())
    }
}
