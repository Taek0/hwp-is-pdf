//! Contiguous character border/fill bands, painted behind their glyphs.
use super::*;
pub(super) struct CharacterSpan {
    page: usize,
    operation: usize,
    next_item: usize,
    border: String,
    shade: Option<[f64; 3]>,
    rect: [f64; 4],
}
impl<'a> Engine<'a> {
    pub(super) fn character_border_width(&self, style: &Node) -> f64 {
        self.doc
            .borders
            .get(style.a("borderFillIDRef"))
            .map(|b| {
                b.children
                    .iter()
                    .filter(|s| s.name.ends_with("Border") && !matches!(s.a("type"), "" | "NONE"))
                    .map(|s| {
                        s.a("width")
                            .split_whitespace()
                            .next()
                            .unwrap_or("0.1")
                            .parse::<f64>()
                            .unwrap_or(0.1)
                            * 72.
                            / 25.4
                    })
                    .fold(0., f64::max)
            })
            .unwrap_or(0.)
    }
    pub(super) fn character_boundary_advance(
        &self,
        width: f64,
        left: &Item,
        right: &Item,
        a: &Glyph,
        b: &Glyph,
    ) -> f64 {
        if !matches!(left.kind, ItemKind::Char(_)) || !matches!(right.kind, ItemKind::Char(_)) {
            return width;
        }
        let x = self.doc.chars.get(&left.style);
        let y = self.doc.chars.get(&right.style);
        if x.map(|s| s.a("borderFillIDRef")) == y.map(|s| s.a("borderFillIDRef")) {
            return width;
        }
        let before = x.map(|s| self.character_border_width(s)).unwrap_or(0.);
        let after = y.map(|s| self.character_border_width(s)).unwrap_or(0.);
        let mut result = width;
        if after > 0. {
            result = result.max(a.ink_right - b.ink_left.min(0.) + after + 0.02);
        }
        if before > 0. {
            let natural = if let ItemKind::Char(c) = left.kind {
                self.fonts.width(a.font, &c.to_string(), a.size) * a.scale
            } else {
                0.
            };
            result = result.max(natural.max(a.ink_right) + before - b.ink_left + 0.02);
        }
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn collect_character_border(
        &self,
        spans: &mut Vec<CharacterSpan>,
        style: &str,
        item: usize,
        x: f64,
        base: f64,
        g: &Glyph,
        advance: f64,
    ) {
        let Some(ch) = self.doc.chars.get(style) else {
            return;
        };
        let stroke = self.character_border_width(ch);
        let border = ch.a("borderFillIDRef");
        let fill = self
            .doc
            .borders
            .get(border)
            .and_then(|b| b.find("winBrush"))
            .is_some_and(|f| !matches!(f.a("faceColor"), "" | "none" | "NONE"));
        let shade = (!matches!(ch.a("shadeColor"), "" | "none" | "NONE"))
            .then(|| color(ch.a("shadeColor")));
        if stroke == 0. && !fill && shade.is_none() {
            return;
        }
        let top = base + g.shift - (g.size * 0.85).max(g.raw_ascent) - stroke / 2.;
        let bottom = base + g.shift + (g.size * 0.15).max(g.raw_descent) + stroke / 2.;
        let rect = [
            x + g.ink_left.min(0.) - stroke / 2.,
            top,
            x + advance.max(g.ink_right) + stroke / 2.,
            bottom,
        ];
        if let Some(last) = spans.last_mut().filter(|s| {
            s.page == self.pages.len()
                && s.next_item == item
                && s.border == border
                && s.shade == shade
        }) {
            last.rect[0] = last.rect[0].min(rect[0]);
            last.rect[1] = last.rect[1].min(rect[1]);
            last.rect[2] = last.rect[2].max(rect[2]);
            last.rect[3] = last.rect[3].max(rect[3]);
            last.next_item = item + 1;
        } else {
            spans.push(CharacterSpan {
                page: self.pages.len(),
                operation: self.current.ops.len(),
                next_item: item + 1,
                border: border.into(),
                shade,
                rect,
            });
        }
    }
    pub(super) fn paint_character_borders(&mut self, spans: Vec<CharacterSpan>) {
        for span in spans.into_iter().rev() {
            let [left, top, right, bottom] = span.rect;
            let begin = self.current.ops.len();
            if let Some(fill) = span.shade {
                self.current.ops.push(Op::Rect {
                    x: left,
                    y: top,
                    w: right - left,
                    h: bottom - top,
                    fill,
                });
            }
            self.border(&span.border, left, top, right - left, bottom - top);
            let decorations = self.current.ops.split_off(begin);
            let count = decorations.len();
            if span.page == self.pages.len() {
                self.current
                    .ops
                    .splice(span.operation..span.operation, decorations);
                for note in &mut self.table_note_anchors {
                    if note.op_index >= span.operation {
                        note.op_index += count;
                    }
                }
            } else if let Some(page) = self.pages.get_mut(span.page) {
                page.ops.splice(span.operation..span.operation, decorations);
            }
        }
    }
}
