# Visual graphic audit — stage 91, 2026-10-20

All 203 held inputs (594 pages) were rerendered at a 1400px long edge and
reviewed through 30 overview sheets. This was followed by targeted magnification,
source-geometry inspection and comparison with available independent prints.
Overview review is not a claim that every pixel was examined at full resolution.

## Checks and finding

- 284 source line/rectangle/ellipse/polygon/curve/arc/group nodes have matching
  output-bound records. A bound alone does not prove visible paint.
- All 43 explicit source line objects were inspected in enlarged crops. Their
  126 visible occurrences have PDF stroke operations; two NONE-style examples
  are intentionally invisible.
- Horizontal-edge screening covered 64 reference PDFs. Its raw candidates are
  retained: glyph outlines, rasterized metafiles, split table segments and known
  layout/position differences are not automatically classified as missing lines.
- Targeted comparisons exposed **missing cell diagonals**, which are not standalone
  shape nodes: two page-counter cells on pages 10–11 of the language exam, and
  eight decorative title cells in the government notice.

The decoder previously discarded the native border-fill diagonal flags and fifth
stroke. The cell border painter also painted only the four outer edges. Both paths
now preserve and render the observed straight slash/backslash. HWPX CENTER flags
use the same painter. Inactive diagonal definitions remain invisible.

The format fields were checked against Hancom's [official HWP 5.0 specification,
section 4.2.5](https://cdn.hancom.com/link/docs/%ED%95%9C%EA%B8%80%EB%AC%B8%EC%84%9C%ED%8C%8C%EC%9D%BC%ED%98%95%EC%8B%9D_5.0_revision1.3.pdf).
The supplied print establishes the rising direction of the exam's slash.
Only the observed straight two-corner diagonals are newly supported. Fan-shaped,
bent and center-line variants are not covered by this addition.

## Results

Ten diagonal strokes are restored in two PDFs. All characters, positions, sizes
and page counts in both PDFs are unchanged; the remaining 201 PDFs are byte-identical
to stage 90. Changed-document pages (45 total) were rendered again, and the restored
details were inspected. Raw overlap/outside-page QA findings are unchanged.
336 tests and all-target Clippy pass. Tests check both directions, inactive flags,
native decoding and the two independently printed page-counter strokes.

All 454 currently accessible source copies still match the frozen hashes. Two old
sibling-editor `.tools/test-temp/hwp-editor-native-test-1656` duplicates are no longer
present at the recorded paths; this run did not remove them. Both canonical inputs
and their conversion results remain included in the 203-input audit.

## Evidence and remaining limits

`output/roadmap/91-visual-geometry-audit/index.html` links all 594 current page images
and 203 PDFs. The folder also contains the page ledger, source graphics, explicit
line crops/stroke checks, retained reference-edge candidates, source hashes and
two before/after diagonal comparison images. The metadata-only Rust example
`audit_graphic_geometry` makes source geometry/diagonal definitions repeatable.

The TILE-fill blank and the two unrotated rotation-labeled test rectangles were
checked against the supplied Hancom print and match it; they were not changed.
Known body/table placement differences, damaged image payloads, incomplete OLE
viewer prints and inputs without independent references remain separate limits.
This audit fixes the confirmed missing strokes, not every existing fidelity difference.
