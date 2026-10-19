//! Clustered OOXML columns with a projected depth plane.
use super::*;
pub(crate) fn draw(
    root: &Node,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fonts: &mut Fonts,
) -> Result<Vec<Op>> {
    let chart = root.find("bar3DChart").context("Missing 3D chart")?;
    ensure!(
        chart.child("barDir").is_some_and(|c| c.a("val") == "col")
            && chart
                .child("grouping")
                .is_some_and(|c| c.a("val") == "clustered"),
        "Unsupported 3D chart grouping"
    );
    let series: Vec<_> = chart.children_named("ser").collect();
    ensure!(
        !series.is_empty() && series.len() <= 64,
        "Invalid bar series"
    );
    let values: Vec<_> = series
        .iter()
        .map(|s| cached(s, "val"))
        .collect::<Result<_>>()?;
    let count = values[0].len();
    ensure!(
        count > 0 && values.iter().all(|v| v.len() == count),
        "Ragged bars"
    );
    let mut categories = Vec::new();
    if let Some(cat) = series[0].child("cat") {
        cat.all("pt", &mut categories);
    }
    categories.sort_by_key(|c| c.u("idx"));
    let labels: Vec<_> = categories
        .iter()
        .map(|p| p.child("v").map(crate::model::text).unwrap_or_default())
        .collect();
    let max = values.iter().flatten().copied().fold(0., f64::max);
    let rough = max.max(1.) / 5.;
    let power = 10f64.powf(rough.log10().floor());
    let step = [1., 2., 5., 10.]
        .into_iter()
        .find(|v| v * power >= rough)
        .unwrap_or(10.)
        * power;
    let ymax = (max / step).ceil().max(1.) * step;
    ensure!(
        values.iter().flatten().all(|v| *v >= 0.),
        "Negative 3D bars require crossing-plane layout"
    );
    let f = fonts
        .family("함초롬돋움", false, false)?
        .context("Chart font unavailable")?;
    let size = 10.;
    let (left, right, top, bottom) = (
        x + size * 3.8,
        x + w - size * 8.3,
        y + size * 5.5,
        y + h - size * 3.4,
    );
    ensure!(right > left && bottom > top, "Bar plot too small");
    let (dx, dy) = (size * 1.5, -size);
    let mut out = Vec::new();
    let line = |x, y, x2, y2| Op::Line {
        x,
        y,
        x2,
        y2,
        width: 0.5,
        color: [0.6; 3],
    };
    let text = |x, y, t: String| Op::Text {
        x,
        y,
        text: t,
        font: f,
        size,
        scale: 1.,
        color: [0.; 3],
    };
    for (a, b, c, d) in [
        (x, y, x + w, y),
        (x + w, y, x + w, y + h),
        (x + w, y + h, x, y + h),
        (x, y + h, x, y),
    ] {
        out.push(line(a, b, c, d));
    }
    for i in 0..=(ymax / step).round().min(100.) as usize {
        let yy = bottom - (i as f64 * step) / ymax * (bottom - top);
        out.push(line(left, yy, left + dx, yy + dy));
        out.push(line(left + dx, yy + dy, right + dx, yy + dy));
        out.push(text(
            left - size * 1.8,
            yy + 3.5,
            format!("{}", i as f64 * step),
        ));
    }
    out.push(line(left, top, left, bottom));
    out.push(line(left, bottom, right, bottom));
    out.push(line(right, bottom, right + dx, bottom + dy));
    let palette = [
        [97. / 255., 129. / 255., 213. / 255.],
        [254. / 255., 131. / 255., 58. / 255.],
        [0.65; 3],
    ];
    let group = (right - left) / count as f64;
    let bw = group * 0.72 / series.len() as f64;
    for (i, values) in values.iter().enumerate() {
        let c = palette[i % palette.len()];
        for (j, v) in values.iter().enumerate() {
            let xx = left + group * j as f64 + group * 0.2 + bw * i as f64;
            let yy = bottom - v / ymax * (bottom - top);
            let depth = bw * 0.42;
            let path = |points, fill| Op::Path {
                points,
                closed: true,
                width: 0.,
                color: fill,
                fill: Some(fill),
            };
            out.push(Op::Rect {
                x: xx,
                y: yy,
                w: bw,
                h: bottom - yy,
                fill: c,
            });
            out.push(path(
                vec![
                    (xx, yy),
                    (xx + depth, yy - depth * 0.6),
                    (xx + bw + depth, yy - depth * 0.6),
                    (xx + bw, yy),
                ],
                c.map(|v| v + (1. - v) * 0.3),
            ));
            out.push(path(
                vec![
                    (xx + bw, yy),
                    (xx + bw + depth, yy - depth * 0.6),
                    (xx + bw + depth, bottom - depth * 0.6),
                    (xx + bw, bottom),
                ],
                c.map(|v| v * 0.5),
            ));
        }
        let label = series[i]
            .child("tx")
            .and_then(|n| n.find("v"))
            .map(crate::model::text)
            .unwrap_or_else(|| format!("Series {}", i + 1));
        let yy = (top + bottom) / 2.
            + size * 0.5
            + (i as f64 - (series.len() - 1) as f64 / 2.) * size * 1.4;
        out.push(marker(right + size * 4., yy, 1, c));
        out.push(text(right + size * 4.7, yy + 3.5, label));
    }
    for (i, label) in labels.iter().enumerate() {
        let xx = left + (i as f64 + 0.5) * group;
        out.push(text(
            xx - fonts.width(f, label, size) / 2.,
            bottom + size * 2.,
            label.clone(),
        ));
    }
    Ok(out)
}
