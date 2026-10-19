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

#[test]
fn compound_wmf_keeps_content_and_reports_damaged_source_bitmaps() {
    let path = input("20_vector__wmf_fill_shapes.hwpx");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    for id in ["image3", "image4", "image5"] {
        let im = image::load_from_memory(&d.binaries[id]).unwrap().to_rgb8();
        assert!(im.width() > 1000);
        assert!(
            im.pixels()
                .filter(|p| p[2] > p[0].saturating_add(40))
                .count()
                > 1000
        );
    }
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 15);
    assert_eq!(
        e.report
            .warnings
            .iter()
            .filter(|w| w.starts_with("Source image "))
            .count(),
        5
    );
}

#[test]
fn embedded_emf_plus_only_icons_survive_inside_blue_circles() {
    let path = input("20_vector__wmf_fill_shapes.hwpx");
    if !path.exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let im = image::load_from_memory(&d.binaries["image4"])
        .unwrap()
        .to_rgb8();
    let (w, h) = im.dimensions();
    let mut seen = vec![false; (w * h) as usize];
    let mut icons = 0;
    let blue = |x: u32, y: u32| {
        let p = im.get_pixel(x, y);
        p[0] < 40 && p[1] > 60 && p[2] > p[1].saturating_add(25)
    };
    for y in 0..h {
        for x in 0..w {
            if seen[(y * w + x) as usize] || !blue(x, y) {
                continue;
            }
            let mut queue = vec![(x, y)];
            seen[(y * w + x) as usize] = true;
            let (mut left, mut top, mut right, mut bottom) = (x, y, x, y);
            let mut pos = 0;
            while pos < queue.len() {
                let (a, b) = queue[pos];
                pos += 1;
                left = left.min(a);
                top = top.min(b);
                right = right.max(a);
                bottom = bottom.max(b);
                for (nx, ny) in [
                    (a.wrapping_sub(1), b),
                    (a + 1, b),
                    (a, b.wrapping_sub(1)),
                    (a, b + 1),
                ] {
                    if nx < w && ny < h && !seen[(ny * w + nx) as usize] && blue(nx, ny) {
                        seen[(ny * w + nx) as usize] = true;
                        queue.push((nx, ny));
                    }
                }
            }
            let (bw, bh) = (right - left + 1, bottom - top + 1);
            if bw < 40 || bh < 40 || !(0.8..1.2).contains(&(bw as f64 / bh as f64)) {
                continue;
            }
            let mut white = 0;
            for yy in top + bh / 4..bottom - bh / 4 {
                for xx in left + bw / 4..right - bw / 4 {
                    if im.get_pixel(xx, yy).0.iter().all(|v| *v > 220) {
                        white += 1;
                    }
                }
            }
            if white > 40 {
                icons += 1;
            }
        }
    }
    assert!(icons >= 3, "EMF+ only icons were lost: {icons}");
}
