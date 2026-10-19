# Merged-column constraints and form captions — stage 62

The supplied 27-column form has 83 cell-width equations with zero inconsistency.
The previous fill-missing-columns-evenly procedure violated some merged intervals,
shifting later controls despite retaining the overall table width.

A bounded boundary graph now reconciles consistent cell widths when the prior
column grid violates them. All authored endpoints must connect to the origin;
contradictory, disconnected or non-increasing grids retain the established path.
Only unreferenced internal boundaries are interpolated. Both native table width
and cell layout use the same result. Tests cover crossing merges, row-order
independence, conflicting widths and already-valid grids.

Checkboxes now use their actual control origin and center caption em cells without
adding the font's Windows internal leading. The first five caption origins in the
printed form agree within 0.25pt, covered by a user-fixture regression test.

Validation: 274 Rust tests, all-target Clippy, 73 inputs / 308 pages, zero failures
or four-class geometry findings, unchanged input hashes. All 40 reference page
counts still agree. The form's ten per-page Hangul multisets match Hancom. Only
07_form_controls.pdf and page 1 of the law exam change relative to stage 61;
the other 71 PDFs are byte-identical. The law exam's text remains unchanged.

Artifacts: output/roadmap/62-table-column-constraints/. See verification.json,
changed-pdfs.json, tests-final.log, clippy.log, compare-form-grid.png and
compare-law-page1.png. A separate follow-on investigates native center-baseline
inline objects visible in the law exam; the column repair does not alter Y.
