//! Preserve directed line endpoints within the declared object bounds.
use crate::model::Node;
pub(super) fn endpoints(n: &Node, x: f64, y: f64, w: f64, h: f64) -> ((f64, f64), (f64, f64)) {
    let (Some(a), Some(b)) = (n.child("startPt"), n.child("endPt")) else {
        return ((x, y), (x + w, y + h));
    };
    let axis = |start: f64, end: f64, origin: f64, extent: f64| {
        if end > start {
            (origin, origin + extent)
        } else if end < start {
            (origin + extent, origin)
        } else {
            (origin, origin)
        }
    };
    let (x1, x2) = axis(a.signed("x"), b.signed("x"), x, w);
    let (y1, y2) = axis(a.signed("y"), b.signed("y"), y, h);
    ((x1, y1), (x2, y2))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_order_preserves_rising_reversed_and_horizontal_lines() {
        for (start, end, expected) in [
            ((0, 100), (200, 0), ((10., 60.), (90., 20.))),
            ((200, 100), (0, 0), ((90., 60.), (10., 20.))),
            ((200, 0), (0, 0), ((90., 20.), (10., 20.))),
        ] {
            let n = crate::model::xml(
                format!(
                    r#"<line><startPt x="{}" y="{}"/><endPt x="{}" y="{}"/></line>"#,
                    start.0, start.1, end.0, end.1
                )
                .as_bytes(),
            )
            .unwrap();
            assert_eq!(endpoints(&n, 10., 20., 80., 40.), expected);
        }
    }
}
