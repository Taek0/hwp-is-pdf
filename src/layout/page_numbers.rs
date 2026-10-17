//! Generated page numbers use the document's named character style.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn styled_page_number(
        &mut self,
        number: &Node,
        page: usize,
        label: &str,
    ) -> Result<bool> {
        let mut styles = Vec::new();
        self.doc.header.all("style", &mut styles);
        let style = if self.doc.chars.contains_key(number.a("charPrIDRef")) {
            Some(number.a("charPrIDRef").to_string())
        } else {
            styles
                .into_iter()
                .find(|s| {
                    s.a("engName").eq_ignore_ascii_case("Page Number")
                        || s.a("name")
                            .chars()
                            .filter(|c| !c.is_whitespace())
                            .collect::<String>()
                            == "쪽번호"
                })
                .map(|s| s.a("charPrIDRef").to_string())
                .filter(|id| self.doc.chars.contains_key(id))
        };
        let Some(style) = style else {
            return Ok(false);
        };
        let g = self.glyph('0', &style)?;
        let face = ttf_parser::Face::parse(&self.fonts.list[g.font].bytes, 0)?;
        let unit = g.size / f64::from(face.units_per_em());
        let ascent = (f64::from(face.ascender()) * unit).max(g.raw_ascent);
        let descent = (-f64::from(face.descender()) * unit)
            .max(g.raw_descent)
            .max(0.);
        let mut p = Node {
            name: "p".into(),
            attrs: BTreeMap::from([
                ("id".into(), "generated-page-number".into()),
                ("paraPrIDRef".into(), "__page_number_style".into()),
                ("reflowed".into(), "1".into()),
            ]),
            ..Default::default()
        };
        p.children.push(Node {
            name: "run".into(),
            attrs: BTreeMap::from([("charPrIDRef".into(), style)]),
            children: vec![Node {
                name: "t".into(),
                children: vec![Node {
                    name: "#text".into(),
                    text: label.into(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        });
        let units = self.paragraph_units(&p)?;
        let width = units.iter().map(|u| u.width).sum::<f64>()
            - units.last().map(|u| u.trailing).unwrap_or(0.);
        let pos = number.u("position");
        let right = match pos {
            3 | 6 => true,
            7 | 8 => page % 2 == 1,
            9 | 10 => page.is_multiple_of(2),
            _ => false,
        };
        let x = if pos == 2 || pos == 5 {
            (self.pw - width) / 2.
        } else if right {
            self.pw - self.right - width
        } else {
            self.left
        };
        let top = if matches!(pos, 1 | 2 | 3 | 7 | 9) {
            self.top - self.header
        } else {
            self.ph - self.bottom + self.footer - ascent - descent
        };
        p.children.push(Node {
            name: "linesegarray".into(),
            children: vec![Node {
                name: "lineseg".into(),
                attrs: BTreeMap::from([
                    ("textpos".into(), "0".into()),
                    ("vertpos".into(), "0".into()),
                    ("horzpos".into(), "0".into()),
                    ("horzsize".into(), (width * 100.).to_string()),
                    ("vertsize".into(), ((ascent + descent) * 100.).to_string()),
                    ("baseline".into(), (ascent * 100.).to_string()),
                ]),
                ..Default::default()
            }],
            ..Default::default()
        });
        let fields = std::mem::take(&mut self.field_stack);
        let result = self.line(&p, 0, x, top, width);
        self.field_stack = fields;
        result?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_page_number_matches_reference_style_and_does_not_inherit_body_links() {
        let d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
        )
        .unwrap();
        let mut e = Engine::new(
            &d,
            Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        e.pw = 595.28;
        e.ph = 841.88;
        e.top = 85.02;
        e.bottom = 85.02;
        e.header = 28.35;
        e.footer = 42.51;
        let fields = vec![("body-link".into(), Some("https://www.hancom.com".into()))];
        e.field_stack = fields.clone();
        let n = crate::model::xml(br#"<pageNumPos position="5" sideChar="-"/>"#).unwrap();
        assert!(e.styled_page_number(&n, 3, "- 3 -").unwrap());
        assert_eq!(e.field_stack, fields);
        assert!(!e.current.ops.iter().any(|o| matches!(o, Op::Link { .. })));
        let chars: Vec<_> = e
            .current
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Text {
                    text, x, y, font, ..
                } => Some((text.as_str(), *x, *y, *font)),
                _ => None,
            })
            .collect();
        assert_eq!(chars.iter().map(|c| c.0).collect::<String>(), "- 3 -");
        assert!((chars[0].1 - 285.12).abs() < 0.2);
        assert!((chars[0].2 - 797.16).abs() < 0.2);
        assert!((chars[2].1 - chars[1].1 - 5.).abs() < 0.01);
        assert!(
            e.fonts.list[chars[0].3]
                .name
                .to_lowercase()
                .contains("handotum")
        );
    }
}
