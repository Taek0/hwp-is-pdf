//! Character-cell tracking for generated list labels without losing logical text.
use super::*;
fn tracking_after(c: char, first: char, g: &Glyph) -> f64 {
    let factor = |c| {
        if text_metrics::language(c).0 == "latin" {
            0.5
        } else {
            1.
        }
    };
    g.tracking * factor(c) / factor(first)
}
pub(super) fn tracking(label: &str, g: &Glyph) -> f64 {
    let chars: Vec<_> = label.chars().collect();
    chars
        .iter()
        .take(chars.len().saturating_sub(1))
        .map(|c| tracking_after(*c, chars[0], g))
        .sum()
}
impl<'a> Engine<'a> {
    pub(super) fn paint_generated_text(
        &mut self,
        label: String,
        g: Glyph,
        x: f64,
        base: f64,
    ) -> Result<()> {
        if g.tracking == 0. || label.chars().count() < 2 {
            self.current.ops.push(Op::Text {
                x,
                y: base,
                text: label,
                font: g.font,
                size: g.size,
                scale: g.scale,
                color: g.color,
            });
            return Ok(());
        }
        let chars: Vec<_> = label.chars().collect();
        let ids = {
            let face = ttf_parser::Face::parse(&self.fonts.list[g.font].bytes, 0)?;
            chars
                .iter()
                .map(|c| crate::pdf::glyph_id(&face, *c).map(|id| id.0).unwrap_or(0))
                .collect::<Vec<_>>()
        };
        let mut offset = 0.;
        let mut glyphs = vec![];
        for (c, id) in chars.iter().zip(ids) {
            self.fonts.list[g.font].used.entry(id).or_insert(*c);
            glyphs.push(crate::pdf::PositionedGlyph {
                id,
                x: offset,
                y: 0.,
            });
            offset += self.fonts.width(g.font, &c.to_string(), g.size) * g.scale
                + tracking_after(*c, chars[0], &g);
        }
        self.current.ops.push(Op::Shaped {
            x,
            y: base,
            text: label,
            font: g.font,
            size: g.size,
            scale: g.scale,
            color: g.color,
            glyphs,
        });
        Ok(())
    }
}
