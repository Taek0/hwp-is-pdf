//! Recover native text/annotation line boundaries without changing saved page slots.
use super::*;
fn preserves_note_pages(saved: &[&Node], starts: &[usize], notes: &[usize]) -> bool {
    let mut page = 0;
    let bands: Vec<_> = saved
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i > 0 && l.n("vertpos") < saved[i - 1].n("vertpos") {
                page += 1;
            }
            page
        })
        .collect();
    notes.iter().all(|pos| {
        let old = saved.iter().rposition(|l| l.u("textpos") <= *pos);
        let new = starts.iter().rposition(|s| *s <= *pos);
        old.zip(new)
            .is_some_and(|(a, b)| bands.get(a) == bands.get(b))
    })
}

impl<'a> Engine<'a> {
    pub(super) fn repair_native_paragraph_cache(
        &mut self,
        source: &Node,
        width: f64,
    ) -> Result<Option<Node>> {
        let saved = lines(source);
        let crosses_page = saved
            .windows(2)
            .any(|w| w[1].n("vertpos") < w[0].n("vertpos"));
        let style = self.doc.paras.get(source.a("paraPrIDRef"));
        let items = Self::items(source);
        let notes: Vec<_> = items.iter().filter_map(|i|
            matches!(&i.kind, ItemKind::Control(n) if matches!(n.name.as_str(),"footNote"|"endNote")).then_some(i.pos)
        ).collect();
        let preserve_slots = crosses_page || !notes.is_empty();
        if self.doc.header.a("binaryHwp") != "1"
            || saved.len() < 2
            || source.a("reflowed") == "1"
            || source.find("lineBreak").is_some()
            || style
                .and_then(|s| s.child("heading"))
                .is_some_and(|h| !matches!(h.a("type"), "" | "NONE"))
            || items.iter()
                .any(|i| !matches!(i.kind, ItemKind::Char(_)) && !matches!(&i.kind, ItemKind::Control(n) if matches!(n.name.as_str(),"footNote"|"endNote")))
            || saved
                .windows(2)
                .any(|w| w[1].n("vertpos") == w[0].n("vertpos"))
            || saved.iter().any(|l| {
                l.n("horzpos") != 0.
                    || (l.n("horzsize") / 100. - width).abs() > 0.1
                    || l.a("hardBreak") == "1"
                    || ["vertsize", "baseline", "spacing"]
                        .iter()
                        .any(|key| (l.n(key) - saved[0].n(key)).abs() > 0.01)
            })
        {
            return Ok(None);
        }
        let settings = crate::reflow::settings(style);
        let units = self.paragraph_units(source)?;
        let wrapped = crate::reflow::wrap(&units, width, &settings)?;
        if (preserve_slots && wrapped.len() != saved.len())
            || wrapped.len() > saved.len()
            || (wrapped.len() == saved.len()
                && (settings.indent == 0.
                    || wrapped
                        .iter()
                        .zip(&saved)
                        .all(|(l, s)| l.pos == s.u("textpos"))))
        {
            return Ok(None);
        }
        if !preserves_note_pages(
            &saved,
            &wrapped.iter().map(|l| l.pos).collect::<Vec<_>>(),
            &notes,
        ) {
            return Ok(None);
        }
        let mut p = source.clone();
        p.attrs.insert("reflowed".into(), "1".into());
        node_mut(&mut p, "linesegarray").children = wrapped
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let mut l = if preserve_slots {
                    saved[i].clone()
                } else {
                    saved[0].clone()
                };
                l.attrs.insert("textpos".into(), line.pos.to_string());
                l.attrs
                    .insert("horzpos".into(), (line.left * 100.).to_string());
                l.attrs
                    .insert("horzsize".into(), (line.width * 100.).to_string());
                l.attrs.insert(
                    "vertpos".into(),
                    if preserve_slots {
                        saved[i].a("vertpos").to_string()
                    } else {
                        (saved[0].n("vertpos") + i as f64 * (l.n("vertsize") + l.n("spacing")))
                            .to_string()
                    },
                );
                l
            })
            .collect();
        self.report.warnings.insert(format!(
            "Recovered native paragraph {}: {} -> {} lines",
            source.a("id"),
            saved.len(),
            wrapped.len()
        ));
        Ok(Some(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_note_page_is_stable_when_line_boundaries_change() {
        let a = crate::model::xml(br#"<lineseg textpos="0" vertpos="60000"/>"#).unwrap();
        let b = crate::model::xml(br#"<lineseg textpos="20" vertpos="0"/>"#).unwrap();
        assert!(!preserves_note_pages(&[&a, &b], &[0, 10], &[12]));
        assert!(preserves_note_pages(&[&a, &b], &[0, 10], &[25]));
        let mut same = b.clone();
        same.attrs.insert("vertpos".into(), "61600".into());
        assert!(preserves_note_pages(&[&a, &same], &[0, 10], &[12]));
    }
    #[test]
    fn note_bearing_native_paragraph_recovers_text_without_moving_page_slots() {
        let d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
        )
        .unwrap();
        let source = d
            .sections
            .iter()
            .flat_map(|s| s.children_named("p"))
            .find(|p| p.a("id") == "hwp-3e482")
            .unwrap();
        let original = source.clone();
        let mut e = Engine::new(
            &d,
            Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        let repaired = e
            .repair_native_paragraph_cache(source, 481.90)
            .unwrap()
            .expect("stale note-bearing cache must be recovered");
        let before = lines(source);
        let after = lines(&repaired);
        assert_eq!(before.len(), after.len());
        assert!(
            before
                .iter()
                .zip(&after)
                .any(|(a, b)| a.u("textpos") != b.u("textpos"))
        );
        for (a, b) in before.iter().zip(&after) {
            for key in ["vertpos", "vertsize", "baseline", "spacing"] {
                assert_eq!(a.a(key), b.a(key));
            }
        }
        assert_eq!(source, &original);
        assert_eq!(source.find("footNote"), repaired.find("footNote"));
    }
}
