//! Independent native HWP print: keep OLE data and rewrap overflowing text.
use hwp_engine::{
    layout::Engine,
    model::{Document, Node},
    pdf::Op,
    qa,
};
use std::path::Path;
fn fonts() -> &'static Path {
    Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn document() -> Option<Document> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-95-Checkpoint-20261019/01_print/25.hwp");
    p.exists().then(|| Document::open(&p).unwrap())
}
#[test]
fn overflowing_inline_groups_move_only_the_text_tail_and_repaginate_following_body() {
    let Some(d) = document() else {
        return;
    };
    let original = d.sections.clone();
    let mut equations = Vec::new();
    for s in &d.sections {
        s.all("equation", &mut equations);
    }
    assert_eq!(
        equations
            .iter()
            .filter(|n| n.a("oleEquation") == "1")
            .count(),
        2
    );
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 2);
    assert_eq!(e.report.equations, 3);
    assert!(e.pages[0].ops.iter().any(|o| matches!(o,
        Op::Text {text,x,y,..} if text == "B" && (*x - 91.69).abs() < 0.1 && *y > 310. && *y < 320.
    )), "leading spaces must retain the reference's formula origin");
    let fraction_rule = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "hwp-d9d" && b.kind == "line")
        .unwrap();
    assert!((fraction_rule.w - 40.96).abs() < 0.01);
    assert!(fraction_rule.h.abs() < 0.01);
    assert!((fraction_rule.y - 285.885).abs() < 0.1);
    assert!(e.pages[0].ops.iter().any(|op| matches!(op,
        Op::Line { x, y, x2, y2, width, .. }
        if (*x - fraction_rule.x).abs() < 0.01 && (*y - fraction_rule.y).abs() < 0.01
            && (*x2 - *x - 40.96).abs() < 0.01 && (*y2 - *y).abs() < 0.01 && *width > 0.
    )));
    let bounds = |id: &str| {
        e.report
            .bounds
            .iter()
            .filter(|b| b.kind == "line" && b.id == id)
            .collect::<Vec<_>>()
    };
    let formula = bounds("hwp-6cb");
    assert_eq!(formula.len(), 2);
    assert!((formula[1].y - 326.14).abs() < 0.1);
    assert!((formula[1].x - 56.69).abs() < 0.1);
    assert!((bounds("hwp-16d7")[0].y - 355.88).abs() < 0.1);
    let continued = bounds("hwp-28db");
    assert_eq!(
        continued.iter().map(|b| b.page).collect::<Vec<_>>(),
        vec![1, 2]
    );
    let heading = bounds("hwp-2a53")[0];
    assert_eq!(heading.page, 2);
    assert!((heading.y - 105.19).abs() < 0.1);
    let text = e
        .pages
        .iter()
        .flat_map(|p| &p.ops)
        .filter_map(|op| match op {
            Op::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    // The viewer print omits this valid OLE expression; never copy that omission.
    assert!(compact.contains("월간일평균유입량") && compact.contains("시설용량"));
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(
        q.outside_page.is_empty() && q.text_overlaps.is_empty() && q.border_crossings.is_empty()
    );
    assert_eq!(d.sections, original);
}
#[test]
fn fitting_inline_objects_do_not_trigger_tail_recovery() {
    let Some(mut d) = document() else {
        return;
    };
    fn narrow(n: &mut Node) {
        if n.name == "group"
            && let Some(sz) = n.children.iter_mut().find(|n| n.name == "sz")
        {
            sz.attrs
                .insert("width".into(), (sz.n("width") - 1000.).to_string());
        }
        for c in &mut n.children {
            narrow(c);
        }
    }
    for p in d
        .sections
        .iter_mut()
        .flat_map(|s| &mut s.children)
        .filter(|p| p.a("id") == "hwp-6cb")
    {
        narrow(p);
    }
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert!(
        !e.report
            .warnings
            .iter()
            .any(|w| w.starts_with("Rewrapped overflowing native inline-object"))
    );
}
