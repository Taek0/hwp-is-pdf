# HWPX cached inline table baselines — stage 79

Some HWPX line caches explicitly reserve the complete table outside box: saved
text height equals table height plus top/bottom outside margins, and the baseline
is 85% of that combined height. Aligning only the bare table added those margins
again. The renderer now uses the proved outside-box baseline in this case, while
retaining existing behavior for rebuilt caches, unmatched extents and native HWP.

The supplied press-release header/contact table shifts upward by 4.811pt. Its
page-5 contact text now agrees with Hancom within approximately 0.02pt; the title
baseline differs by about 0.11pt. This also improves several supplied math and
OLE table positions. Other document-flow/font differences remain separate from
this baseline rule; nearest-character statistics alone are not a fidelity score.

All 202 sources convert to 594 pages. Thirteen PDFs change and 189 are byte-identical
to stage 78. Raw geometry findings remain the same four source-authored cases;
no missing-glyph warnings occur. The complete 323-test suite and all-target Clippy
pass. The new controlled test verifies unequal outside margins explicitly.

Artifacts: `output/roadmap/79-hwpx-inline-baseline` (including before/after reference
statistics and the complete input regression). No filename-specific layout rules
are used.
