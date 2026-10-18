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
        let shear = g.shear;
        if shear != 0. {
            self.current.ops.push(Op::TransformBegin {
                matrix: [1., 0., -shear, 1., shear * base, 0.],
            });
        }
        let outline = g.outline.clone();
        text_outlines::begin(&mut self.current.ops, &outline);
        let result = self.paint_generated_glyphs(label, g, x, base);
        text_outlines::end(&mut self.current.ops, &outline);
        if shear != 0. {
            self.current.ops.push(Op::TransformEnd);
        }
        result
    }
    fn paint_generated_glyphs(&mut self, label: String, g: Glyph, x: f64, base: f64) -> Result<()> {
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
            self.fonts.list[g.font]
                .used
                .entry(id)
                .or_insert_with(|| c.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_labels_keep_synthetic_slant_with_and_without_tracking() {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let mut style = d.chars["0"].clone();
        style
            .children
            .retain(|c| !matches!(c.name.as_str(), "bold" | "italic"));
        style.children.push(Node {
            name: "italic".into(),
            ..Default::default()
        });
        node_mut(&mut style, "fontRef")
            .attrs
            .insert("latin".into(), "probe".into());
        d.fonts
            .insert(("LATIN".into(), "probe".into()), "함초롬바탕".into());
        d.chars.insert("probe".into(), style);
        let mut e = Engine::new(
            &d,
            Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        for tracking in [0., -0.2] {
            let mut g = e.glyph('1', "probe").unwrap();
            g.tracking = tracking;
            let shear = g.shear;
            assert!(shear > 0.);
            e.current.ops.clear();
            e.paint_generated_text("12)".into(), g, 20., 40.).unwrap();
            assert!(
                matches!(e.current.ops.first(),Some(Op::TransformBegin{matrix}) if (matrix[2]+shear).abs()<0.00001 && (matrix[4]-shear*40.).abs()<0.00001)
            );
            assert!(matches!(e.current.ops.last(), Some(Op::TransformEnd)));
            let text: String = e
                .current
                .ops
                .iter()
                .filter_map(|o| match o {
                    Op::Text { text, .. } | Op::Shaped { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(text, "12)");
        }
    }
}
