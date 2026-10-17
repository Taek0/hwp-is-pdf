//! Language-dependent character properties and optional Latin pair kerning.
use super::*;
/// Same-face positive advances can legitimately have overlapping bounding boxes
/// (serifs, side bearings and authored tracking). They are not ink collisions.
pub(super) fn compatible_advance(width: f64, left: &Glyph, right: &Glyph) -> f64 {
    if width > (left.ink_right - left.ink_left).max(0.) * 0.5
        && left.font == right.font
        && left.size == right.size
        && left.scale == right.scale
    {
        width
    } else {
        width.max(left.ink_right - right.ink_left + 0.02)
    }
}
/// Tracking separates advances; it is not a margin after the final glyph.
pub(super) fn trim_trailing_tracking(
    items: &[Item],
    glyphs: &mut [Option<Glyph>],
    widths: &mut [f64],
) {
    for i in (0..items.len()).rev() {
        if matches!(items[i].kind,ItemKind::Char(c) if !c.is_control()) {
            if let Some(g) = &mut glyphs[i] {
                widths[i] -= g.tracking;
                g.width -= g.tracking;
                g.tracking = 0.;
            }
            break;
        }
        if widths[i] != 0. {
            break;
        }
    }
}
pub(super) fn language(c: char) -> (&'static str, &'static str) {
    match c as u32 {
        0x3040..=0x30ff | 0x31f0..=0x31ff | 0xff66..=0xff9f => ("japanese", "JAPANESE"),
        0x3400..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x323af => ("hanja", "HANJA"),
        0x1100..=0x11ff | 0x3130..=0x318f | 0xa960..=0xa97f | 0xac00..=0xd7ff => {
            ("hangul", "HANGUL")
        }
        0..=0x024f => ("latin", "LATIN"),
        0xe000..=0xf8ff | 0xf0000..=0x10fffd => ("user", "USER"),
        // Hancom selects the OTHER face for the one-dot leader, not the
        // symbol face. The supplied native reference embeds HCR Batang here.
        0x2024 => ("other", "OTHER"),
        0x2000..=0x2bff | 0x3200..=0x33ff => ("symbol", "SYMBOL"),
        _ => ("other", "OTHER"),
    }
}
pub(super) fn property(ch: &Node, name: &str, lang: &str, fallback: &str, default: f64) -> f64 {
    ch.child(name)
        .and_then(|n| n.attrs.get(lang).or_else(|| n.attrs.get(fallback)))
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
impl<'a> Engine<'a> {
    pub(super) fn pair_kerning(
        &mut self,
        a: &Item,
        b: &Item,
        left: &Glyph,
        right: &Glyph,
    ) -> Option<f64> {
        let (ItemKind::Char(c), ItemKind::Char(d)) = (&a.kind, &b.kind) else {
            return None;
        };
        if a.style != b.style
            || left.font != right.font
            || left.size != right.size
            || left.scale != right.scale
            || *c as u32 > 0x024f
            || *d as u32 > 0x024f
            || c.is_whitespace()
            || d.is_whitespace()
            || self
                .doc
                .chars
                .get(&a.style)
                .is_none_or(|s| !matches!(s.a("useKerning"), "1" | "true"))
        {
            return None;
        }
        Some(self.fonts.kerning(left.font, *c, *d, left.size) * left.scale)
    }
}
