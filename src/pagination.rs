//! Indivisible vector groups used when splitting mixed-content tables.
use crate::{
    graphics::{self, Matrix},
    pdf::{Fonts, Op},
};
use anyhow::{Result, ensure};
pub(crate) struct Atom {
    pub start: usize,
    pub end: usize,
    pub top: f64,
    pub bottom: f64,
}
pub(crate) fn bounds(op: &Op, fonts: &Fonts) -> Option<[f64; 4]> {
    match op {
        Op::GlyphPaint {
            x,
            y,
            font,
            glyph,
            size,
            scale,
            ..
        } => {
            let face = ttf_parser::Face::parse(&fonts.list[*font].bytes, 0).ok()?;
            let b = face.glyph_bounding_box(ttf_parser::GlyphId(*glyph))?;
            let u = size / face.units_per_em() as f64;
            Some([
                x + b.x_min as f64 * u * scale,
                y - b.y_max as f64 * u,
                x + b.x_max as f64 * u * scale,
                y - b.y_min as f64 * u,
            ])
        }
        Op::FieldText {
            x, y, width, size, ..
        } => Some([*x, y - size * 1.2, x + width, y + size * 0.4]),
        Op::Shaped {
            x,
            y,
            size,
            scale,
            font,
            glyphs,
            ..
        } => {
            let face = ttf_parser::Face::parse(&fonts.list[*font].bytes, 0).ok()?;
            let unit = size / face.units_per_em() as f64;
            let mut points = Vec::new();
            for g in glyphs {
                if let Some(b) = face.glyph_bounding_box(ttf_parser::GlyphId(g.id)) {
                    points.extend([
                        (
                            x + g.x + b.x_min as f64 * unit * scale,
                            y + g.y - b.y_max as f64 * unit,
                        ),
                        (
                            x + g.x + b.x_max as f64 * unit * scale,
                            y + g.y - b.y_min as f64 * unit,
                        ),
                    ]);
                }
            }
            points_bounds(&points, fonts.ink_padding(*font, *size) * 2.)
        }
        Op::Text {
            x,
            y,
            size,
            scale,
            font,
            text,
            ..
        } => {
            let face = ttf_parser::Face::parse(&fonts.list[*font].bytes, 0).ok()?;
            let unit = size / face.units_per_em() as f64;
            let mut cursor = *x;
            let mut points = Vec::new();
            for c in text.chars() {
                if let Some(gid) = crate::pdf::glyph_id(&face, c) {
                    if let Some(bb) = face.glyph_bounding_box(gid) {
                        points.extend([
                            (
                                cursor + bb.x_min as f64 * unit * scale,
                                y - bb.y_max as f64 * unit,
                            ),
                            (
                                cursor + bb.x_max as f64 * unit * scale,
                                y - bb.y_min as f64 * unit,
                            ),
                        ]);
                    }
                    cursor += face.glyph_hor_advance(gid).unwrap_or(face.units_per_em()) as f64
                        * unit
                        * scale;
                }
            }
            Some(
                points_bounds(&points, fonts.ink_padding(*font, *size) * 2.)
                    .unwrap_or([*x, *y, cursor, *y]),
            )
        }
        Op::Rect { x, y, w, h, .. }
        | Op::Image { x, y, w, h, .. }
        | Op::Gradient { x, y, w, h, .. }
        | Op::ImageTile { x, y, w, h, .. }
        | Op::ImageClip { x, y, w, h, .. } => Some([*x, *y, x + w, y + h]),
        Op::Line {
            x,
            y,
            x2,
            y2,
            width,
            ..
        } => Some([
            x.min(*x2) - width / 2.,
            y.min(*y2) - width / 2.,
            x.max(*x2) + width / 2.,
            y.max(*y2) + width / 2.,
        ]),
        Op::Path { points, width, .. } => points_bounds(points, *width),
        Op::Bezier {
            start,
            segments,
            width,
            ..
        } => {
            let mut p = vec![*start];
            for s in segments {
                p.extend([(s[0], s[1]), (s[2], s[3]), (s[4], s[5])]);
            }
            points_bounds(&p, *width)
        }
        _ => None,
    }
}
fn points_bounds(p: &[(f64, f64)], width: f64) -> Option<[f64; 4]> {
    if p.is_empty() {
        return None;
    }
    Some([
        p.iter().map(|p| p.0).fold(f64::INFINITY, f64::min) - width / 2.,
        p.iter().map(|p| p.1).fold(f64::INFINITY, f64::min) - width / 2.,
        p.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max) + width / 2.,
        p.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max) + width / 2.,
    ])
}
pub(crate) fn atoms(ops: &[Op], fonts: &Fonts) -> Result<Vec<Atom>> {
    let mut out = Vec::new();
    let mut stack: Vec<Matrix> = Vec::new();
    let mut matrix = graphics::IDENTITY;
    let mut start = 0;
    let mut top = f64::INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::TransformBegin { matrix: m } => {
                if stack.is_empty() {
                    start = i;
                    top = f64::INFINITY;
                    bottom = f64::NEG_INFINITY;
                }
                stack.push(matrix);
                matrix = graphics::compose(matrix, *m);
            }
            Op::TransformEnd => {
                matrix = stack
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("Unbalanced table transform"))?;
                if stack.is_empty() {
                    out.push(Atom {
                        start,
                        end: i + 1,
                        top: if top.is_finite() { top } else { 0. },
                        bottom: if bottom.is_finite() { bottom } else { 0. },
                    });
                }
            }
            _ => {
                let atomic = !stack.is_empty()
                    || match op {
                        Op::Image { .. }
                        | Op::GlyphPaint { .. }
                        | Op::Gradient { .. }
                        | Op::ImageTile { .. }
                        | Op::ImageClip { .. }
                        | Op::Bezier { .. }
                        | Op::Shaped { .. }
                        | Op::FieldText { .. } => true,
                        Op::Path {
                            points,
                            closed,
                            fill,
                            ..
                        } => {
                            *closed
                                || fill.is_some()
                                || points.len() != 2
                                || (points[0].1 - points[1].1).abs() > 0.001
                        }
                        Op::Line { x, y, x2, y2, .. } => {
                            (x - x2).abs() > 0.001 && (y - y2).abs() > 0.001
                        }
                        _ => false,
                    };
                if atomic && let Some(b) = bounds(op, fonts) {
                    let r = graphics::rect(matrix, b);
                    if stack.is_empty() {
                        out.push(Atom {
                            start: i,
                            end: i + 1,
                            top: r[1],
                            bottom: r[3],
                        });
                    } else {
                        top = top.min(r[1]);
                        bottom = bottom.max(r[3]);
                    }
                }
            }
        }
    }
    ensure!(stack.is_empty(), "Unclosed table transform");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn painted_glyphs_are_owned_by_the_splitter_instead_of_discarded() {
        let mut fonts = Fonts::new(std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }));
        let font = fonts.get("arial.ttf").unwrap();
        let face = ttf_parser::Face::parse(&fonts.list[font].bytes, 0).unwrap();
        let glyph = face.glyph_index('M').unwrap().0;
        let op = Op::GlyphPaint {
            x: 10.,
            y: 100.,
            font,
            glyph,
            size: 20.,
            scale: 1.,
            color: [0.; 3],
        };
        let a = atoms(&[op], &fonts).unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!((a[0].start, a[0].end), (0, 1));
        assert!(a[0].top < 100. && a[0].bottom >= 99.);
    }
    #[test]
    fn transformed_text_reserves_actual_ink_not_a_full_em_padding_box() {
        let mut fonts = Fonts::new(std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }));
        let font = fonts.get("arial.ttf").unwrap();
        let text = Op::Text {
            x: 10.,
            y: 100.,
            text: "M".into(),
            font,
            size: 20.,
            scale: 1.,
            color: [0.; 3],
        };
        let ops = [
            Op::TransformBegin {
                matrix: [1., 0., 0.2, 1., 0., 0.],
            },
            text,
            Op::TransformEnd,
        ];
        let groups = atoms(&ops, &fonts).unwrap();
        assert_eq!(groups.len(), 1);
        assert!(groups[0].top > 80. && groups[0].bottom <= 100.1);
        assert!(groups[0].bottom - groups[0].top < 20.);
        let space = Op::Text {
            x: 10.,
            y: 100.,
            text: " ".into(),
            font,
            size: 20.,
            scale: 1.,
            color: [0.; 3],
        };
        let b = bounds(&space, &fonts).unwrap();
        assert_eq!(b[1], b[3]);
    }
}
