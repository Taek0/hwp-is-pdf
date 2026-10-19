//! Shared vertical reference semantics for saved and rebuilt object anchors.
pub(super) fn within_paper(n: &crate::model::Node, area: [f64; 4], paper: [f64; 2]) -> bool {
    let [x, y, mut w, mut h] = area;
    let angle = crate::graphics::source_angle(n);
    let flip = n.child("flip");
    let fx = flip.is_some_and(|f| f.a("horizontal") == "1");
    let fy = flip.is_some_and(|f| f.a("vertical") == "1");
    if (angle.rem_euclid(360.).abs() > 0.001 || fx || fy)
        && let Some(sz) = n
            .child("curSz")
            .filter(|s| s.n("width") > 0. && s.n("height") > 0.)
    {
        w = sz.n("width") / 100.;
        h = sz.n("height") / 100.;
    }
    let m = crate::graphics::rotation(x, y, w, h, angle, fx, fy);
    let r = crate::graphics::rect(m, [x, y, x + w, y + h]);
    r.iter().all(|v| v.is_finite())
        && r[0] >= -0.001
        && r[1] >= -0.001
        && r[2] <= paper[0] + 0.001
        && r[3] <= paper[1] + 0.001
}
pub(super) fn outside_inset(align: &str, leading: f64, trailing: f64) -> f64 {
    match align {
        "CENTER" => (leading - trailing) / 2.,
        "RIGHT" | "BOTTOM" => -trailing,
        _ => leading,
    }
}
pub(super) fn vertical(
    origin: f64,
    extent: f64,
    height: f64,
    reference: &str,
    align: &str,
    offset: f64,
) -> f64 {
    // Paragraph TOP/CENTER/BOTTOM all refer to the anchor line, not page height.
    if !matches!(reference, "PAGE" | "PAPER") {
        return origin + offset;
    }
    origin
        + match align {
            "CENTER" => (extent - height) / 2. + offset,
            "BOTTOM" => extent - height - offset,
            _ => offset,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paper_containment_uses_rotated_geometry_not_print_margins() {
        let mut n = crate::model::Node::default();
        assert!(within_paper(&n, [50., 50., 320., 80.], [400., 400.]));
        assert!(!within_paper(&n, [50., 50., 360., 80.], [400., 400.]));
        n.children
            .push(crate::model::xml(br#"<rotationInfo angle="90"/>"#).unwrap());
        // Native rotation anchors the resulting placement envelope at x/y;
        // a 90-degree turn therefore makes this box 80 wide and 320 high.
        assert!(within_paper(&n, [50., 50., 320., 80.], [400., 400.]));
        assert!(!within_paper(&n, [50., 150., 320., 80.], [400., 400.]));
    }
}
