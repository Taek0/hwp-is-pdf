//! Paragraph line breaking with optional rectangular exclusion. Positions are UTF-16 offsets.
use crate::model::Node;
#[derive(Clone, Default)]
pub(crate) struct Unit {
    pub pos: usize,
    pub ch: Option<char>,
    pub width: f64,
    /// Advance adjustment that applies only when another unit follows on the line.
    pub trailing: f64,
    pub ascent: f64,
    pub baseline_ascent: f64,
    pub descent: f64,
    pub em: f64,
    pub block: bool,
}
pub(crate) struct Line {
    pub pos: usize,
    pub height: f64,
    /// Visible line box; trailing interline space may cross a column boundary.
    pub extent: f64,
    pub baseline: f64,
    pub left: f64,
    pub width: f64,
    pub hard: bool,
    pub block: bool,
}
fn inline_width(units: &[Unit]) -> f64 {
    units.iter().map(|u| u.width).sum::<f64>()
        - units
            .iter()
            .rev()
            .find(|u| u.width != 0. || u.trailing != 0.)
            .map(|u| u.trailing)
            .unwrap_or(0.)
}
#[derive(Default)]
pub(crate) struct Settings {
    pub condense: f64,
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
    pub prefer_words: bool,
    pub discard_boundary_space: bool,
    pub tabs: Vec<crate::tabs::Stop>,
    pub tab_step: f64,
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
        condense: n
            .map(|p| p.n("condense").clamp(0., 75.) / 100.)
            .unwrap_or(0.),
        discard_boundary_space: n.is_some_and(|p| p.a("binaryHwp") == "1"),
        tabs: Vec::new(),
        tab_step: 40.,
        left: value("left") + (-value("intent")).max(0.),
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
            // Hancom's HWPX names are inverted for Korean: KEEP_WORD allows
            // character breaks, while BREAK_WORD preserves the whole word.
            .is_some_and(|s| s.a("breakNonLatinWord") == "BREAK_WORD"),
        prefer_words: n
            .and_then(|n| n.child("align"))
            .is_some_and(|a| a.a("horizontal") == "CENTER"),
    }
}
fn cjk(c: char) -> bool {
    matches!(c as u32,0x1100..=0x11ff|0x3000..=0x9fff|0xac00..=0xd7ff|0x20000..=0x323af)
}
fn opportunity(a: Option<char>, b: Option<char>, s: &Settings) -> bool {
    let Some(a) = a else {
        return false;
    };
    if "([{<〈《「『【（［｛".contains(a)
        || b.is_some_and(|c| ",.!?:;)]}>〉》」』】、。，！？：；）］｝…‥".contains(c))
    {
        return false;
    }
    a == ' '
        || a == '\t'
        || a == '-'
        || (!s.cjk_word && (cjk(a) || b.is_some_and(cjk)))
        || s.latin_char
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
    wrap_exclusion(units, width, s, exclusion.as_slice(), true, &[])
}
pub(crate) fn wrap_flow(
    units: &[Unit],
    width: f64,
    s: &Settings,
    exclusion: &[(f64, f64, f64)],
) -> anyhow::Result<Vec<Line>> {
    wrap_exclusion(units, width, s, exclusion, false, &[])
}
pub(crate) fn wrap_regions(
    units: &[Unit],
    width: f64,
    s: &Settings,
    regions: &[crate::wrap_region::Region],
) -> anyhow::Result<Vec<Line>> {
    wrap_exclusion(units, width, s, &[], false, regions)
}
fn wrap_exclusion(
    units: &[Unit],
    width: f64,
    s: &Settings,
    exclusion: &[(f64, f64, f64)],
    reserve_height: bool,
    regions: &[crate::wrap_region::Region],
) -> anyhow::Result<Vec<Line>> {
    let mut out = Vec::new();
    let mut begin = 0;
    let mut y = 0.;
    let mut row_gaps = std::collections::VecDeque::new();
    let mut row_metrics: Option<(f64, f64, f64)> = None;
    loop {
        let (cut_left, cut_right) = exclusion
            .iter()
            .filter(|(_, _, h)| y < *h - 0.001)
            .fold((0f64, 0f64), |(l, r), (a, b, _)| (l.max(*a), r.max(*b)));
        let mut left = (s.left + if out.is_empty() { s.indent } else { 0. }).max(cut_left);
        let mut available = width - left - s.right.max(cut_right);
        let mut skipped = 0.;
        if let Some((a, b)) = row_gaps.pop_front() {
            left = a;
            available = b - a;
            skipped = row_metrics.unwrap().2;
        } else if !regions.is_empty() {
            // Reserve the entire possible glyph/line band before selecting a gap.
            let asc = units.iter().map(|u| u.ascent).fold(0f64, f64::max);
            let desc = units.iter().map(|u| u.descent).fold(0f64, f64::max);
            let em = units.iter().map(|u| u.em).fold(12f64, f64::max);
            let ink = (asc + desc).max(em);
            let band = match s.spacing.as_str() {
                "FIXED" | "AT_LEAST" => s.value / 100.,
                "BETWEEN_LINES" => ink + s.value / 100.,
                _ => em * s.value / 100.,
            }
            .max(ink + 1.);
            let right = width - s.right;
            let required = units[begin..]
                .iter()
                .find(|u| u.width > 0.)
                .map(|u| u.width)
                .unwrap_or(1.)
                .max(24.);
            let mut found = Vec::new();
            for _ in 0..10000 {
                let gaps = crate::wrap_region::gaps(regions, y + skipped, band, left, right)
                    .into_iter()
                    .filter(|(a, b)| b - a >= required)
                    .collect::<Vec<_>>();
                if !gaps.is_empty() {
                    found = gaps;
                    break;
                }
                skipped += 1.;
            }
            anyhow::ensure!(
                !found.is_empty(),
                "No usable text interval around floating objects"
            );
            if regions.iter().any(|r| r.flow == "LARGEST_ONLY") {
                found = vec![
                    *found
                        .iter()
                        .max_by(|a, b| {
                            (a.1 - a.0)
                                .total_cmp(&(b.1 - b.0))
                                .then_with(|| b.0.total_cmp(&a.0))
                        })
                        .unwrap(),
                ];
            }
            let (a, b) = found.remove(0);
            if !found.is_empty() {
                row_gaps.extend(found);
                row_metrics = Some((
                    band,
                    if asc > 0. { asc } else { em * 0.8 } + (band - ink) / 2.,
                    skipped,
                ));
            }
            left = a;
            available = b - a;
        }
        // Keep a Latin token intact when a later interval on this row can hold it.
        let mut token_end = begin;
        let mut token_started = false;
        for unit in &units[begin..] {
            match unit.ch {
                Some(c) if c.is_ascii_alphanumeric() || (token_started && "'-._".contains(c)) => {
                    token_started = true;
                }
                None if !token_started => {}
                Some(' ') if !token_started => {}
                _ => break,
            }
            token_end += 1;
        }
        let token_width = inline_width(&units[begin..token_end]);
        if token_started
            && token_width > available
            && row_gaps.iter().any(|(a, b)| b - a >= token_width)
        {
            while let Some((a, b)) = row_gaps.pop_front() {
                left = a;
                available = b - a;
                if available >= token_width {
                    break;
                }
            }
        }
        anyhow::ensure!(
            available > 1.,
            "Paragraph margins leave no usable line width"
        );
        let mut end = begin;
        let mut used = 0.;
        let mut space_budget = 0.;
        let mut has_tab = false;
        let mut last_trailing = 0.;
        let mut last_break: Option<usize> = None;
        let mut last_space: Option<usize> = None;
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
            let advance = if units[end].ch == Some('\t') {
                crate::tabs::advance(
                    &s.tabs,
                    s.tab_step,
                    left + used,
                    units[end + 1..]
                        .iter()
                        .take_while(|u| !u.block)
                        .map(|u| (u.ch, u.width)),
                )
            } else {
                units[end].width
            };
            let trailing = if advance != 0. || units[end].trailing != 0. {
                units[end].trailing
            } else {
                last_trailing
            };
            has_tab |= units[end].ch == Some('\t');
            let next_budget = if has_tab {
                0.
            } else {
                space_budget
                    + if units[end].ch == Some(' ') {
                        advance.max(0.) * s.condense
                    } else {
                        0.
                    }
            };
            if end > begin && used + advance - trailing - next_budget > available + 0.001 {
                // A break space does not make an otherwise fitting final word
                // overflow. Consume it here instead of backing up into that word.
                if s.discard_boundary_space
                    && units[end].ch == Some(' ')
                    && used - last_trailing <= available + 0.001
                {
                    while end < units.len() && units[end].ch == Some(' ') {
                        end += 1;
                    }
                    break;
                }
                if s.prefer_words
                    && let Some(b) = last_space.filter(|b| *b > begin)
                {
                    let token_len = units[b..]
                        .iter()
                        .take_while(|u| !matches!(u.ch, Some(' ' | '\n' | '\t')) && !u.block)
                        .count();
                    let token_width = inline_width(&units[b..b + token_len]);
                    if token_width <= available {
                        end = b;
                        break;
                    }
                }
                if let Some(b) = last_break {
                    let mut at = b;
                    while at < units.len() && units[at].ch != Some('\n') {
                        at += 1;
                        if opportunity(units[at - 1].ch, units.get(at).and_then(|u| u.ch), s) {
                            break;
                        }
                    }
                    let word_width = inline_width(&units[b..at]);
                    if !units[b].ch.is_some_and(|c| c.is_ascii_alphanumeric())
                        || word_width <= (width - s.left - s.right).max(available)
                    {
                        end = b;
                    }
                }
                break;
            }
            used += advance;
            space_budget = next_budget;
            last_trailing = trailing;
            end += 1;
            if units[end - 1].ch == Some(' ') {
                last_space = Some(end);
            }
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
        let continues = !row_gaps.is_empty() && !hard && !block && end < units.len();
        let (row_height, row_baseline) = if let Some((h, b, skip)) = row_metrics {
            (h + skip, b + skip)
        } else {
            (
                height + skipped,
                if ascent > 0. { ascent } else { em * 0.8 } + (height - ink) / 2. + skipped,
            )
        };
        let advance = if continues { 0. } else { row_height };
        out.push(Line {
            pos: units
                .get(begin)
                .map(|u| u.pos)
                .unwrap_or_else(|| units.last().map(|u| u.pos + 1).unwrap_or(0)),
            height: advance,
            extent: row_height,
            baseline: row_baseline,
            left,
            width: available,
            hard,
            block,
        });
        begin = end;
        y += advance;
        if !continues {
            row_gaps.clear();
            row_metrics = None;
        }
        if begin >= units.len() {
            if hard {
                out.push(Line {
                    pos: units.last().unwrap().pos + 1,
                    height,
                    extent: height,
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
    if reserve_height {
        let height = exclusion.iter().map(|(_, _, h)| *h).fold(0f64, f64::max);
        let used: f64 = out.iter().map(|l| l.height).sum();
        if let Some(last) = out.last_mut() {
            last.height += (height - used).max(0.);
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    #[test]
    fn hwpx_negative_indent_reserves_the_continuation_margin() {
        let n = crate::model::xml(
            br#"<paraPr><margin><intent value="-2382"/><left value="0"/></margin></paraPr>"#,
        )
        .unwrap();
        let s = super::settings(Some(&n));
        assert!((s.left - 23.82).abs() < 0.001);
        assert!((s.left + s.indent).abs() < 0.001);
    }

    use super::*;
    #[test]
    fn ellipsis_keeps_a_preceding_character_at_the_line_start_boundary() {
        for mark in ['…', '‥'] {
            let units: Vec<_> = ['가', '나', '다', mark]
                .into_iter()
                .enumerate()
                .map(|(pos, ch)| Unit {
                    pos,
                    ch: Some(ch),
                    width: 10.,
                    em: 10.,
                    ..Default::default()
                })
                .collect();
            let wrapped = wrap(&units, 30., &Settings::default()).unwrap();
            assert_eq!(
                wrapped.iter().map(|l| l.pos).collect::<Vec<_>>(),
                vec![0, 2]
            );
        }
    }
    #[test]
    fn native_hanging_indent_keeps_first_line_at_left_margin() {
        let p=crate::model::xml(br#"<paraPr binaryHwp="1"><margin><left value="500"/><intent value="-1500"/></margin></paraPr>"#).unwrap();
        let s = settings(Some(&p));
        assert_eq!(s.left + s.indent, 5.);
        assert_eq!(s.left, 20.);
    }
    #[test]
    fn boundary_space_does_not_eject_a_fitting_word() {
        let mut s = config();
        s.discard_boundary_space = true;
        let rows = wrap(&units("ab cd ef"), 50., &s).unwrap();
        assert_eq!(rows.iter().map(|l| l.pos).collect::<Vec<_>>(), vec![0, 6]);
    }
    #[test]
    fn declared_condensation_can_fit_a_line_without_shrinking_letters() {
        let u = units("AB CD");
        let mut s = config();
        assert_eq!(wrap(&u, 48., &s).unwrap().len(), 2);
        s.condense = 0.25;
        assert_eq!(wrap(&u, 48., &s).unwrap().len(), 1);
        assert_eq!(wrap(&u, 46., &s).unwrap().len(), 2);
        assert!(u.iter().all(|u| u.width == 10.));
    }
    #[test]
    fn latin_word_uses_a_wider_later_interval_without_splitting() {
        let region = crate::wrap_region::Region {
            parts: None,
            points: vec![(40., 0.), (80., 0.), (80., 100.), (40., 100.)],
            margin: [0.; 4],
            flow: "BOTH_SIDES".into(),
        };
        let lines = wrap_regions(&units("English"), 160., &config(), &[region]).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].left, 80.);
    }
    fn units(s: &str) -> Vec<Unit> {
        let mut pos = 0;
        s.chars()
            .map(|c| {
                let u = Unit {
                    pos,
                    ch: Some(c),
                    width: 10.,
                    trailing: 0.,
                    ascent: 8.,
                    baseline_ascent: 8.,
                    descent: 2.,
                    em: 10.,
                    block: false,
                };
                pos += c.len_utf16();
                u
            })
            .collect()
    }
    #[test]
    fn positive_tracking_does_not_add_a_margin_after_the_last_glyph() {
        let mut input = units("AB");
        for u in &mut input {
            u.width = 14.;
            u.trailing = 4.;
        }
        assert_eq!(wrap(&input, 24., &config()).unwrap().len(), 1);
        assert_eq!(wrap(&input, 23.9, &config()).unwrap().len(), 2);
        input.push(Unit {
            pos: 2,
            ..Default::default()
        });
        assert_eq!(wrap(&input, 24., &config()).unwrap().len(), 1);
    }
    #[test]
    fn negative_tracking_does_not_hide_the_last_glyph_width() {
        let mut input = units("AB");
        for u in &mut input {
            u.width = 8.;
            u.trailing = -2.;
        }
        assert_eq!(wrap(&input, 18., &config()).unwrap().len(), 1);
        assert_eq!(wrap(&input, 17.9, &config()).unwrap().len(), 2);
    }
    fn config() -> Settings {
        Settings {
            spacing: "PERCENT".into(),
            value: 160.,
            ..Default::default()
        }
    }
    #[test]
    fn hanging_indent_keeps_the_first_line_at_the_declared_left_margin() {
        for binary in ["0", "1"] {
            let p = crate::model::xml(format!(r#"<paraPr binaryHwp="{binary}"><margin><left value="1600"/><intent value="-1500"/></margin></paraPr>"#).as_bytes()).unwrap();
            let s = settings(Some(&p));
            assert_eq!(s.left + s.indent, 16.);
            assert_eq!(s.left, 31.);
        }
    }
    #[test]
    fn opposing_exclusions_expire_at_separate_heights() {
        let lines = wrap_flow(
            &units(&"가".repeat(100)),
            100.,
            &config(),
            &[(20., 0., 30.), (0., 30., 70.)],
        )
        .unwrap();
        assert_eq!((lines[0].left, lines[0].width), (20., 50.));
        assert_eq!((lines[2].left, lines[2].width), (0., 70.));
        assert_eq!((lines[5].left, lines[5].width), (0., 100.));
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
    #[test]
    fn korean_hwpx_names_and_mixed_punctuation_follow_hancom() {
        let character =
            crate::model::xml(br#"<paraPr><breakSetting breakNonLatinWord="KEEP_WORD"/></paraPr>"#)
                .unwrap();
        let word = crate::model::xml(
            br#"<paraPr><breakSetting breakNonLatinWord="BREAK_WORD"/></paraPr>"#,
        )
        .unwrap();
        let c = settings(Some(&character));
        let w = settings(Some(&word));
        assert!(!c.cjk_word);
        assert!(w.cjk_word);
        assert_eq!(
            wrap(&units("가 나다라"), 30., &c)
                .unwrap()
                .iter()
                .map(|l| l.pos)
                .collect::<Vec<_>>(),
            vec![0, 3]
        );
        assert_eq!(
            wrap(&units("가 나다라"), 30., &w)
                .unwrap()
                .iter()
                .map(|l| l.pos)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert_eq!(
            wrap(&units("<가나>마"), 30., &c)
                .unwrap()
                .iter()
                .map(|l| l.pos)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert_eq!(
            wrap(&units("[4점]"), 20., &c)
                .unwrap()
                .iter()
                .map(|l| l.pos)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
    }
}
