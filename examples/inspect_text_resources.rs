//! Read-only diagnostics for legacy private-use glyphs and field commands.
use hwp_engine::model::{Document, Node, text};
use std::collections::BTreeSet;
fn walk(n: &Node, d: &Document, out: &mut BTreeSet<String>) {
    if n.name == "run" {
        let plain = n.children_named("t").map(text).collect::<String>();
        for c in plain
            .chars()
            .filter(|c| ('\u{f020}'..='\u{f0ff}').contains(c))
        {
            let style = d.chars.get(n.a("charPrIDRef"));
            let id = style
                .and_then(|s| s.child("fontRef"))
                .map(|f| f.a("user"))
                .unwrap_or("0");
            out.insert(format!(
                "U+{:04X} style={} face={:?} context={:?}",
                c as u32,
                n.a("charPrIDRef"),
                d.fonts.get(&("USER".into(), id.into())),
                plain
            ));
            if let Some(refs) = style.and_then(|s| s.child("fontRef")) {
                out.insert(format!(
                    "REFS {} {:?}",
                    n.a("charPrIDRef"),
                    refs.attrs
                        .iter()
                        .map(|(lang, id)| (lang, d.fonts.get(&(lang.to_uppercase(), id.clone()))))
                        .collect::<Vec<_>>()
                ));
            }
        }
    }
    if n.name == "fieldBegin" && n.a("type") == "HYPERLINK" {
        out.insert(format!("LINK {:?}", n.attrs));
    }
    for child in &n.children {
        walk(child, d, out);
    }
}
fn main() -> anyhow::Result<()> {
    let input = std::env::args().nth(1).expect("input document");
    let d = Document::open(std::path::Path::new(&input))?;
    let mut out = BTreeSet::new();
    for s in &d.sections {
        walk(s, &d, &mut out);
    }
    for line in out {
        println!("{line}");
    }
    let mut counts = std::collections::BTreeMap::new();
    for section in &d.sections {
        for c in text(section)
            .chars()
            .filter(|c| ('\u{f020}'..='\u{f0ff}').contains(c))
        {
            *counts.entry(format!("{:04X}", c as u32)).or_insert(0usize) += 1;
        }
    }
    println!("PUA_COUNTS {}", serde_json::to_string(&counts)?);
    Ok(())
}
