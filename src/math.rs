//! HWP equation-script subset. Unrecognized text is preserved literally.
use crate::pdf::{Fonts, Op};
use anyhow::Result;
mod operators;
mod placement;
pub(crate) mod style;
mod symbols;
pub(crate) use placement::moved;
pub use placement::place;
use style::Style;
#[derive(Default)]
pub struct MathBox {
    pub ops: Vec<Op>,
    pub width: f64,
    pub ascent: f64,
    pub descent: f64,
}
pub(crate) fn group(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if s.starts_with('"')
        && let Some(end) = s[1..].find('"')
    {
        return (&s[..end + 2], &s[end + 2..]);
    }
    if !s.starts_with('{') {
        if let Some(arrow) = ["<=>", "<->", "->", "<-", "=>", "<="]
            .into_iter()
            .find(|a| s.starts_with(a))
        {
            return (&s[..arrow.len()], &s[arrow.len()..]);
        }
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
    let mut delimiters = 0usize;
    let mut quote = false;
    for (i, c) in s.char_indices() {
        if c == '"' {
            quote = !quote;
            continue;
        }
        if quote {
            continue;
        }
        if s[i..]
            .get(..4)
            .is_some_and(|v| v.eq_ignore_ascii_case("left"))
        {
            delimiters += 1;
        }
        if s[i..]
            .get(..5)
            .is_some_and(|v| v.eq_ignore_ascii_case("right"))
        {
            delimiters = delimiters.saturating_sub(1);
        }
        if c == '{' {
            d += 1
        } else if c == '}' {
            d -= 1
        }
        if d == 0
            && delimiters == 0
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
pub fn layout(script: &str, size: f64, fonts: &mut Fonts) -> Result<MathBox> {
    anyhow::ensure!(script.len() <= 1_048_576, "Equation script exceeds 1 MiB");
    layout_inner(script, size, fonts, 0, Style::default())
}
pub(crate) fn layout_inner(
    script: &str,
    size: f64,
    fonts: &mut Fonts,
    depth: usize,
    mut style: Style,
) -> Result<MathBox> {
    anyhow::ensure!(depth < 64, "Equation nesting exceeds 64 levels");
    let mut s = script.trim();
    while let Some(rest) = style.consume(s) {
        s = rest.trim_start();
    }
    let mut result = MathBox {
        ascent: size * 0.85,
        descent: size * 0.2,
        ..Default::default()
    };
    if let Some(i) = top_keyword(s, "over") {
        let left = s[..i].trim();
        let num = left;
        let (den, rest) = group(&s[i + 4..]);
        let n = layout_inner(num, size, fonts, depth + 1, style)?;
        let d = layout_inner(den, size, fonts, depth + 1, style)?;
        let width = n.width.max(d.width) + size * 0.3;
        let nr = -(size * 0.34 + n.descent);
        let dr = d.ascent - size * 0.09;
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
            color: style.color,
        });
        result.ops.extend(
            d.ops
                .into_iter()
                .map(|o| moved(o, (width - d.width) / 2., dr)),
        );
        result.ascent = n.ascent - nr;
        result.descent = dr + d.descent;
        result.width = width;
        let tail = layout_inner(rest, size, fonts, depth + 1, style)?;
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
            let top = left;
            let (bottom, rest) = group(&s[i + keyword.len()..]);
            let mut b = crate::math_stacks::compose(
                top,
                bottom,
                size,
                fonts,
                depth + 1,
                keyword == "choose",
                style,
            )?;
            let tail = layout_inner(rest, size, fonts, depth + 1, style)?;
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
    let mut script_anchor: Option<(f64, f64)> = None;
    while p < s.len() {
        let tail = &s[p..];
        if let Some((b, used)) = crate::math_delimiters::parse(tail, size, fonts, depth + 1, style)?
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
        if let Some((b, used)) = operators::parse(tail, size, fonts, depth + 1, style)? {
            result
                .ops
                .extend(b.ops.into_iter().map(|o| moved(o, result.width, 0.)));
            result.width += b.width;
            result.ascent = result.ascent.max(b.ascent);
            result.descent = result.descent.max(b.descent);
            p += used;
            continue;
        }
        if let Some((b, used)) = crate::math_relations::parse(tail, size, fonts, depth + 1, style)?
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
        if let Some(rest) = style.consume(tail) {
            p += tail.len() - rest.len();
            continue;
        }
        if tail.starts_with('{') {
            let (body, rest) = group(tail);
            let b = layout_inner(body, size, fonts, depth + 1, style)?;
            result
                .ops
                .extend(b.ops.into_iter().map(|o| moved(o, result.width, 0.)));
            result.width += b.width;
            result.ascent = result.ascent.max(b.ascent);
            result.descent = result.descent.max(b.descent);
            script_anchor = None;
            p += tail.len() - rest.len();
            continue;
        }
        if let Some((b, used)) =
            crate::math_accents::parse(tail, size, fonts, depth + 1, style)?.or(
                crate::math_stacks::prefix(tail, size, fonts, depth + 1, style)?,
            )
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
        if let Some((b, used)) = crate::math_extra::parse(tail, size, fonts, depth + 1, style)? {
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
            let child = layout_inner(g, size * 0.68, fonts, depth + 1, style)?;
            let dy = if up { -size * 0.55 } else { size * 0.28 };
            let (anchor, previous) = script_anchor.unwrap_or((result.width, 0.));
            result
                .ops
                .extend(child.ops.into_iter().map(|o| moved(o, anchor, dy)));
            let width = previous.max(child.width);
            result.width = anchor + width;
            script_anchor = Some((anchor, width));
            result.ascent = result.ascent.max(child.ascent - dy);
            result.descent = result.descent.max(child.descent + dy);
            p += 1 + take;
            continue;
        }
        script_anchor = None;
        if let Some(rest) = tail.strip_prefix('"')
            && let Some(end) = rest.find('"')
        {
            for c in rest[..end].chars() {
                let font = style.font(fonts, c)?;
                let t = c.to_string();
                let width = fonts.width(font, &t, size);
                result.ops.push(Op::Text {
                    x: result.width,
                    y: 0.,
                    text: t,
                    font,
                    size,
                    scale: 1.,
                    color: style.color,
                });
                result.width += width;
            }
            p += end + 2;
            continue;
        }
        if let Some((text, used, roman)) = symbols::token(tail) {
            let mut applied = style;
            if roman {
                applied.italic = Some(false);
            }
            let font = applied.font(fonts, text.chars().next().unwrap())?;
            let font = if text.chars().all(|c| fonts.has(font, c)) {
                font
            } else {
                fonts.get("seguisym.ttf")?
            };
            result.ops.push(Op::Text {
                x: result.width,
                y: 0.,
                text: text.into(),
                font,
                size,
                scale: 1.,
                color: style.color,
            });
            result.width += fonts.width(font, text, size);
            p += used;
            continue;
        }
        let mut matched = false;
        for (key, replacement) in [
            ("LEFT", ""),
            ("RIGHT", ""),
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
                    let font = style.font(fonts, replacement.chars().next().unwrap())?;
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
                        color: style.color,
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
            let space_font = style.font(fonts, ' ')?;
            result.width += fonts.width(space_font, " ", size) * if c == '`' { 0.25 } else { 1. };
            continue;
        }
        if c.is_whitespace() {
            continue;
        }
        let font = style.font(fonts, c)?;
        let f = if fonts.has(font, c) {
            font
        } else {
            fonts.get("malgun.ttf")?
        };
        let binary = matches!(c, '+' | '-' | '=' | '<' | '>')
            && result.width > 0.
            && s[..p - c.len_utf8()]
                .trim_end()
                .chars()
                .last()
                .is_some_and(|c| !"=<>+-({".contains(c));
        let operator_gap = if binary { size * 0.18 } else { 0. };
        result.width += operator_gap;
        let t = c.to_string();
        let w = fonts.width(f, &t, size);
        result.ops.push(Op::Text {
            x: result.width,
            y: 0.,
            text: t,
            font: f,
            size,
            scale: 1.,
            color: style.color,
        });
        if bar {
            result.ops.push(Op::Line {
                x: result.width,
                y: -size * 0.83,
                x2: result.width + w,
                y2: -size * 0.83,
                width: 0.4,
                color: style.color,
            });
        }
        result.width += w + operator_gap;
    }
    Ok(result)
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
