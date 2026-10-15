# Independent cell continuation — 2026-10-16

This follows CELL_FLOW_COMPARISON.md. It improves table packing without claiming
identical Hancom row or note distribution.

## Change

Centered cells in adjacent columns can have staggered line positions. Requiring
one common ink-free cut made the old paginator retreat across a chain of lines,
leaving 10–45pt unused on several report pages. Native plain-text floating tables
now carry the crossing lines of each cell independently. The carry inserts space,
extends merged borders/fills and shifts following rows; glyphs are never scaled,
clipped or deleted. Repeated headers and note anchors remain attached.

The existing shared-ink path remains for nested/transformed objects, links and
unsupported operation types. The optimization runs only when independent carry
saves more space than its insertion and makes forward progress. Each fragment's
bottom rule uses the original cell border style; absent borders stay absent.

## Reference results

- Report page 35 now includes row 9, as in the Hancom reference, and has a closed
  lower border. Previously that row started on page 36.
- The large table's page-42 tail shrinks from 183.54pt to 85.77pt. Its end is now
  about 195.61pt versus Hancom 235.44pt: packing improved, but this boundary
  overshoots the reference and is **not** an exact match.
- The research table starts 19.2pt earlier on page 46 (627.42pt vs 646.62pt).
  Its final page-48 fragment shrinks from 50.76pt to 34.81pt. It still spans
  pages 46–48, while Hancom spans 46–47. Remaining work includes saved body-line
  distribution, row-fragment alignment and precise note-area allocation.
- Total report length stays 50 pages. The other three Hancom targets remain
  byte-identical to stage 21, with 2/15/9 pages.

## Validation

158 Rust tests and all-target Clippy pass. New coverage renders a three-column
staggered table over three pages, preserves every label exactly once, repeats
headers and checks geometry. A border test preserves rule style and absent rules.
The supplied report regression also checks the large table's compact final tail.

All 43 documents / 205 pages were reconverted. Source SHA-256 hashes and
page/picture/table/equation/chart counts are unchanged. All extracted character
multisets, including repeated headers, match stage 21. Four geometry audit
classes report zero findings. Forty-one PDFs are byte-identical; report and
government-notice PDFs change. The latter changes only pages 14 and 25, where
fragment bottom rules are restored. Report raster changes are recorded in
changed-pages.json (31 pages); unchanged report pages are pixel-identical at
1000px. Comparison sheets and changed-page contact sheets were visually reviewed.

Local artifacts: output/roadmap/22-table-fragments/SUMMARY.html,
verification-summary.json, corpus-text-verification.json, test-list.log,
focus-final.log and clippy.log. Generated PDFs and images remain outside Git.
