//! Shadow geometry is decorative ink, not repeated document text.
use super::*;
#[derive(Clone)]
pub(super) struct Shadow {
    dx: f64,
    dy: f64,
    color: [f64; 3],
    continuous: bool,
    relief: bool,
}
pub(super) fn read(n: &Node, size: f64) -> Option<Shadow> {
    if n.child("emboss").is_some() || n.child("engrave").is_some() {
        let sign = if n.child("emboss").is_some() { -1. } else { 1. };
        return Some(Shadow {
            dx: size * 0.04 * sign,
            dy: size * 0.04 * sign,
            color: [0.6; 3],
            continuous: false,
            relief: true,
        });
    }
    let s = n.child("shadow")?;
    if !matches!(s.a("type"), "DROP" | "CONTINUOUS") {
        return None;
    }
    Some(Shadow {
        dx: size * s.n("offsetX") / 100.,
        dy: size * s.n("offsetY") / 100.,
        color: color(s.a("color")),
        continuous: s.a("type") == "CONTINUOUS",
        relief: false,
    })
}
pub(super) fn main_offset(g: &Glyph) -> (f64, f64) {
    g.shadow
        .as_ref()
        .filter(|s| s.relief)
        .map(|s| (s.dx / 2., s.dy / 2.))
        .unwrap_or((0., 0.))
}
pub(super) fn paint(ops: &mut Vec<Op>, g: &Glyph, id: u16, x: f64, y: f64, scale: f64) {
    let Some(s) = &g.shadow else {
        return;
    };
    if s.relief {
        for (dx, dy, color) in [(0., 0., s.color), (s.dx, s.dy, [1.; 3])] {
            ops.push(Op::GlyphPaint {
                x: x + dx,
                y: y + dy,
                font: g.font,
                glyph: id,
                size: g.size,
                scale,
                color,
            });
        }
        return;
    }
    let steps = if s.continuous {
        ((s.dx.abs().max(s.dy.abs()) / 0.12).ceil() as usize).clamp(1, 128)
    } else {
        1
    };
    for i in (1..=steps).rev() {
        let t = i as f64 / steps as f64;
        ops.push(Op::GlyphPaint {
            x: x + s.dx * t,
            y: y + s.dy * t,
            font: g.font,
            glyph: id,
            size: g.size,
            scale,
            color: s.color,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relief_highlight_offsets_match_printed_24pt_layers() {
        for (kind, expected) in [("emboss", -0.96), ("engrave", 0.96)] {
            let n = crate::model::xml(format!("<charPr><{kind}/></charPr>").as_bytes()).unwrap();
            let s = read(&n, 24.).unwrap();
            assert!(s.relief);
            assert!((s.dx - expected).abs() < 1e-9);
            assert_eq!(s.color, [0.6; 3]);
        }
    }
}
