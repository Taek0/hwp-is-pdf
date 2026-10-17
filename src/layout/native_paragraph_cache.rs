//! Recover native plain-text lines after resolving fonts, preserving page boundaries.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn repair_native_paragraph_cache(
        &mut self,
        source: &Node,
        width: f64,
    ) -> Result<Option<Node>> {
        let saved = lines(source);
        let style = self.doc.paras.get(source.a("paraPrIDRef"));
        if self.doc.header.a("binaryHwp") != "1"
            || saved.len() < 2
            || source.a("reflowed") == "1"
            || source.find("lineBreak").is_some()
            || style
                .and_then(|s| s.child("heading"))
                .is_some_and(|h| !matches!(h.a("type"), "" | "NONE"))
            || Self::items(source)
                .iter()
                .any(|i| !matches!(i.kind, ItemKind::Char(_)))
            || saved
                .windows(2)
                .any(|w| w[1].n("vertpos") <= w[0].n("vertpos"))
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
        if wrapped.len() > saved.len()
            || (wrapped.len() == saved.len()
                && (settings.indent == 0.
                    || wrapped
                        .iter()
                        .zip(&saved)
                        .all(|(l, s)| l.pos == s.u("textpos"))))
        {
            return Ok(None);
        }
        let mut p = source.clone();
        p.attrs.insert("reflowed".into(), "1".into());
        node_mut(&mut p, "linesegarray").children = wrapped
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let mut l = saved[0].clone();
                l.attrs.insert("textpos".into(), line.pos.to_string());
                l.attrs
                    .insert("horzpos".into(), (line.left * 100.).to_string());
                l.attrs
                    .insert("horzsize".into(), (line.width * 100.).to_string());
                l.attrs.insert(
                    "vertpos".into(),
                    (saved[0].n("vertpos") + i as f64 * (l.n("vertsize") + l.n("spacing")))
                        .to_string(),
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
