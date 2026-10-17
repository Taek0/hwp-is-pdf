//! Dual-axis column/line charts from decoded VtChart presentation records.
use super::*;
use anyhow::Context;

fn label(v: f64) -> String {
    if v.fract().abs() < 0.000001 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .into()
    }
}
fn font(layout: &Node, index: usize, fonts: &mut Fonts) -> Result<(usize, f64)> {
    let n = layout
        .children_named("axisFont")
        .find(|f| f.u("index") == index)
        .context("Missing native chart axis font")?;
    let face = fonts
        .family(n.a("face"), n.a("bold") == "1", n.a("italic") == "1")?
        .or(fonts.family("함초롬돋움", n.a("bold") == "1", n.a("italic") == "1")?)
        .context("Missing chart font")?;
    Ok((face, n.n("size")))
}
pub(super) fn draw(n: &Node, x: f64, y: f64, w: f64, h: f64, fonts: &mut Fonts) -> Result<Vec<Op>> {
    let layout = n.child("nativeLayout").unwrap();
    let extent = layout.child("extent").context("Missing chart extent")?;
    let plot = layout.child("plot").context("Missing chart plot")?;
    let sx = w / (extent.n("right") - extent.n("left"));
    let sy = h / (extent.n("bottom") - extent.n("top"));
    ensure!(
        sx.is_finite() && sy.is_finite() && sx > 0. && sy > 0.,
        "Invalid chart extent"
    );
    let axes: Vec<_> = (0..2)
        .map(|i| {
            layout
                .children_named("axis")
                .find(|a| a.u("index") == i)
                .context("Missing manual chart axis")
        })
        .collect::<Result<_>>()?;
    let af = [font(layout, 1, fonts)?, font(layout, 2, fonts)?];
    let cf = font(layout, 0, fonts)?;
    for a in &axes {
        ensure!(
            a.n("max") > a.n("min") && (1..=100).contains(&a.u("divisions")),
            "Invalid manual axis"
        );
    }
    let tick_strings: Vec<Vec<String>> = axes
        .iter()
        .map(|a| {
            (0..=a.u("divisions"))
                .map(|i| {
                    label(a.n("min") + (a.n("max") - a.n("min")) * i as f64 / a.n("divisions"))
                })
                .collect()
        })
        .collect();
    let text_width = |i: usize| {
        tick_strings[i]
            .iter()
            .map(|s| fonts.width(af[i].0, s, af[i].1))
            .fold(0., f64::max)
    };
    let left = x + (plot.n("left") - extent.n("left")) * sx + text_width(0) + af[0].1;
    let right = x + (plot.n("right") - extent.n("left")) * sx - text_width(1) - af[1].1;
    let top = y + (plot.n("top") - extent.n("top")) * sy + af[0].1.max(af[1].1) * 1.6;
    let bottom = y + (plot.n("bottom") - extent.n("top")) * sy - cf.1 * 2.3;
    ensure!(
        right > left && bottom > top,
        "Chart plot too small for labels"
    );
    let mut ops = vec![];
    let line = |x, y, x2, y2, width| Op::Line {
        x,
        y,
        x2,
        y2,
        width,
        color: [0.; 3],
    };
    for (a, b, c, d) in [
        (x, y, x + w, y),
        (x + w, y, x + w, y + h),
        (x + w, y + h, x, y + h),
        (x, y + h, x, y),
    ] {
        ops.push(line(a, b, c, d, 0.5));
    }
    for (i, a) in axes.iter().enumerate() {
        let xx = if i == 0 { left } else { right };
        ops.push(line(xx, top, xx, bottom, 0.7));
        for (j, s) in tick_strings[i].iter().enumerate() {
            let yy = bottom - (bottom - top) * j as f64 / a.n("divisions");
            if i == 0 {
                ops.push(line(left, yy, right, yy, 0.5));
            }
            let tx = if i == 0 {
                left - 8. - fonts.width(af[i].0, s, af[i].1)
            } else {
                right + 8.
            };
            ops.push(Op::Text {
                x: tx,
                y: yy + af[i].1 * 0.4,
                text: s.clone(),
                font: af[i].0,
                size: af[i].1,
                scale: 1.,
                color: [0.; 3],
            });
            ops.push(line(xx - 2., yy, xx + 2., yy, 0.7));
        }
    }
    let categories: Vec<_> = n.children_named("category").collect();
    ensure!(!categories.is_empty(), "Empty chart categories");
    let step = (right - left) / categories.len() as f64;
    // Column painters precede line painters so the line remains visible.
    for painter in [1, 6] {
        for s in n
            .children_named("series")
            .filter(|s| s.u("painter") == painter)
        {
            let a = axes
                .get(s.u("axis"))
                .context("Invalid chart axis selector")?;
            let value_y = |value: f64| {
                bottom
                    - (value.clamp(a.n("min"), a.n("max")) - a.n("min")) / (a.n("max") - a.n("min"))
                        * (bottom - top)
            };
            let values: Vec<_> = s.children_named("value").collect();
            ensure!(values.len() == categories.len(), "Ragged combination chart");
            let mut points = vec![];
            for (j, v) in values.iter().enumerate() {
                let xx = left + (j as f64 + 0.5) * step;
                let yy = value_y(v.n("n"));
                if painter == 1 {
                    let zero = value_y(0.);
                    ops.push(Op::Rect {
                        x: xx - step * 0.25,
                        y: yy.min(zero),
                        w: step * 0.5,
                        h: (zero - yy).abs(),
                        fill: rgb(s.a("color")),
                    });
                } else {
                    points.push((xx, yy));
                }
            }
            if painter == 6 {
                ops.push(Op::Path {
                    points,
                    closed: false,
                    width: (s.n("lineWidth") * sx).max(0.1),
                    color: rgb(s.a("lineColor")),
                    fill: None,
                });
            }
        }
    }
    ensure!(
        n.children_named("series")
            .all(|s| matches!(s.u("painter"), 1 | 6)),
        "Unsupported combination chart painter"
    );
    let mut label_right = f64::NEG_INFINITY;
    let align = layout
        .children_named("axisFont")
        .find(|f| f.u("index") == 0)
        .map(|f| f.u("horizontal"))
        .unwrap_or(2);
    for (j, c) in categories
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.a("name").is_empty())
    {
        let xx = left + (j as f64 + 0.5) * step;
        let width = fonts.width(cf.0, c.a("name"), cf.1);
        // Sparse date labels are common in native monthly data series.
        let tx = xx
            - match align {
                0 => 0.,
                1 => width,
                _ => width / 2.,
            };
        if tx < label_right + 2. {
            continue;
        }
        ops.push(line(xx, top, xx, bottom, 0.5));
        ops.push(Op::Text {
            x: tx,
            y: bottom + cf.1 * 1.7,
            text: c.a("name").into(),
            font: cf.0,
            size: cf.1,
            scale: 1.,
            color: [0.; 3],
        });
        label_right = tx + width;
    }
    Ok(ops)
}
