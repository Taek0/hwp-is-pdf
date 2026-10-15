use hwp_engine::{
    layout::Engine,
    model::{Document, text},
    pdf::Op,
    qa,
};
fn fonts() -> &'static std::path::Path {
    std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn open(name: &str) -> Document {
    Document::open(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("samples/reflow/{name}.hwpx")),
    )
    .unwrap()
}
fn check(e: &Engine) {
    let q = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.outside_page.is_empty(), "{:?}", q.outside_page);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
    assert!(q.border_crossings.is_empty(), "{:?}", q.border_crossings);
    assert!(
        q.foreground_image_overlaps.is_empty(),
        "{:?}",
        q.foreground_image_overlaps
    );
}
#[test]
fn offset_shapes_and_contours_preserve_text_notes_and_geometry() {
    for name in ["contour-single", "contour-columns"] {
        let d = open(name);
        let mut square = Engine::new(&d, fonts());
        square.render_reflow().unwrap();
        check(&square);
        let mut e = Engine::new(&d, fonts());
        e.render_contours().unwrap();
        check(&e);
        let all = e
            .pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();
        for i in 1..=6 {
            assert_eq!(all.matches(&format!("BODY-{i}:")).count(), 1);
            assert_eq!(all.matches(&format!("END-{i}.")).count(), 1);
        }
        assert_eq!(all.matches("CONTOUR-NOTE").count(), 1);
        assert_eq!(e.report.pictures, 1);
        let counts = |s: &str| {
            let mut m = std::collections::BTreeMap::new();
            for c in s.chars().filter(|c| !c.is_whitespace()) {
                *m.entry(c).or_insert(0usize) += 1;
            }
            m
        };
        let actual = counts(&all);
        for (c, n) in counts(&text(&d.sections[0])) {
            assert!(actual.get(&c).copied().unwrap_or(0) >= n, "lost {c}");
        }
        let shape = e
            .report
            .bounds
            .iter()
            .find(|b| b.id == "offset-ellipse")
            .unwrap();
        let note = e
            .report
            .bounds
            .iter()
            .find(|b| b.id == "contour-note-body" && b.kind == "line")
            .unwrap();
        assert_eq!(shape.page, note.page);
        assert_ne!(
            serde_json::to_value(&square.pages).unwrap(),
            serde_json::to_value(&e.pages).unwrap()
        );
        // At least one body line uses a corner inside the ellipse bounding box,
        // which rectangle-only exclusion would forbid.
        assert!(
            e.report.bounds.iter().any(|b| b.kind == "reflow-line"
                && b.page == shape.page
                && b.y >= shape.y
                && b.y < shape.y + shape.h
                && b.x < shape.x + shape.w
                && b.x + b.w > shape.x),
            "contour not used"
        );
    }
}
#[test]
fn later_paragraph_anchor_joins_active_region_group() {
    let mut d = open("contour-single");
    let placed = d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "placed")
        .unwrap();
    let run = placed
        .children
        .iter_mut()
        .find(|r| r.name == "run")
        .unwrap();
    let index = run
        .children
        .iter()
        .position(|n| n.a("id") == "offset-picture")
        .unwrap();
    let picture = run.children.remove(index);
    d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "body-1")
        .unwrap()
        .children
        .iter_mut()
        .find(|r| r.name == "run")
        .unwrap()
        .children
        .insert(0, picture);
    let mut e = Engine::new(&d, fonts());
    e.render_contours().unwrap();
    check(&e);
    let a = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "offset-ellipse")
        .unwrap();
    let b = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "offset-picture")
        .unwrap();
    assert_eq!(a.page, b.page);
    assert!(b.y > a.y);
    assert_eq!(e.report.pictures, 1);
}
#[test]
fn fully_blocked_band_moves_text_below_the_object() {
    let mut d = open("contour-single");
    let run = d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "placed")
        .unwrap()
        .children
        .iter_mut()
        .find(|r| r.name == "run")
        .unwrap();
    run.children
        .retain(|n| !matches!(n.name.as_str(), "ellipse" | "polygon" | "rect"));
    let picture = run.children.iter_mut().find(|n| n.name == "pic").unwrap();
    picture
        .children
        .iter_mut()
        .find(|n| n.name == "sz")
        .unwrap()
        .attrs
        .insert("width".into(), "49528".into());
    let pos = picture
        .children
        .iter_mut()
        .find(|n| n.name == "pos")
        .unwrap();
    pos.attrs.insert("horzOffset".into(), "0".into());
    pos.attrs.insert("vertOffset".into(), "0".into());
    let mut e = Engine::new(&d, fonts());
    e.render_contours().unwrap();
    check(&e);
    let picture = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "offset-picture")
        .unwrap();
    let first = e.pages[picture.page - 1]
        .ops
        .iter()
        .find_map(|o| match o {
            Op::Text { text, y, .. } if text.starts_with('A') => Some(*y),
            _ => None,
        })
        .unwrap();
    assert!(first > picture.y + picture.h);
}
#[test]
fn paragraph_relative_objects_can_follow_text_in_the_source_run() {
    let mut d = open("contour-single");
    let run = d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "placed")
        .unwrap()
        .children
        .iter_mut()
        .find(|r| r.name == "run")
        .unwrap();
    let mut objects = Vec::new();
    let mut rest = Vec::new();
    for n in std::mem::take(&mut run.children) {
        if matches!(n.name.as_str(), "pic" | "rect" | "ellipse" | "polygon") {
            objects.push(n)
        } else {
            rest.push(n)
        }
    }
    rest.extend(objects);
    run.children = rest;
    let mut e = Engine::new(&d, fonts());
    e.render_contours().unwrap();
    check(&e);
    assert_eq!(e.report.pictures, 1);
    for id in [
        "offset-ellipse",
        "offset-triangle",
        "offset-rotation",
        "offset-picture",
    ] {
        assert_eq!(e.report.bounds.iter().filter(|b| b.id == id).count(), 1);
    }
}
#[test]
fn overlap_disabled_separates_conflicting_object_anchors() {
    let mut d = open("contour-single");
    let run = d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "placed")
        .unwrap()
        .children
        .iter_mut()
        .find(|r| r.name == "run")
        .unwrap();
    run.children
        .retain(|n| !matches!(n.name.as_str(), "polygon" | "rect"));
    let pos = run
        .children
        .iter_mut()
        .find(|n| n.name == "pic")
        .unwrap()
        .children
        .iter_mut()
        .find(|n| n.name == "pos")
        .unwrap();
    pos.attrs.insert("horzOffset".into(), "1000".into());
    pos.attrs.insert("vertOffset".into(), "500".into());
    let mut e = Engine::new(&d, fonts());
    e.render_contours().unwrap();
    check(&e);
    let a = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "offset-ellipse")
        .unwrap();
    let b = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "offset-picture")
        .unwrap();
    assert!(b.page > a.page || b.y >= a.y + a.h);
    assert!(e.report.warnings.iter().any(|w| w.contains("allowOverlap")));
}
#[test]
fn enlarged_polygon_contour_uses_original_coordinate_scale() {
    let mut d = open("contour-single");
    let run = d.sections[0]
        .children
        .iter_mut()
        .find(|p| p.a("id") == "placed")
        .unwrap()
        .children
        .iter_mut()
        .find(|r| r.name == "run")
        .unwrap();
    run.children
        .retain(|n| !matches!(n.name.as_str(), "pic" | "rect" | "ellipse"));
    let polygon = run
        .children
        .iter_mut()
        .find(|n| n.name == "polygon")
        .unwrap();
    polygon
        .children
        .iter_mut()
        .find(|n| n.name == "sz")
        .unwrap()
        .attrs
        .insert("width".into(), "18000".into());
    let pos = polygon
        .children
        .iter_mut()
        .find(|n| n.name == "pos")
        .unwrap();
    for (k, v) in [
        ("horzAlign", "LEFT"),
        ("horzOffset", "0"),
        ("vertOffset", "0"),
    ] {
        pos.attrs.insert(k.into(), v.into());
    }
    for point in polygon.children.iter_mut().filter(|n| n.name == "pt") {
        point
            .attrs
            .insert("x".into(), (9000. - point.n("x")).to_string());
    }
    let mut e = Engine::new(&d, fonts());
    e.render_contours().unwrap();
    check(&e);
    let shape = e
        .report
        .bounds
        .iter()
        .find(|b| b.id == "offset-triangle")
        .unwrap();
    assert!(e.report.bounds.iter().any(|b| b.kind == "reflow-line"
        && b.page == shape.page
        && (b.y - (shape.y + shape.h / 2.)).abs() < 20.
        && b.x > shape.x + 140.));
}
