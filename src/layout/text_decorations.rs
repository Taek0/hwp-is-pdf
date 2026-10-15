//! Styled underline and strike spans emitted after text placement.
use super::*;
type UnderlineSpan = (String, f64, f64, f64, f64, f64);
type StrikeSpan = (String, f64, f64, f64, f64);
impl<'a> Engine<'a> {
    pub(super) fn paint_line_decorations(
        &mut self,
        base: f64,
        underlines: Vec<UnderlineSpan>,
        strikes: Vec<StrikeSpan>,
    ) {
        for (style, x1, x2, size, descent, shift) in underlines {
            let def = self.doc.chars[&style].child("underline").unwrap();
            let Some(line_style) = crate::decoration::style(def) else {
                if def.a("shape") != "NONE" {
                    self.report.warnings.insert(format!(
                        "Unsupported underline shape: {} {}",
                        def.a("shape"),
                        def.a("style")
                    ));
                }
                continue;
            };
            let y = if def.a("type") == "TOP" {
                base + shift - size
            } else {
                base + shift + descent + 0.9
            };
            self.current.ops.extend(crate::decoration::underline(
                x1,
                x2,
                y,
                line_style,
                color(def.a("color")),
            ));
        }
        for (style, x1, x2, size, shift) in strikes {
            let def = self.doc.chars[&style].child("strikeout").unwrap();
            self.current.ops.extend(crate::decoration::underline(
                x1,
                x2,
                base + shift - size * 0.3,
                crate::decoration::style(def).unwrap(),
                color(def.a("color")),
            ));
        }
    }
}
