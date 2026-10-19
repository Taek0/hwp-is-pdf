//! Endpoint ornaments in the line's local tangent/normal frame.
use super::*;
pub(super) fn paint(n: &Node, start: (f64, f64), end: (f64, f64)) -> Vec<Op> {
    let Some(line) = n.child("lineShape") else {
        return Vec::new();
    };
    if line.a("style") == "NONE" {
        return Vec::new();
    }
    let length = (end.0 - start.0).hypot(end.1 - start.1);
    if length < 1e-6 {
        return Vec::new();
    }
    let unit = val(Some(line), "width").max(0.1);
    let c = color(line.a("color"));
    let mut out = Vec::new();
    for (prefix, p, sign) in [("head", start, 1.), ("tail", end, -1.)] {
        let kind = line.a(&format!("{prefix}Style"));
        if matches!(kind, "" | "NORMAL" | "NONE") {
            continue;
        }
        let sz = line.a(&format!("{prefix}Sz"));
        let dims: Vec<_> = sz.split('_').collect();
        let arrow = matches!(kind, "ARROW" | "SPEAR" | "CONCAVE_ARROW");
        let size = |v: &str| {
            unit * match (v, arrow) {
                ("SMALL", true) => 5.76,
                ("LARGE", true) => 17.28,
                (_, true) => 11.52,
                ("SMALL", false) => 4.8,
                ("LARGE", false) => 17.28,
                _ => 9.6,
            }
        };
        let w = size(dims.first().copied().unwrap_or("MEDIUM"));
        let h = size(dims.get(1).copied().unwrap_or("MEDIUM"));
        let u = (
            (end.0 - start.0) / length * sign,
            (end.1 - start.1) / length * sign,
        );
        let point = |x: f64, y: f64| (p.0 + u.0 * x - u.1 * y, p.1 + u.1 * x + u.0 * y);
        let filled = line.a(&format!("{prefix}fill")) != "0";
        let mut points = match kind {
            "ARROW" | "SPEAR" => vec![point(w, -h / 2.), p, point(w, h / 2.)],
            "CONCAVE_ARROW" => vec![point(w, -h / 2.), p, point(w, h / 2.), point(w * 0.66, 0.)],
            "EMPTY_DIAMOND" => vec![
                point(-w / 2., 0.),
                point(0., -h / 2.),
                point(w / 2., 0.),
                point(0., h / 2.),
            ],
            "EMPTY_BOX" => vec![
                point(-w / 2., -h / 2.),
                point(w / 2., -h / 2.),
                point(w / 2., h / 2.),
                point(-w / 2., h / 2.),
            ],
            "EMPTY_CIRCLE" => (0..64)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 64.;
                    point(w / 2. * a.cos(), h / 2. * a.sin())
                })
                .collect(),
            _ => continue,
        };
        let open = kind == "SPEAR";
        if !open {
            points.push(points[0]);
        }
        out.push(Op::Path {
            points,
            closed: !open,
            width: if filled && !open { 0. } else { unit },
            color: c,
            fill: if filled && !open { Some(c) } else { None },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reversed_line_keeps_heads_at_endpoints_and_spear_unfilled() {
        let n=crate::model::xml(br##"<line><lineShape style="SOLID" width="100" color="#185890" headStyle="ARROW" tailStyle="SPEAR" headSz="MEDIUM_MEDIUM" tailSz="LARGE_LARGE" headfill="1" tailfill="1"/></line>"##).unwrap();
        let ops = paint(&n, (100., 20.), (20., 20.));
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[0],Op::Path{points,fill:Some(_),..} if points.contains(&(100.,20.))));
        assert!(
            matches!(&ops[1],Op::Path{points,closed:false,fill:None,..} if points.contains(&(20.,20.)))
        );
        assert!(paint(&n, (0., 0.), (0., 0.)).is_empty());
    }
}
