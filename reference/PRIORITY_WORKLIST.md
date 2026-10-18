# Priority status — 2026-10-18

The currently identified supplied-document issues have passed the latest corpus
checks. These categories also contain broader, unmeasured format work; a passing
example set does not prove universal compatibility. Future stages should retain
tests, image review, regression conversion, commits and pushes.

| Priority | Current-example result | Broader follow-up |
|---|---|---|
| 1. Paragraph/column flow | Math question starts and supplied blank-column/offset probes verified | Other writer/cache patterns and additional independent column references |
| 2. Font/style/position | Named page-number styles, CJK slots, legacy Latin/symbol fallbacks, synthetic italic/width scaling and generated labels corrected | Missing original faces, variable/color fonts and wider style calibration |
| 3. Wrapping | Native hanging indents, word spacing, annotated same-count caches and ellipsis boundaries verified | Unmeasured stale caches, justification and protection combinations |
| 4. Tables/notes | Remaining-cell merged grids, repeated headers, padding, table tails and supplied note-page distribution verified | Wider mixed-object structures; established fallback paths remain for unsupported fragment combinations |
| 5. Pictures/borders | Supplied grayscale/contrast probes and placement verified; source PCX preserved with optional viewer-compatible hiding | Wider tone parameters, visibility policies and compound-rule variants |
| 6. Formats/PDF semantics | Supplied chart/WMF/EMF/OLE examples and shaped ToUnicode strings verified | Complete metafile/chart families, outline/shadow/emboss, vertical typography and broader equation grammar |

See [CURRENT_CORPUS_VALIDATION.md](CURRENT_CORPUS_VALIDATION.md) for current evidence,
[FORMAT_COVERAGE.md](../FORMAT_COVERAGE.md) for supported scope, and
[CORPUS_VALIDATION_HISTORY.md](CORPUS_VALIDATION_HISTORY.md) for earlier findings.

Keep excluded: password-encrypted documents, HWP 2.x/3.x and source-file editing.
Do not replace format rules with filename-specific coordinates or page conditions.
