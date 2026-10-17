# Current-corpus validation — 2026-10-18

The current manifest has 53 unique HWP5/HWPX documents and 261 output pages.
Stage 41 passes 202 Rust tests and all-target Clippy. Source hashes and all
page/object counts remain unchanged; all four geometry categories, missing-glyph
warnings and unresolved link/reference warnings are empty. All 261 pages were
rasterized. This is structural/geometry validation, not a declaration of complete
Hancom fidelity.

The latest placement correction preserves native pictures that extend into the
print margins but remain inside the physical paper, including rotated-envelope
checks. The WMF reference's large page-3 picture now begins at x=93.60pt and keeps
its 427.20pt width, versus Hancom x=93.55pt / 426.97pt. Previously it was moved to
x=85.04pt and reduced to 425.20pt. The other 52 PDFs remain byte-identical.

## Additional discrepancy found during deeper comparison

Page-by-page comparison of the report reveals differences that geometry QA
cannot detect: in particular its merged two-row table header is absent from the
engine's pages 33–34 while Hancom repeats it. Several table and footnote fragments
also distribute text differently. These are active follow-up work in stage 42;
the user's request to continue until the currently observed document problems
are resolved has not been marked complete.

Artifacts under `output/roadmap/41-paper-picture-placement/` include the 53 PDFs,
all-inputs.json, all-verification.json, all page images, tests-final.log,
clippy-final.log, picture-comparison.png and hancom-page-text-audit.json. The latter
is a diagnostic: character counts alone do not distinguish pagination from
reference-font extraction differences.

Recent completed stages:

* [Native column bands and nested columns](NATIVE_COLUMN_BANDS.md)
* [Combination charts, WMF and picture baselines](EMBEDDED_GRAPHICS.md)
* [Korean wrapping and tracking](KOREAN_BREAKS_AND_TRACKING.md)
* [Legacy symbol glyphs and native links](LEGACY_SYMBOLS_AND_LINKS.md)
