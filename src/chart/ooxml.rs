//! Numeric XY charts from OOXML caches; formulas are never executed.
use super::*;
use anyhow::Context;
fn cached(n: &Node, key: &str) -> Result<Vec<f64>> {
    let n = n.child(key).context("Missing XY values")?;
    let cache = n
        .find("numCache")
        .or_else(|| n.find("numLit"))
        .context("Missing numeric cache")?;
    let mut values: Vec<_> = cache.children_named("pt").collect();
    values.sort_by_key(|p| p.u("idx"));
    ensure!(values.len() <= 100000, "Excessive XY points");
    ensure!(
        values.iter().enumerate().all(|(i, p)| p.u("idx") == i),
        "Sparse or duplicate chart indices require explicit gap handling"
    );
    values
        .into_iter()
        .map(|p| {
            let value =
                crate::model::text(p.child("v").context("Missing XY point")?).parse::<f64>()?;
            ensure!(value.is_finite(), "Non-finite XY point");
            Ok(value)
        })
        .collect()
}
fn axis(values: impl Iterator<Item = f64>) -> (f64, f64, f64) {
    let (lo, hi) = values.fold((0f64, 0f64), |(a, b), v| (a.min(v), b.max(v)));
    let range = (hi - lo).max(1.);
    let rough = range * 1.1 / 6.;
    let power = 10f64.powf(rough.log10().floor());
    let step = [1., 2., 5., 10.]
        .into_iter()
        .find(|v| v * power >= rough)
        .unwrap_or(10.)
        * power;
    let min = (lo / step).floor() * step;
    let max = ((hi + range * 0.1) / step).ceil() * step;
    (min, max.max(min + step), step)
}
fn marker(x: f64, y: f64, i: usize, c: [f64; 3]) -> Op {
    let r = 2.5;
    let points = if i.is_multiple_of(2) {
        vec![(x, y - r), (x + r, y), (x, y + r), (x - r, y)]
    } else {
        vec![
            (x - r, y - r),
            (x + r, y - r),
            (x + r, y + r),
            (x - r, y + r),
        ]
    };
    Op::Path {
        points,
        closed: true,
        width: 0.,
        color: c,
        fill: Some(c),
    }
}
mod bars3d;
pub(crate) mod cartesian;
pub(super) use bars3d::draw as bars3d;
pub(super) fn scatter(
    root: &Node,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fonts: &mut Fonts,
) -> Result<Vec<Op>> {
    let chart = root.find("scatterChart").context("Missing scatter chart")?;
    ensure!(
        chart
            .child("scatterStyle")
            .is_some_and(|s| s.a("val") == "marker"),
        "Only marker XY style is supported"
    );
    let series: Vec<_> = chart.children_named("ser").collect();
    ensure!(
        !series.is_empty() && series.len() <= 64,
        "Invalid XY series count"
    );
    let mut data = Vec::new();
    for s in &series {
        let xs = cached(s, "xVal")?;
        let ys = cached(s, "yVal")?;
        ensure!(xs.len() == ys.len(), "XY cache lengths differ");
        data.push((xs, ys));
    }
    let xa = axis(data.iter().flat_map(|(xs, _)| xs.iter().copied()));
    let ya = axis(data.iter().flat_map(|(_, ys)| ys.iter().copied()));
    let f = fonts
        .family("함초롬돋움", false, false)?
        .context("Chart font unavailable")?;
    let size = root
        .child("txPr")
        .and_then(|n| n.find("defRPr"))
        .map(|n| n.n("sz") / 100.)
        .filter(|s| *s > 0.)
        .unwrap_or(10.);
    let (left, right, top, bottom) = (
        x + size * 2.4,
        x + w - size * 6.,
        y + size * 4.5,
        y + h - size * 3.,
    );
    ensure!(right > left && bottom > top, "XY plot too small");
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
    for i in 0..=((ya.1 - ya.0) / ya.2).round().min(100.) as usize {
        let value = ya.0 + i as f64 * ya.2;
        let yy = bottom - (value - ya.0) / (ya.1 - ya.0) * (bottom - top);
        out.push(line(left, yy, right, yy));
        out.push(text(
            left - size * 1.5,
            yy + size * 0.35,
            format!("{value}"),
        ));
    }
    out.push(line(left, top, left, bottom));
    for i in 0..=((xa.1 - xa.0) / xa.2).round().min(100.) as usize {
        let value = xa.0 + i as f64 * xa.2;
        let xx = left + (value - xa.0) / (xa.1 - xa.0) * (right - left);
        let label = format!("{value}");
        out.push(line(xx, bottom, xx, bottom + 3.));
        out.push(text(
            xx - fonts.width(f, &label, size) / 2.,
            bottom + size * 1.7,
            label,
        ));
    }
    let palette = [
        [97. / 255., 129. / 255., 213. / 255.],
        [254. / 255., 131. / 255., 58. / 255.],
        [0.65; 3],
    ];
    for (i, (xs, ys)) in data.iter().enumerate() {
        let c = series[i]
            .child("spPr")
            .and_then(|s| s.child("solidFill"))
            .and_then(|s| s.child("srgbClr"))
            .map(|n| rgb(n.a("val")))
            .unwrap_or(palette[i % palette.len()]);
        for (xx, yy) in xs.iter().zip(ys) {
            out.push(marker(
                left + (xx - xa.0) / (xa.1 - xa.0) * (right - left),
                bottom - (yy - ya.0) / (ya.1 - ya.0) * (bottom - top),
                i,
                c,
            ));
        }
        let label = series[i]
            .child("tx")
            .and_then(|n| n.find("v"))
            .map(crate::model::text)
            .unwrap_or_else(|| format!("Series {}", i + 1));
        let yy = (top + bottom) / 2.
            + size * 0.5
            + (i as f64 - (data.len() - 1) as f64 / 2.) * size * 1.4;
        out.push(marker(right + size * 2.2, yy, i, c));
        out.push(text(right + size * 2.9, yy + size * 0.35, label));
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_axes_keep_fractional_x_and_headroom() {
        assert_eq!(axis([0.7, 1.8, 2.6].into_iter()), (0., 3., 0.5));
        assert_eq!(axis([1., 2., 4.].into_iter()), (0., 5., 1.));
    }
}
