//! Expand empty TOC placeholders from outline headings with fixed-width page fields.
use super::*;
struct Heading {
    name: String,
    title: String,
    level: usize,
    section: usize,
    paragraph: usize,
}
fn node(name: &str) -> Node {
    Node {
        name: name.into(),
        ..Default::default()
    }
}
fn attr(mut n: Node, key: &str, value: impl ToString) -> Node {
    n.attrs.insert(key.into(), value.to_string());
    n
}
impl<'a> Engine<'a> {
    fn visible_text(p: &Node) -> String {
        Self::items(p)
            .iter()
            .filter_map(|i| {
                if let ItemKind::Char(c) = i.kind {
                    Some(c)
                } else {
                    None
                }
            })
            .collect()
    }
    fn toc_field(p: &Node) -> Option<Node> {
        Self::items(p).iter().find_map(|i| {
            if let ItemKind::Control(n) = &i.kind
                && n.name == "fieldBegin"
                && n.a("type") == "TABLEOFCONTENTS"
            {
                Some(n.clone())
            } else {
                None
            }
        })
    }
    pub(super) fn prepare_field_sections(&mut self) -> Result<Option<Vec<Node>>> {
        let mut headings = Vec::new();
        let mut has_toc = false;
        let mut used = BTreeSet::new();
        for section in self.doc.sections.iter().chain(&self.doc.masters) {
            let mut nodes = Vec::new();
            section.all("bookmark", &mut nodes);
            section.all("fieldBegin", &mut nodes);
            for n in nodes {
                if n.name == "bookmark" || n.a("type") == "BOOKMARK" {
                    used.insert(super::field_values::bookmark_name(n));
                }
            }
        }
        for (s, section) in self.doc.sections.iter().enumerate() {
            for (pi, p) in section.children_named("p").enumerate() {
                if Self::toc_field(p).is_some() {
                    has_toc = true;
                    continue;
                }
                if let Some(h) = self
                    .doc
                    .paras
                    .get(p.a("paraPrIDRef"))
                    .and_then(|s| s.child("heading"))
                    .filter(|h| h.a("type") == "OUTLINE")
                {
                    let title = Self::visible_text(p);
                    if !title.trim().is_empty() {
                        let mut name = format!("heading-{s}-{pi}");
                        while used.contains(&name) {
                            name.push('_');
                        }
                        used.insert(name.clone());
                        headings.push(Heading {
                            name,
                            title,
                            level: h.u("level").min(9),
                            section: s,
                            paragraph: pi,
                        });
                    }
                }
            }
        }
        if headings.is_empty() && !has_toc {
            return Ok(None);
        }
        anyhow::ensure!(headings.len() <= 10000, "Too many TOC headings");
        let mut sections = self.doc.sections.clone();
        let mut generated = 0;
        for (s, section) in sections.iter_mut().enumerate() {
            let mut replacement = Vec::new();
            let mut pi = 0;
            for mut p in std::mem::take(&mut section.children) {
                if p.name != "p" {
                    replacement.push(p);
                    continue;
                }
                if let Some(h) = headings
                    .iter()
                    .find(|h| h.section == s && h.paragraph == pi)
                {
                    p.attrs.insert("navigationName".into(), h.name.clone());
                    p.attrs.insert("navigationTitle".into(), h.title.clone());
                    p.attrs
                        .insert("navigationLevel".into(), h.level.to_string());
                }
                pi += 1;
                let Some(field) = Self::toc_field(&p) else {
                    replacement.push(p);
                    continue;
                };
                let items = Self::items(&p);
                let begin=items.iter().position(|i|matches!(&i.kind,ItemKind::Control(n) if n.name=="fieldBegin"&&n.a("type")=="TABLEOFCONTENTS")).unwrap();
                let end=items.iter().enumerate().skip(begin+1).find(|(_,i)|matches!(&i.kind,ItemKind::Control(n) if n.name=="fieldEnd" && (n.a("beginIDRef").is_empty()||n.a("beginIDRef")==field.a("id")))).map(|(i,_)|i);
                let Some(end) = end else {
                    self.report.warnings.insert("Cross-paragraph TOC field retained; automatic regeneration requires a closed empty placeholder".into());
                    replacement.push(p);
                    continue;
                };
                if items[begin + 1..end].iter().any(|i| {
                    matches!(i.kind,ItemKind::Char(c) if !c.is_whitespace())
                        || matches!(i.kind, ItemKind::Object(_))
                }) {
                    replacement.push(p);
                    continue;
                }
                if let Some(list) = field
                    .child("subList")
                    .filter(|l| l.children_named("p").next().is_some())
                {
                    replacement.push(p);
                    for entry in list.children_named("p") {
                        let mut entry = entry.clone();
                        entry.children.retain(|c| c.name != "linesegarray");
                        replacement.push(entry);
                    }
                    self.report
                        .warnings
                        .insert("Recovered cached TOC subList as paragraph flow".into());
                    continue;
                }
                if headings.is_empty() {
                    self.report
                        .warnings
                        .insert("Empty TOC has no outline headings to populate".into());
                    replacement.push(p);
                    continue;
                }
                let mut style = p
                    .children_named("run")
                    .next()
                    .map(|r| r.a("charPrIDRef"))
                    .unwrap_or("0")
                    .to_string();
                if !self.doc.chars.contains_key(&style) {
                    style = self.doc.chars.keys().next().cloned().unwrap_or_default();
                }
                let empty = Self::visible_text(&p).trim().is_empty();
                let template = p.clone();
                if !empty {
                    replacement.push(p.clone());
                }
                for (i, h) in headings.iter().enumerate() {
                    generated += 1;
                    anyhow::ensure!(generated <= 10000, "TOC expansion exceeds 10000 entries");
                    let mut entry = if empty && i == 0 {
                        p.clone()
                    } else {
                        attr(
                            attr(node("p"), "id", format!("toc-{s}-{pi}-{i}")),
                            "paraPrIDRef",
                            template.a("paraPrIDRef"),
                        )
                    };
                    entry.children.retain(|c| c.name != "linesegarray");
                    entry.attrs.insert("tocEntry".into(), h.name.clone());
                    entry.attrs.insert("tocLevel".into(), h.level.to_string());
                    entry.attrs.remove("navigationName");
                    let mut run = attr(node("run"), "charPrIDRef", &style);
                    let mut t = node("t");
                    t.children.push(Node {
                        name: "#text".into(),
                        text: h.title.clone(),
                        ..Default::default()
                    });
                    t.children.push(attr(node("tab"), "width", 0));
                    run.children.push(t);
                    let mut ctrl = node("ctrl");
                    ctrl.children.push(attr(
                        attr(node("pageField"), "kind", "PAGEREF"),
                        "target",
                        &h.name,
                    ));
                    run.children.push(ctrl);
                    entry.children.push(run);
                    replacement.push(entry);
                }
                self.report.warnings.insert("Empty TOC rebuilt from outline headings; custom TOC styles and filters are not evaluated".into());
            }
            section.children = replacement;
        }
        Ok(Some(sections))
    }
}
