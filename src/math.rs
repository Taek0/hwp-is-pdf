//! HWP equation-script subset. Unrecognized text is preserved literally.
use crate::pdf::{Fonts, Op};
use anyhow::Result;
#[derive(Default)]
pub struct MathBox {
    pub ops: Vec<Op>,
    pub width: f64,
    pub ascent: f64,
    pub descent: f64,
}
pub(crate) fn group(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if !s.starts_with('{') {
        let end = s.find(char::is_whitespace).unwrap_or(s.len());
        return (&s[..end], &s[end..]);
    }
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        if c == '{' {
            depth += 1
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return (&s[1..i], &s[i + 1..]);
            }
        }
    }
    (&s[1..], "")
}
fn top_keyword(s: &str, keyword: &str) -> Option<usize> {
    let mut d = 0;
    for (i, c) in s.char_indices() {
        if c == '{' {
            d += 1
        } else if c == '}' {
            d -= 1
        }
        if d == 0
            && s[i..]
                .get(..keyword.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(keyword))
            && !(keyword == "over"
                && s[i..]
                    .get(..8)
                    .is_some_and(|s| s.eq_ignore_ascii_case("overline")))
        {
            return Some(i);
        }
    }
    None
}
pub(crate) fn moved(op: Op, dx: f64, dy: f64) -> Op {
    match op {
        Op::Bezier {
            start,
            segments,
            closed,
            width,
            color,
            fill,
        } => Op::Bezier {
            start: (start.0 + dx, start.1 + dy),
            segments: segments
                .into_iter()
                .map(|s| {
                    [
                        s[0] + dx,
                        s[1] + dy,
                        s[2] + dx,
                        s[3] + dy,
                        s[4] + dx,
                        s[5] + dy,
                    ]
                })
                .collect(),
            closed,
            width,
            color,
            fill,
        },
        Op::Shaped {
            x,
            y,
            text,
            font,
            size,
            scale,
            color,
            glyphs,
        } => Op::Shaped {
            x: x + dx,
            y: y + dy,
            text,
            font,
            size,
            scale,
            color,
            glyphs,
        },
        Op::Path {
            points,
            closed,
            width,
            color,
            fill,
        } => Op::Path {
            points: points.into_iter().map(|(x, y)| (x + dx, y + dy)).collect(),
            closed,
            width,
            color,
            fill,
        },
        Op::Text {
            x,
            y,
            text,
            font,
            size,
            scale,
            color,
        } => Op::Text {
            x: x + dx,
            y: y + dy,
            text,
            font,
            size,
            scale,
            color,
        },
        Op::Line {
            x,
            y,
            x2,
            y2,
            width,
            color,
        } => Op::Line {
            x: x + dx,
            y: y + dy,
            x2: x2 + dx,
            y2: y2 + dy,
            width,
            color,
        },
        other => other,
    }
}
pub fn layout(script: &str, size: f64, fonts: &mut Fonts) -> Result<MathBox> {
    anyhow::ensure!(script.len() <= 1_048_576, "Equation script exceeds 1 MiB");
    layout_inner(script, size, fonts, 0)
}
pub(crate) fn layout_inner(
    script: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
) -> Result<MathBox> {
    anyhow::ensure!(depth < 64, "Equation nesting exceeds 64 levels");
    let s = script.trim();
    let font = fonts.get("times.ttf")?;
    let mut result = MathBox {
        ascent: size * 0.85,
        descent: size * 0.2,
        ..Default::default()
    };
    if let Some(i) = top_keyword(s, "over") {
        let left = s[..i].trim();
        let num = if left.starts_with('{') {
            group(left).0
        } else {
            left
        };
        let (den, rest) = group(&s[i + 4..]);
        let n = layout_inner(num, size * 0.85, fonts, depth + 1)?;
        let d = layout_inner(den, size * 0.85, fonts, depth + 1)?;
        let width = n.width.max(d.width) + size * 0.3;
        let nr = -(size * 0.3 + n.descent);
        let dr = size * 0.15 + d.ascent;
        result.ops.extend(
            n.ops
                .into_iter()
                .map(|o| moved(o, (width - n.width) / 2., nr)),
        );
        result.ops.push(Op::Line {
            x: 0.,
            y: -size * 0.2,
            x2: width,
            y2: -size * 0.2,
            width: 0.45,
            color: [0.; 3],
        });
        result.ops.extend(
            d.ops
                .into_iter()
                .map(|o| moved(o, (width - d.width) / 2., dr)),
        );
        result.ascent = n.ascent - nr;
        result.descent = dr + d.descent;
        result.width = width;
        let tail = layout_inner(rest, size, fonts, depth + 1)?;
        result.ops.extend(
            tail.ops
                .into_iter()
                .map(|o| moved(o, width + size * 0.1, 0.)),
        );
        result.width += tail.width + size * 0.1;
        return Ok(result);
    }
    for keyword in ["atop", "choose"] {
        if let Some(i) = top_keyword(s, keyword) {
            let left = s[..i].trim();
            let top = if left.starts_with('{') {
                group(left).0
            } else {
                left
            };
            let (bottom, rest) = group(&s[i + keyword.len()..]);
            let mut b = crate::math_stacks::compose(
                top,
                bottom,
                size,
                fonts,
                depth + 1,
                keyword == "choose",
            )?;
            let tail = layout_inner(rest, size, fonts, depth + 1)?;
            b.ops.extend(
                tail.ops
                    .into_iter()
                    .map(|o| moved(o, b.width + size * 0.1, 0.)),
            );
            b.width += tail.width;
            return Ok(b);
        }
    }
    let mut p = 0;
    let mut bar = false;
    while p < s.len() {
        let tail = &s[p..];
        if let Some((b, used)) = crate::math_accents::parse(tail, size, fonts, depth + 1)?
            .or(crate::math_stacks::prefix(tail, size, fonts, depth + 1)?)
        {
            result
                .ops
                .extend(b.ops.into_iter().map(|o| moved(o, result.width, 0.)));
            result.width += b.width;
            result.ascent = result.ascent.max(b.ascent);
            result.descent = result.descent.max(b.descent);
            p += used;
            continue;
        }
        if let Some((b, used)) = crate::math_extra::parse(tail, size, fonts, depth + 1)? {
            result
                .ops
                .extend(b.ops.into_iter().map(|o| moved(o, result.width, 0.)));
            result.width += b.width;
            result.ascent = result.ascent.max(b.ascent);
            result.descent = result.descent.max(b.descent);
            p += used;
            continue;
        }
        if tail.starts_with('^') || tail.starts_with('_') {
            let up = tail.starts_with('^');
            let (g, rest) = group(&tail[1..]);
            let take = tail.len() - 1 - rest.len();
            let child = layout_inner(g, size * 0.65, fonts, depth + 1)?;
            let dy = if up { -size * 0.55 } else { size * 0.28 };
            result
                .ops
                .extend(child.ops.into_iter().map(|o| moved(o, result.width, dy)));
            result.width += child.width;
            result.ascent = result.ascent.max(child.ascent - dy);
            result.descent = result.descent.max(child.descent + dy);
            p += 1 + take;
            continue;
        }
        let mut matched = false;
        for (key, replacement) in [
            ("LEFT", ""),
            ("RIGHT", ""),
            ("bold", ""),
            ("rm", ""),
            ("ANGLE", "∠"),
            ("DEG", "°"),
            ("pi", "π"),
            ("bar", ""),
            ("sum", "∑"),
            ("SUM", "∑"),
            ("prod", "∏"),
            ("PROD", "∏"),
            ("oint", "∮"),
            ("int", "∫"),
            ("INT", "∫"),
            ("inf", "∞"),
            ("alpha", "α"),
            ("beta", "β"),
            ("gamma", "γ"),
            ("theta", "θ"),
            ("lambda", "λ"),
            ("omega", "ω"),
            ("times", "×"),
            ("cdot", "·"),
            ("leq", "≤"),
            ("geq", "≥"),
            ("neq", "≠"),
        ] {
            if tail.starts_with(key) {
                if key == "bar" {
                    bar = true;
                }
                if !replacement.is_empty() {
                    let f = if fonts.has(font, replacement.chars().next().unwrap()) {
                        font
                    } else {
                        fonts.get("seguisym.ttf")?
                    };
                    let w = fonts.width(f, replacement, size);
                    result.ops.push(Op::Text {
                        x: result.width,
                        y: 0.,
                        text: replacement.into(),
                        font: f,
                        size,
                        scale: 1.,
                        color: [0.; 3],
                    });
                    result.width += w;
                }
                p += key.len();
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        let c = tail.chars().next().unwrap();
        p += c.len_utf8();
        if c == '{' || c == '}' {
            continue;
        }
        if c == '`' || c == '~' {
            result.width += size * 0.18;
            continue;
        }
        if c.is_whitespace() {
            continue;
        }
        let f = if fonts.has(font, c) {
            font
        } else {
            fonts.get("malgun.ttf")?
        };
        let t = c.to_string();
        let w = fonts.width(f, &t, size);
        result.ops.push(Op::Text {
            x: result.width,
            y: 0.,
            text: t,
            font: f,
            size,
            scale: 1.,
            color: [0.; 3],
        });
        if bar {
            result.ops.push(Op::Line {
                x: result.width,
                y: -size * 0.83,
                x2: result.width + w,
                y2: -size * 0.83,
                width: 0.4,
                color: [0.; 3],
            });
        }
        result.width += w;
    }
    Ok(result)
}
pub fn place(b: MathBox, x: f64, baseline: f64, w: f64, h: f64) -> Vec<Op> {
    let sx = if b.width > 0. { w / b.width } else { 1. };
    let sy = if b.ascent + b.descent > h {
        h / (b.ascent + b.descent)
    } else {
        1.
    };
    b.ops
        .into_iter()
        .map(|op| match op {
            Op::Bezier {
                start,
                segments,
                closed,
                width,
                color,
                fill,
            } => Op::Bezier {
                start: (x + start.0 * sx, baseline + start.1 * sy),
                segments: segments
                    .into_iter()
                    .map(|s| {
                        [
                            x + s[0] * sx,
                            baseline + s[1] * sy,
                            x + s[2] * sx,
                            baseline + s[3] * sy,
                            x + s[4] * sx,
                            baseline + s[5] * sy,
                        ]
                    })
                    .collect(),
                closed,
                width,
                color,
                fill,
            },
            Op::Shaped {
                x: xx,
                y,
                text,
                font,
                size,
                scale,
                color,
                mut glyphs,
            } => {
                for g in &mut glyphs {
                    g.x *= sx;
                    g.y *= sy;
                }
                Op::Shaped {
                    x: x + xx * sx,
                    y: baseline + y * sy,
                    text,
                    font,
                    size: size * sy,
                    scale: scale * sx / sy,
                    color,
                    glyphs,
                }
            }
            Op::Path {
                points,
                closed,
                width,
                color,
                fill,
            } => Op::Path {
                points: points
                    .into_iter()
                    .map(|(xx, yy)| (x + xx * sx, baseline + yy * sy))
                    .collect(),
                closed,
                width,
                color,
                fill,
            },
            Op::Text {
                x: xx,
                y,
                text,
                font,
                size,
                scale,
                color,
            } => Op::Text {
                x: x + xx * sx,
                y: baseline + y * sy,
                text,
                font,
                size: size * sy,
                scale: scale * sx / sy,
                color,
            },
            Op::Line {
                x: xx,
                y,
                x2,
                y2,
                width,
                color,
            } => Op::Line {
                x: x + xx * sx,
                y: baseline + y * sy,
                x2: x + x2 * sx,
                y2: baseline + y2 * sy,
                width,
                color,
            },
            o => o,
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn balanced_group() {
        assert_eq!(group("{12} cm"), ("12", " cm"));
    }
    #[test]
    fn fraction_operator_outside_groups() {
        assert_eq!(top_keyword("{10} over {3}", "over"), Some(5));
    }
}
