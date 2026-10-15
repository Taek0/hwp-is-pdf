//! Conservative polygon projection over a complete text-line band.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Region {
    pub points: Vec<(f64, f64)>,
    pub margin: [f64; 4],
    pub flow: String,
}
impl Region {
    pub fn bounds(&self) -> [f64; 4] {
        [
            self.points
                .iter()
                .map(|p| p.0)
                .fold(f64::INFINITY, f64::min)
                - self.margin[0],
            self.points
                .iter()
                .map(|p| p.1)
                .fold(f64::INFINITY, f64::min)
                - self.margin[2],
            self.points
                .iter()
                .map(|p| p.0)
                .fold(f64::NEG_INFINITY, f64::max)
                + self.margin[1],
            self.points
                .iter()
                .map(|p| p.1)
                .fold(f64::NEG_INFINITY, f64::max)
                + self.margin[3],
        ]
    }
    pub fn shift_y(&mut self, dy: f64) {
        for p in &mut self.points {
            p.1 += dy;
        }
    }
    pub fn interval(&self, y: f64, h: f64) -> Option<(f64, f64)> {
        let low = y - self.margin[3];
        let high = y + h + self.margin[2];
        let mut xs = Vec::new();
        for i in 0..self.points.len() {
            let a = self.points[i];
            let b = self.points[(i + 1) % self.points.len()];
            if a.1 >= low && a.1 <= high {
                xs.push(a.0);
            }
            if (b.1 - a.1).abs() > 1e-9 {
                for edge in [low, high] {
                    let t = (edge - a.1) / (b.1 - a.1);
                    if (0.0..=1.0).contains(&t) {
                        xs.push(a.0 + t * (b.0 - a.0));
                    }
                }
            }
        }
        if xs.is_empty() {
            return None;
        }
        let left = xs.iter().copied().fold(f64::INFINITY, f64::min) - self.margin[0];
        let right = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) + self.margin[1];
        Some(match self.flow.as_str() {
            "LEFT_ONLY" => (left, f64::INFINITY),
            "RIGHT_ONLY" => (f64::NEG_INFINITY, right),
            _ => (left, right),
        })
    }
}
pub(crate) fn gap(
    regions: &[Region],
    y: f64,
    height: f64,
    left: f64,
    right: f64,
) -> Option<(f64, f64)> {
    let mut gaps = vec![(left, right)];
    for region in regions {
        if let Some((a, b)) = region.interval(y, height) {
            let mut next = Vec::new();
            for (l, r) in gaps {
                if b <= l || a >= r {
                    next.push((l, r));
                } else {
                    if a > l {
                        next.push((l, a.min(r)));
                    }
                    if b < r {
                        next.push((b.max(l), r));
                    }
                }
            }
            gaps = next;
        }
    }
    gaps.into_iter().filter(|(l, r)| r - l > 1.).max_by(|a, b| {
        (a.1 - a.0)
            .total_cmp(&(b.1 - b.0))
            .then_with(|| b.0.total_cmp(&a.0))
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sloped_outline_uses_whole_line_band() {
        let r = Region {
            points: vec![(0., 0.), (100., 100.), (0., 100.)],
            margin: [2.; 4],
            flow: String::new(),
        };
        let (a, b) = r.interval(20., 10.).unwrap();
        assert_eq!(a, -2.);
        assert!((b - 34.).abs() < 1e-9);
    }
    #[test]
    fn multiple_obstacles_choose_largest_open_interval() {
        let make = |a, b| Region {
            points: vec![(a, 0.), (b, 0.), (b, 100.), (a, 100.)],
            margin: [0.; 4],
            flow: String::new(),
        };
        assert_eq!(
            gap(&[make(30., 50.), make(90., 120.)], 10., 20., 0., 200.),
            Some((120., 200.))
        );
    }
    #[test]
    fn explicit_flow_direction_limits_the_available_side() {
        let mut r = Region {
            points: vec![(40., 0.), (60., 0.), (60., 100.), (40., 100.)],
            margin: [0.; 4],
            flow: "LEFT_ONLY".into(),
        };
        assert_eq!(gap(&[r.clone()], 10., 10., 0., 100.), Some((0., 40.)));
        r.flow = "RIGHT_ONLY".into();
        assert_eq!(gap(&[r], 10., 10., 0., 100.), Some((60., 100.)));
    }
}
