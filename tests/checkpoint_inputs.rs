//! User-supplied print inputs: prevent silent alternate-content loss.
use hwp_engine::{layout::Engine, model::Document, pdf::Op};
use std::path::{Path, PathBuf};

fn input(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Hancom-95-Checkpoint-20261019/01_print")
        .join(name)
}
fn fonts() -> &'static Path {
    Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}

#[test]
fn alternate_charts_are_visible_once_with_original_geometry() {
    for name in [
        "13_chart__bar_horizontal.hwpx",
        "14_chart__bar_stacked.hwpx",
        "15_chart__bar_percent.hwpx",
        "16_chart__line_markers.hwpx",
        "17_chart__pie_3d.hwpx",
        "18_chart__pie_of_pie.hwpx",
        "19_chart__stock_ohlc.hwpx",
    ] {
        let path = input(name);
        if !path.exists() || !fonts().exists() {
            continue;
        }
        let d = Document::open(&path).unwrap();
        let chart = d.sections[0].find("chart").unwrap();
        let id = chart.a("presentationIDRef");
        assert!(!id.is_empty());
        let pixels = image::load_from_memory(&d.binaries[id]).unwrap();
        assert!(pixels.width() > 100 && pixels.height() > 100);
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        assert_eq!(e.pages.len(), 1, "{name}");
        assert_eq!(e.report.charts, 1, "{name}");
        let images: Vec<_> = e.pages[0]
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Image {
                    id: painted, w, h, ..
                } if painted == id => Some((*w, *h)),
                _ => None,
            })
            .collect();
        let sz = chart.child("sz").unwrap();
        assert_eq!(images, vec![(sz.n("width") / 100., sz.n("height") / 100.)]);
    }
}

#[test]
fn static_ole_wmf_is_a_picture_not_a_legacy_chart() {
    let path = input("21_vector__wmf_ole.hwp");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    assert!(d.binaries.keys().any(|id| id.ends_with("-ole-metafile")));
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 4);
    assert!(
        e.pages
            .iter()
            .flat_map(|p| &p.ops)
            .any(|op| matches!(op, Op::Bezier { .. }))
    );
}
