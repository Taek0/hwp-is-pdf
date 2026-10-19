# Native nested table fragments, stage 68

The new Hancom reference for `inner-table-01.hwp` has two pages, not the three
previously retained by the regression fixture. The old expected count has been
replaced with the independent print evidence, not simply relaxed.

Three general cache/layout rules explain the difference:

1. An object-only paragraph's saved line can follow a floating table. When the
   preceding gap equals the table height plus outside margins, the table belongs
   before that line. Its height must not be added twice to the cell.
2. A native table containing cell-local page resets can store the first fragment
   height in its declared object size. Retain that value before cache repair
   expands the full table, and use it for the first visible fragment.
3. Centered cell content aligns within each visible fragment. Preserve the first
   cached page break and align the continuation independently when it fits the
   remaining row height. Content is not scaled. Existing top/bottom border styles
   are reused at continuation seams; absent source rules remain absent.

The mixed vector splitter accepts inner tables and text boxes. Glyph protection
uses actual font ink padding and horizontal decoration widths, rather than a
fixed extra 0.6pt on both sides that chained tightly spaced lines together.

The measured anchors for 사업개요, 사업내용, 데이터허브 구축, the first continuation
line, the nested table's first body row and the final 끝 are within 0.15pt of the
Hancom reference. Both pages have identical Hangul multisets. The source model
is unchanged by rendering. An additional fix preserves one printable page for
an otherwise empty saved document, instead of writing a zero-page PDF.

Validation: 296 tests and all-target Clippy pass. All 194 frozen inputs were
attempted: 193 convert to 560 pages, no zero-page outputs remain. One input has
a malformed CFB FAT and remains explicitly recorded. All 63 Hancom reference
pairs / 247 pages agree in page count. This does not establish pixel identity.

Evidence: `output/roadmap/68-nested-table-flow/`, including `all-held/`, the
reference page check, raw geometry findings, Hangul page check, empty-page image,
`table-comparison.png`, and test/Clippy logs. Raw geometry findings in original
zero-spacing, metafile and equation examples require source-aware classification;
they are not silently discarded. Broader text/position measurements for every
reference page are in `output/roadmap/69-reference-audit/`.
