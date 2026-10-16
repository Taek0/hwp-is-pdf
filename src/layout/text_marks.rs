//! Font-independent emphasis marks; vector decoration does not duplicate PDF text.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Emphasis {
    Dot,
    Ring,
    Tilde,
    Caron,
    Side,
    Colon,
    Grave,
    Acute,
    Circumflex,
    Macron,
    Hook,
    Below,
}
impl Emphasis {
    pub(super) fn named(name: &str) -> Option<Self> {
        Some(match name {
            "DOT_ABOVE" => Self::Dot,
            "RING_ABOVE" => Self::Ring,
            "TILDE" => Self::Tilde,
            "CARON" => Self::Caron,
            "SIDE" => Self::Side,
            "COLON" => Self::Colon,
            "GRAVE_ACCENT" => Self::Grave,
            "ACUTE_ACCENT" => Self::Acute,
            "CIRCUMFLEX" => Self::Circumflex,
            "MACRON" => Self::Macron,
            "HOOK_ABOVE" => Self::Hook,
            "DOT_BELOW" => Self::Below,
            _ => return None,
        })
    }
    pub(super) fn extent(self, size: f64) -> (f64, f64) {
        if self == Self::Below {
            (0., size * 0.28)
        } else {
            (size * 0.36, 0.)
        }
    }
}
impl<'a> Engine<'a> {
    pub(super) fn paint_emphasis(&mut self, g: &Glyph, x: f64, baseline: f64, advance: f64) {
        let Some(mark) = g.emphasis else {
            return;
        };
        let size = g.size;
        let cx = x + advance / 2.;
        let cy = if mark == Emphasis::Below {
            baseline + g.raw_descent + size * 0.18
        } else {
            baseline - g.raw_ascent - size * 0.20
        };
        let stroke = (size * 0.035).max(0.25);
        let mut circle = |y: f64, r: f64, filled: bool| {
            let points = (0..24)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 24.;
                    (cx + r * a.cos(), y + r * a.sin())
                })
                .collect();
            self.current.ops.push(Op::Path {
                points,
                closed: true,
                width: if filled { 0. } else { stroke },
                color: g.color,
                fill: filled.then_some(g.color),
            });
        };
        match mark {
            Emphasis::Dot | Emphasis::Below => circle(cy, size * 0.055, true),
            Emphasis::Ring => circle(cy, size * 0.09, false),
            Emphasis::Side => circle(cy, size * 0.035, true),
            Emphasis::Colon => {
                circle(cy - size * 0.065, size * 0.035, true);
                circle(cy + size * 0.065, size * 0.035, true);
            }
            Emphasis::Tilde => self.current.ops.push(Op::Bezier {
                start: (cx - size * 0.18, cy + size * 0.04),
                segments: vec![[
                    cx - size * 0.06,
                    cy - size * 0.16,
                    cx + size * 0.06,
                    cy + size * 0.16,
                    cx + size * 0.18,
                    cy - size * 0.04,
                ]],
                closed: false,
                width: stroke,
                color: g.color,
                fill: None,
            }),
            Emphasis::Hook => self.current.ops.push(Op::Bezier {
                start: (cx - size * 0.06, cy - size * 0.07),
                segments: vec![[
                    cx + size * 0.18,
                    cy - size * 0.18,
                    cx + size * 0.16,
                    cy + size * 0.05,
                    cx,
                    cy + size * 0.10,
                ]],
                closed: false,
                width: stroke,
                color: g.color,
                fill: None,
            }),
            _ => {
                let points = match mark {
                    Emphasis::Caron => vec![
                        (cx - size * 0.14, cy - size * 0.06),
                        (cx, cy + size * 0.08),
                        (cx + size * 0.14, cy - size * 0.06),
                    ],
                    Emphasis::Circumflex => vec![
                        (cx - size * 0.14, cy + size * 0.06),
                        (cx, cy - size * 0.08),
                        (cx + size * 0.14, cy + size * 0.06),
                    ],
                    Emphasis::Grave => vec![
                        (cx - size * 0.08, cy - size * 0.08),
                        (cx + size * 0.06, cy + size * 0.08),
                    ],
                    Emphasis::Acute => vec![
                        (cx - size * 0.06, cy + size * 0.08),
                        (cx + size * 0.08, cy - size * 0.08),
                    ],
                    _ => vec![(cx - size * 0.15, cy), (cx + size * 0.15, cy)],
                };
                self.current.ops.push(Op::Path {
                    points,
                    closed: false,
                    width: stroke,
                    color: g.color,
                    fill: None,
                });
            }
        }
    }
}
