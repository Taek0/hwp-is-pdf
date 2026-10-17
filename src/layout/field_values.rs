//! Whitelisted empty-field values; cached display text remains authoritative.
use super::*;
pub(super) fn command(n: &Node) -> String {
    if !n.a("command").is_empty() {
        return n.a("command").trim_end_matches('\0').into();
    }
    let mut params = Vec::new();
    n.all("stringParam", &mut params);
    params
        .into_iter()
        .find(|p| {
            matches!(
                p.a("name").to_ascii_lowercase().as_str(),
                "command" | "url" | "href"
            )
        })
        .map(text)
        .unwrap_or_default()
}
pub(super) fn bookmark_name(n: &Node) -> String {
    if !n.a("name").is_empty() {
        n.a("name").into()
    } else {
        command(n).split(';').next().unwrap_or("").trim().into()
    }
}
pub(super) fn link_target(n: &Node) -> Option<String> {
    if n.a("type") != "HYPERLINK" && n.name != "hyperlink" {
        return None;
    }
    let value = command(n);
    let value = value.trim().trim_matches('"');
    let native = n.a("sourceControl") == "%hlk";
    // Native field commands escape URL delimiters, including the scheme colon.
    // Consume the first unescaped separator; preserve the literal URL content.
    if native {
        let mut chars = value.chars().peekable();
        let mut url = String::new();
        while let Some(c) = chars.next() {
            if c == ';' {
                break;
            }
            if c == '\\' && chars.peek().is_some_and(|c| matches!(c, ':' | ';' | '\\')) {
                url.push(chars.next().unwrap());
            } else {
                url.push(c);
            }
        }
        return (!url.is_empty()).then_some(url);
    }
    let parts: Vec<_> = value.split(';').collect();
    let value = if parts.len() > 3
        && parts[1..]
            .iter()
            .all(|s| s.chars().all(|c| c.is_ascii_digit()))
    {
        parts[0]
    } else {
        value
    };
    (!value.is_empty()).then(|| value.to_string())
}
fn dynamic(n: &Node) -> Option<(&'static str, String)> {
    if !matches!(n.a("type"), "CROSSREF" | "SUMMERY" | "SUMMARY") {
        return None;
    }
    let value = command(n);
    let value = value.trim();
    let upper = value.to_ascii_uppercase();
    match upper.as_str() {
        "PAGE" | "PAGENUMBER" => Some(("PAGE", String::new())),
        "NUMPAGES" | "PAGECOUNT" | "TOTALPAGES" => Some(("NUMPAGES", String::new())),
        _ => upper.strip_prefix("PAGEREF ").map(|_| {
            (
                "PAGEREF",
                value[8..].trim().trim_matches(['"', '{', '}']).to_string(),
            )
        }),
    }
}
impl<'a> Engine<'a> {
    pub(super) fn normalize_field_items(items: &mut [Item]) {
        let mut replace = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let ItemKind::Control(n) = &item.kind else {
                continue;
            };
            if n.name != "fieldBegin" {
                continue;
            }
            let Some((kind, target)) = dynamic(n) else {
                continue;
            };
            let mut end = false;
            let mut populated = false;
            for after in &items[i + 1..] {
                match &after.kind {
                    ItemKind::Control(c) if c.name == "fieldEnd" => {
                        end = true;
                        break;
                    }
                    ItemKind::Char(c) if !c.is_whitespace() => populated = true,
                    ItemKind::Object(_) => populated = true,
                    ItemKind::Control(c) if !matches!(c.name.as_str(), "bookmark") => {
                        populated = true
                    }
                    _ => {}
                }
            }
            if end && !populated {
                let mut field = n.clone();
                field.name = "pageField".into();
                field.attrs.insert("kind".into(), kind.into());
                field.attrs.insert("target".into(), target);
                field.attrs.insert("originalField".into(), "1".into());
                replace.push((i, field));
            }
        }
        for (i, n) in replace {
            items[i].kind = ItemKind::Control(n);
        }
    }
    pub(super) fn paint_page_field(&mut self, n: &Node, g: &Glyph, x: f64, y: f64, width: f64) {
        self.current.ops.push(Op::FieldText {
            kind: n.a("kind").into(),
            target: n.a("target").into(),
            x,
            y: y + g.shift,
            width,
            font: g.font,
            size: g.size,
            scale: g.scale,
            color: g.color,
            page_base: self.page_base,
        });
    }
    pub(super) fn resolve_page_fields(&mut self) {
        let mut destinations = BTreeMap::new();
        for (page, p) in self.pages.iter().enumerate() {
            for op in &p.ops {
                if let Op::Destination {
                    name, page_base, ..
                } = op
                {
                    destinations
                        .entry(name.clone())
                        .or_insert((page as i64 + page_base).max(1).to_string());
                }
            }
        }
        let count = self.pages.len();
        let mut warnings = BTreeSet::new();
        for (page, p) in self.pages.iter_mut().enumerate() {
            for op in &mut p.ops {
                if let Op::FieldText {
                    kind,
                    target,
                    x,
                    y,
                    width,
                    font,
                    size,
                    scale,
                    color,
                    page_base,
                } = op
                {
                    let value = match kind.as_str() {
                        "PAGE" => (page as i64 + *page_base).max(1).to_string(),
                        "NUMPAGES" => count.to_string(),
                        _ => destinations
                            .get(target.trim_start_matches('#'))
                            .cloned()
                            .unwrap_or_else(|| {
                                warnings.insert(format!("Unresolved page reference: {target}"));
                                "?".into()
                            }),
                    };
                    let measured = self.fonts.width(*font, &value, *size) * *scale;
                    let fit = (width.max(0.1) / measured.max(0.1)).min(1.);
                    let xx = *x + (*width - measured * fit).max(0.);
                    *op = Op::Text {
                        x: xx,
                        y: *y,
                        text: value,
                        font: *font,
                        size: *size,
                        scale: *scale * fit,
                        color: *color,
                    };
                }
                if let Op::Link { target, .. } = op {
                    let lower = target.to_ascii_lowercase();
                    if !lower.starts_with("https://")
                        && !lower.starts_with("http://")
                        && !lower.starts_with("mailto:")
                        && !destinations.contains_key(target.trim_start_matches('#'))
                    {
                        warnings.insert(format!(
                            "Unresolved or unsupported PDF link target: {target}"
                        ));
                    }
                }
            }
        }
        self.report.warnings.extend(warnings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_hyperlink_escapes_are_decoded_without_rewriting_the_url() {
        let mut n = Node {
            name: "fieldBegin".into(),
            ..Default::default()
        };
        n.attrs.insert("type".into(), "HYPERLINK".into());
        n.attrs.insert("sourceControl".into(), "%hlk".into());
        n.attrs
            .insert("command".into(), r"http\://example.org/a\;b);1;0;0;".into());
        assert_eq!(link_target(&n).as_deref(), Some("http://example.org/a;b)"));
        n.attrs.insert(
            "command".into(),
            r"https\://example.org/a\\b\q;1;0;0;".into(),
        );
        assert_eq!(
            link_target(&n).as_deref(),
            Some(r"https://example.org/a\b\q")
        );
        n.attrs.remove("sourceControl");
        n.attrs
            .insert("command".into(), "https://example.org/a;b?x=1".into());
        assert_eq!(
            link_target(&n).as_deref(),
            Some("https://example.org/a;b?x=1")
        );
    }
}
