//! Select inter-word gaps without stretching typed paragraph padding.
use super::*;

pub(super) fn interword_spaces(items: &[Item], widths: &[f64]) -> Vec<bool> {
    let content =
        |(item, width): (&Item, &f64)| *width > 0. && !matches!(item.kind, ItemKind::Char(' '));
    let first = items
        .iter()
        .zip(widths)
        .position(content)
        .unwrap_or(items.len());
    let last = items.iter().zip(widths).rposition(content).unwrap_or(0);
    items
        .iter()
        .enumerate()
        .map(|(i, item)| i > first && i < last && matches!(item.kind, ItemKind::Char(' ')))
        .collect()
}
