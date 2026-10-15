//! Conservative polygon projection over a complete text-line band.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Region {
    pub points: Vec<(f64, f64)>,
    #[serde(default)]
    pub parts: Option<Vec<Vec<(f64, f64)>>>,
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
        if let Some(parts) = &mut self.parts {
            for part in parts {
                for p in part {
                    p.1 += dy;
                }
            }
        }
    }
    pub fn interval(&self, y: f64, h: f64) -> Option<(f64, f64)> {
        self.project(&self.points, y, h)
    }
    pub fn intervals(&self, y: f64, h: f64) -> Vec<(f64, f64)> {
        if let Some(parts) = &self.parts {
            parts.iter().filter_map(|p| self.project(p, y, h)).collect()
        } else {
            self.filled_intervals(y, h)
        }
    }
    fn filled_intervals(&self, y: f64, h: f64) -> Vec<(f64, f64)> {
        if self.points.len() > 4096 {
            return self.interval(y, h).into_iter().collect();
        }
        let low = y - self.margin[3];
        let high = y + h + self.margin[2];
        let mut samples = vec![low, high, (low + high) / 2.];
        for i in 0..self.points.len() {
            let p = self.points[i];
            let a = self.points[(i + self.points.len() - 1) % self.points.len()];
            let b = self.points[(i + 1) % self.points.len()];
            if p.1 >= low
                && p.1 <= high
                && ((p.0 - a.0) * (b.0 - p.0) <= 0. || (p.1 - a.1) * (b.1 - p.1) <= 0.)
            {
                for dy in [-0.000001, 0., 0.000001] {
                    samples.push((p.1 + dy).clamp(low, high));
                }
            }
        }
        let mut intervals = Vec::new();
        for scan in samples {
            let mut crossings = Vec::new();
            for i in 0..self.points.len() {
                let a = self.points[i];
                let b = self.points[(i + 1) % self.points.len()];
                if (a.1 <= scan && scan < b.1) || (b.1 <= scan && scan < a.1) {
                    crossings.push((
                        a.0 + (scan - a.1) * (b.0 - a.0) / (b.1 - a.1),
                        if b.1 > a.1 { 1 } else { -1 },
                    ));
                }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            let mut begin = 0.;
            for (x, delta) in crossings {
                let old = winding;
                winding += delta;
                if old == 0 && winding != 0 {
                    begin = x;
                }
                if old != 0 && winding == 0 && x > begin {
                    let a = begin - self.margin[0];
                    let b = x + self.margin[1];
                    intervals.push(match self.flow.as_str() {
                        "LEFT_ONLY" => (a, f64::INFINITY),
                        "RIGHT_ONLY" => (f64::NEG_INFINITY, b),
                        _ => (a, b),
                    });
                }
            }
        }
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (a, b) in intervals {
            if let Some(last) = merged.last_mut().filter(|last| last.1 >= a) {
                last.1 = last.1.max(b);
            } else {
                merged.push((a, b));
            }
        }
        merged
    }
    fn project(&self, points: &[(f64, f64)], y: f64, h: f64) -> Option<(f64, f64)> {
        let low = y - self.margin[3];
        let high = y + h + self.margin[2];
        let mut xs = Vec::new();
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
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
#[cfg(test)]
pub(crate) fn gap(
    regions: &[Region],
    y: f64,
    height: f64,
    left: f64,
    right: f64,
) -> Option<(f64, f64)> {
    gaps(regions, y, height, left, right)
        .into_iter()
        .max_by(|a, b| {
            (a.1 - a.0)
                .total_cmp(&(b.1 - b.0))
                .then_with(|| b.0.total_cmp(&a.0))
        })
}
pub(crate) fn gaps(
    regions: &[Region],
    y: f64,
    height: f64,
    left: f64,
    right: f64,
) -> Vec<(f64, f64)> {
    let mut gaps = vec![(left, right)];
    for region in regions {
        for (a, b) in region.intervals(y, height) {
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
    gaps.into_iter().filter(|(l, r)| r - l > 1.).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonzero_winding_hole_is_usable_only_when_the_entire_band_fits() {
        let r = Region {
            parts: None,
            points: vec![
                (0., 0.),
                (100., 0.),
                (100., 100.),
                (0., 100.),
                (0., 0.),
                (30., 30.),
                (30., 70.),
                (70., 70.),
                (70., 30.),
                (30., 30.),
                (0., 0.),
            ],
            margin: [0.; 4],
            flow: String::new(),
        };
        assert_eq!(
            gap(std::slice::from_ref(&r), 40., 10., 0., 100.),
            Some((30., 70.))
        );
        assert_eq!(gap(&[r], 40., 40., 0., 100.), None);
    }
    #[test]
    fn sloped_outline_uses_whole_line_band() {
        let r = Region {
            parts: None,
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
            parts: None,
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
            parts: None,
            points: vec![(40., 0.), (60., 0.), (60., 100.), (40., 100.)],
            margin: [0.; 4],
            flow: "LEFT_ONLY".into(),
        };
        assert_eq!(gap(&[r.clone()], 10., 10., 0., 100.), Some((0., 40.)));
        r.flow = "RIGHT_ONLY".into();
        assert_eq!(gap(&[r], 10., 10., 0., 100.), Some((60., 100.)));
    }
}
