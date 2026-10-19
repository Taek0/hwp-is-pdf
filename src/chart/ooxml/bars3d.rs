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
    let view = root.find("view3D");
    let setting = |key: &str, default: f64| {
        view.and_then(|v| v.child(key))
            .map(|v| v.n("val"))
            .unwrap_or(default)
    };
    let depth_scale = (setting("depthPercent", 100.) / 100.).clamp(0.01, 20.);
    let rot_x = setting("rotX", 15.).clamp(-90., 90.).to_radians();
    let rot_y = setting("rotY", 20.).rem_euclid(360.).to_radians();
    let (dx, dy) = (
        size * 1.5 * depth_scale * rot_y.sin() / 20f64.to_radians().sin(),
        -size * depth_scale * rot_x.sin() / 15f64.to_radians().sin(),
    );
    let mut out = Vec::new();
    let line = |x, y, x2, y2| Op::Line {
        x,
        y,
        x2,
        y2,
        width: 0.5,
        color: [134. / 255.; 3],
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
        [97. / 255., 130. / 255., 214. / 255.],
        [1., 132. / 255., 58. / 255.],
        [178. / 255.; 3],
    ];
    let group = (right - left) / count as f64;
    let gap = chart
        .child("gapWidth")
        .map(|n| n.n("val") / 100.)
        .unwrap_or(1.5)
        .clamp(0., 5.);
    let depth_gap = chart
        .child("gapDepth")
        .map(|n| n.n("val") / 100.)
        .unwrap_or(1.5)
        .clamp(0., 5.);
    let bw = group / (series.len() as f64 + gap);
    let z = depth_gap / (2. * (1. + depth_gap));
    let (bar_dx, bar_dy) = (dx / (1. + depth_gap), dy / (1. + depth_gap));
    for (i, values) in values.iter().enumerate() {
        let c = palette[i % palette.len()];
        for (j, v) in values.iter().enumerate() {
            let xx = left + group * j as f64 + gap * bw / 2. + bw * i as f64 + dx * z;
            let bar_bottom = bottom + dy * z;
            let yy = bar_bottom - v / ymax * (bottom - top);
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
                h: bar_bottom - yy,
                fill: c,
            });
            out.push(path(
                vec![
                    (xx, yy),
                    (xx + bar_dx, yy + bar_dy),
                    (xx + bw + bar_dx, yy + bar_dy),
                    (xx + bw, yy),
                ],
                c.map(|v| v * 0.72),
            ));
            out.push(path(
                vec![
                    (xx + bw, yy),
                    (xx + bw + bar_dx, yy + bar_dy),
                    (xx + bw + bar_dx, bar_bottom + bar_dy),
                    (xx + bw, bar_bottom),
                ],
                c.map(|v| v * 0.486),
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
