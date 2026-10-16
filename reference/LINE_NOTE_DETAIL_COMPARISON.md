# Line and note detail comparison — 2026-10-16

## Corrected fragment spacing

A continuing note previously reserved the last line's full advance, including
leading intended for a line on the next page. This rejected a complete line
whose ink box actually fitted. Note batches now test the final line height,
retain advances between lines on the same page, and omit trailing line/inter-note
spacing at a page boundary. Minimum anchor reservation uses the same last-line
rule. No source text, font scale or document-specific coordinates are changed.

Report note 44 now occupies three lines on page 44 and one on page 45, matching
Hancom. The research table still spans pages 46–47; its start moves from 589.02pt
to 569.82pt. Total report length stays 50 pages.

## Measured comparison

The table measures text bounding-box tops in PDF points, not baseline positions.
Negative error means Rust's text is higher. These are selected diagnostic anchors,
not a document-wide compatibility score.

| Anchor | Hancom Y | Previous Y | Current Y | Current error |
|---|---:|---:|---:|---:|
| Report p44, note 41 | 584.19 | 593.17 | 584.17 | -0.02 |
| Report p44, note 42 | 607.83 | 616.72 | 607.72 | -0.11 |
| Report p44, note 43 | 689.91 | 698.87 | 689.87 | -0.04 |
| Report p44, note 44 | 725.19 | 734.14 | 725.14 | -0.05 |
| Form p1, title | 66.023 | 68.436 | 68.436 | +2.413 |
| Law p8, title | 105.748 | 101.140 | 101.140 | -4.608 |
| Math p5, question 15 | 126.297 | 78.252 | 78.252 | -48.045 |

The last three differences remain; this patch does not claim to fix them.
The math source cache and preceding blank paragraphs were inspected, but their
relationship to Hancom's column/page flow needs further investigation before
changing pagination. Native body wrapping and graphic/furniture differences also
remain. The report's precise note split finding in RESEARCH_PAGINATION_COMPARISON.md
is superseded by this three-plus-one result.

## Verification and artifacts

159 tests and all-target Clippy pass. Boundary tests cover fitting two note lines
without trailing leading, retaining an anchor when just one line fits, and moving
it when no complete line fits. The report test asserts the three-plus-one split.
Unequal-width column, caption and existing long-note tests remain enabled.

All 43 documents / 205 pages retain source hashes, page/object counts and complete
extracted character multisets. Geometry findings are zero. Forty-two PDFs are
byte-identical to stage 23. Only report pages 42, 44, 45 and 46 change at 1000px;
all its other pages are pixel-identical. The four Hancom targets remain 2/15/9/50
pages. All 76 current pages were rasterized; representative paired pages and the
note-detail crop were visually reviewed, not every pixel of every reference page.

Local evidence: output/roadmap/24-line-note-detail/SUMMARY.html,
verification-summary.json, four-document-anchors.json, corpus-text-verification.json,
comparisons/, tests.log and clippy.log. The HTML links all four PDFs and paired
views for form pages 1–2, law pages 1/8/15, math pages 1/5/9 and report pages 42/44–48.
