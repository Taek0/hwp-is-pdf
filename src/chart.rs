//! PDF drawing for decoded legacy chart data. Unknown styles use a labelled data table.
use crate::{
    model::Node,
    pdf::{Fonts, Op},
};
use anyhow::{Result, ensure};
mod native_combo;
fn rgb(s: &str) -> [f64; 3] {
    let c = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0x4472c4);
    [
        ((c >> 16) & 255) as f64 / 255.,
        ((c >> 8) & 255) as f64 / 255.,
        (c & 255) as f64 / 255.,
    ]
}
pub fn draw(
    n: &Node,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fonts: &mut Fonts,
) -> Result<(Vec<Op>, Option<String>)> {
    if n.a("nativeType") == "9" && n.child("nativeLayout").is_some() {
        return native_combo::draw(n, x, y, w, h, fonts).map(|ops| (ops, None));
    }
    let categories: Vec<_> = n.children_named("category").map(|c| c.a("name")).collect();
    let series: Vec<_> = n.children_named("series").collect();
    ensure!(
        !series.is_empty() && !categories.is_empty(),
        "Empty chart grid"
    );
    let font = fonts.get("malgun.ttf")?;
    let mut out = Vec::new();
    let kind = n.a("nativeType").parse::<i32>().unwrap_or(-1);
    let text = |x, y, s: String, size: f64| Op::Text {
        x,
        y,
        text: s,
        font,
        size,
        scale: 1.,
        color: [0.; 3],
    };
    let supported = matches!(
        kind,
        0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 25
    );
    if !supported {
        let title = if n.a("title").is_empty() {
            "차트 데이터 (원본 차트 스타일 미해석)"
        } else {
            n.a("title")
        };
        out.push(text(x, y + 10., title.into(), 8.));
        let cols = categories.len() + 1;
        let rows = series.len() + 1;
        let cw = w / cols as f64;
        let rh = (h - 15.) / rows as f64;
        let size = (cw / 8.).min(rh * 0.4).clamp(3., 9.);
        for r in 0..=rows {
            out.push(Op::Line {
                x,
                y: y + 15. + r as f64 * rh,
                x2: x + w,
                y2: y + 15. + r as f64 * rh,
                width: 0.3,
                color: [0.4; 3],
            });
        }
        for c in 0..=cols {
            out.push(Op::Line {
                x: x + c as f64 * cw,
                y: y + 15.,
                x2: x + c as f64 * cw,
                y2: y + h,
                width: 0.3,
                color: [0.4; 3],
            });
        }
        for (c, label) in categories.iter().enumerate() {
            out.push(text(
                x + (c + 1) as f64 * cw + 2.,
                y + 15. + rh * 0.65,
                label.to_string(),
                size,
            ));
        }
        for (r, s) in series.iter().enumerate() {
            out.push(text(
                x + 2.,
                y + 15. + (r as f64 + 1.65) * rh,
                s.a("name").into(),
                size,
            ));
            for (c, v) in s.children_named("value").enumerate() {
                out.push(text(
                    x + (c + 1) as f64 * cw + 2.,
                    y + 15. + (r as f64 + 1.65) * rh,
                    v.a("n").into(),
                    size,
                ));
            }
        }
        return Ok((
            out,
            Some(format!(
                "Chart style {kind} not fully resolved; exact grid values rendered as a labelled table"
            )),
        ));
    }
    let palette = [
        [0.27, 0.45, 0.77],
        [0.93, 0.49, 0.19],
        [0.65, 0.65, 0.65],
        [1., 0.75, 0.],
        [0.44, 0.68, 0.28],
    ];
    let color = |i: usize| {
        if series[i].a("color").is_empty() {
            palette[i % palette.len()]
        } else {
            rgb(series[i].a("color"))
        }
    };
    let values: Vec<Vec<f64>> = series
        .iter()
        .map(|s| s.children_named("value").map(|v| v.n("n")).collect())
        .collect();
    ensure!(
        values.iter().all(|v| v.len() == categories.len()),
        "Ragged chart data"
    );
    let mut warning = if matches!(kind, 0 | 2 | 4 | 6 | 10 | 12 | 13 | 25) {
        Some("3D chart rendered in a 2D projection; original perspective is not preserved".into())
    } else {
        None
    };
    if !n.a("title").is_empty() {
        out.push(text(x + 8., y + 12., n.a("title").into(), 10.));
    }
    if matches!(kind, 13..=15) {
        let total: f64 = values[0].iter().sum();
        ensure!(
            total > 0. && values[0].iter().all(|v| *v >= 0.),
            "Invalid pie values"
        );
        let radius = (w.min(h) - 40.) / 2.;
        let cx = x + w / 2.;
        let cy = y + h / 2.;
        let mut angle = -std::f64::consts::FRAC_PI_2;
        for (i, value) in values[0].iter().enumerate() {
            let span = *value / total * std::f64::consts::TAU;
            let mut points = vec![(cx, cy)];
            for step in 0..=48 {
                let a = angle + span * step as f64 / 48.;
                points.push((cx + radius * a.cos(), cy + radius * a.sin()));
            }
            out.push(Op::Path {
                points,
                closed: true,
                width: 0.5,
                color: [1.; 3],
                fill: Some(palette[i % palette.len()]),
            });
            angle += span;
        }
        if kind == 15 {
            let points = (0..64)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 64.;
                    (cx + radius * 0.5 * a.cos(), cy + radius * 0.5 * a.sin())
                })
                .collect();
            out.push(Op::Path {
                points,
                closed: true,
                width: 0.,
                color: [1.; 3],
                fill: Some([1.; 3]),
            });
        }
        warning.get_or_insert(
            "Pie uses the first series; original label/legend formatting may differ".into(),
        );
        return Ok((out, warning));
    }
    let min = values.iter().flatten().copied().fold(0., f64::min);
    let max = values.iter().flatten().copied().fold(0., f64::max);
    let span = (max - min).max(1.);
    let left = x + 34.;
    let top = y + 24.;
    let pw = (w - 48.).max(20.);
    let ph = (h - 62.).max(20.);
    let bottom = top + ph;
    out.push(Op::Line {
        x: left,
        y: top,
        x2: left,
        y2: bottom,
        width: 0.5,
        color: [0.; 3],
    });
    out.push(Op::Line {
        x: left,
        y: bottom,
        x2: left + pw,
        y2: bottom,
        width: 0.5,
        color: [0.; 3],
    });
    for i in 0..=5 {
        let value = min + span * i as f64 / 5.;
        let yy = bottom - ph * i as f64 / 5.;
        out.push(text(x, yy + 3., format!("{value:.1}"), 7.));
        out.push(Op::Line {
            x: left,
            y: yy,
            x2: left + pw,
            y2: yy,
            width: 0.2,
            color: [0.8; 3],
        });
    }
    let group = pw / categories.len() as f64;
    let zero = bottom - (0. - min) / span * ph;
    for (i, s) in values.iter().enumerate() {
        let mut points = Vec::new();
        for (c, value) in s.iter().enumerate() {
            let xx = left + (c as f64 + 0.5) * group;
            let yy = bottom - (*value - min) / span * ph;
            if matches!(kind, 0 | 1 | 10 | 11 | 12) {
                let bw = group * 0.7 / series.len() as f64;
                out.push(Op::Rect {
                    x: left + c as f64 * group + group * 0.15 + i as f64 * bw,
                    y: yy.min(zero),
                    w: bw,
                    h: (zero - yy).abs(),
                    fill: color(i),
                });
            } else {
                points.push((xx, yy));
            }
        }
        if !points.is_empty() {
            if matches!(kind, 4 | 5) {
                points.insert(0, (points[0].0, zero));
                points.push((points[points.len() - 1].0, zero));
            }
            out.push(Op::Path {
                points,
                closed: matches!(kind, 4 | 5),
                width: 1.2,
                color: color(i),
                fill: if matches!(kind, 4 | 5) {
                    Some(color(i))
                } else {
                    None
                },
            });
        }
    }
    for (c, label) in categories.iter().enumerate() {
        out.push(text(
            left + c as f64 * group,
            bottom + 12.,
            label.to_string(),
            7.,
        ));
    }
    for (i, s) in series.iter().enumerate() {
        let lx = left + i as f64 * pw / series.len() as f64;
        out.push(Op::Rect {
            x: lx,
            y: y + h - 12.,
            w: 7.,
            h: 7.,
            fill: color(i),
        });
        out.push(text(lx + 10., y + h - 5., s.a("name").into(), 7.));
    }
    if matches!(kind, 10 | 11 | 16 | 25) {
        warning = Some(
            "Chart data preserved; horizontal/XY native axis styling still requires refinement"
                .into(),
        );
    }
    Ok((out, warning))
}
