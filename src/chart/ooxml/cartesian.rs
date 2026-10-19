//! Source-backed 2D Cartesian charts, independent of stale OLE preview titles.
use super::*;
mod data;
mod marks;
#[cfg(test)]
mod tests;
pub(crate) use data::supported;
use data::{label, number, numeric_axis, values};

pub(crate) fn draw(
    root: &Node,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fonts: &mut Fonts,
) -> Result<Vec<Op>> {
    ensure!(supported(root), "Unsupported Cartesian chart layout");
    let plot = root.find("plotArea").unwrap();
    let chart = plot
        .children
        .iter()
        .find(|n| matches!(n.name.as_str(), "barChart" | "lineChart" | "stockChart"))
        .unwrap();
    let mut series: Vec<_> = chart.children_named("ser").collect();
    series.sort_by_key(|s| s.child("order").map(|n| n.u("val")).unwrap_or(0));
    ensure!(
        !series.is_empty() && series.len() <= 64,
        "Invalid Cartesian series count"
    );
    let mut data = series
        .iter()
        .map(|s| cached(s, "val"))
        .collect::<Result<Vec<_>>>()?;
    let count = data[0].len();
    ensure!(
        count > 0 && data.iter().all(|v| v.len() == count),
        "Ragged Cartesian values"
    );
    let mut categories = values(series[0], "cat")?;
    if categories.is_empty() {
        categories = (1..=count).map(|v| v.to_string()).collect();
    }
    ensure!(categories.len() == count, "Chart category/value mismatch");
    let bar = chart.name == "barChart";
    let stock = chart.name == "stockChart";
    let horizontal = bar && chart.child("barDir").is_some_and(|v| v.a("val") == "bar");
    let show_marker = |s: &Node| {
        s.child("marker").is_some() || chart.child("marker").is_some_and(|n| n.a("val") == "1")
    };
    let grouping = chart
        .child("grouping")
        .map(|n| n.a("val"))
        .unwrap_or("clustered");
    let stacked = bar && matches!(grouping, "stacked" | "percentStacked");
    let percent = grouping == "percentStacked";
    if percent {
        for c in 0..count {
            let pos = data.iter().map(|s| s[c].max(0.)).sum::<f64>();
            let neg = data.iter().map(|s| (-s[c]).max(0.)).sum::<f64>();
            for s in &mut data {
                let total = if s[c] < 0. { neg } else { pos };
                if total > 0. {
                    s[c] /= total;
                }
            }
        }
    }
    let (mut lo, mut hi) = (0f64, 0f64);
    for c in 0..count {
        if stacked {
            lo = lo.min(data.iter().map(|s| s[c].min(0.)).sum());
            hi = hi.max(data.iter().map(|s| s[c].max(0.)).sum());
        } else {
            for s in &data {
                lo = lo.min(s[c]);
                hi = hi.max(s[c]);
            }
        }
    }
    let (mut min, mut max, mut step) = if percent {
        (if lo < 0. { -1. } else { 0. }, 1., 0.2)
    } else {
        numeric_axis(lo, hi, horizontal)
    };
    let axis = plot.child("valAx").unwrap();
    if let Some(sc) = axis.child("scaling") {
        if let Some(v) = sc.child("min") {
            min = v.n("val");
        }
        if let Some(v) = sc.child("max") {
            max = v.n("val");
        }
    }
    let label_percent = percent
        || axis
            .child("numFmt")
            .is_some_and(|n| n.a("formatCode") == "0%");
    if let Some(v) = axis.child("majorUnit") {
        step = v.n("val");
    }
    ensure!(
        min.is_finite() && max.is_finite() && max > min && step > 0. && (max - min) / step <= 100.,
        "Invalid Cartesian axis range"
    );
    let font = fonts
        .family("함초롬돋움", false, false)?
        .context("Chart font unavailable")?;
    let size = root
        .child("txPr")
        .and_then(|n| n.find("defRPr"))
        .map(|n| n.n("sz") / 100.)
        .filter(|v| *v > 0.)
        .unwrap_or(10.);
    let names: Vec<_> = series
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let t = label(s);
            if t.is_empty() {
                format!("Series {}", i + 1)
            } else {
                t
            }
        })
        .collect();
    let legend = root.find("legend").is_some();
    let legend_width = names
        .iter()
        .map(|s| fonts.width(font, s, size))
        .fold(0., f64::max);
    let label_x = x + w - size * if bar { 0.75 } else { 0.63 } - legend_width;
    let key_left = label_x - size * 0.3125 - size * 0.6;
    let axis_label_width = (0..=((max - min) / step).ceil() as usize)
        .map(|i| fonts.width(font, &number(min + i as f64 * step, label_percent), size))
        .fold(0., f64::max);
    let left = x + if horizontal {
        categories
            .iter()
            .map(|s| fonts.width(font, s, size))
            .fold(0., f64::max)
            + size * 1.96
    } else {
        axis_label_width + size * 1.866
    };
    let right = if legend {
        key_left
            - size * if stock { 1.03 } else { 1.09 }
            - if horizontal { size * 0.53 } else { 0. }
    } else {
        x + w - size * 1.5
    };
    let top = y + size * if horizontal { 3.65 } else { 4.5 };
    // Measured font-relative margin, unrelated to the circle constant.
    let bottom = y + h - size * if horizontal { 304. / 100. } else { 314. / 100. };
    ensure!(
        right - left > size && bottom - top > size,
        "Cartesian chart too small for labels"
    );
    let val_reverse = axis
        .child("scaling")
        .and_then(|n| n.child("orientation"))
        .is_some_and(|n| n.a("val") == "maxMin");
    let cat_reverse = plot
        .child("catAx")
        .and_then(|n| n.child("scaling"))
        .and_then(|n| n.child("orientation"))
        .is_some_and(|n| n.a("val") == "maxMin");
    let coord = |v: f64| {
        let mut t = (v - min) / (max - min);
        if val_reverse {
            t = 1. - t;
        }
        if horizontal {
            left + t * (right - left)
        } else {
            bottom - t * (bottom - top)
        }
    };
    let group = if horizontal {
        (bottom - top) / count as f64
    } else {
        (right - left) / count as f64
    };
    let center = |i: usize| {
        let i = if cat_reverse { count - 1 - i } else { i };
        if horizontal {
            bottom - (i as f64 + 0.5) * group
        } else {
            left + (i as f64 + 0.5) * group
        }
    };
    let text = |x, y, text: String| Op::Text {
        x,
        y,
        text,
        font,
        size,
        scale: 1.,
        color: [0.; 3],
    };
    let line = |x, y, x2, y2| Op::Line {
        x,
        y,
        x2,
        y2,
        width: 0.75,
        color: [134. / 255.; 3],
    };
    let mut out = vec![Op::Rect {
        x,
        y,
        w,
        h,
        fill: [1.; 3],
    }];
    if root
        .child("spPr")
        .and_then(|n| n.child("ln"))
        .is_none_or(|n| n.child("noFill").is_none())
    {
        out.push(Op::Path {
            points: vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)],
            closed: true,
            width: 0.75,
            color: [134. / 255.; 3],
            fill: None,
        });
    }
    if let Some(title) = root.find("title").and_then(|n| n.child("tx")) {
        let t = title
            .find("rich")
            .map(crate::model::text)
            .or_else(|| title.find("v").map(crate::model::text))
            .unwrap_or_default();
        if !t.trim().is_empty() {
            let title_size = size * 1.4;
            out.push(Op::Text {
                x: x + (w - fonts.width(font, &t, title_size)) / 2.,
                y: y + size * 2.5,
                text: t,
                font,
                size: title_size,
                scale: 1.,
                color: [0.; 3],
            });
        }
    }
    let grid = axis.child("majorGridlines").is_some();
    for i in 0..=((max - min) / step).floor() as usize {
        let v = min + i as f64 * step;
        let pos = coord(v);
        let label = number(v, label_percent);
        if horizontal {
            if grid {
                out.push(line(pos, top, pos, bottom));
            }
            out.push(line(pos, bottom, pos, bottom + size * 0.3));
            out.push(text(
                pos - fonts.width(font, &label, size) / 2.,
                bottom + size * 1.98,
                label,
            ));
        } else {
            if grid {
                out.push(line(left, pos, right, pos));
            }
            out.push(line(left - size * 0.3, pos, left, pos));
            out.push(text(
                left - size * 0.85 - fonts.width(font, &label, size),
                pos + size * 0.363,
                label,
            ));
        }
    }
    out.push(line(left, top, left, bottom));
    out.push(line(left, bottom, right, bottom));
    for (c, label) in categories.iter().enumerate() {
        let p = center(c);
        if horizontal {
            out.push(text(
                left - size * 1.15 - fonts.width(font, label, size),
                p + size * 0.304,
                label.clone(),
            ));
        } else {
            out.push(text(
                p - fonts.width(font, label, size) / 2.,
                bottom + size * 2.073,
                label.clone(),
            ));
        }
    }
    if !horizontal {
        for i in 0..=count {
            let xx = left + i as f64 * group;
            out.push(line(xx, bottom, xx, bottom + size * 0.3));
        }
    } else {
        for i in 0..=count {
            let yy = top + i as f64 * group;
            out.push(line(left - size * 0.3, yy, left, yy));
        }
    }
    out.push(Op::TransformBegin {
        matrix: crate::graphics::IDENTITY,
    });
    out.push(Op::ClipRect {
        x: left,
        y: top,
        w: right - left,
        h: bottom - top,
    });
    if bar {
        let gap = chart
            .child("gapWidth")
            .map(|n| n.n("val") / 100.)
            .unwrap_or(1.5)
            .clamp(0., 5.);
        let bw = group / (if stacked { 1. } else { series.len() as f64 } + gap);
        for (c, _) in categories.iter().enumerate() {
            let (mut positive, mut negative) = (0., 0.);
            for (i, s) in series.iter().enumerate() {
                let v = data[i][c];
                let start = if stacked {
                    if v < 0. { negative } else { positive }
                } else {
                    0.
                };
                let end = start + v;
                if stacked {
                    if v < 0. {
                        negative = end;
                    } else {
                        positive = end;
                    }
                }
                let a = coord(start.clamp(min, max));
                let b = coord(end.clamp(min, max));
                let at = if stacked {
                    center(c) - bw / 2.
                } else if horizontal {
                    center(c) + (series.len() as f64 / 2. - i as f64 - 1.) * bw
                } else {
                    center(c) - series.len() as f64 * bw / 2. + i as f64 * bw
                };
                let rect = if horizontal {
                    (a.min(b), at, (b - a).abs(), bw)
                } else {
                    (at, a.min(b), bw, (b - a).abs())
                };
                out.push(Op::Rect {
                    x: rect.0,
                    y: rect.1,
                    w: rect.2,
                    h: rect.3,
                    fill: marks::color(s, i),
                });
            }
        }
    } else if stock {
        let (open, high, low, close) = if series.len() == 4 {
            (Some(0), 1, 2, 3)
        } else {
            (None, 0, 1, 2)
        };
        let gap = chart
            .child("upDownBars")
            .and_then(|n| n.child("gapWidth"))
            .map(|n| n.n("val") / 100.)
            .unwrap_or(1.5)
            .clamp(0., 5.);
        let bw = group / (1. + gap);
        for (c, _) in categories.iter().enumerate() {
            let xx = center(c);
            out.push(Op::Line {
                x: xx,
                y: coord(data[high][c]),
                x2: xx,
                y2: coord(data[low][c]),
                width: 0.75,
                color: [0.; 3],
            });
            if let Some(open) = open {
                let a = coord(data[open][c]);
                let b = coord(data[close][c]);
                out.push(Op::Path {
                    points: vec![
                        (xx - bw / 2., a),
                        (xx + bw / 2., a),
                        (xx + bw / 2., b),
                        (xx - bw / 2., b),
                    ],
                    closed: true,
                    width: 0.75,
                    color: [0.; 3],
                    fill: Some(if data[close][c] >= data[open][c] {
                        [1.; 3]
                    } else {
                        [63. / 255.; 3]
                    }),
                });
            }
            for (i, s) in series.iter().enumerate() {
                if show_marker(s) {
                    out.extend(marks::marker(s, i, xx, coord(data[i][c])));
                }
            }
        }
    } else {
        for (i, s) in series.iter().enumerate() {
            let points: Vec<_> = data[i]
                .iter()
                .enumerate()
                .map(|(c, v)| (center(c), coord(*v)))
                .collect();
            if marks::line_visible(s) {
                out.push(Op::Path {
                    points: points.clone(),
                    closed: false,
                    width: marks::line_width(s),
                    color: marks::color(s, i),
                    fill: None,
                });
            }
            if show_marker(s) {
                for (xx, yy) in points {
                    out.extend(marks::marker(s, i, xx, yy));
                }
            }
        }
    }
    out.push(Op::TransformEnd);
    if legend {
        for j in 0..series.len() {
            let i = if bar { series.len() - 1 - j } else { j };
            let s = series[i];
            let center_y = y
                + h / 2.
                + size * 1.35
                + (j as f64 - (series.len() - 1) as f64 / 2.)
                    * size
                    * if bar { 1.35 } else { 1.44 };
            if bar {
                out.push(Op::Rect {
                    x: key_left,
                    y: center_y - size * 0.3,
                    w: size * 0.6,
                    h: size * 0.6,
                    fill: marks::color(s, i),
                });
            } else {
                if marks::line_visible(s) {
                    out.push(Op::Line {
                        x: key_left - size * 0.3,
                        y: center_y,
                        x2: key_left + size * 0.9,
                        y2: center_y,
                        width: marks::line_width(s),
                        color: marks::color(s, i),
                    });
                }
                if show_marker(s) {
                    out.extend(marks::marker(s, i, key_left + size * 0.3, center_y));
                }
            }
            out.push(text(label_x, center_y + size * 0.27, names[i].clone()));
        }
    }
    Ok(out)
}
