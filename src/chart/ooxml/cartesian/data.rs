//! Validated chart caches and numeric axis ranges.
use super::*;
pub(crate) fn supported(root: &Node) -> bool {
    let Some(plot) = root.find("plotArea") else {
        return false;
    };
    let charts: Vec<_> = plot
        .children
        .iter()
        .filter(|n| n.name.ends_with("Chart"))
        .collect();
    if charts.len() != 1 || root.find("manualLayout").is_some() {
        return false;
    }
    if plot.children_named("valAx").count() != 1 || plot.children_named("catAx").count() != 1 {
        return false;
    }
    if root.find("legendPos").is_some_and(|p| p.a("val") != "r") {
        return false;
    }
    if root.find("dPt").is_some() {
        return false;
    }
    if root.find("gradFill").is_some() || root.find("pattFill").is_some() {
        return false;
    }
    if ["logBase", "lumMod", "lumOff", "tint", "shade", "alpha"]
        .iter()
        .any(|tag| root.find(tag).is_some())
    {
        return false;
    }
    if ["valAx", "catAx"].iter().any(|tag| {
        plot.child(tag)
            .and_then(|n| n.child("delete"))
            .is_some_and(|n| n.a("val") == "1")
    }) {
        return false;
    }
    if root
        .find("legend")
        .and_then(|n| n.child("overlay"))
        .is_some_and(|n| n.a("val") == "1")
    {
        return false;
    }
    for shape in [root.child("spPr"), plot.child("spPr")]
        .into_iter()
        .flatten()
    {
        if shape.child("noFill").is_some()
            && root.child("spPr").is_some_and(|n| std::ptr::eq(n, shape))
        {
            return false;
        }
        if let Some(fill) = shape.child("solidFill")
            && !fill
                .child("srgbClr")
                .is_some_and(|c| c.a("val").eq_ignore_ascii_case("ffffff"))
        {
            return false;
        }
    }
    if plot
        .child("valAx")
        .and_then(|a| a.child("numFmt"))
        .is_some_and(|n| !matches!(n.a("formatCode"), "General" | "0" | "0%"))
    {
        return false;
    }
    if ["showVal", "showCatName", "showSerName", "showPercent"]
        .iter()
        .any(|key| root.find(key).is_some_and(|n| n.a("val") == "1"))
    {
        return false;
    }
    let n = charts[0];
    for s in n.children_named("ser") {
        if s.child("marker").and_then(|n| n.child("spPr")).is_some() {
            return false;
        }
        if s.child("marker")
            .and_then(|n| n.child("symbol"))
            .is_some_and(|n| {
                !matches!(
                    n.a("val"),
                    "auto" | "none" | "diamond" | "square" | "triangle" | "x" | "circle"
                )
            })
        {
            return false;
        }
        if let Some(line) = s.child("spPr").and_then(|n| n.child("ln")) {
            if n.name == "barChart" && line.child("noFill").is_none() {
                return false;
            }
            if line
                .child("prstDash")
                .is_some_and(|n| n.a("val") != "solid")
            {
                return false;
            }
        }
    }
    match n.name.as_str() {
        "barChart" => n
            .child("grouping")
            .is_none_or(|g| matches!(g.a("val"), "clustered" | "stacked" | "percentStacked")),
        "lineChart" => n.child("grouping").is_none_or(|g| g.a("val") == "standard"),
        "stockChart" => matches!(n.children_named("ser").count(), 3 | 4),
        _ => false,
    }
}
pub(super) fn values(n: &Node, key: &str) -> Result<Vec<String>> {
    let Some(cat) = n.child(key) else {
        return Ok(Vec::new());
    };
    let cache = cat
        .find("strCache")
        .or_else(|| cat.find("numCache"))
        .or_else(|| cat.find("strLit"))
        .or_else(|| cat.find("numLit"))
        .context("Missing chart label cache")?;
    let mut points: Vec<_> = cache.children_named("pt").collect();
    points.sort_by_key(|p| p.u("idx"));
    ensure!(
        points.len() <= 100000 && points.iter().enumerate().all(|(i, p)| p.u("idx") == i),
        "Invalid chart category indices"
    );
    Ok(points
        .into_iter()
        .map(|p| p.child("v").map(crate::model::text).unwrap_or_default())
        .collect())
}
pub(super) fn label(n: &Node) -> String {
    n.child("tx")
        .and_then(|n| n.find("v"))
        .map(crate::model::text)
        .unwrap_or_default()
}
pub(super) fn number(v: f64, percent: bool) -> String {
    let v = if percent { v * 100. } else { v };
    let mut s = if (v - v.round()).abs() < 1e-8 {
        format!("{:.0}", if v.abs() < 1e-8 { 0. } else { v })
    } else {
        format!("{v:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    };
    if percent {
        s.push('%');
    }
    s
}
pub(super) fn numeric_axis(lo: f64, hi: f64, horizontal: bool) -> (f64, f64, f64) {
    let range = if hi > lo { hi - lo } else { 1. };
    let rough = range * 1.1 / if horizontal { 6. } else { 4. };
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axis_density_respects_orientation_and_fractional_data() {
        assert_eq!(numeric_axis(0., 5., true), (0., 6., 1.));
        assert_eq!(numeric_axis(0., 5., false), (0., 6., 2.));
        let r = numeric_axis(0., 0.4, false);
        assert!((r.1 - 0.6).abs() < 1e-9 && (r.2 - 0.2).abs() < 1e-9);
    }
}
