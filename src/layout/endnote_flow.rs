//! Rebuild endnote paragraphs and continue them across pages and variable columns.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn render_reflow_endnotes(
        &mut self,
        notes: Vec<Node>,
        start: (f64, usize, f64, f64),
    ) -> Result<()> {
        let (mut y, mut col, _, gap) = start;
        let above = self.note_gap(true, "aboveLine", 0.);
        let below = self.note_gap(true, "belowLine", 0.);
        let between = self.note_gap(true, "betweenNotes", 8.);
        let mut separator = true;
        self.note_rendering = true;
        for note in notes {
            if let Some(list) = note.child("subList") {
                for original in list.children_named("p") {
                    let mut source = original.clone();
                    let mut start = 0;
                    loop {
                        self.reflow_column = col;
                        source.attrs.insert("reflowStart".into(), start.to_string());
                        let width = self.column_width(col);
                        let x = self.column_x(col);
                        let (p, ls, s) = self.reflow_paragraph(&source, width, 1)?;
                        y += s.before;
                        let mut next = None;
                        for (i, row) in ls.iter().enumerate() {
                            anyhow::ensure!(
                                !row.block,
                                "Block objects inside reflow endnotes require mixed-note pagination"
                            );
                            let overhead = if separator { above + below + 1. } else { 0. };
                            if y + overhead + row.height
                                > self.ph - self.bottom - self.footnote_reservation(&[], x) + 0.001
                            {
                                anyhow::ensure!(
                                    y > self.top + 0.01,
                                    "An endnote row exceeds the printable column"
                                );
                                next = Some(row.pos);
                                break;
                            }
                            if separator {
                                self.note_separator(true, x, y + above, width);
                                y += overhead;
                                separator = false;
                            }
                            self.line(&p, i, x, y - lines(&p)[i].n("vertpos") / 100., width)?;
                            self.report.bounds.push(Bounds {
                                page: self.pages.len() + 1,
                                kind: "endnote-fragment".into(),
                                id: note.a("number").into(),
                                x,
                                y,
                                w: width,
                                h: row.height,
                            });
                            y += row.height;
                        }
                        if let Some(pos) = next {
                            if self.column_widths.is_empty() {
                                self.next_note_column(&mut col, &mut y, gap)?;
                            } else {
                                let section = &self.doc.sections[self.active_section];
                                self.advance_complex_column(section, &mut col, &mut y)?;
                            }
                            self.note_rendering = true;
                            separator = true;
                            start = pos;
                        } else {
                            y += s.after;
                            break;
                        }
                    }
                }
                y += between;
            }
        }
        self.note_rendering = false;
        self.report.warnings.insert("Endnote text and inline equations reflow through destination columns; mixed block-note pagination remains limited".into());
        Ok(())
    }
}
