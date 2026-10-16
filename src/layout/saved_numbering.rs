//! Repair an omitted hanging-number reservation without rebuilding object flow.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn repair_numbered_cache(&mut self, source: &Node) -> Result<Option<Node>> {
        let saved = lines(source);
        if self.doc.header.a("binaryHwp") == "1"
            || saved.len() < 2
            || !self.list_auto_indent(source)
            || source.find("lineBreak").is_some()
            || saved.iter().any(|l| l.a("hardBreak") == "1")
            || saved
                .windows(2)
                .any(|p| p[1].n("vertpos") <= p[0].n("vertpos"))
        {
            return Ok(None);
        }
        let items = Self::items(source);
        if items.iter().any(|i| match &i.kind {
            ItemKind::Tab(_) => true,
            ItemKind::Object(n) => {
                n.name != "equation" || !n.child("pos").is_some_and(|p| p.a("treatAsChar") == "1")
            }
            ItemKind::Control(n) => n.name != "endNote",
            _ => false,
        }) {
            return Ok(None);
        }
        let Some((_, _, reserved)) = self.paragraph_number(source, false)? else {
            return Ok(None);
        };
        let first_x = saved[0].n("horzpos") / 100.;
        if saved
            .iter()
            .skip(1)
            .all(|l| l.n("horzpos") / 100. >= first_x + reserved - 0.01)
        {
            return Ok(None);
        }
        let width = saved[0].n("horzsize") / 100.;
        if width <= reserved
            || saved
                .iter()
                .any(|l| (l.n("horzsize") / 100. - width).abs() > 0.01)
        {
            return Ok(None);
        }
        let mut settings = crate::reflow::settings(self.doc.paras.get(source.a("paraPrIDRef")));
        settings.left = reserved;
        settings.right = 0.;
        settings.indent = 0.;
        let units = self.paragraph_units(source)?;
        let wrapped = crate::reflow::wrap(&units, width, &settings)?;
        if wrapped.len() <= saved.len() {
            return Ok(None);
        }
        let mut p = source.clone();
        let mut y = saved[0].n("vertpos");
        let mut cache = Vec::new();
        for (i, line) in wrapped.iter().enumerate() {
            let mut l = saved[i.min(saved.len() - 1)].clone();
            l.attrs.insert("textpos".into(), line.pos.to_string());
            l.attrs.insert("vertpos".into(), y.to_string());
            l.attrs
                .insert("horzpos".into(), (first_x * 100.).to_string());
            l.attrs
                .insert("hardBreak".into(), if line.hard { "1" } else { "0" }.into());
            y += if i + 1 < saved.len() {
                saved[i + 1].n("vertpos") - saved[i].n("vertpos")
            } else {
                l.n("vertsize") + l.n("spacing")
            };
            cache.push(l);
        }
        node_mut(&mut p, "linesegarray").children = cache;
        self.report.warnings.insert(format!("Repaired missing numbered-paragraph indent in saved wrapping: paragraph {}, {} -> {} lines", source.a("id"), saved.len(), wrapped.len()));
        Ok(Some(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expanded_numbered_cache_keeps_source_and_explicit_breaks() {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        d.chars
            .get_mut("0")
            .unwrap()
            .attrs
            .insert("height".into(), "1000".into());
        d.paras.insert(
            "test".into(),
            crate::model::xml(
                br#"<paraPr><heading type="NUMBER" idRef="test" level="0"/></paraPr>"#,
            )
            .unwrap(),
        );
        d.header.children.push(crate::model::xml(br#"<numbering id="test" start="1"><paraHead level="1" charPrIDRef="0" autoIndent="1" textOffsetType="PERCENT" textOffset="50">^1.</paraHead></numbering>"#).unwrap());
        let source = crate::model::xml(br#"<p id="p" paraPrIDRef="test"><run charPrIDRef="0"><t>alpha beta gamma delta epsilon zeta eta theta</t></run><linesegarray><lineseg textpos="0" vertpos="1000" vertsize="1000" spacing="600" baseline="850" horzsize="6000"/><lineseg textpos="20" vertpos="3000" vertsize="1000" spacing="600" baseline="850" horzsize="6000"/></linesegarray></p>"#).unwrap();
        let original = source.clone();
        let fonts = Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        });
        let mut e = Engine::new(&d, fonts);
        let repaired = e.repair_numbered_cache(&source).unwrap().unwrap();
        let ls = lines(&repaired);
        assert!(ls.len() > 2);
        assert_eq!(ls[0].n("vertpos"), 1000.);
        assert_eq!(ls[1].n("vertpos"), 3000.);
        assert!(ls.windows(2).all(
            |w| w[1].u("textpos") > w[0].u("textpos") && w[1].n("vertpos") > w[0].n("vertpos")
        ));
        assert_eq!(text(&source), text(&repaired));
        assert_eq!(source, original);
        let mut explicit = source.clone();
        node_mut(node_mut(&mut explicit, "run"), "t")
            .children
            .push(Node {
                name: "lineBreak".into(),
                ..Default::default()
            });
        assert!(e.repair_numbered_cache(&explicit).unwrap().is_none());
        let mut cached = source.clone();
        node_mut(&mut cached, "linesegarray").children[1]
            .attrs
            .insert("horzpos".into(), "3000".into());
        assert!(e.repair_numbered_cache(&cached).unwrap().is_none());
    }
}
