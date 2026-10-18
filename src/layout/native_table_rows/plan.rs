//! Side-effect-free row fit calculations shared by preflight and painting.
use super::*;
pub(super) struct Fit {
    pub takes: Vec<usize>,
    pub height: f64,
}
pub(super) fn fit(row: &Row<'_>, room: f64) -> Option<Fit> {
    let full = row.remaining() <= room + 0.01;
    if row.band.is_some() && !full {
        return None;
    }
    let mut takes: Vec<_> = row
        .cells
        .iter()
        .map(|c| {
            if full {
                c.flow.len() - c.at
            } else {
                c.fit(room)
            }
        })
        .collect();
    if !full
        && row.minimum > room + 0.01
        && takes
            .iter()
            .zip(&row.cells)
            .all(|(take, c)| *take == c.flow.len() - c.at)
        && let Some(i) = takes.iter().rposition(|take| *take > 0)
    {
        takes[i] -= 1;
    }
    let content = takes
        .iter()
        .zip(&row.cells)
        .map(|(take, c)| c.required(*take))
        .fold(0., f64::max);
    let height = if full {
        row.remaining()
    } else {
        content.max(row.minimum.min(room.max(0.)))
    };
    if height <= 0.1
        || (takes.iter().all(|take| *take == 0) && row.cells.iter().any(|c| c.at < c.flow.len()))
    {
        return None;
    }
    Some(Fit { takes, height })
}
pub(super) struct Budget {
    pub first_top: f64,
    pub next_top: f64,
    pub bottom: f64,
    pub header: f64,
    pub outside_bottom: f64,
    pub caption_top: f64,
    pub caption_bottom: f64,
}
/// Admit merged bands only when each stays unbroken in the exact plain-row flow.
/// Unsupported placement returns before any page, operation or note is mutated.
pub(super) fn merged_bands_fit(source: &[Row<'_>], b: Budget) -> bool {
    let mut rows = source.to_vec();
    let mut index = 0;
    let mut top = b.first_top;
    let mut painted = false;
    let mut first = true;
    for _ in 0..10000 {
        if index == rows.len() {
            return true;
        }
        if rows[index].done() {
            index += 1;
            continue;
        }
        let available = b.bottom - top - b.outside_bottom - if painted { 0. } else { b.header };
        let room = if index + 1 == rows.len() {
            (available - b.caption_bottom).max(0.)
        } else {
            available
        };
        if rows[index].band.is_some() && rows[index].remaining() > room + 0.01 {
            return false;
        }
        let Some(step) = fit(&rows[index], room) else {
            if !painted && top <= b.next_top + (if first { b.caption_top } else { 0. }) + 0.1 {
                return false;
            }
            if painted {
                first = false;
            }
            top = b.next_top + if first { b.caption_top } else { 0. };
            painted = false;
            continue;
        };
        if !painted {
            top += b.header;
            painted = true;
        }
        for (cell, take) in rows[index].cells.iter_mut().zip(step.takes) {
            cell.at += take;
        }
        rows[index].minimum = (rows[index].minimum - step.height).max(0.);
        top += step.height;
        if rows[index].done() {
            index += 1;
        } else {
            first = false;
            top = b.next_top;
            painted = false;
        }
    }
    false
}
