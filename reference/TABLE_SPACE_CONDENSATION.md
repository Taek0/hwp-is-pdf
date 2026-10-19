# Floating table anchors, merged grids and spaces — stage 75

Print input 7's legal citation was painted after its three-page table and ended
up on page 3. A paragraph-relative square-wrapped table located entirely below
its anchor now follows the same deferred painting rule as top/bottom wrapping.
The anchor remains on page 1 without moving the table or losing text.

Its upper complete rows retained a 448.57pt total whereas the declared extent and
lower rows used 450.02pt. The incompatible constraints previously fell back to
an incorrect grid. Native complete rows within 1% of the declared extent are
normalized before solving crossing merged boundaries, only when the old grid is
already inconsistent. Partial rowspan rows and genuinely conflicting constraints
are not invented or averaged.

HWP paragraph property bits 9–15 retain the documented 0–75% `condense` setting.
Line fitting now spends this explicit interword-space budget before scaling
letters or breaking the line. Valid saved cell boundaries that fit the budget
remain authoritative. Tabs do not use this condensation path. HWPX's same-named
attribute uses the common layout settings. The supplied header's one-line third
cell and two-line second cell now preserve their printed line endings.

Evidence: `output/roadmap/75-square-table-anchor`. All three per-page Hangul
multisets match the independent print. The corpus remains 194 inputs / 561 pages;
original source files are not edited. The full suite passes 318 tests; the final
saved-cell refinement is also checked with all checkpoint, native-cell,
row-fragment, form, saved-table and short-note tests. Clippy passes all targets.

The field definition is from Hancom's HWP 5.0 revision 1.3, paragraph-shape
property table: https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf.
The measured behavior here does not establish every printer's interword rounding
or complex-script condensation semantics.
