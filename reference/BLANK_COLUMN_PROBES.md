# Empty-paragraph column-cache probes — 2026-10-16

Status: all four Hancom exports received. The validated implementation and
remaining mixed-content differences are described in BLANK_COLUMN_FLOW.md.

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
Both user exports contain two pages, with identical label positions: A at
page 1 (39.96,39.925), B at page 1 (210,71.965), C at page 2 (39.96,39.925).
PDF titles identify the two different inputs. This establishes identical results
for this pair despite different line caches, not a rule for all document caches.

The unchanged forced-reflow engine places B at about 121.779pt and C at about
105.779pt; even the normal saved-cache result differs. Cache invalidation alone
is therefore insufficient. `--bottom-margin-probe` creates only
bottom-margin-80.hwpx, with the normal probe's bottom margin changed from 40pt
to 80pt. XML comparison confirms no other section change. Its Hancom export is
used to distinguish empty-paragraph fitting from cached-coordinate recovery.
The returned bottom-margin-80 export also has two pages: A at (39.96,39.925),
B at page 1 (210,119.965), C at page 2 (39.96,103.885). Relative to the 40pt
bottom-margin reference, B moves down 48pt and C down 63.96pt. Thus the bottom
margin affects the logical column/page break; treating the repeated cache
coordinates alone cannot establish the correct flow. These observations do not
yet establish a general rule for carrying or discarding empty paragraphs.
The additional top-margin-20 export has one page: A at (39.96,20.005),
B at (210,35.965), C at (210,227.965). It rejected a draft that used top margin
as an empty-line allowance. Reading the source visibility flag and combining
bounded leading-empty-line suppression with content-height fitting explains
all four results. The rejected margin formula was not committed to the engine.

Current unchanged engine: normal-cache produces 2 pages, stalled-cache 1 page.
Both preserve A START, B AFTER BLANKS and C NEXT PAGE. Logical XML was compared
after removing only linesegarray nodes and is identical. Both outputs were
rendered for inspection. The new examples compile and pass focused Clippy;
production source code and the established 180-test baseline are unchanged.

Artifacts: output/roadmap/33-blank-flow/math-columns.json, normal-cache.pdf,
stalled-cache.pdf, their page PNGs, and diagnostic-clippy.log. Probe packages
contain source-package resources and remain local, outside Git.
