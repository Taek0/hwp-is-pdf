//! Recover overflowing text after a complete native inline-object line.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn repair_native_inline_tail(
        &mut self,
        source: &Node,
        width: f64,
    ) -> Result<Option<Node>> {
        let saved = lines(source);
        if self.doc.header.a("binaryHwp") != "1" || source.a("reflowed") == "1" || saved.len() != 1
        {
            return Ok(None);
        }
        let line = saved[0];
        let style = self.doc.paras.get(source.a("paraPrIDRef"));
        if line.n("horzpos") != 0.
            || (line.n("horzsize") / 100. - width).abs() > 0.1
            || line.a("hardBreak") == "1"
            || style
                .and_then(|s| s.child("heading"))
                .is_some_and(|h| !matches!(h.a("type"), "" | "NONE"))
        {
            return Ok(None);
        }
        let items = Self::items(source);
        let mut last_object = None;
        for item in &items {
            match &item.kind {
                ItemKind::Char(c) if !matches!(c, '\r' | '\n') => {}
                ItemKind::Object(n)
                    if matches!(n.name.as_str(), "group" | "rect" | "pic" | "equation")
                        && n.child("pos").is_some_and(|p| p.a("treatAsChar") == "1")
                        && n.find("footNote").is_none()
                        && n.find("endNote").is_none() =>
                {
                    last_object = Some(item.pos);
                }
                _ => return Ok(None),
            }
        }
        let Some(last_object) = last_object else {
            return Ok(None);
        };
        let settings = crate::reflow::settings(style);
        let units = self.paragraph_units(source)?;
        let measured = units.iter().map(|u| u.width).sum::<f64>();
        if measured <= width - settings.left - settings.right - settings.indent + 0.1 {
            return Ok(None);
        }
        let wrapped = crate::reflow::wrap(&units, width, &settings)?;
        // Preserve the existing object-bearing slot. Moving/splitting an object
        // needs a separate layout policy; this repair moves only its text tail.
        if wrapped.len() < 2 || last_object >= wrapped[1].pos {
            return Ok(None);
        }
        let mut p = source.clone();
        p.attrs.insert("reflowed".into(), "1".into());
        let mut cache = vec![line.clone()];
        let mut y = line.n("vertpos") + line.n("vertsize") + line.n("spacing");
        for (i, part) in wrapped.iter().enumerate().skip(1) {
            let end = wrapped.get(i + 1).map(|l| l.pos).unwrap_or(usize::MAX);
            let em = units
                .iter()
                .filter(|u| u.pos >= part.pos && u.pos < end)
                .map(|u| u.em)
                .fold(0., f64::max);
            if em <= 0. {
                return Ok(None);
            }
            let start = units
                .iter()
                .find(|u| u.pos >= part.pos && u.pos < end && u.ch != Some(' '))
                .map(|u| u.pos)
                .unwrap_or(part.pos);
            let mut l = line.clone();
            for (key, value) in [
                ("textpos", start.to_string()),
                ("vertpos", y.to_string()),
                ("vertsize", (em * 100.).to_string()),
                ("textheight", (em * 100.).to_string()),
                ("baseline", (em * 85.).to_string()),
                ("spacing", ((part.height - em) * 100.).to_string()),
                ("horzpos", (part.left * 100.).to_string()),
                ("horzsize", (part.width * 100.).to_string()),
            ] {
                l.attrs.insert(key.into(), value);
            }
            cache.push(l);
            y += part.height * 100.;
        }
        node_mut(&mut p, "linesegarray").children = cache;
        self.report.warnings.insert(format!(
            "Rewrapped overflowing native inline-object text tail in paragraph {}",
            source.a("id")
        ));
        Ok(Some(p))
    }
}
