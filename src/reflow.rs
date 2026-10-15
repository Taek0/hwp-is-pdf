//! Paragraph line breaking with optional rectangular exclusion. Positions are UTF-16 offsets.
use crate::model::Node;
#[derive(Clone, Default)]
pub(crate) struct Unit {
    pub pos: usize,
    pub ch: Option<char>,
    pub width: f64,
    pub ascent: f64,
    pub descent: f64,
    pub em: f64,
    pub block: bool,
}
pub(crate) struct Line {
    pub pos: usize,
    pub height: f64,
    pub baseline: f64,
    pub left: f64,
    pub width: f64,
    pub hard: bool,
    pub block: bool,
}
#[derive(Default)]
pub(crate) struct Settings {
    pub left: f64,
    pub right: f64,
    pub indent: f64,
    pub before: f64,
    pub after: f64,
    pub spacing: String,
    pub value: f64,
    pub keep: bool,
    pub next: bool,
    pub widow: bool,
    pub page: bool,
    pub latin_char: bool,
    pub cjk_word: bool,
}
pub(crate) fn settings(n: Option<&Node>) -> Settings {
    let value = |key: &str| {
        n.and_then(|n| n.find("margin"))
            .and_then(|m| m.child(key))
            .map(|v| v.signed("value") / 100.)
            .unwrap_or(0.)
    };
    let flag = |key: &str| {
        n.and_then(|n| n.find("breakSetting"))
            .is_some_and(|b| matches!(b.a(key), "1" | "true"))
    };
    let spacing = n.and_then(|n| n.find("lineSpacing"));
    Settings {
        left: value("left"),
        right: value("right"),
        indent: value("intent"),
        before: value("prev").max(0.),
        after: value("next").max(0.),
        spacing: spacing.map(|s| s.a("type")).unwrap_or("PERCENT").into(),
        value: spacing
            .map(|s| s.n("value"))
            .filter(|v| *v > 0.)
            .unwrap_or(160.),
        keep: flag("keepLines"),
        next: flag("keepWithNext"),
        widow: flag("widowOrphan"),
        page: flag("pageBreakBefore"),
        latin_char: n
            .and_then(|n| n.find("breakSetting"))
            .is_some_and(|s| s.a("breakLatinWord") == "BREAK_WORD"),
        cjk_word: n
            .and_then(|n| n.find("breakSetting"))
            .is_some_and(|s| s.a("breakNonLatinWord") == "KEEP_WORD"),
    }
}
fn cjk(c: char) -> bool {
    matches!(c as u32,0x1100..=0x11ff|0x3000..=0x9fff|0xac00..=0xd7ff|0x20000..=0x323af)
}
fn opportunity(a: Option<char>, b: Option<char>, s: &Settings) -> bool {
    let Some(a) = a else {
        return false;
    };
    if "([{〈《「『【（［｛".contains(a)
        || b.is_some_and(|c| ",.!?:;)]}〉》」』】、。，！？：；）］｝".contains(c))
    {
        return false;
    }
    a == ' ' || a == '-' || (!s.cjk_word && cjk(a)) || s.latin_char
}
pub(crate) fn wrap(units: &[Unit], width: f64, s: &Settings) -> anyhow::Result<Vec<Line>> {
    wrap_around(units, width, s, None)
}
/// A paragraph-leading rectangular exclusion: left inset, right inset, height.
pub(crate) fn wrap_around(
    units: &[Unit],
    width: f64,
    s: &Settings,
    exclusion: Option<(f64, f64, f64)>,
) -> anyhow::Result<Vec<Line>> {
    wrap_exclusion(units, width, s, exclusion, true)
}
pub(crate) fn wrap_flow(
    units: &[Unit],
    width: f64,
    s: &Settings,
    exclusion: Option<(f64, f64, f64)>,
) -> anyhow::Result<Vec<Line>> {
    wrap_exclusion(units, width, s, exclusion, false)
}
fn wrap_exclusion(
    units: &[Unit],
    width: f64,
    s: &Settings,
    exclusion: Option<(f64, f64, f64)>,
    reserve_height: bool,
) -> anyhow::Result<Vec<Line>> {
    let mut out = Vec::new();
    let mut begin = 0;
    let mut y = 0.;
    loop {
        let (cut_left, cut_right) = exclusion
            .filter(|(_, _, h)| y < *h - 0.001)
            .map(|(l, r, _)| (l, r))
            .unwrap_or((0., 0.));
        let left = (s.left + if out.is_empty() { s.indent } else { 0. }).max(cut_left);
        let available = width - left - s.right.max(cut_right);
        anyhow::ensure!(
            available > 1.,
            "Paragraph margins leave no usable line width"
        );
        let mut end = begin;
        let mut used = 0.;
        let mut last_break: Option<usize> = None;
        let mut hard = false;
        let mut block = false;
        while end < units.len() {
            if units[end].block {
                if end > begin
                    && units[begin..end]
                        .iter()
                        .any(|u| u.width > 0. || u.ch.is_some())
                {
                    break;
                }
                end += 1;
                block = true;
                break;
            }
            if units[end].ch == Some('\n') {
                end += 1;
                hard = true;
                break;
            }
            if end > begin && used + units[end].width > available + 0.001 {
                if let Some(b) = last_break {
                    let mut word_width = 0.;
                    let mut at = b;
                    while at < units.len() && units[at].ch != Some('\n') {
                        word_width += units[at].width;
                        at += 1;
                        if opportunity(units[at - 1].ch, units.get(at).and_then(|u| u.ch), s) {
                            break;
                        }
                    }
                    if !units[b].ch.is_some_and(|c| c.is_ascii_alphanumeric())
                        || word_width <= (width - s.left - s.right).max(available)
                    {
                        end = b;
                    }
                }
                break;
            }
            used += units[end].width;
            end += 1;
            if opportunity(units[end - 1].ch, units.get(end).and_then(|u| u.ch), s) {
                last_break = Some(end);
            }
        }
        let slice = &units[begin..end];
        let ascent = slice.iter().map(|u| u.ascent).fold(0., f64::max);
        let descent = slice.iter().map(|u| u.descent).fold(0., f64::max);
        let em = slice.iter().map(|u| u.em).fold(0., f64::max);
        let em = if em > 0. { em } else { 12. };
        let ink = (ascent + descent).max(em);
        let requested = match s.spacing.as_str() {
            "FIXED" | "AT_LEAST" => s.value / 100.,
            "BETWEEN_LINES" => ink + s.value / 100.,
            _ => em * s.value / 100.,
        };
        let height = if block {
            ascent + descent
        } else {
            requested.max(ink + 1.)
        };
        out.push(Line {
            pos: units
                .get(begin)
                .map(|u| u.pos)
                .unwrap_or_else(|| units.last().map(|u| u.pos + 1).unwrap_or(0)),
            height,
            baseline: if ascent > 0. { ascent } else { em * 0.8 } + (height - ink) / 2.,
            left,
            width: available,
            hard,
            block,
        });
        begin = end;
        y += height;
        if begin >= units.len() {
            if hard {
                out.push(Line {
                    pos: units.last().unwrap().pos + 1,
                    height,
                    baseline: em * 0.8 + (height - ink) / 2.,
                    left: s.left,
                    width: width - s.left - s.right,
                    hard: false,
                    block: false,
                });
            }
            break;
        }
    }
    if reserve_height && let Some((_, _, height)) = exclusion {
        let used: f64 = out.iter().map(|l| l.height).sum();
        if let Some(last) = out.last_mut() {
            last.height += (height - used).max(0.);
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn units(s: &str) -> Vec<Unit> {
        let mut pos = 0;
        s.chars()
            .map(|c| {
                let u = Unit {
                    pos,
                    ch: Some(c),
                    width: 10.,
                    ascent: 8.,
                    descent: 2.,
                    em: 10.,
                    block: false,
                };
                pos += c.len_utf16();
                u
            })
            .collect()
    }
    fn config() -> Settings {
        Settings {
            spacing: "PERCENT".into(),
            value: 160.,
            ..Default::default()
        }
    }
    #[test]
    fn rectangular_exclusion_ends_at_its_bottom_and_reserves_short_paragraph_height() {
        for (left, right) in [(30., 0.), (0., 30.)] {
            let lines = wrap_around(
                &units(&"가".repeat(80)),
                100.,
                &config(),
                Some((left, right, 30.)),
            )
            .unwrap();
            assert_eq!(lines[0].width, 70.);
            assert_eq!(lines[1].width, 70.);
            assert_eq!(lines[2].width, 100.);
            assert_eq!(lines[0].left, left);
        }
        let lines = wrap_around(&units("가"), 100., &config(), Some((30., 0., 90.))).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].height, 90.);
        assert!(lines[0].baseline < 20.);
    }
    #[test]
    fn block_objects_are_separate_lines_without_font_leading() {
        let mut input = units("AZ");
        input[1].pos = 9;
        input.insert(
            1,
            Unit {
                pos: 1,
                ascent: 200.,
                block: true,
                ..Default::default()
            },
        );
        let lines = wrap(&input, 80., &config()).unwrap();
        assert_eq!(
            lines.iter().map(|l| l.pos).collect::<Vec<_>>(),
            vec![0, 1, 9]
        );
        assert!(lines[1].block);
        assert_eq!(lines[1].height, 200.);
    }
    #[test]
    fn word_boundaries_preserve_all_offsets() {
        let l = wrap(&units("Alpha beta gamma"), 80., &config()).unwrap();
        assert_eq!(l.iter().map(|l| l.pos).collect::<Vec<_>>(), vec![0, 6, 11]);
        let long = wrap(&units("a ABCDEFGHIJKLM"), 80., &config()).unwrap();
        assert_eq!(long[1].pos, 8);
    }
    #[test]
    fn surrogate_pairs_are_not_split() {
        let l = wrap(&units("가😀나"), 15., &config()).unwrap();
        assert_eq!(l.iter().map(|l| l.pos).collect::<Vec<_>>(), vec![0, 1, 3]);
    }
    #[test]
    fn explicit_blank_lines_and_closing_punctuation() {
        let l = wrap(&units("A\n\nB"), 80., &config()).unwrap();
        assert_eq!(l.iter().map(|l| l.pos).collect::<Vec<_>>(), vec![0, 2, 3]);
        let l = wrap(&units("가나,다"), 20., &config()).unwrap();
        assert!(!l.iter().any(|l| l.pos == 2));
    }
}
