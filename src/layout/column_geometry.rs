//! Column geometry, definition changes and rule painting.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn complex_columns(section: &Node) -> bool {
        let defs: Vec<_> = section
            .children_named("p")
            .filter_map(paragraph_columns)
            .collect();
        defs.len() > 1
            || defs.iter().any(|d| {
                d.children_named("colSz").next().is_some()
                    || (d.u("colCount") > 1
                        && (d.a("type") == "BALANCED_NEWSPAPER"
                            || matches!(d.a("layout"), "RIGHT" | "MIRROR")))
            })
            || section
                .find("footNotePr")
                .and_then(|n| n.child("placement"))
                .is_some_and(|p| matches!(p.a("place"), "MERGED_COLUMN" | "RIGHT_MOST_COLUMN"))
    }
    pub(super) fn configure_columns(&mut self, definition: Option<&Node>) -> Result<()> {
        let count = definition.map(|d| d.u("colCount")).unwrap_or(1).max(1);
        anyhow::ensure!(count <= 64, "Too many reflow columns");
        let body = self.pw - self.left - self.right;
        let gap = val(definition, "sameGap");
        let sizes: Vec<_> = definition
            .map(|d| d.children_named("colSz").collect())
            .unwrap_or_default();
        let mut widths = vec![(body - gap * (count - 1) as f64) / count as f64; count];
        let mut gaps = vec![gap; count];
        if !sizes.is_empty() {
            anyhow::ensure!(
                sizes.len() == count,
                "Column width count does not match colCount"
            );
            for (i, s) in sizes.iter().enumerate() {
                widths[i] = s.n("width") / 100.;
                gaps[i] = if s.a("gap").is_empty() {
                    gap
                } else {
                    s.n("gap") / 100.
                };
            }
        }
        let available = body - gaps[..count - 1].iter().sum::<f64>();
        let sum = widths.iter().sum::<f64>();
        anyhow::ensure!(
            available > 0.
                && widths.iter().all(|w| w.is_finite() && *w > 1.)
                && gaps.iter().all(|g| g.is_finite() && *g >= 0.),
            "Invalid column geometry"
        );
        if sum > available + 0.01 {
            let scale = available / sum;
            for w in &mut widths {
                *w *= scale;
            }
            self.report
                .warnings
                .insert("Column widths normalized to printable frame".into());
        }
        let mut x = self.left;
        let mut origins = Vec::new();
        for i in 0..count {
            origins.push(x);
            x += widths[i] + gaps[i];
        }
        let reverse = definition.is_some_and(|d| {
            d.a("layout") == "RIGHT"
                || (d.a("layout") == "MIRROR" && (self.pages.len() + 1).is_multiple_of(2))
        });
        if reverse {
            widths.reverse();
            origins.reverse();
        }
        self.column_widths = widths;
        self.column_origins = origins;
        self.column_definition = definition.cloned();
        self.active_columns = count;
        self.active_gap = gap;
        Ok(())
    }
    pub(super) fn column_x(&self, col: usize) -> f64 {
        self.column_origins
            .get(col)
            .copied()
            .unwrap_or_else(|| self.left + col as f64 * (self.column_width(col) + self.active_gap))
    }
    pub(super) fn column_width(&self, col: usize) -> f64 {
        self.column_widths.get(col).copied().unwrap_or_else(|| {
            (self.pw
                - self.left
                - self.right
                - self.active_gap * (self.active_columns.max(1) - 1) as f64)
                / self.active_columns.max(1) as f64
        })
    }
    pub(super) fn column_top(&self) -> f64 {
        self.top
            .max(self.page_float_floor)
            .max(if self.column_widths.is_empty() {
                self.top
            } else {
                self.column_band_top
            })
    }
    pub(super) fn column_rules(&mut self, bottom: f64) {
        let bottom = if self
            .column_definition
            .as_ref()
            .is_some_and(|d| d.a("type") == "BALANCED_NEWSPAPER")
        {
            self.report
                .bounds
                .iter()
                .filter(|b| {
                    b.page == self.pages.len() + 1
                        && b.kind == "reflow-line"
                        && b.y >= self.column_band_top
                })
                .map(|b| b.y + b.h)
                .fold(self.column_band_top, f64::max)
                .min(bottom)
        } else {
            bottom
        };
        let Some(line) = self
            .column_definition
            .as_ref()
            .and_then(|d| d.child("colLine"))
        else {
            return;
        };
        if matches!(line.a("type"), "NONE" | "0" | "") {
            return;
        }
        let c = color(line.a("color"));
        let mut order: Vec<usize> = (0..self.column_widths.len()).collect();
        order.sort_by(|a, b| self.column_origins[*a].total_cmp(&self.column_origins[*b]));
        for pair in order.windows(2) {
            let x = (self.column_origins[pair[0]]
                + self.column_widths[pair[0]]
                + self.column_origins[pair[1]])
                / 2.;
            if bottom > self.column_band_top {
                self.current.ops.push(Op::Line {
                    x,
                    y: self.column_band_top,
                    x2: x,
                    y2: bottom,
                    width: 0.34,
                    color: c,
                });
            }
        }
    }
    pub(super) fn advance_complex_column(
        &mut self,
        section: &Node,
        col: &mut usize,
        y: &mut f64,
    ) -> Result<()> {
        *col += 1;
        if *col >= self.active_columns {
            self.finish()?;
            *col = 0;
            self.page_float_floor = self.top;
            self.column_band_top = self.top;
            let definition = self.column_definition.clone();
            self.configure_columns(definition.as_ref())?;
            self.decorations(section, false, self.active_columns, self.active_gap)?;
        }
        self.reflow_column = *col;
        *y = self.column_top();
        Ok(())
    }
}
