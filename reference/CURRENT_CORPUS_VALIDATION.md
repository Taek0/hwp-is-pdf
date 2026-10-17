# Current-corpus validation — 2026-10-18

Stage 42 converts 53 unique HWP5/HWPX documents into 260 pages. All 206 Rust
tests and all-target Clippy pass. Source hashes are unchanged. All four geometry
categories, missing-glyph warnings and unresolved link/reference warnings are
empty. Every page was rasterized; six contact sheets and selected full-size
Hancom comparisons were reviewed. This is not a declaration of complete fidelity.

Native table cell flags now preserve custom margins and explicit header rows.
The report's merged two-row header repeats on pages 33–34, recovering 72 missing
Hangul characters. Its pages 9–16 have the same per-page Hangul multisets as the
reference. Fragment-local padding and alignment avoid repeating short cell text
or manufacturing a header-only final fragment.

Native paragraph margins use URC normalization and preserve hanging indents.
JUSTIFY expands word spaces where possible. Resolved native font metrics can
recover redundant cached lines without changing explicit page boundaries.
Report page 3's 19.2pt displacement is removed: body/caption vertical positions
are within about 0.1pt of the reference. The missing Human Myeongjo face resolves
to HCR Batang, matching the reference's embedded font. This rule does not alter
the math document's independently verified Batang substitution.

The FAQ changes from 14 to 13 pages. Its original line cache has 13 page bands,
including an explicit section-3 start on page 7. The previous output inserted an
extra table continuation before that boundary. The final page retains the full
contact table. No source document was edited.

A nested TOP_AND_BOTTOM table now reserves its measured height before following
text. In inner-table-01, the final “끝” moves below the nested table instead of
crossing its right border, with all three pages retained. Narrow punctuation
with excessive negative tracking remains protected against actual collisions.

## Still active

The report remains 50 pages, and its research table remains on pages 46–47.
There are still per-page text differences around pages 3–6, 20–29 and especially
the merged body cells with notes on pages 35–43. Some text moves from page 47 to
48; footer typography and finer line endings also differ. Geometry QA does not
catch these. Continue reference comparison and general layout fixes; the user's
request to resolve current document problems is not complete.

Artifacts: `output/roadmap/42-report-cell-flow/`, including `regression-final/`,
`page-images/`, `all-verification.json`, `report-page-comparison.json`,
`faq-cache-pages.log`, `tests.log`, `clippy.log` and six `contact-*.jpg` sheets.

Hancom explains URC indent units and hanging-indent semantics at:
https://forum.developer.hancom.com/t/hwpml/2464

Earlier completed work: [column bands](NATIVE_COLUMN_BANDS.md),
[embedded graphics](EMBEDDED_GRAPHICS.md), [Korean wrapping](KOREAN_BREAKS_AND_TRACKING.md),
and [legacy symbols and links](LEGACY_SYMBOLS_AND_LINKS.md).
