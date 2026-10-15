# Cell wrapping and floating-table flow — 2026-10-16

This follow-up supersedes the receipt-wrap and report-start-page findings in
FINAL_COMPARISON.md, without claiming complete Hancom pagination identity.

## Changes and reference evidence

- The form's centered "(Receiving Official)" label previously squeezed a small
  width overflow into one line. Its explicit BREAK/KEEP_WORD settings now produce
  two lines. Reference x origins are 471.360 / 477.000 pt; Rust is 471.454 /
  477.075 pt. Both line-top positions differ by approximately 0.55 pt.
- Long footnotes now measure the final line without unused trailing leading,
  consistent with ordinary note measurement. Inter-note gaps exist only between
  queued notes, not after the last one.
- The report's table anchor has a positive 64.06pt vertical offset after a
  three-line, 50.4pt paragraph. Painting the table first incorrectly moved that
  paragraph to page 42. Deferring the table until the paragraph completes restores
  the paragraph on page 35 before the table, as in Hancom.
- The research table now begins on page 46, versus page 47 before the change.
  The 50-page total is retained.

## Validation

156 tests and all-target Clippy pass. All 43 baseline documents / 205 pages retain
their source SHA-256 hashes and page/picture/table/equation/chart counts. Geometry
findings and new missing glyphs are zero.

Forty-one PDFs are byte-identical to stage 20. The form's character multiset is
unchanged. The report removes no characters and adds exactly one repeated
"제목(년도) 주요내용" table header, required for its additional table fragment.
The form's first page and report pages 35, 42–48 were visually reviewed; other
pages of those documents are pixel-identical to the previous outputs at 1000px.

## Remaining difference

The research table starts on the correct page, but its internal row distribution
still spans Rust pages 46–48 versus Hancom 46–47. Earlier table fragments and note
distribution also differ. Original-font availability and other precision limits
remain as documented in FORMAT_COVERAGE.md. No global compatibility percentage
is inferred from the automated checks.

## Artifacts

Local report: output/roadmap/21-cell-flow/SUMMARY.html.
It links the four target PDFs, the enlarged receipt comparison, report pages
35/46 comparisons, changed-page contact sheet, tests and machine-readable audits.
Generated PDFs and images are intentionally kept outside Git.

Regression coverage includes a generic slight-overflow label test, a synthetic
multipage paragraph-below table, a note-height consistency test, and both supplied
reference documents. Rendering code contains no document-name or fixed-page-count
branches.
