//! Vertical plain-text flow shares the horizontal line breaker in rotated axes.
use super::*;
pub(super) fn upright(mode: u8, c: char) -> bool {
    (mode == 2 && !c.is_ascii_punctuation())
        || (mode == 1 && matches!(c as u32,0x1100..=0x11ff|0x3000..=0x9fff|0xac00..=0xd7ff))
}
pub(super) fn begin(ops: &mut Vec<Op>, g: &Glyph, x: f64, y: f64, line_top: f64, line_height: f64) {
    // OpenType vertical origins put glyphs on a common top edge; horizontal
    // advances center narrow Latin glyphs within the vertical column.
    let e = x + g.vertical_origin - y;
    let f = x + line_top + line_height / 2. + g.native_advance / 2. + g.shift;
    ops.push(Op::TransformBegin {
        matrix: [0., -1., 1., 0., e, f],
    });
}
impl<'a> Engine<'a> {
    pub(super) fn render_vertical(&mut self, force_reflow: bool) -> Result<bool> {
        let modes: Vec<_> = self
            .doc
            .sections
            .iter()
            .map(
                |s| match s.find("secPr").map(|p| p.a("textDirection")).unwrap_or("") {
                    "VERTICAL" => 1,
                    "VERTICALALL" => 2,
                    _ => 0,
                },
            )
            .collect();
        if modes.iter().all(|m| *m == 0) {
            return Ok(false);
        }
        for (s, mode) in self
            .doc
            .sections
            .iter()
            .zip(&modes)
            .filter(|(_, mode)| **mode != 0)
        {
            let _ = mode;
            anyhow::ensure!(
                !s.children_named("p")
                    .flat_map(Self::items)
                    .any(|i| matches!(i.kind, ItemKind::Object(_))),
                "Vertical inline objects require additional object orientation layout"
            );
        }
        let mut document = self.doc.clone();
        for (section, mode) in document.sections.iter_mut().zip(&modes) {
            if *mode == 0 {
                continue;
            }
            fn prepare(n: &mut Node, mode: u8) {
                if n.name == "secPr" {
                    n.attrs.insert("textDirection".into(), "HORIZONTAL".into());
                    n.attrs
                        .insert("engineVerticalMode".into(), mode.to_string());
                    if let Some(page) = n.children.iter_mut().find(|c| c.name == "pagePr") {
                        let width = page.a("width").to_string();
                        let height = page.a("height").to_string();
                        page.attrs.insert("width".into(), height);
                        page.attrs.insert("height".into(), width);
                        if let Some(m) = page.children.iter_mut().find(|c| c.name == "margin") {
                            let values = [
                                m.n("left") + m.n("header"),
                                m.n("right") + m.n("footer"),
                                m.n("top"),
                                m.n("bottom"),
                            ];
                            for (key, value) in
                                ["left", "right", "top", "bottom"].into_iter().zip(values)
                            {
                                m.attrs.insert(key.into(), value.to_string());
                            }
                            m.attrs.insert("header".into(), "0".into());
                            m.attrs.insert("footer".into(), "0".into());
                        }
                    }
                }
                n.children.retain(|c| c.name != "linesegarray");
                for c in &mut n.children {
                    prepare(c, mode);
                }
            }
            prepare(section, *mode);
        }
        let mut child = Engine::new(&document, &self.fonts.root);
        child.hancom_hide_pcx = self.hancom_hide_pcx;
        child.general_wrap = self.general_wrap;
        child.contour_wrap = self.contour_wrap;
        child
            .report
            .warnings
            .extend(self.report.warnings.iter().cloned());
        child.render_mode(force_reflow)?;
        anyhow::ensure!(
            child.page_sections.len() == child.pages.len(),
            "Missing page section ownership"
        );
        for (i, page) in child.pages.iter_mut().enumerate() {
            if modes[child.page_sections[i]] == 0 {
                continue;
            }
            let original_width = page.height;
            let matrix = [0., 1., -1., 0., original_width, 0.];
            page.ops.insert(0, Op::TransformBegin { matrix });
            page.ops.push(Op::TransformEnd);
            std::mem::swap(&mut page.width, &mut page.height);
            for b in child.report.bounds.iter_mut().filter(|b| b.page == i + 1) {
                let r = crate::graphics::rect(matrix, [b.x, b.y, b.x + b.w, b.y + b.h]);
                b.x = r[0];
                b.y = r[1];
                b.w = r[2] - r[0];
                b.h = r[3] - r[1];
            }
        }
        child.report.warnings.insert("Section-specific vertical text flow; inline vertical objects require separate orientation layout".into());
        self.pages = child.pages;
        self.page_sections = child.page_sections;
        self.fonts = child.fonts;
        self.report = child.report;
        Ok(true)
    }
}
