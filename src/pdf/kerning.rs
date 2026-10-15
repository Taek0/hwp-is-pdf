//! OpenType pair-kerning advances; shaping remains separate for complex scripts.
use super::*;
impl Fonts {
    pub fn kerning(&mut self, font: usize, left: char, right: char, size: f64) -> f64 {
        let adjustment = *self
            .kern_cache
            .entry((font, left, right))
            .or_insert_with(|| {
                let Some(face) = rustybuzz::Face::from_slice(&self.list[font].bytes, 0) else {
                    return 0.;
                };
                let text = format!("{left}{right}");
                let measure = |on: bool| {
                    let mut buffer = rustybuzz::UnicodeBuffer::new();
                    buffer.push_str(&text);
                    buffer.guess_segment_properties();
                    buffer.set_direction(rustybuzz::Direction::LeftToRight);
                    let features = [
                        if on { "kern=1" } else { "kern=0" },
                        "liga=0",
                        "clig=0",
                        "calt=0",
                    ]
                    .map(|s| s.parse::<rustybuzz::Feature>().unwrap());
                    rustybuzz::shape(&face, &features, buffer)
                };
                let a = measure(false);
                let b = measure(true);
                if a.glyph_infos().len() != 2
                    || b.glyph_infos().len() != 2
                    || a.glyph_infos()
                        .iter()
                        .zip(b.glyph_infos())
                        .any(|(a, b)| a.glyph_id != b.glyph_id)
                {
                    return 0.;
                }
                let advance = |b: &rustybuzz::GlyphBuffer| {
                    b.glyph_positions()
                        .iter()
                        .map(|p| p.x_advance as f64)
                        .sum::<f64>()
                };
                (advance(&b) - advance(&a)) / face.units_per_em() as f64
            });
        adjustment * size
    }
}
