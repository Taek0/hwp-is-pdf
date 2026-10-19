//! Bounded vector bands for source-stepped gradients, with a raster test oracle.
use std::fmt::Write;
#[allow(clippy::too_many_arguments)]
pub(super) fn vector(
    kind: &str,
    angle: f64,
    center: [f64; 2],
    w: f64,
    h: f64,
    padding: f64,
    steps: usize,
    step_center: f64,
    colors: &[[f64; 3]],
) -> String {
    let mut out = format!("q 0 0 {w} {h} re W n\n");
    let center = (center[0] * w, (1. - center[1]) * h);
    let a = angle.to_radians();
    let steps = steps.clamp(2, 256);
    let pivot = step_center.clamp(0.001, 0.999);
    let distance = |t: f64| {
        if t <= 0.5 {
            2. * pivot * t
        } else {
            pivot + 2. * (1. - pivot) * (t - 0.5)
        }
    };
    let radius = (w + padding).hypot(h + padding) / 2.;
    let solid = |out: &mut String, t: f64| {
        let pos = t * (colors.len() - 1) as f64;
        let lo = (pos.floor() as usize).min(colors.len() - 1);
        let hi = (lo + 1).min(colors.len() - 1);
        let f = pos - lo as f64;
        let c: [f64; 3] = std::array::from_fn(|i| colors[lo][i] * (1. - f) + colors[hi][i] * f);
        writeln!(out, "{} {} {} rg", c[0], c[1], c[2]).unwrap();
    };
    solid(&mut out, 1.);
    writeln!(out, "0 0 {w} {h} re f").unwrap();
    let polygon = |out: &mut String, points: &[(f64, f64)]| {
        for (i, (x, y)) in points.iter().enumerate() {
            writeln!(out, "{x} {y} {}", if i == 0 { "m" } else { "l" }).unwrap();
        }
        out.push_str("h f\n");
    };
    for i in (0..steps).rev() {
        solid(&mut out, i as f64 / (steps - 1) as f64);
        let t = distance((i + 1) as f64 / steps as f64);
        match kind {
            "RADIAL" => {
                let r = radius * t;
                let (x, y) = center;
                let k = r * 0.5522847498307936;
                writeln!(out,"{} {y} m {} {} {} {} {x} {} c {} {} {} {} {} {y} c {} {} {} {} {x} {} c {} {} {} {} {} {y} c h f",x+r,x+r,y+k,x+k,y+r,y+r,x-k,y+r,x-r,y+k,x-r,x-r,y-k,x-k,y-r,y-r,x+k,y-r,x+r,y-k,x+r).unwrap();
            }
            "CONICAL" => {
                let lo = distance(i as f64 / steps as f64) * std::f64::consts::PI;
                let hi = t * std::f64::consts::PI;
                // Cover the entire clipped box even for off-center origins.
                let r = 2. * (w.hypot(h) + center.0.abs() + center.1.abs());
                for sign in [-1., 1.] {
                    let mut points = vec![center];
                    for j in 0..=4 {
                        let theta = a + sign * (lo + (hi - lo) * j as f64 / 4.);
                        points.push((center.0 + r * theta.cos(), center.1 + r * theta.sin()));
                    }
                    polygon(&mut out, &points);
                }
            }
            "SQUARE" => {
                let r = (w + padding).max(h + padding) / 2. * t;
                let points: Vec<_> = [(-r, -r), (r, -r), (r, r), (-r, r)]
                    .into_iter()
                    .map(|(x, y)| {
                        (
                            center.0 + x * a.cos() - y * a.sin(),
                            center.1 + x * a.sin() + y * a.cos(),
                        )
                    })
                    .collect();
                polygon(&mut out, &points);
            }
            _ => {
                let d = ((w + padding) * a.sin().abs() + (h + padding) * a.cos().abs()) / 2. * t;
                let length = 2. * (w.hypot(h) + center.0.abs() + center.1.abs());
                let points: Vec<_> = [(-d, -length), (d, -length), (d, length), (-d, length)]
                    .into_iter()
                    .map(|(n, t)| {
                        (
                            center.0 + n * a.sin() - t * a.cos(),
                            center.1 + n * a.cos() + t * a.sin(),
                        )
                    })
                    .collect();
                polygon(&mut out, &points);
            }
        }
    }
    out.push_str("Q\n");
    out
}
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn raster(
    kind: &str,
    angle: f64,
    center: [f64; 2],
    aspect: f64,
    padding: f64,
    steps: usize,
    step_center: f64,
    colors: &[[f64; 3]],
) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(256 * 256 * 3);
    let a = angle.to_radians();
    for row in 0..256 {
        for col in 0..256 {
            let x = (col as f64 + 0.5) / 256.;
            let y = (row as f64 + 0.5) / 256.;
            let dx = x - center[0];
            let dy = (y - center[1]) * aspect;
            let rx = dx * a.cos() - dy * a.sin();
            let ry = dx * a.sin() + dy * a.cos();
            let t = match kind {
                "RADIAL" => (dx * 2.).hypot(dy * 2.) / (1. + padding).hypot(aspect + padding),
                "SQUARE" => {
                    (rx * 2.).abs().max((ry * 2.).abs()) / (1. + padding).max(aspect + padding)
                }
                "CONICAL" => {
                    (((-dy).atan2(dx) - a + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                        - std::f64::consts::PI)
                        .abs()
                        / std::f64::consts::PI
                }
                _ => {
                    2. * (dx * a.sin() - dy * a.cos()).abs()
                        / ((1. + padding) * a.sin().abs() + (aspect + padding) * a.cos().abs())
                            .max(0.01)
                }
            }
            .clamp(0., 1.);
            let t = if t <= step_center {
                0.5 * t / step_center
            } else {
                0.5 + 0.5 * (t - step_center) / (1. - step_center)
            };
            let t = ((t * steps as f64).ceil() - 1.).max(0.) / (steps - 1) as f64;
            let pos = t * (colors.len() - 1) as f64;
            let left = (pos.floor() as usize).min(colors.len() - 1);
            let right = (left + 1).min(colors.len() - 1);
            let mix = pos - left as f64;
            for (a, b) in colors[left].iter().zip(colors[right].iter()) {
                pixels.push(((a * (1. - mix) + b * mix) * 255.).round().clamp(0., 255.) as u8);
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vector_gradients_are_clipped_and_bound_the_number_of_bands() {
        for kind in ["LINEAR", "RADIAL", "SQUARE", "CONICAL"] {
            let p = vector(
                kind,
                45.,
                [0.5, 0.5],
                160.,
                42.,
                1.,
                50,
                0.5,
                &[[0., 0., 0.], [1., 1., 1.]],
            );
            assert!(p.starts_with("q 0 0 160 42 re W n"));
            assert!(p.ends_with("Q\n"));
            assert_eq!(p.matches(" rg\n").count(), 51);
            assert!(!p.contains("NaN"));
            assert!(!p.contains("/Image"));
            if kind == "RADIAL" {
                assert_eq!(p.matches(" c").count(), 200);
            }
        }
    }
    #[test]
    fn two_step_native_schedule_is_a_horizontal_ten_percent_band() {
        let p = raster(
            "LINEAR",
            0.,
            [0.5, 0.5],
            2.64,
            0.,
            2,
            0.1,
            &[[0.; 3], [1.; 3]],
        );
        let black_rows = (0..256).filter(|r| p[(r * 256 + 128) * 3] == 0).count();
        assert!((24..=28).contains(&black_rows));
        assert_eq!(p[(128 * 256) * 3], 0);
        assert_eq!(p[128 * 3], 255);
    }
    #[test]
    fn radial_distance_uses_physical_aspect_not_square_pixels() {
        let p = raster(
            "RADIAL",
            0.,
            [0.5, 0.5],
            0.25,
            0.,
            50,
            0.5,
            &[[0.; 3], [1.; 3]],
        );
        let at = |x: usize, y: usize| p[(y * 256 + x) * 3];
        assert!(at(128, 128) < 3);
        assert!(at(128, 0) < 70);
        assert!(at(0, 128) > 240);
        assert_eq!(at(0, 0), 255);
        assert_eq!(p.len(), 256 * 256 * 3);
    }
}
