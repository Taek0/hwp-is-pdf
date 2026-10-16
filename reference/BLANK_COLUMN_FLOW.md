# Saved blank-column continuation — 2026-10-16

Four user-printed Hancom probes establish the corrected continuation behavior.
The original packages contain `secPr/visibility hideFirstEmptyLine="1"`.
After an automatic column/page break, up to two leading empty lines consume no
height. Content ends this suppression; explicit breaks preserve authored empty
lines. A line fits using its saved content height, excluding trailing line spacing.
Those two rules explain all four references without margin-dependent offsets.

The bounded two-line suppression is also independently described in the
[rhwp implementation changelog](https://github.com/edwardkim/rhwp/blob/main/CHANGELOG.md)
(hide_empty_line entry for kps-ai). No upstream implementation code or dependency
was copied. The four supplied outputs are the numerical acceptance evidence.

## Implementation scope

The recovery path operates on logical line order and retains positive authored
gaps between cached lines. It does not treat a repeated or reset empty-line Y
coordinate as an authoritative column break. Existing source text wrapping,
object placement, full-width table reservations and endnotes remain integrated.
The source tree is unchanged. Numbered empty items and paragraphs containing
objects or notes are not suppressed as blank content.

Activation is limited to cached, equal-width HWPX newspaper columns with the
visibility flag enabled and empty-line boundary evidence. Native HWP, unequal
columns, actual footnotes and protected paragraph groups retain their established
paths. A column-definition change ends this recovery range. Explicit page and
column breaks are honored. Wider format coverage remains separate work.

Two accompanying corrections are required for reliable output:

- A current rectangle height already reserved by a saved line must not increase
  subsequent line positions again. Its inline baseline uses that reserved height.
- Column dividers are emitted only when `colLine` requests one. The probe source
  has no divider; the math document explicitly has a solid divider.

## Reference results

Positions below are line origins, before glyph font metrics and printer rounding:

| Probe | Pages | B origin | C origin |
|---|---:|---|---|
| Normal cache, top/bottom 40pt | 2 | page 1, right column, 72pt | page 2, left column, 40pt |
| Stalled cache, top/bottom 40pt | 2 | page 1, right column, 72pt | page 2, left column, 40pt |
| Bottom margin 80pt | 2 | page 1, right column, 120pt | page 2, left column, 104pt |
| Top margin 20pt | 1 | page 1, right column, 36pt | page 1, right column, 228pt |

All visible label tops differ from Hancom by less than 0.1pt. The Hancom PDFs
were printed onto A4 landscape; comparisons use source-content coordinates,
not PDF media-box equality. Page count and label position are checked separately.

## Math document and remaining work

The math document retains nine pages, 249 equations, 22 pictures and 17 table
draws. Questions 1–13 and 21–26 now start within 0.1pt of the Hancom reference.
Previously the measured start error reached about 105.42pt. Questions 14–20
still start about 16pt early: question 13 wraps to five lines in Hancom and four
in the engine, moving the following image/table and affecting subsequent flow.
That difference needs paragraph/font wrapping work, not a per-question offset.
The document is not claimed to be pixel-identical to Hancom.

Follow-up evidence: the reference's `13)` endnote anchor is white (RGB 1,1,1)
but occupies horizontal space at x=68.4..79.68pt. The current text painter skips
endnote marker width when paragraph numbering is present. Its contribution to
the remaining wrapping difference is a specific next investigation, not part
of this completed blank-column correction.

Artifacts under output/roadmap/34-blank-column-recovery/: math.pdf,
math-page*-comparison.png, probe-comparison.png, comparison.json,
math-question-positions.json, corpus-text-verification.json, verification-summary.json,
tests-confirmed.log and clippy-final.log. Source documents and outputs are not
committed. Synthetic tests cover all four cases and the already-reserved rectangle;
the real-document regression retains shape-content and collision checks.

183 tests and all-target Clippy pass. The 43-document / 205-page corpus retains
source hashes, page/object counts and character multisets with zero findings in
the four geometry classes. The other 42 PDFs are byte-identical; math pages 8
and 9 remain pixel-identical. Tests-confirmed.log is the final successful run;
the earlier test run correctly exposed an obsolete assertion requiring the
now-removed duplicate 30.47pt shape-height adjustment.
