//! Page border selection, visibility and coordinate normalization.
use super::*;

fn shown(value: &str, first: bool) -> bool {
    match value {
        "HIDE_FIRST" => !first,
        "SHOW_FIRST" => first,
        "HIDE_ALL" => false,
        _ => true,
    }
}

impl<'a> Engine<'a> {
    pub(super) fn paint_page_border(&mut self, sec: &Node, first: bool, hiding: Option<&Node>) {
        let visible = sec.child("visibility");
        let stroke = shown(visible.map(|v| v.a("border")).unwrap_or(""), first)
            && hiding.is_none_or(|h| h.a("hideBorder") != "1");
        let fill = shown(visible.map(|v| v.a("fill")).unwrap_or(""), first)
            && hiding.is_none_or(|h| h.a("hideFill") != "1");
        if !stroke && !fill {
            return;
        }
        let parity = if (self.pages.len() + 1).is_multiple_of(2) {
            "EVEN"
        } else {
            "ODD"
        };
        // Empty parity entries are serialized even when only BOTH is configured.
        let active = |n: &&Node| {
            self.doc
                .borders
                .get(n.a("borderFillIDRef"))
                .is_some_and(|b| {
                    ["leftBorder", "rightBorder", "topBorder", "bottomBorder"]
                        .iter()
                        .any(|side| {
                            b.child(side)
                                .is_some_and(|s| !matches!(s.a("type"), "" | "NONE"))
                        })
                        || b.find("winBrush")
                            .is_some_and(|f| !matches!(f.a("faceColor"), "" | "none"))
                })
        };
        let Some(b) = sec
            .children_named("pageBorderFill")
            .find(|n| n.a("type") == parity && active(n))
            .or_else(|| {
                sec.children_named("pageBorderFill")
                    .find(|n| n.a("type") == "BOTH")
            })
        else {
            return;
        };
        let m = margin(b, "offset");
        // The supplied Hancom HWPX export uses PAPER for a text-relative border:
        // (left - offset, top + header - offset), verified against its printed PDF.
        // Keep that wire enum separate from the documented HWP5 bit-0 origin.
        let paper = if b.a("borderReference").is_empty() {
            b.a("textBorder") == "CONTENT"
        } else {
            b.a("borderReference") == "PAPER"
        };
        let top = self.top
            - if b.a("headerInside") == "1" {
                self.header
            } else {
                0.
            };
        let bottom = self.bottom
            - if b.a("footerInside") == "1" {
                self.footer
            } else {
                0.
            };
        let rect = if paper {
            [m[0], m[2], self.pw - m[0] - m[1], self.ph - m[2] - m[3]]
        } else {
            [
                self.left - m[0],
                top - m[2],
                self.pw - self.left - self.right + m[0] + m[1],
                self.ph - top - bottom + m[2] + m[3],
            ]
        };
        let Some(style) = self.doc.borders.get(b.a("borderFillIDRef")) else {
            return;
        };
        if fill
            && let Some(brush) = style
                .find("winBrush")
                .filter(|f| !matches!(f.a("faceColor"), "" | "none"))
        {
            let [x, y, w, h] = match b.a("fillArea") {
                "PAPER" => [0., 0., self.pw, self.ph],
                "PAGE" => [
                    self.left,
                    self.top,
                    self.pw - self.left - self.right,
                    self.ph - self.top - self.bottom,
                ],
                _ => rect,
            };
            self.current.ops.push(Op::Rect {
                x,
                y,
                w,
                h,
                fill: color(brush.a("faceColor")),
            });
        }
        if stroke {
            // Reuse the common side painter but suppress its border-sized fill.
            let start = self.current.ops.len();
            self.border(b.a("borderFillIDRef"), rect[0], rect[1], rect[2], rect[3]);
            let lines = self
                .current
                .ops
                .drain(start..)
                .filter(|o| !matches!(o, Op::Rect { .. }))
                .collect::<Vec<_>>();
            self.current.ops.extend(lines);
        }
    }
}
