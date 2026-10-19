//! Static printable form appearances; never executes control commands.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn checkbox(&mut self, n: &Node, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        self.current.ops.push(Op::TransformBegin {
            matrix: crate::graphics::IDENTITY,
        });
        self.checkbox_contents(n, x, y, w, h)?;
        self.current.ops.push(Op::TransformEnd);
        Ok(())
    }
    fn checkbox_contents(&mut self, n: &Node, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        if n.a("printable") == "0" {
            return Ok(());
        }
        if n.a("backStyle") == "OPAQUE" {
            self.current.ops.push(Op::Rect {
                x,
                y,
                w,
                h,
                fill: color(n.a("backColor")),
            });
        }
        let style = n
            .child("formCharPr")
            .map(|c| c.a("charPrIDRef"))
            .unwrap_or("0");
        let label = n.a("caption").to_string();
        let mut g = self.glyph(label.chars().next().unwrap_or(' '), style)?;
        g.color = color(n.a("foreColor"));
        let side = g.size.min(h).min(w).max(1.) * 1.05;
        let yy = y + (h - side) / 2.;
        let marlett = self.fonts.root.join("marlett.ttf").exists();
        if marlett {
            let font = self.fonts.get("marlett.ttf")?;
            let face = ttf_parser::Face::parse(&self.fonts.list[font].bytes, 0)?;
            for (c, color) in [
                ('g', [1.; 3]),
                ('f', [227. / 255.; 3]),
                ('e', [105. / 255.; 3]),
                ('d', [1.; 3]),
                ('c', [160. / 255.; 3]),
                ('b', g.color),
            ] {
                if c == 'b' && n.a("value") != "CHECKED" {
                    continue;
                }
                if let Some(id) = crate::pdf::glyph_id(&face, c) {
                    self.current.ops.push(Op::GlyphPaint {
                        x: x + 1.2,
                        y: yy + side,
                        font,
                        glyph: id.0,
                        size: side,
                        scale: 1.,
                        color,
                    });
                }
            }
        } else {
            self.box_lines(x, yy, side, side, 0.5, g.color);
        }
        if n.a("value") == "CHECKED" && !marlett {
            self.current.ops.push(Op::Path {
                points: vec![
                    (x + side * 0.15, yy + side * 0.5),
                    (x + side * 0.4, yy + side * 0.8),
                    (x + side * 0.9, yy + side * 0.15),
                ],
                closed: false,
                width: 0.9,
                color: g.color,
                fill: None,
            });
        } else if matches!(n.a("value"), "INDETERMINATE" | "MIXED") {
            self.current.ops.push(Op::Rect {
                x: x + side * 0.2,
                y: yy + side * 0.2,
                w: side * 0.6,
                h: side * 0.6,
                fill: [0.5; 3],
            });
        }
        let face = ttf_parser::Face::parse(&self.fonts.list[g.font].bytes, 0)?;
        let baseline = y
            + (h + (face.ascender() as f64 + face.descender() as f64) * g.size
                / face.units_per_em() as f64)
                / 2.;
        self.paint_generated_text(label, g, x + side + 3.6, baseline)
    }
}
