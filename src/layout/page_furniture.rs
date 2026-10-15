//! Recover missing nested caches and measure repeated page furniture separately.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn missing_nested_cache(n: &Node) -> bool {
        if matches!(n.name.as_str(), "header" | "footer") {
            return false;
        }
        if n.name == "p"
            && lines(n).is_empty()
            && Self::items(n).iter().any(|i| match &i.kind {
                ItemKind::Char(c) => !c.is_whitespace(),
                ItemKind::Object(_) => true,
                ItemKind::Control(c) => {
                    matches!(c.name.as_str(), "footNote" | "endNote" | "autoNum")
                }
                _ => false,
            })
        {
            return true;
        }
        n.children.iter().any(Self::missing_nested_cache)
    }
    pub(super) fn section_needs_reflow(section: &Node) -> bool {
        section.children_named("p").any(|p| lines(p).is_empty())
            || Self::missing_nested_cache(section)
    }
    pub(super) fn prepare_furniture(&mut self, section: &Node, force: bool) -> Result<()> {
        self.furniture.clear();
        let definitions: Vec<_> = section
            .children_named("p")
            .flat_map(|p| p.children_named("run"))
            .flat_map(|r| r.children_named("ctrl"))
            .flat_map(|c| &c.children)
            .filter(|n| matches!(n.name.as_str(), "header" | "footer"))
            .cloned()
            .collect();
        let numbering = std::mem::take(&mut self.numbering);
        let labels = std::mem::take(&mut self.labels);
        let wrap = self.general_wrap;
        self.general_wrap = false;
        let result = (|| -> Result<()> {
            let mut header = self.header;
            let mut footer = self.footer;
            for (mut control, index) in definitions.into_iter().zip(0usize..) {
                let kind = control.name.clone();
                if let Some(list) = control.children.iter_mut().find(|n| n.name == "subList")
                    && (force || Self::missing_nested_cache(list))
                {
                    anyhow::ensure!(
                        list.find("footNote").is_none() && list.find("endNote").is_none(),
                        "Notes inside reflow page furniture are not supported"
                    );
                    fn qualify(n: &mut Node, index: usize) {
                        if n.name == "p" {
                            n.attrs
                                .insert("id".into(), format!("furniture-{index}-{}", n.a("id")));
                        }
                        for c in &mut n.children {
                            qualify(c, index);
                        }
                    }
                    qualify(list, index);
                    let h = self.reflow_list(list, self.pw - self.left - self.right, 1)?;
                    if kind == "header" {
                        header = header.max(h);
                    } else {
                        footer = footer.max(h);
                    }
                    self.report.warnings.insert(
                        "Rebuilt header/footer line caches and reserved their measured height"
                            .into(),
                    );
                }
                self.furniture.push(control);
            }
            self.top += header - self.header;
            self.bottom += footer - self.footer;
            self.header = header;
            self.footer = footer;
            anyhow::ensure!(
                self.ph - self.top - self.bottom > 24.,
                "Page furniture leaves no printable body area"
            );
            Ok(())
        })();
        self.numbering = numbering;
        self.labels = labels;
        self.general_wrap = wrap;
        result
    }
    pub(super) fn paint_furniture(
        &mut self,
        section: &Node,
        first: bool,
        hiding: Option<&Node>,
    ) -> Result<()> {
        let sec = section.find("secPr").context("Section properties")?;
        let odd = (self.pages.len() + 1) % 2 == 1;
        let mut selected = Vec::new();
        for kind in ["header", "footer"] {
            let hide = if kind == "header" {
                "hideHeader"
            } else {
                "hideFooter"
            };
            let hide_first = if kind == "header" {
                "hideFirstHeader"
            } else {
                "hideFirstFooter"
            };
            if hiding.is_some_and(|h| h.a(hide) == "1") || (first && sec.a(hide_first) == "1") {
                continue;
            }
            if let Some(c) = self.furniture.iter().rev().find(|c| {
                c.name == kind
                    && match c.a("applyPageType") {
                        "ODD" => odd,
                        "EVEN" => !odd,
                        _ => true,
                    }
            }) {
                selected.push(c.clone());
            }
        }
        let fields = std::mem::take(&mut self.field_stack);
        let numbering = std::mem::take(&mut self.numbering);
        let labels = std::mem::take(&mut self.labels);
        let result = (|| -> Result<()> {
            for c in selected {
                if let Some(list) = c.child("subList") {
                    let y = if c.name == "header" {
                        self.top - self.header
                    } else {
                        // A tall saved footer grows into the lower margin. A
                        // negative bottom-alignment offset pulls it into the
                        // body and through the page border instead.
                        self.ph - self.bottom + (self.footer - content_height(list)).max(0.)
                    };
                    self.local(list, self.left, y, self.pw - self.left - self.right)?;
                }
            }
            Ok(())
        })();
        self.numbering = numbering;
        self.labels = labels;
        self.field_stack = fields;
        result
    }
}
