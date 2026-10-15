//! Layout responsibility: page decorations.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn decorations(
        &mut self,
        section: &Node,
        first: bool,
        cols: usize,
        gap: f64,
    ) -> Result<()> {
        let hiding = if first {
            section.find("pageHiding")
        } else {
            None
        };
        if let Some(number) = section
            .find("pageNumPos")
            .filter(|_| hiding.is_none_or(|h| h.a("hidePageNum") != "1"))
        {
            let pos = number.u("position");
            if pos > 0 {
                let page = (self.pages.len() as i64 + self.page_base).max(1) as usize;
                let font = self.fonts.get("times.ttf")?;
                let label = format!("{}{}{}", number.a("prefix"), page, number.a("suffix"));
                let w = self.fonts.width(font, &label, 10.);
                let right = match pos {
                    3 | 6 => true,
                    7 | 8 => page % 2 == 1,
                    9 | 10 => page.is_multiple_of(2),
                    _ => false,
                };
                let x = if pos == 2 || pos == 5 {
                    (self.pw - w) / 2.
                } else if right {
                    self.pw - self.right - w
                } else {
                    self.left
                };
                let y = if matches!(pos, 1 | 2 | 3 | 7 | 9) {
                    self.top - self.header.max(15.) / 2.
                } else {
                    self.ph - self.bottom + self.footer.max(15.) / 2. + 7.
                };
                self.current.ops.push(Op::Text {
                    x,
                    y,
                    text: label,
                    font,
                    size: 10.,
                    scale: 1.,
                    color: [0.; 3],
                });
            }
        }
        let sec = section.find("secPr").unwrap();
        self.paint_page_border(sec, first, hiding);
        if cols > 1 && sec.a("binaryHwp") != "1" && self.column_widths.is_empty() {
            let x = self.left + (self.pw - self.left - self.right) / 2.;
            let first_body = section
                .children_named("p")
                .skip(1)
                .flat_map(lines)
                .next()
                .map(|l| l.n("vertpos") / 100.)
                .unwrap_or(0.);
            self.current.ops.push(Op::Line {
                x,
                y: if first {
                    self.top + first_body
                } else {
                    self.top
                },
                x2: x,
                y2: self.ph - self.bottom,
                width: 0.34,
                color: [0.; 3],
            });
        }
        for master in &self.doc.masters {
            if hiding.is_some_and(|h| h.a("hideMasterPage") == "1") {
                continue;
            }
            if first && sec.a("hideFirstMasterPage") == "1" {
                continue;
            }
            let odd = (self.pages.len() + 1) % 2 == 1;
            if (master.a("type") == "ODD" && !odd) || (master.a("type") == "EVEN" && odd) {
                continue;
            }
            if let Some(list) = master.find("subList") {
                self.local(
                    list,
                    self.left,
                    if master.a("binaryHwp") == "1" {
                        0.
                    } else {
                        self.top
                    },
                    self.pw - self.left - self.right,
                )?;
            }
        }
        self.paint_furniture(section, first, hiding)?;
        let _ = gap;
        Ok(())
    }
}
