//! Tab advances are recomputed from each destination line, not cached widths.
#[derive(Clone, Default)]
pub(crate) struct Stop {
    pub pos: f64,
    pub kind: String,
    pub leader: String,
}
pub(crate) fn advance(
    stops: &[Stop],
    step: f64,
    x: f64,
    after: impl Iterator<Item = (Option<char>, f64)>,
) -> f64 {
    let mut total = 0.;
    let mut decimal = None;
    for (ch, w) in after {
        if matches!(ch, Some('\t' | '\n')) {
            break;
        }
        if decimal.is_none() && ch == Some('.') {
            decimal = Some(total);
        }
        total += w;
    }
    if let Some(stop) = stops.iter().find(|s| s.pos > x + 0.01) {
        let suffix = match stop.kind.as_str() {
            "RIGHT" => total,
            "CENTER" => total / 2.,
            "DECIMAL" => decimal.unwrap_or(total),
            _ => 0.,
        };
        return (stop.pos - x - suffix).max(0.);
    }
    let step = if step > 0. { step } else { 40. };
    ((x / step).floor() + 1.) * step - x
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alignment_offsets() {
        let after = [
            (Some('1'), 6.),
            (Some('2'), 6.),
            (Some('.'), 3.),
            (Some('5'), 6.),
        ];
        for (kind, w) in [
            ("LEFT", 80.),
            ("RIGHT", 59.),
            ("CENTER", 69.5),
            ("DECIMAL", 68.),
        ] {
            assert_eq!(
                advance(
                    &[Stop {
                        pos: 100.,
                        kind: kind.into(),
                        ..Default::default()
                    }],
                    40.,
                    20.,
                    after.iter().copied()
                ),
                w
            );
        }
    }
}
