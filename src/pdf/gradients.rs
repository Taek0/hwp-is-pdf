//! Bounded RGB sampling for HWP gradient fills; layout remains vector based.
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
