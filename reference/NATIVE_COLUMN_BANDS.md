# Native column bands and nested columns — 2026-10-18

The native paragraph division flags now preserve both a new column definition
and an explicit column break. Saved body coordinates are local to a column
band. A new band follows the deepest occupied column on the current page;
repeated equal Y coordinates can start adjacent columns. New pages reset the
band origin. If the first line of a new band cannot fit, the band starts on the
next page instead of extending below the paper.

The section's `spaceColumns` field supplies the vertical gap between distinct
column bands. This is different from a column definition's horizontal sameGap.
Hancom documents this distinction in its official [column width and spacing
help](https://help.hancom.com/hoffice/multi/ko_kr/hwp/format/columns/columns%28width%29.htm).
The supplied document uses 1134 HWP units; no document-specific offsets are used.

Nested equal-width left-to-right text lists also preserve independent column
origins and heights. Their saved X positions are normalized in the HWP adapter;
measurement no longer stacks the second column beneath the first. Multiple or
ambiguous nested definitions are left untouched by this bounded normalization.

Wrapping spaces at saved line ends no longer count toward visible line width.
Previously a large-font line could be squeezed to accommodate an invisible
trailing space. This affected the 120pt and 72pt labels in the column reference.

## Verification

The supplied four-page column reference now has zero geometry findings, down
from 12 text overlaps. Three short lines correctly remain at the bottom of page
2 instead of overlapping at the top of page 3. The page-2 line origins match
Hancom to within 0.1pt, and the text box and table on page 4 retain two columns.
Body/nested comparison images were visually inspected.

All 43 existing documents / 205 pages retain page/object counts and extracted
non-whitespace character multisets, with zero geometry findings and unchanged
source SHA256 hashes. 32 PDFs are byte-identical; the remaining changes include
corrected saved-line trailing-space widths. The initial regression caught an
insufficient-space band transition; the final run includes its fix.

192 tests and all-target Clippy pass. Artifacts are under
`output/roadmap/37-column-bands/`: columns.pdf, final test/Clippy logs,
corpus-text-verification.json, verification-summary.json, and images/body-comparison.png
and images/nested-comparison.png. The initial failure logs are retained separately.

This stage does not claim to resolve the mixed-chart or WMF reference failures;
those remain the next embedded-graphics work.
