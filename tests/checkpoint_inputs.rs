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
        let direct = !name.starts_with("17_") && !name.starts_with("18_");
        assert_eq!(chart.child("chartSpace").is_some(), direct, "{name}");
        if !direct {
            assert!(!id.is_empty());
            let pixels = image::load_from_memory(&d.binaries[id]).unwrap();
            assert!(pixels.width() > 100 && pixels.height() > 100);
        }
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
        if direct {
            assert!(images.is_empty());
            let text = e.pages[0]
                .ops
                .iter()
                .filter_map(|o| match o {
                    Op::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>();
            assert!(!text.contains("차트 제목"));
            assert!(text.contains(if name.starts_with("19_") {
                "종가"
            } else {
                "계열 1"
            }));
            let bounds = e.report.bounds.iter().find(|b| b.kind == "chart").unwrap();
            assert_eq!(
                (bounds.w, bounds.h),
                (sz.n("width") / 100., sz.n("height") / 100.)
            );
        } else {
            assert_eq!(images, vec![(sz.n("width") / 100., sz.n("height") / 100.)]);
        }
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

#[test]
fn nested_table_matches_printed_fragments_and_following_text_positions() {
    let path = input("08_tables-notes__inner-table-01.hwp");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let source = d.sections.clone();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 2);
    for (page, word, expected) in [
        (0, "사업개요", 587.16),
        (0, "사업내용", 655.2),
        (0, "데이터허브구축", 720.24),
        (1, "전사데이터", 142.2),
        (1, "SFR-CLOUD-001", 367.32),
        (1, "끝", 587.04),
    ] {
        let mut chars = Vec::new();
        let mut matrix = hwp_engine::graphics::IDENTITY;
        let mut stack = Vec::new();
        for op in &e.pages[page].ops {
            match op {
                Op::TransformBegin { matrix: m } => {
                    stack.push(matrix);
                    matrix = hwp_engine::graphics::compose(matrix, *m);
                }
                Op::TransformEnd => matrix = stack.pop().unwrap(),
                Op::Text { text, x, y, .. } | Op::Shaped { text, x, y, .. } => {
                    let (_, y) = hwp_engine::graphics::point(matrix, *x, *y);
                    chars.extend(text.chars().filter(|c| !c.is_whitespace()).map(|c| (c, y)));
                }
                _ => {}
            }
        }
        let needle: Vec<_> = word.chars().collect();
        let actual = chars
            .windows(needle.len())
            .find(|s| s.iter().zip(&needle).all(|((a, _), b)| a == b))
            .unwrap()[0]
            .1;
        assert!(
            (actual - expected).abs() < 0.15,
            "{word}: {actual} vs {expected}"
        );
    }
    assert_eq!(d.sections, source);
    let q = hwp_engine::qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(
        q.outside_page.is_empty() && q.text_overlaps.is_empty() && q.border_crossings.is_empty()
    );
}

#[test]
fn limit_reference_preserves_arrow_and_has_no_script_fraction_collision() {
    let path = input("11_equation__limits.hwp");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 1);
    assert_eq!(e.report.equations, 1);
    let text = e.pages[0]
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert!(text.contains("lim") && text.contains('→') && !text.contains("rarrow"));
    let q = hwp_engine::qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(
        q.text_overlaps.is_empty() && q.border_crossings.is_empty() && q.outside_page.is_empty()
    );
}

#[test]
fn sparse_native_drawing_recovers_without_modifying_source_and_keeps_line_direction() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("output/four-priority-sources/mix-shape-01.hwp");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let source = std::fs::read(&path).unwrap();
    let d = Document::open(&path).unwrap();
    assert_eq!(d.header.a("repairedUnusedFat"), "1");
    let line = d.sections.iter().find_map(|s| s.find("line")).unwrap();
    assert!(line.child("startPt").unwrap().signed("y") > line.child("endPt").unwrap().signed("y"));
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 1);
    assert!(
        e.report
            .warnings
            .iter()
            .any(|w| w.starts_with("Recovered unreachable"))
    );
    assert!(
        e.pages[0]
            .ops
            .iter()
            .any(|op| matches!(op,Op::Line{x,y,x2,y2,..} if x2>x && y2<y))
    );
    assert_eq!(std::fs::read(&path).unwrap(), source);
}

#[test]
fn native_ooxml_columns_are_drawn_as_a_chart_instead_of_a_fallback_grid() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("output/roadmap-reference-downloads/chart__bar_vertical_hwp5.hwp");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    assert!(d.sections.iter().any(|s| s.find("barChart").is_some()));
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 1);
    assert_eq!(e.report.charts, 1);
    assert!(
        !e.report
            .warnings
            .iter()
            .any(|w| w.contains("labelled table"))
    );
}

#[test]
fn hwpx_ole_equation_is_source_text_not_a_blank_placeholder() {
    let path = input("12_equation__ole.hwpx");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let eq = d.sections.iter().find_map(|s| s.find("equation")).unwrap();
    assert_eq!(eq.a("oleEquation"), "1");
    let script = hwp_engine::model::text(eq.child("script").unwrap());
    assert!(script.contains("over") && script.contains("times") && !script.contains("TAB"));
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 2);
    assert!(e.report.equations > 0);
}

#[test]
fn hwpx_static_emf_ole_restores_logo_and_declared_font_keeps_one_page() {
    let path = input("unused")
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("Hancom-Essential-7/07_emf_ole.hwpx");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    assert!(d.binaries.keys().any(|k| k.ends_with("-ole-enhanced")));
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 1);
    assert!(
        e.pages[0]
            .ops
            .iter()
            .any(|o| matches!(o,Op::Image{id,..} if id.ends_with("-ole-enhanced")))
    );
    let q = hwp_engine::qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty() && q.outside_page.is_empty());
}

#[test]
fn nested_native_group_title_uses_root_frame_once_and_resolved_textbox_height() {
    let path = input("24_extended__shapecontainer-2.hwp");
    if !path.exists() || !fonts().exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(e.pages.len(), 1);
    for (needle, x, y) in [("목", 95.04, 118.88), ("차", 117.12, 132.2)] {
        let (actual_x, actual_y) = e.pages[0]
            .ops
            .iter()
            .find_map(|o| match o {
                Op::Text { text, x, y, .. } if text == needle => Some((*x, *y)),
                _ => None,
            })
            .unwrap();
        assert!(
            (actual_x - x).abs() < 0.15 && (actual_y - y).abs() < 0.15,
            "{needle}: {actual_x}, {actual_y}"
        );
    }
    let q = hwp_engine::qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty() && q.outside_page.is_empty());
}
