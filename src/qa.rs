//! Conservative geometry checks on the same vector operations used for the PDF.
//! Intentional background images and text contained in boxes are not collisions.
use crate::{
    layout::Bounds,
    pdf::{Fonts, Op, Page},
};
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Default, Serialize, Debug)]
pub struct Audit {
    pub glyphs_checked: usize,
    pub outside_page: Vec<Finding>,
    pub text_overlaps: Vec<Finding>,
    pub border_crossings: Vec<Finding>,
    pub foreground_image_overlaps: Vec<Finding>,
    pub background_images_exempted: usize,
    pub intentional_overlays_exempted: usize,
}
#[derive(Serialize, Debug)]
pub struct Finding {
    pub page: usize,
    pub text: String,
    pub other: String,
    pub rect: [f64; 4],
}
struct Ink {
    r: [f64; 4],
    base: f64,
    text: char,
    font: usize,
    glyph: ttf_parser::GlyphId,
    x: f64,
    unit: f64,
    scale: f64,
}
#[derive(Default)]
struct Outline {
    paths: Vec<Vec<(f64, f64)>>,
    current: Vec<(f64, f64)>,
}
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.current.is_empty() {
            self.paths.push(std::mem::take(&mut self.current));
        }
        self.current.push((x as f64, y as f64));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push((x as f64, y as f64));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b) = *self.current.last().unwrap();
        for i in 1..=12 {
            let t = i as f64 / 12.;
            let s = 1. - t;
            self.current.push((
                s * s * a + 2. * s * t * x1 as f64 + t * t * x as f64,
                s * s * b + 2. * s * t * y1 as f64 + t * t * y as f64,
            ));
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b) = *self.current.last().unwrap();
        for i in 1..=16 {
            let t = i as f64 / 16.;
            let s = 1. - t;
            self.current.push((
                s * s * s * a
                    + 3. * s * s * t * x1 as f64
                    + 3. * s * t * t * x2 as f64
                    + t * t * t * x as f64,
                s * s * s * b
                    + 3. * s * s * t * y1 as f64
                    + 3. * s * t * t * y2 as f64
                    + t * t * t * y as f64,
            ));
        }
    }
    fn close(&mut self) {
        self.paths.push(std::mem::take(&mut self.current));
    }
}
impl Outline {
    fn contains(&self, x: f64, y: f64) -> bool {
        let mut winding = 0;
        for path in &self.paths {
            for (i, &(a, b)) in path.iter().enumerate() {
                let (c, d) = path[(i + 1) % path.len()];
                let side = (c - a) * (y - b) - (x - a) * (d - b);
                if b <= y && d > y && side > 0. {
                    winding += 1
                }
                if b > y && d <= y && side < 0. {
                    winding -= 1
                }
            }
        }
        winding != 0
    }
}
fn ink_intersects(a: &Ink, b: &Ink, faces: &[ttf_parser::Face]) -> bool {
    let mut aa = Outline::default();
    let mut bb = Outline::default();
    faces[a.font].outline_glyph(a.glyph, &mut aa);
    faces[b.font].outline_glyph(b.glyph, &mut bb);
    let x0 = a.r[0].max(b.r[0]);
    let y0 = a.r[1].max(b.r[1]);
    let w = a.r[2].min(b.r[2]) - x0;
    let h = a.r[3].min(b.r[3]) - y0;
    let nx = (w / 0.2).ceil().clamp(3., 60.) as usize;
    let ny = (h / 0.2).ceil().clamp(3., 60.) as usize;
    for i in 0..nx {
        for j in 0..ny {
            let x = x0 + (i as f64 + 0.5) * w / nx as f64;
            let y = y0 + (j as f64 + 0.5) * h / ny as f64;
            if aa.contains((x - a.x) / a.unit / a.scale, (a.base - y) / a.unit)
                && bb.contains((x - b.x) / b.unit / b.scale, (b.base - y) / b.unit)
            {
                return true;
            }
        }
    }
    false
}
pub fn inspect(pages: &[Page], fonts: &Fonts, bounds: &[Bounds]) -> Audit {
    let faces: Vec<_> = fonts
        .list
        .iter()
        .map(|f| ttf_parser::Face::parse(&f.bytes, 0).unwrap())
        .collect();
    let mut audit = Audit {
        background_images_exempted: bounds.iter().filter(|b| b.kind == "watermark").count(),
        intentional_overlays_exempted: bounds
            .iter()
            .filter(|b| b.kind == "intentional-overlay")
            .count(),
        ..Default::default()
    };
    for (pi, page) in pages.iter().enumerate() {
        let mut glyphs = Vec::new();
        for op in &page.ops {
            if let Op::Text {
                x,
                y,
                text,
                font,
                size,
                scale,
                ..
            } = op
            {
                let face = &faces[*font];
                let unit = *size / face.units_per_em() as f64;
                let mut x = *x;
                for c in text.chars() {
                    if let Some(g) = face.glyph_index(c) {
                        if let Some(bb) = face.glyph_bounding_box(g) {
                            glyphs.push(Ink {
                                r: [
                                    x + bb.x_min as f64 * unit * scale,
                                    *y - bb.y_max as f64 * unit,
                                    x + bb.x_max as f64 * unit * scale,
                                    *y - bb.y_min as f64 * unit,
                                ],
                                base: *y,
                                text: c,
                                font: *font,
                                glyph: g,
                                x,
                                unit,
                                scale: *scale,
                            });
                        }
                        x += face.glyph_hor_advance(g).unwrap_or(0) as f64 * unit * scale;
                    }
                }
            }
        }
        audit.glyphs_checked += glyphs.len();
        let mut buckets: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        for (i, g) in glyphs.iter().enumerate() {
            let [x0, y0, x1, y1] = g.r;
            let finding = |other: String| Finding {
                page: pi + 1,
                text: g.text.to_string(),
                other,
                rect: g.r,
            };
            if x0 < -0.5 || y0 < -0.5 || x1 > page.width + 0.5 || y1 > page.height + 0.5 {
                audit.outside_page.push(finding("page boundary".into()));
            }
            let mut seen = std::collections::BTreeSet::new();
            for gx in (x0 / 24.).floor() as i32..=(x1 / 24.).floor() as i32 {
                for gy in (y0 / 24.).floor() as i32..=(y1 / 24.).floor() as i32 {
                    if let Some(previous) = buckets.get(&(gx, gy)) {
                        for &j in previous {
                            if !seen.insert(j) {
                                continue;
                            }
                            let b = &glyphs[j];
                            let overlap_x = x1.min(b.r[2]) - x0.max(b.r[0]);
                            let overlap_y = y1.min(b.r[3]) - y0.max(b.r[1]);
                            let substantial_same_line = (g.base - b.base).abs() <= 2.
                                && overlap_x > (x1 - x0).min(b.r[2] - b.r[0]) * 0.45
                                && overlap_y > (y1 - y0).min(b.r[3] - b.r[1]) * 0.5;
                            let intentional = bounds.iter().any(|o| {
                                o.page == pi + 1
                                    && o.kind == "intentional-overlay"
                                    && x0 >= o.x - 1.
                                    && x1 <= o.x + o.w + 1.
                                    && y0 >= o.y - 1.
                                    && y1 <= o.y + o.h + 1.
                                    && b.r[0] >= o.x - 1.
                                    && b.r[2] <= o.x + o.w + 1.
                            });
                            if overlap_x > 0.6
                                && overlap_y > 0.6
                                && ((g.base - b.base).abs() > 2. || substantial_same_line)
                                && !intentional
                                && ink_intersects(g, b, &faces)
                            {
                                audit.text_overlaps.push(finding(b.text.to_string()));
                            }
                        }
                    }
                    buckets.entry((gx, gy)).or_default().push(i);
                }
            }
            for op in &page.ops {
                if let Op::Line {
                    x,
                    y,
                    x2,
                    y2,
                    width,
                    ..
                } = op
                {
                    let inset = 0.35 + width / 2.;
                    let hit = if (y - y2).abs() < 0.05 {
                        *y > y0 + inset
                            && *y < y1 - inset
                            && x.min(*x2) < x1 - 0.5
                            && x.max(*x2) > x0 + 0.5
                    } else if (x - x2).abs() < 0.05 {
                        *x > x0 + inset
                            && *x < x1 - inset
                            && y.min(*y2) < y1 - 0.5
                            && y.max(*y2) > y0 + 0.5
                    } else {
                        false
                    };
                    if hit {
                        audit
                            .border_crossings
                            .push(finding(format!("line ({x:.2},{y:.2})-({x2:.2},{y2:.2})")));
                    }
                }
            }
            for b in bounds
                .iter()
                .filter(|b| b.page == pi + 1 && b.kind == "pic")
            {
                let cx = (x0 + x1) / 2.;
                let cy = (y0 + y1) / 2.;
                if cx > b.x && cx < b.x + b.w && cy > b.y && cy < b.y + b.h {
                    audit.foreground_image_overlaps.push(finding(b.id.clone()));
                }
            }
        }
    }
    audit
}
