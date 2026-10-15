//! Logical field scopes and invisible PDF navigation operations.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn line_field_targets(
        &mut self,
        p: &Node,
        items: &[Item],
        x: f64,
        y: f64,
        anchor_y: f64,
        first: bool,
    ) -> Vec<Option<String>> {
        if first
            && self.depth == 0
            && !self.note_rendering
            && !p.a("navigationName").is_empty()
            && p.a("tocEntry").is_empty()
        {
            self.current.ops.push(Op::Destination {
                name: p.a("navigationName").into(),
                title: p.a("navigationTitle").into(),
                level: p.u("navigationLevel"),
                x,
                y,
                anchor_y,
                page_base: self.page_base,
            });
        }
        let mut targets = Vec::new();
        for item in items {
            if let ItemKind::Control(n) = &item.kind {
                if n.a("metadataIncomplete") == "1" {
                    self.report.warnings.insert(
                        "A field/bookmark has incomplete metadata; its cached display is retained"
                            .into(),
                    );
                }
                match n.name.as_str() {
                    "fieldBegin" => {
                        self.field_stack.push((
                            if n.a("id").is_empty() { "0" } else { n.a("id") }.into(),
                            super::field_values::link_target(n),
                        ));
                        if n.a("type") == "BOOKMARK" {
                            self.named_destination(n, x, y, anchor_y);
                        }
                    }
                    "pageField" if n.a("originalField") == "1" => {
                        self.field_stack.push((
                            n.a("id").into(),
                            (!n.a("target").is_empty()).then(|| format!("#{}", n.a("target"))),
                        ));
                    }
                    "fieldEnd" => {
                        if !n.a("beginIDRef").is_empty() {
                            if let Some(i) = self
                                .field_stack
                                .iter()
                                .rposition(|(id, _)| id == n.a("beginIDRef"))
                            {
                                self.field_stack.truncate(i);
                            }
                        } else {
                            self.field_stack.pop();
                        }
                    }
                    "bookmark" => self.named_destination(n, x, y, anchor_y),
                    _ => {}
                }
            }
            targets.push(if !p.a("tocEntry").is_empty() {
                Some(format!("#{}", p.a("tocEntry")))
            } else {
                self.field_stack
                    .iter()
                    .rev()
                    .find_map(|(_, target)| target.clone())
            });
        }
        targets
    }
    fn named_destination(&mut self, n: &Node, x: f64, y: f64, anchor_y: f64) {
        let name = super::field_values::bookmark_name(n);
        if !name.is_empty() {
            self.current.ops.push(Op::Destination {
                title: name.clone(),
                name,
                level: usize::MAX,
                x,
                y,
                anchor_y,
                page_base: self.page_base,
            });
        }
    }
    pub(super) fn link_ink(
        &mut self,
        target: &Option<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        anchor_y: f64,
    ) {
        if let Some(target) = target
            && width > 0.01
            && height > 0.01
        {
            self.current.ops.push(Op::Link {
                target: target.clone(),
                rect: [x, y, x + width, y + height],
                anchor_y,
            });
        }
    }
}
