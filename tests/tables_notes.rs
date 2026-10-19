use hwp_engine::{
    layout::Engine,
    model::{Document, text},
    pdf::Op,
    qa,
};
use std::{collections::BTreeMap, path::PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn fonts() -> PathBuf {
    if cfg!(windows) {
        "C:/Windows/Fonts".into()
    } else {
        "/mnt/c/Windows/Fonts".into()
    }
}
fn counts(s: &str) -> BTreeMap<char, usize> {
    let mut m = BTreeMap::new();
    for c in s.chars().filter(|c| !c.is_whitespace()) {
        *m.entry(c).or_default() += 1;
    }
    m
}
#[test]
fn reserved_underline_value_does_not_decorate_table_or_footnote_text() {
    let d = Document::open(
        &root().join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
    )
    .unwrap();
    // Footnote 20 uses shape 6. These shapes contain reserved type 2,
    // whereas shape 16 explicitly requests a real bottom underline.
    for id in ["6", "10", "93"] {
        assert!(d.chars[id].child("underline").is_none(), "shape {id}");
    }
    assert_eq!(
        d.chars["16"].child("underline").unwrap().a("type"),
        "BOTTOM"
    );
}
#[test]
fn public_tables_and_notes_keep_text_and_avoid_collisions() {
    for name in [
        "endnote-01",
        "footnote-01",
        "hwpers_test4_complex_table",
        "inner-table-01",
        "issue1937_rowbreak_footnote_overpagination",
        "table-complex",
    ] {
        let d = Document::open(&root().join(format!("samples/tables-notes/{name}.hwp"))).unwrap();
        let source = d.sections.iter().map(text).collect::<String>();
        let mut e = Engine::new(&d, &fonts());
        e.render().unwrap();
        if name == "issue1937_rowbreak_footnote_overpagination" {
            // Match the user-exported Hancom reference without losing source text.
            assert_eq!(e.pages.len(), 50);
            let tail = e
                .report
                .bounds
                .iter()
                .filter(|b| b.kind == "tbl" && b.id == "hwp-44c83")
                .map(|b| b.page)
                .max()
                .unwrap();
            for number in 18..=39 {
                let notes: Vec<_> = e
                    .report
                    .bounds
                    .iter()
                    .filter(|b| b.kind.starts_with("footnote") && b.id == number.to_string())
                    .collect();
                assert!(!notes.is_empty(), "missing table note {number}");
                assert!(
                    notes.iter().all(|b| b.page >= tail),
                    "note {number} precedes table end"
                );
            }
        }
        if name == "inner-table-01" {
            assert_eq!(
                e.pages.len(),
                2,
                "The supplied Hancom print continues the nested table across two pages"
            );
        }
        let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(
            q.outside_page.is_empty(),
            "{name}: outside {:?}",
            q.outside_page
        );
        assert!(
            q.text_overlaps.is_empty(),
            "{name}: overlap {:?}",
            q.text_overlaps
        );
        assert!(
            q.border_crossings.is_empty(),
            "{name}: borders {:?}",
            q.border_crossings
        );
        assert!(
            q.foreground_image_overlaps.is_empty(),
            "{name}: image overlap"
        );
        let output = e
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|o| {
                if let Op::Text { text, .. } = o {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<String>();
        let actual = counts(&output);
        for (c, n) in counts(&source) {
            assert!(
                actual.get(&c).copied().unwrap_or(0) >= n,
                "{name}: lost {c:?}, expected {n}, actual {:?}",
                actual.get(&c)
            );
        }
    }
}
#[test]
fn notes_in_tables_are_collected_and_use_body_width() {
    for (file, kind) in [("footnote-01", "footnote"), ("endnote-01", "endnote")] {
        let d = Document::open(&root().join(format!("samples/tables-notes/{file}.hwp"))).unwrap();
        let mut e = Engine::new(&d, &fonts());
        e.render().unwrap();
        let notes: Vec<_> = e.report.bounds.iter().filter(|b| b.kind == kind).collect();
        let mut source_notes = Vec::new();
        for s in &d.sections {
            s.all(
                if kind == "footnote" {
                    "footNote"
                } else {
                    "endNote"
                },
                &mut source_notes,
            );
        }
        assert_eq!(notes.len(), source_notes.len(), "{file}");
        assert!(notes.iter().all(|b| b.w > 300.));
        let labels: Vec<_> = e
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|o| {
                if let Op::Text { text, .. } = o {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        for n in source_notes.iter().map(|n| n.u("number")) {
            assert!(
                labels.contains(&format!("{n})").as_str()),
                "missing note label {file}:{n}"
            );
        }
    }
}
#[test]
fn end_of_section_notes_do_not_leak_to_document_end() {
    fn change(n: &mut hwp_engine::model::Node) {
        if n.name == "endNotePr" {
            n.children
                .iter_mut()
                .find(|c| c.name == "placement")
                .unwrap()
                .attrs
                .insert("place".into(), "END_OF_SECTION".into());
        }
        for c in &mut n.children {
            change(c)
        }
    }
    let mut d = Document::open(&root().join("samples/tables-notes/endnote-01.hwp")).unwrap();
    for s in &mut d.sections {
        change(s)
    }
    let second = d.sections.clone();
    d.sections.extend(second);
    let mut e = Engine::new(&d, &fonts());
    e.render().unwrap();
    let notes: Vec<_> = e
        .report
        .bounds
        .iter()
        .filter(|b| b.kind == "endnote")
        .collect();
    assert_eq!(notes.len(), 12);
    assert!(notes[0].page < notes[6].page);
}
