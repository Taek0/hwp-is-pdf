//! Page-space geometry uses x right, y down, and points (1/72 inch).
pub type Matrix = [f64; 6];
pub const IDENTITY: Matrix = [1., 0., 0., 1., 0., 0.];
/// Rendering matrices are executable geometry; rotationInfo can be stale UI metadata.
pub fn source_angle(n: &crate::model::Node) -> f64 {
    if let Some(info) = n.child("renderingInfo") {
        let rotations: Vec<_> = info.children_named("rotMatrix").collect();
        if !rotations.is_empty()
            && rotations
                .iter()
                .all(|m| m.n("e1").hypot(m.n("e4")) > 0.000001)
        {
            return rotations
                .iter()
                .map(|m| m.n("e4").atan2(m.n("e1")).to_degrees())
                .sum();
        }
    }
    n.child("rotationInfo").map(|r| r.n("angle")).unwrap_or(0.)
}
/// Adaptive cubic subdivision with a conservative page-space tolerance.
pub(crate) fn flatten_cubics(start: (f64, f64), segments: &[[f64; 6]]) -> Vec<(f64, f64)> {
    fn split(p: [(f64, f64); 4], depth: usize, out: &mut Vec<(f64, f64)>) {
        let distance = |q: (f64, f64)| {
            let (dx, dy) = (p[3].0 - p[0].0, p[3].1 - p[0].1);
            ((q.0 - p[0].0) * dy - (q.1 - p[0].1) * dx).abs() / dx.hypot(dy).max(0.0001)
        };
        if depth >= 12
            || (distance(p[1]).max(distance(p[2])) < 0.1
                && (p[1].0 - p[0].0).hypot(p[1].1 - p[0].1)
                    + (p[2].0 - p[1].0).hypot(p[2].1 - p[1].1)
                    + (p[3].0 - p[2].0).hypot(p[3].1 - p[2].1)
                    < (p[3].0 - p[0].0).hypot(p[3].1 - p[0].1) + 0.2)
        {
            out.push(p[3]);
            return;
        }
        let mid = |a: (f64, f64), b: (f64, f64)| ((a.0 + b.0) / 2., (a.1 + b.1) / 2.);
        let a = mid(p[0], p[1]);
        let b = mid(p[1], p[2]);
        let c = mid(p[2], p[3]);
        let d = mid(a, b);
        let e = mid(b, c);
        let f = mid(d, e);
        split([p[0], a, d, f], depth + 1, out);
        split([f, e, c, p[3]], depth + 1, out);
    }
    let mut out = vec![start];
    let mut at = start;
    for s in segments {
        let end = (s[4], s[5]);
        split([at, (s[0], s[1]), (s[2], s[3]), end], 0, &mut out);
        at = end;
    }
    out
}
pub fn point(m: Matrix, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}
pub fn compose(a: Matrix, b: Matrix) -> Matrix {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}
pub fn inverse(m: Matrix) -> Matrix {
    let det = m[0] * m[3] - m[1] * m[2];
    [
        m[3] / det,
        -m[1] / det,
        -m[2] / det,
        m[0] / det,
        (m[2] * m[5] - m[3] * m[4]) / det,
        (m[1] * m[4] - m[0] * m[5]) / det,
    ]
}
pub fn rect(m: Matrix, r: [f64; 4]) -> [f64; 4] {
    let p = [
        point(m, r[0], r[1]),
        point(m, r[2], r[1]),
        point(m, r[2], r[3]),
        point(m, r[0], r[3]),
    ];
    [
        p.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
        p.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
        p.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
        p.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
    ]
}
/// Rotate the unrotated body into its axis-aligned placement envelope.
pub fn rotation(x: f64, y: f64, w: f64, h: f64, angle: f64, flip_x: bool, flip_y: bool) -> Matrix {
    let a = angle.to_radians();
    let sx = if flip_x { -1. } else { 1. };
    let sy = if flip_y { -1. } else { 1. };
    let mut m = [
        a.cos() * sx,
        a.sin() * sx,
        -a.sin() * sy,
        a.cos() * sy,
        0.,
        0.,
    ];
    for v in &mut m[..4] {
        if (*v - v.round()).abs() < 1e-12 {
            *v = v.round();
        }
    }
    let b = rect(m, [0., 0., w, h]);
    m[4] = x - b[0] - m[0] * x - m[2] * y;
    m[5] = y - b[1] - m[1] * x - m[3] * y;
    m
}
/// Interpolating cardinal spline; line flags stay straight, closed knots wrap.
/// Tension 0.5 is a rendering choice, not a claim of Hancom pixel equivalence.
pub fn curve(points: &[(f64, f64)], kinds: &[usize]) -> Vec<[f64; 6]> {
    if points.len() < 2 {
        return vec![];
    }
    let closed = points.len() > 3 && points.first() == points.last();
    let n = points.len();
    let mut out = vec![];
    for i in 0..n - 1 {
        let p = points[i];
        let q = points[i + 1];
        let (a, b) = if kinds.get(i) == Some(&1) {
            let prev = if i > 0 {
                points[i - 1]
            } else if closed {
                points[n - 2]
            } else {
                p
            };
            let next = if i + 2 < n {
                points[i + 2]
            } else if closed {
                points[1]
            } else {
                q
            };
            (
                (p.0 + (q.0 - prev.0) / 6., p.1 + (q.1 - prev.1) / 6.),
                (q.0 - (next.0 - p.0) / 6., q.1 - (next.1 - p.1) / 6.),
            )
        } else {
            (
                (p.0 + (q.0 - p.0) / 3., p.1 + (q.1 - p.1) / 3.),
                (p.0 + 2. * (q.0 - p.0) / 3., p.1 + 2. * (q.1 - p.1) / 3.),
            )
        };
        out.push([a.0, a.1, b.0, b.1, q.0, q.1]);
    }
    out
}
