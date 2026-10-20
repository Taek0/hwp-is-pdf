//! Explicit diagonal cell borders, separate from the four frame edges.
use super::*;

pub(super) fn paint(border: &Node, rect: [f64; 4]) -> Vec<Op> {
    let [x, y, w, h] = rect;
    let Some(style) = border.child("diagonal").filter(|s| s.a("type") != "NONE") else {
        return Vec::new();
    };
    if w <= 0. || h <= 0. {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (name, ascending) in [("slash", true), ("backSlash", false)] {
        let Some(diagonal) = border.child(name) else {
            continue;
        };
        if !matches!(diagonal.a("type"), "2" | "CENTER")
            || !matches!(diagonal.a("Crooked"), "" | "0" | "false")
        {
            continue;
        }
        let (a, b) = if ascending {
            ((x, y + h), (x + w, y))
        } else {
            ((x, y), (x + w, y + h))
        };
        out.extend(border_strokes::stroke(style, a, b));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_directions_are_independent_of_the_diagonal_stroke_definition() {
        for (kind, direction) in [("NONE", 0), ("2", 1), ("CENTER", 1)] {
            let n=crate::model::xml(format!(r##"<borderFill><slash type="{kind}"/><backSlash type="NONE"/><diagonal type="SOLID" width="0.1 mm" color="#000000"/></borderFill>"##).as_bytes()).unwrap();
            let ops = paint(&n, [10., 20., 40., 15.]);
            assert_eq!(ops.len(), direction);
            if direction == 1 {
                assert!(matches!(
                    ops[0],
                    Op::Line {
                        x: 10.,
                        y: 35.,
                        x2: 50.,
                        y2: 20.,
                        ..
                    }
                ));
            }
        }
        let n=crate::model::xml(br##"<borderFill><backSlash type="2"/><diagonal type="SOLID" width="0.1 mm" color="#000000"/></borderFill>"##).unwrap();
        assert!(matches!(
            paint(&n, [10., 20., 40., 15.])[0],
            Op::Line {
                x: 10.,
                y: 20.,
                x2: 50.,
                y2: 35.,
                ..
            }
        ));
    }
}
