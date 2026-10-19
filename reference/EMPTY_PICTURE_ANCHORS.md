# Empty saved picture anchors — stage 82

The expanded saved-preview review exposed a real discrepancy in the scaled-picture
sample: an object-only paragraph's cached line is after the picture, but the engine
used it as the picture origin. The first image was displaced downward by 225pt.

At a fresh native page/column, an empty single-line paragraph can now back-position
its block picture when the complete saved gap exactly equals the picture height
and bottom margin. Nonzero top margins, captions, explicit vertical offsets,
unmatched gaps and ordinary body paragraphs retain the previous path. The rule is
in the shared object-position module and has positive and negative regression cases.

The first picture moves from y=324.2pt to y=99.2pt, preserving its 375×225pt size.
Its source thumbnail independently corroborates that placement; the second page
uses the same source-backed rule. The source preview is lower resolution and is
not a substitute for a complete independent print.

All 202 held sources still convert to 594 pages. Only this PDF changes from stage
80; the other 201 are byte-identical. Artifacts are in
`output/roadmap/82-empty-picture-anchor`, with final page images in
`output/roadmap/81-final-held-audit/rasters`.
