# Trailing character tracking — 2026-10-16

Tracking is space between adjacent glyphs, not a margin after a line's final
glyph. Saved-line alignment, cell overflow detection and rebuilt line widths
now exclude that final adjustment. Positive and negative tracking are handled;
zero-width controls do not restore a spurious final gap. Complex-script shaping
continues to own its shaped advances.

The law exam's page 8 centered title starts at 366.9621pt in the Hancom PDF.
Its engine start changed from 365.6800pt to 366.9300pt (error -0.0321pt).
The previous vertical correction remains, with a text-top error of -0.0081pt.
This is a selected title measurement, not whole-document equivalence.

174 tests and all-target Clippy pass. All 43 documents / 205 pages retain page
and object counts, extracted character multisets and original source hashes.
Geometry findings are zero; 33 PDFs are byte-identical and ten change.
Focused tests cover both signs of tracking, centered/right alignment, wrapping
at a boundary and zero-width controls at the end of a line.

Artifacts: output/roadmap/29-tracking/verification-summary.json,
corpus-text-verification.json, tracking-comparison.png, images/, tests.log and
clippy.log. Remaining work is tracked in PRIORITY_WORKLIST.md.
