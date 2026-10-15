//! Paragraph border/fill bands and joins across adjacent source paragraphs.
use super::*;
fn enabled(n: &Node, key: &str) -> bool {
    matches!(n.a(key), "1" | "true")
}
impl<'a> Engine<'a> {
    pub(super) fn has_paragraph_borders(&self) -> bool {
        self.doc
            .paras
            .values()
            .filter_map(|p| p.child("border"))
            .filter_map(|b| self.doc.borders.get(b.a("borderFillIDRef")))
            .any(|b| {
                b.children
                    .iter()
                    .any(|s| s.name.ends_with("Border") && s.a("type") != "NONE")
                    || b.find("winBrush")
                        .is_some_and(|f| f.a("faceColor") != "none")
            })
    }
    pub(super) fn prepare_border_lists(&self, n: &mut Node) {
        let indices: Vec<_> = n
            .children
            .iter()
            .enumerate()
            .filter_map(|(i, c)| (c.name == "p").then_some(i))
            .collect();
        let joined = |a: &Node, b: &Node| {
            if b.a("pageBreak") == "1" {
                return false;
            }
            let aa = self.doc.paras.get(a.a("paraPrIDRef"));
            let bb = self.doc.paras.get(b.a("paraPrIDRef"));
            match (
                aa.and_then(|p| p.child("border")),
                bb.and_then(|p| p.child("border")),
            ) {
                (Some(x), Some(y)) => {
                    enabled(x, "connect")
                        && enabled(y, "connect")
                        && {
                            let mut xa = x.attrs.clone();
                            let mut ya = y.attrs.clone();
                            xa.remove("borderFillIDRef");
                            ya.remove("borderFillIDRef");
                            xa == ya
                                && self
                                    .doc
                                    .borders
                                    .get(x.a("borderFillIDRef"))
                                    .zip(self.doc.borders.get(y.a("borderFillIDRef")))
                                    .is_some_and(|(a, b)| a.children == b.children)
                        }
                        && {
                            let a = crate::reflow::settings(aa);
                            let b = crate::reflow::settings(bb);
                            !enabled(x, "ignoreMargin") || (a.left == b.left && a.right == b.right)
                        }
                }
                _ => false,
            }
        };
        let joins: Vec<_> = indices
            .windows(2)
            .map(|pair| joined(&n.children[pair[0]], &n.children[pair[1]]))
            .collect();
        for (at, &index) in indices.iter().enumerate() {
            let next_gap = if joins.get(at) == Some(&true) {
                lines(&n.children[index])
                    .last()
                    .zip(lines(&n.children[indices[at + 1]]).first())
                    .map(|(a, b)| (b.n("vertpos") - a.n("vertpos")).max(0.))
                    .unwrap_or(0.)
            } else {
                0.
            };
            let p = &mut n.children[index];
            p.attrs.insert(
                "borderFirst".into(),
                usize::from(at == 0 || !joins[at - 1]).to_string(),
            );
            p.attrs.insert(
                "borderLast".into(),
                usize::from(!joins.get(at).copied().unwrap_or(false)).to_string(),
            );
            if next_gap > 0. {
                p.attrs.insert("borderNextGap".into(), next_gap.to_string());
            }
        }
        for c in &mut n.children {
            self.prepare_border_lists(c);
        }
    }
    pub(super) fn paint_paragraph_border(
        &mut self,
        p: &Node,
        li: usize,
        ox: f64,
        oy: f64,
        width: f64,
    ) {
        let Some(style) = self.doc.paras.get(p.a("paraPrIDRef")) else {
            return;
        };
        let Some(border) = style.child("border") else {
            return;
        };
        let Some(def) = self.doc.borders.get(border.a("borderFillIDRef")) else {
            return;
        };
        if !def
            .children
            .iter()
            .any(|s| s.name.ends_with("Border") && s.a("type") != "NONE")
            && def.find("winBrush").is_none()
        {
            return;
        }
        let ls = lines(p);
        let line = ls[li];
        let settings = crate::reflow::settings(Some(style));
        // BorderText=0 uses the column box; 1 encloses the text area.
        let left = if enabled(border, "ignoreMargin") {
            ls.iter()
                .map(|l| l.n("horzpos") / 100.)
                .reduce(f64::min)
                .unwrap_or(settings.left)
        } else {
            0.
        };
        let right = if enabled(border, "ignoreMargin") {
            width
                - ls.iter()
                    .map(|l| (l.n("horzpos") + l.n("horzsize")) / 100.)
                    .filter(|v| *v > 0.)
                    .reduce(f64::max)
                    .unwrap_or(width - settings.right)
        } else {
            0.
        };
        let x = ox + left - border.n("offsetLeft") / 100.;
        let x2 = ox + width - right + border.n("offsetRight") / 100.;
        let first = li == 0 && p.u("reflowStart") == 0 && p.a("borderFirst") != "0";
        let last = li + 1 == ls.len() && p.a("borderLast") != "0";
        let y = oy + line.n("vertpos") / 100.
            - if first {
                border.n("offsetTop") / 100.
            } else {
                0.
            };
        let advance = ls
            .get(li + 1)
            .map(|next| next.n("vertpos") - line.n("vertpos"))
            .filter(|v| *v > 0.)
            .unwrap_or_else(|| {
                if !last && p.n("borderNextGap") > 0. {
                    p.n("borderNextGap")
                } else {
                    line.n("vertsize") + if last { 0. } else { line.n("spacing") }
                }
            });
        let mut y2 = oy
            + (line.n("vertpos") + advance) / 100.
            + if last {
                border.n("offsetBottom") / 100.
            } else {
                0.
            };
        if self.depth == 0 {
            y2 = y2.min(self.ph - self.bottom);
        }
        if x2 <= x || y2 <= y {
            return;
        }
        if let Some(fill) = def.find("winBrush").filter(|f| f.a("faceColor") != "none") {
            self.current.ops.push(Op::Rect {
                x,
                y,
                w: x2 - x,
                h: y2 - y,
                fill: color(fill.a("faceColor")),
            });
        }
        for (name, draw, (a, b, c, d)) in [
            ("leftBorder", true, (x, y, x, y2)),
            ("rightBorder", true, (x2, y, x2, y2)),
            ("topBorder", first, (x, y, x2, y)),
            ("bottomBorder", last, (x, y2, x2, y2)),
        ] {
            if draw && let Some(s) = def.child(name).filter(|s| s.a("type") != "NONE") {
                let mm = s
                    .a("width")
                    .split_whitespace()
                    .next()
                    .unwrap_or("0.1")
                    .parse::<f64>()
                    .unwrap_or(0.1);
                self.current.ops.push(Op::Line {
                    x: a,
                    y: b,
                    x2: c,
                    y2: d,
                    width: mm * 72. / 25.4,
                    color: color(s.a("color")),
                });
            }
        }
    }
}
