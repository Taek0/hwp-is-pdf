# Priority status — 2026-10-20

The newly printed format probes exposed additional differences. The latest
implementation and remaining visible limits are documented in
[CURRENT_CORPUS_VALIDATION.md](CURRENT_CORPUS_VALIDATION.md) and the stage 65–88
reports linked there. Page/content/geometry
checks and visual fidelity are separate results; a passing corpus does not prove
universal compatibility. Retain tests, images, commits and pushes for each stage.

| Priority | Current-example result | Broader follow-up |
|---|---|---|
| 1. Paragraph/column flow | Math question starts and supplied blank-column/offset probes verified | Other writer/cache patterns and additional independent column references |
| 2. Font/style/position | Named page-number styles, CJK slots, legacy Latin/symbol fallbacks, synthetic italic/width scaling and generated labels corrected | Missing original faces, variable/color fonts and wider style calibration |
| 3. Wrapping | Native hanging indents, word spacing, annotated same-count caches and ellipsis boundaries verified | Unmeasured stale caches, justification and protection combinations |
| 4. Tables/notes | Remaining-cell merged grids, repeated headers, padding, table tails and supplied note-page distribution verified | HWPX CELL splitting/check appearances now supported; wider mixed-object structures retain guarded fallback paths |
| 5. Pictures/borders | Supplied grayscale/contrast probes and placement verified; source PCX preserved with optional viewer-compatible hiding | Wider tone parameters, visibility policies and compound-rule variants |
| 6. Formats/PDF semantics | Supplied chart/WMF/EMF/OLE examples and shaped ToUnicode strings verified | Complete metafile/chart families, vertical objects, broader equation commands/original faces, unmeasured chart styles and brush variants |

See [CURRENT_CORPUS_VALIDATION.md](CURRENT_CORPUS_VALIDATION.md) for current evidence,
[FORMAT_COVERAGE.md](../FORMAT_COVERAGE.md) for supported scope, and
[CORPUS_VALIDATION_HISTORY.md](CORPUS_VALIDATION_HISTORY.md) for earlier findings.

Keep excluded: password-encrypted documents, HWP 2.x/3.x and source-file editing.
Do not replace format rules with filename-specific coordinates or page conditions.

The held-file audit now covers every one of 202 distinct inputs (454 copies),
including the sibling editor's old exports. Paired independent PDFs are
still unavailable for 139 inputs, and the OLE equation viewer reference is incomplete.
The separately exercised numbering-edited math probe now produces 9 pages under
full `--reflow`, with all 26 answers advancing by 10pt on page 9. Primary saved
layout also remains 9. Body paragraph origins and some font/rule appearances
still differ; see [REFLOW_LINE_EXTENTS.md](REFLOW_LINE_EXTENTS.md). These limits
are explicitly recorded rather than converted into a universal 95% score.
