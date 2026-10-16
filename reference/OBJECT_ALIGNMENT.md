# Inline margins and text-box vertical alignment — 2026-10-16

Two format-level omissions explained the measured title offsets:

- Native saved inline tables used their inner height for the 85% baseline,
  while the cached line baseline includes top and bottom outside margins.
  The baseline calculation now uses that outside box before adding the top margin.
- Drawing text ignored CENTER/BOTTOM in its sub-list. It now aligns measured
  logical text height within the box after subtracting both text margins.
  TOP remains unchanged, and oversized content never receives a negative shift.

No document name, page number or reference coordinate is used by the renderer.

| Selected text-box top (pt) | Hancom | Before | After | Remaining error |
|---|---:|---:|---:|---:|
| Form p1 title | 66.023 | 68.436 | 66.056 | +0.033 |
| Law p8 repeated title | 105.748 | 101.140 | 105.740 | -0.008 |

These measurements describe selected text tops, not whole-document identity.
Horizontal tracking, generated italic numbers, the math saved-column origin,
watermark placement/tone and other previously listed differences remain.

171 tests and all-target Clippy pass. Tests include asymmetric outside margins,
TOP/CENTER/BOTTOM text alignment, source immutability and actual title baselines.
All 43 documents / 205 pages preserve source hashes, page/object counts and
extracted character multisets; geometry findings are zero. Thirty-three PDFs are
byte-identical. Ten change through their inline tables or centered/bottom text.

All four comparison PDFs were rasterized. Form pages 1–2, all law pages and
report page 28 change; math is byte-identical. Representative paired pages and
the seven other affected documents' first-page contact sheet were reviewed.
The source-corresponding watermark/printing differences are not hidden or removed
to make the screenshots match.

Artifacts: output/roadmap/28-alignment/SUMMARY.html, title-positions.json,
verification-summary.json, corpus-text-verification.json, comparisons/,
regression-contact.png, tests.log and clippy.log.
