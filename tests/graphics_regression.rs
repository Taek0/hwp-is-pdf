use hwp_engine::{
    graphics as g,
    layout::Engine,
    model::Document,
    pdf::{Fonts, Op, Page},
    qa,
};
use std::path::PathBuf;
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
#[test]
fn curve_preserves_knots_and_tangent_continuity() {
    let p = [(0., 0.), (20., 30.), (40., 0.), (60., 20.)];
    let s = g::curve(&p, &[1, 1, 1]);
    for (i, c) in s.iter().enumerate() {
        assert_eq!((c[4], c[5]), p[i + 1]);
    }
    for i in 0..s.len() - 1 {
        for axis in 0..2 {
            assert!(
                ((s[i][4 + axis] - s[i][2 + axis]) - (s[i + 1][axis] - s[i][4 + axis])).abs()
                    < 1e-9
            );
        }
    }
    let line = g::curve(&p, &[0, 1, 0]);
    assert_eq!(line[0][0], 20. / 3.);
    assert_eq!(line[0][1], 10.);
}
#[test]
fn rotation_and_inverse_use_the_same_page_coordinates() {
    let m = g::rotation(10., 20., 30., 50., 90., false, false);
    let r = g::rect(m, [10., 20., 40., 70.]);
    for (a, b) in r.iter().zip([10., 20., 60., 50.]) {
        assert!((a - b).abs() < 1e-8);
    }
    let p = g::point(m, 14., 26.);
    let q = g::point(g::inverse(m), p.0, p.1);
    assert!((q.0 - 14.).abs() < 1e-9 && (q.1 - 26.).abs() < 1e-9);
}
#[test]
fn qa_checks_transformed_glyphs_instead_of_old_coordinates() {
    let mut f = Fonts::new(&fonts());
    let font = f.get("arial.ttf").unwrap();
    let p = Page {
        width: 100.,
        height: 100.,
        ops: vec![
            Op::TransformBegin {
                matrix: [1., 0., 0., 1., 150., 0.],
            },
            Op::Text {
                x: 10.,
                y: 20.,
                text: "H".into(),
                font,
                size: 12.,
                scale: 1.,
                color: [0.; 3],
            },
            Op::TransformEnd,
        ],
    };
    assert_eq!(qa::inspect(&[p], &f, &[]).outside_page.len(), 1);
}
#[test]
fn real_files_emit_cubic_rotation_and_crop_operations() {
    for (file, kind, count) in [
        ("samples/features/curves.hwp", "curve", 4),
        ("samples/additional/matrix.hwp", "rotation", 2),
        ("samples/graphics/pic-crop-01.hwp", "crop", 1),
        ("samples/graphics/rotated-picture.hwp", "rotation", 1),
    ] {
        let doc = Document::open(&root().join(file)).unwrap();
        let mut e = Engine::new(&doc, &fonts());
        e.render().unwrap();
        let ops = e.pages.iter().flat_map(|p| &p.ops);
        let actual = ops
            .filter(|o| match kind {
                "curve" => matches!(o, Op::Bezier { .. }),
                "rotation" => matches!(o, Op::TransformBegin { .. }),
                _ => matches!(o, Op::ImageClip { .. }),
            })
            .count();
        assert_eq!(actual, count, "{file}");
        let audit = qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
        assert!(
            audit.outside_page.is_empty()
                && audit.text_overlaps.is_empty()
                && audit.border_crossings.is_empty()
                && audit.foreground_image_overlaps.is_empty(),
            "{file}"
        );
    }
}

#[test]
fn signed_master_line_dimensions_do_not_create_huge_transforms() {
    let file = root().parent().unwrap().join("21_언어_기출_편집가능본.hwp");
    if !file.exists() {
        return;
    }
    let doc = Document::open(&file).unwrap();
    let mut e = Engine::new(&doc, &fonts());
    e.render().unwrap();
    for p in &e.pages {
        for op in &p.ops {
            if let Op::TransformBegin { matrix } = op {
                assert!(matrix.iter().all(|v| v.is_finite() && v.abs() < 10000.));
            }
        }
    }
    let mut lines = vec![];
    for m in &doc.masters {
        m.all("line", &mut lines);
    }
    assert!(
        lines
            .iter()
            .any(|n| n.child("curSz").is_some_and(|s| s.n("height") == 93268.))
    );
}

#[test]
fn hwpx_point_and_segment_geometry_is_not_silently_omitted() {
    fn alter(n: &mut hwp_engine::model::Node) -> bool {
        if n.name == "polygon" {
            n.name = "curve".into();
            n.children.retain(|c| c.name != "pt");
            for (a, b) in [((0, 0), (760, 1000)), ((760, 1000), (1526, 0))] {
                n.children.push(hwp_engine::model::Node {
                    name: "seg".into(),
                    attrs: [
                        ("type".into(), "CURVE".into()),
                        ("x1".into(), a.0.to_string()),
                        ("y1".into(), a.1.to_string()),
                        ("x2".into(), b.0.to_string()),
                        ("y2".into(), b.1.to_string()),
                    ]
                    .into(),
                    ..Default::default()
                });
            }
            return true;
        }
        n.children.iter_mut().any(alter)
    }
    let mut doc = Document::open(&root().join("samples/graphics/shapes.hwpx")).unwrap();
    assert!(doc.sections.iter_mut().any(alter));
    let mut e = Engine::new(&doc, &fonts());
    e.render().unwrap();
    assert!(
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .any(|op| matches!(op,Op::Bezier{segments,..} if segments.len()==2))
    );
    assert!(
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .any(|op| matches!(op,Op::Path{points,..} if points.len()==4))
    );
}
