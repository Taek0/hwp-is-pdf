//! Carry staggered cell lines independently instead of retreating across all columns.
use super::*;

fn shift(op: &mut Op, from: f64, delta: f64) {
    let move_y = |y: &mut f64| {
        if *y >= from {
            *y += delta;
        }
    };
    match op {
        Op::Text { y, .. } | Op::Shaped { y, .. } => move_y(y),
        Op::Line { y, y2, .. } => {
            move_y(y);
            move_y(y2);
        }
        Op::Rect { y, h, .. } => {
            let mut end = *y + *h;
            move_y(y);
            move_y(&mut end);
            *h = end - *y;
        }
        Op::Path { points, .. } => {
            for (_, y) in points {
                move_y(y);
            }
        }
        _ => {}
    }
}

fn intervals(ops: &[Op], fonts: &Fonts) -> Vec<(f64, f64)> {
    ops.iter()
        .filter_map(|o| match o {
            Op::Text { text, .. } if !text.trim().is_empty() => {
                crate::pagination::bounds(o, fonts).map(|b| (b[1] - 0.6, b[3] + 0.6))
            }
            Op::Path { .. } | Op::Shaped { .. } => {
                crate::pagination::bounds(o, fonts).map(|b| (b[1] - 0.6, b[3] + 0.6))
            }
            _ => None,
        })
        .collect()
}

fn retreat(mut cut: f64, ink: &[(f64, f64)]) -> f64 {
    loop {
        let next = ink
            .iter()
            .filter(|(a, b)| *a < cut && *b > cut)
            .map(|(a, _)| *a)
            .fold(cut, f64::min);
        if cut - next < 0.001 {
            return cut;
        }
        cut = next;
    }
}

/// Close the visible fragment using the cell's own bottom rule, if it has one.
pub(super) fn bottom_rules(ops: &[Op], cells: &[TableCellPaint], cut: f64) -> Vec<Op> {
    fragment_rules(ops, cells, cut, true)
}
pub(super) fn top_rules(ops: &[Op], cells: &[TableCellPaint], cut: f64) -> Vec<Op> {
    fragment_rules(ops, cells, cut, false)
}
fn fragment_rules(ops: &[Op], cells: &[TableCellPaint], cut: f64, bottom_edge: bool) -> Vec<Op> {
    let mut out = Vec::new();
    for c in cells
        .iter()
        .filter(|c| c.y < cut - 0.01 && c.y + c.height > cut + 0.01)
    {
        let bottom = if bottom_edge { c.y + c.height } else { c.y };
        for op in &ops[c.borders.clone()] {
            if matches!(op, Op::Line{y,y2,..} if (*y-bottom).abs()<0.01 && (*y-*y2).abs()<0.001)
                || matches!(op, Op::Path{points,..} if points.len()==2 && points.iter().all(|p|(p.1-bottom).abs()<0.01))
            {
                let mut rule = op.clone();
                shift(&mut rule, f64::NEG_INFINITY, cut - bottom);
                out.push(rule);
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub(super) fn pack_cells(
    ops: &mut [Op],
    bounds: &mut [Bounds],
    cells: &mut [TableCellPaint],
    headers: &mut [TableHeaderPaint],
    notes: &mut [PlannedTableNote],
    fonts: &Fonts,
    mut total: f64,
    first_capacity: f64,
    page_capacity: f64,
) -> f64 {
    // Complex transformed objects retain the shared-ink paginator. This path
    // only inserts space; it never scales glyphs or changes their line breaks.
    if cells.is_empty()
        || ops.iter().any(|op| {
            !matches!(
                op,
                Op::Text { .. }
                    | Op::Shaped { .. }
                    | Op::Line { .. }
                    | Op::Rect { .. }
                    | Op::Path {
                        closed: false,
                        fill: None,
                        ..
                    }
            )
        })
        || ops.iter().any(|op| {
            matches!(op,Op::Path {points,..}
        if points.len()!=2 || (points[0].1-points[1].1).abs()>0.001)
        })
    {
        return total;
    }
    let header_height = headers.iter().map(|h| h.height).sum::<f64>();
    let capacity = page_capacity - header_height;
    if first_capacity <= header_height + 24. || capacity <= 24. {
        return total;
    }
    let mut cut = first_capacity;
    while cut < total - 0.01 {
        let ink = intervals(ops, fonts);
        let shared = retreat(cut, &ink);
        let plans: Vec<_> = cells
            .iter()
            .map(|c| {
                if c.y < cut && c.y + c.height > cut {
                    let local = retreat(cut, &intervals(&ops[c.ops.clone()], fonts));
                    (local, if local < cut { cut - local + 0.02 } else { 0. })
                } else {
                    (cut, 0.)
                }
            })
            .collect();
        let extra = plans.iter().map(|(_, d)| *d).fold(0., f64::max);
        // Only improve the case where columns form a chain of overlapping ink.
        // Ordinary one-line retreats preserve the original vector coordinates.
        if extra > 0. && extra < capacity - 24. && cut - shared > extra + 1. {
            let mut owned = vec![false; ops.len()];
            for (c, (local, delta)) in cells.iter_mut().zip(&plans) {
                let (from, delta) = if c.y >= cut {
                    (cut, extra)
                } else {
                    (*local, *delta)
                };
                for index in c.ops.clone() {
                    owned[index] = true;
                    shift(&mut ops[index], from, delta);
                }
                for b in &mut bounds[c.bounds.clone()] {
                    if b.y >= from || (b.y + b.h > from && b.y < cut && delta > 0.) {
                        b.y += delta;
                    }
                }
                for note in notes.iter_mut().filter(|n| c.ops.contains(&n.op_index)) {
                    if note.y >= from {
                        note.y += delta;
                        note.top += delta;
                    }
                }
                let end = c.y + c.height;
                if c.y >= cut {
                    c.y += extra;
                }
                c.height = end + if end >= cut { extra } else { 0. } - c.y;
            }
            // Borders/fills stretch across the inserted space. All later rows
            // move together, including cells merged across the split row.
            for (i, op) in ops.iter_mut().enumerate() {
                if !owned[i] {
                    shift(op, cut, extra);
                }
            }
            for h in headers.iter_mut() {
                if h.end >= cut {
                    h.end += extra;
                }
            }
            total += extra;
            cut += capacity;
        } else {
            let next = shared + capacity;
            if next <= cut + 0.1 {
                break;
            }
            cut = next;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragment_closing_rule_preserves_style_and_absent_borders() {
        let ops = vec![Op::Line {
            x: 10.,
            y: 100.,
            x2: 60.,
            y2: 100.,
            width: 0.4,
            color: [0.2, 0.3, 0.4],
        }];
        let cell = TableCellPaint {
            y: 0.,
            height: 100.,
            ops: 1..1,
            borders: 0..1,
            bounds: 0..0,
        };
        let rules = bottom_rules(&ops, &[cell], 70.);
        assert!(matches!(rules.as_slice(),[Op::Line{x,y,x2,y2,width,color}]
            if (*x,*y,*x2,*y2,*width,*color)==(10.,70.,60.,70.,0.4,[0.2,0.3,0.4])));
        let cell = TableCellPaint {
            y: 0.,
            height: 100.,
            ops: 0..0,
            borders: 0..0,
            bounds: 0..0,
        };
        assert!(bottom_rules(&[], &[cell], 70.).is_empty());
    }
    #[test]
    fn continuation_top_uses_existing_top_rule_without_inventing_a_border() {
        let ops = vec![Op::Line {
            x: 10.,
            y: 20.,
            x2: 60.,
            y2: 20.,
            width: 0.4,
            color: [0.2, 0.3, 0.4],
        }];
        let cell = TableCellPaint {
            y: 20.,
            height: 100.,
            ops: 1..1,
            borders: 0..1,
            bounds: 0..0,
        };
        let rules = top_rules(&ops, &[cell], 70.);
        assert!(
            matches!(rules.as_slice(),[Op::Line{y,y2,width,..}] if *y==70. && *y2==70. && *width==0.4)
        );
        let cell = TableCellPaint {
            y: 20.,
            height: 100.,
            ops: 0..0,
            borders: 0..0,
            bounds: 0..0,
        };
        assert!(top_rules(&[], &[cell], 70.).is_empty());
    }
}
