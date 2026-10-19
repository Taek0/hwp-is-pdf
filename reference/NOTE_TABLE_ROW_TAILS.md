# Repeated headers, body notes and empty row tails — stage 80

Plain native body rows can now use the remaining-cell paginator with repeated
multi-row headers and deferred body notes. Header notes and body rowspans retain
their existing guarded paths. The old blanket multi-row-header restriction sent
the supplied report through vector slicing even though these body rows are simple.

When all cell content fits but at most half a saved line of empty minimum-height
tail remains beyond the page edge, the paginator closes that row on the current
page. Larger authored extents and content that does not fit are preserved. The
same decision is used by preflight and painting; no glyph or note is discarded.

For the independent 50-page Hancom report, the extra approximately 9pt band on
page 33 disappears. The checked title baseline agrees within 0.2pt, and the major
row rules on pages 32–34 agree within about 0.2pt except the physical page cut.
All 50 per-page Hangul multisets still match; automatic collision checks are clean.
Tests cover the permitted empty-tail boundary, larger tails, non-fitting content
and the reference page/baseline. Source files remain unchanged.

Artifacts: `output/roadmap/80-note-table-rows`.

Final regression: all 202 inputs / 594 pages convert; only the report PDF changes
from stage 79, with the other 201 byte-identical. All 325 tests and all-target
Clippy pass. The report retains zero raw geometry findings.
