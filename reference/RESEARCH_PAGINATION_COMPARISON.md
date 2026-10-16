# Research table pagination — 2026-10-16

This supersedes the outstanding research-table page-range finding in
TABLE_FRAGMENT_COMPARISON.md.

## Cause and correction

The report's short footnote 44 was treated as an indivisible reserved area.
Its anchor line consequently moved from page 44 to 45, and the complete note
then displaced more body lines onto page 46. Hancom continues that note across
pages 44–45 instead.

Saved single-column body layout now permits a short note to continue when the
current page can hold existing notes, separators and at least the incoming
note's first complete line advance. Otherwise the anchor moves as before.
Oversized groups keep the established bounded reservation policy. Multi-column
reservation remains unchanged. Final note painting measures actual free body
space; it no longer assumes a half-page area when a late anchor reserves less.
Footer furniture outside the printable body is excluded from that measurement.
For multi-column continuation, clearance is measured in the note's own horizontal
area, so a full adjacent column cannot incorrectly block its remaining space.

The calculation lives in layout/note_space.rs. It has no document identifiers,
target page numbers or forced height/scale constants.

## Result

- Research table: **pages 46–47**, matching the Hancom reference page range.
  Previous engine range: 46–48.
- Report remains 50 pages. The table begins at 589.02pt on page 46, versus
  627.42pt previously. The entire final row ends on page 47.
- Footnote 44 continues on pages 44–45. Its precise line distribution still
  differs: Rust places two note lines on each page; Hancom places three and one.
- In-page table/text positions and some font-dependent line breaks still differ.
  Matching the page range is not a claim of pixel identity.

## Verification

159 tests and all-target Clippy pass. A synthetic short-note test checks both
sufficient and insufficient first-line space, anchor retention/movement, text
preservation and non-overlap. The report regression asserts table pages 46–47,
note pages 44–45 and the 50-page total.
The existing unequal-width, right-to-left column test also passes after verifying
that each note uses its own column's clearance.

All 43 baseline documents / 205 pages were reconverted. Source hashes and
page/picture/table/equation/chart counts remain unchanged; geometry findings are
zero. Forty-two PDFs are byte-identical to stage 22. The report's only character
multiset reduction is exactly one repeated "제목(년도)주요내용" header from the
eliminated third table fragment. No body characters are removed or added.

The four reference targets remain 2/15/9/50 pages. Comparisons of report pages
44–48 show note continuation, the two-page table, and the following body.
Existing Hancom reference PDFs suffice; no additional viewer export is required
for this correction.

Artifacts: output/roadmap/23-research-pages/SUMMARY.html, comparisons/,
verification-summary.json, corpus-text-verification.json, tests.log and clippy.log.
PDFs, images and logs are local ignored outputs.
