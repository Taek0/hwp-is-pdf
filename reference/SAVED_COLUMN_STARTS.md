# Saved column starts: expanded spanning tables — 2026-10-16

This stage repairs two confirmed omissions in the saved layout path:

1. The flow floor after a growing floating table omitted its bottom outside
   margin. Same-page floating-table continuation now includes that margin.
   Inline and cross-page continuation retain their existing spacing behavior.
2. A full-width table's repaired reservation was cleared at the next saved
   column reset. The reservation now carries to subsequent columns on the same
   page. It expires on a new page or a changed column definition. Only a floating
   table in the first column whose outside width covers the printable width can
   establish the shared reservation.

The initial column divider now also starts below the measured spanning table
and its outside margins, rather than using only the stale saved body origin.
These rules contain no filename, question-number or target-coordinate branches.

## Hancom comparison

Selected question-number text tops, points from the top of page 1:

| Question | Hancom | Before | After | Remaining absolute error |
|---|---:|---:|---:|---:|
| 1 | 169.253 | 166.432 | 169.262 | 0.009 |
| 2 | 426.896 | 424.122 | 426.952 | 0.056 |
| 3 | 185.227 | 159.842 | 169.262 | 15.964 |
| 4 | 523.024 | 497.572 | 506.992 | 16.031 |

**The math document's saved column-flow priority is not complete.** In
particular, the remaining one-line gap on the first right column and the later
column origins differ. The source contains repeated empty-paragraph positions
near column boundaries. Blindly carrying every overflowing blank across cached
resets accumulates large errors and was not adopted. Those boundaries require
further logical-flow reconstruction; the positions in this table do not justify
adding page-specific offsets. Broader text, equation and table differences also
remain outside this correction.

## Validation

180 tests and all-target Clippy pass. All 43 documents / 205 pages preserve
source hashes, page/object counts and extracted character multisets, with zero
geometry findings. The other 42 PDFs are byte-identical to stage 31. The math
document retains 9 pages, 249 equations and 22 pictures; pages 2–9 are also
pixel-identical to the previous output. Its divider begins at 153.38pt versus
153.36pt in Hancom. The page-1 before/after/reference image was visually reviewed.

The regression fixture uses asymmetric table margins, a measured cell taller
than its declared table, two saved columns, and an explicit new page. It checks
both text origins, the divider start, expiration of the shared reservation and
source immutability. The existing corpus and reference comparisons are recorded
under output/roadmap/32-saved-column-starts/.

Key artifacts: math.pdf, math-page1-comparison.png, images/,
question-positions.json, corpus-text-verification.json, verification-summary.json,
tests-final.log and clippy-final.log. Original documents and generated outputs
remain excluded from Git.
