# Empty-paragraph column-cache probes — 2026-10-16

Status: waiting for independent Hancom exports; no layout correction is enabled
by this diagnostic stage.

The math document contains repeated final empty-line coordinates followed by a
column-origin reset. A simple overflow carry can improve early columns while
accumulating incorrect displacement later. Neither dropping duplicate empty
paragraphs nor applying a per-page correction is justified by that evidence.

`inspect_saved_columns INPUT [FONT_DIR]` labels root paragraphs in an in-memory
copy and emits source line caches alongside rendered root-line bounds. It never
writes the input. Its diagnostic IDs are not used by the production renderer.

`blank_column_probes INPUT.hwpx OUTPUT_DIR PARA_ID CHAR_ID` makes two copies of
a complete HWPX package with 32 identical logical paragraphs and three visible
labels. Choose a 10pt character style and a 160% paragraph style; the current
local math source uses IDs 62 and 5 respectively. The two copies differ only in
saved line Y coordinates: one describes normal two-column flow, while the other
stalls the last coordinate and resets later. The generator refuses to overwrite
existing probes, removes stale preview entries and verifies source immutability.

Local inputs: output/hancom-blank-column-probes/normal-cache.hwpx and
stalled-cache.hwpx. Export both in Hancom to corresponding PDFs in that folder.
Whether Hancom preserves or reconstructs those different caches is the evidence
needed before selecting a recovery rule. Do not assume the expected result.

Current unchanged engine: normal-cache produces 2 pages, stalled-cache 1 page.
Both preserve A START, B AFTER BLANKS and C NEXT PAGE. Logical XML was compared
after removing only linesegarray nodes and is identical. Both outputs were
rendered for inspection. The new examples compile and pass focused Clippy;
production source code and the established 180-test baseline are unchanged.

Artifacts: output/roadmap/33-blank-flow/math-columns.json, normal-cache.pdf,
stalled-cache.pdf, their page PNGs, and diagnostic-clippy.log. Probe packages
contain source-package resources and remain local, outside Git.
