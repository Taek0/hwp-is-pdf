# Current HWP/HWPX coverage — 2026-10-16

본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.

Scope: standalone Rust HWP 5.x / HWPX visual conversion, including public-format
distribution documents. Password-encrypted documents, HWP 2.x/3.x and editing are
excluded by user decision. **Full format compatibility and pixel identity with
Hancom are not achieved.**

## Verified capabilities and limits

| Area | Implemented | Remaining limits |
|---|---|---|
| Text | Installed family/TTC face selection, native or synthetic styles, scripts/offsets, language slots, optional Latin pair kerning, Unicode and installed Hancom PUA glyphs | Missing original fonts, exact synthetic metrics, some outline/shadow/emphasis and vertical typography |
| Body layout | Saved-cache rendering and recovery, supported mixed reflow, floats/contours, column changes and shared-note flows | Complex overlaps, all writer variants and exact Hancom line/page boundaries |
| Tables | Merged/nested cells, structural splitting, repeated headers/captions, mixed pictures/rotations/notes, stale text recovery and single automatic-row height allocation | Ambiguous multiple auto rows and exact row/paragraph composition |
| Notes | Footnotes/endnotes, long-note continuation, source-driven placement and numbering | Some precise page distribution differs |
| Graphics | Curves, rotation/crop/group transforms, static OLE images, legacy charts, common border patterns and alpha-safe raster effects | General WMF/EMF/OLE, all chart families, advanced effects/text art and exact pattern/3D metrics |
| Equations | Supported HWP grammar, fractions, scripts, roots, matrices, accents and layout integration | Complete grammar and exact Hancom math metrics |
| Fields/PDF semantics | Cached fields, supported TOC/field evaluation, links/bookmarks, extended lists and page-number side characters | Arbitrary field recalculation and unsupported variants |
| PDF fonts | Stable-ID TrueType subsetting, recursive composites, metrics/hinting/checksums, Unicode mappings | Variable/color/non-glyf/no-subsetting faces retain full embedding; no universal color-font support claimed |

## Final validation

158 Rust tests and all-target Clippy pass. 43 baseline documents / 205 pages retain
their source hashes and page/picture/table/equation/chart counts, with zero findings
in the four geometry audit classes. All 43 outputs retain character multisets
relative to stage 21, including repeated headers. 41 PDFs are byte-identical to
stage 21. Native plain-text tables can carry staggered cell lines independently
and close fragment bottoms using the source cell rule; transformed objects retain
the shared-ink fallback. Earlier native-symbol repair
separately restores two U+F012B, 100 U+F081C and one
U+329E in place of three lossy "(인)" substitutes.

Four user-exported Hancom references agree in page counts (2/15/9/50). The 76-page
reference set and earlier paired rasters are retained; changed pages have fresh
comparisons. Font subsetting alone preserves every
pixel at 1000px while reducing target PDF sizes by 96.4–98.1%.
Subsequent layout corrections are verified separately from font subsetting.

Centered Latin receipt-label wrapping is corrected. Known differences include approximately 2.5pt
remaining form frame offset, internal report table/footnote page boundaries, some typography,
watermark tone/geometry and border pattern details. The empty report TOC page is
also empty in Hancom. QA zero is not a compatibility percentage or proof of reading order.

Latest evidence: output/roadmap/22-table-fragments/SUMMARY.html and
[TABLE_FRAGMENT_COMPARISON.md](reference/TABLE_FRAGMENT_COMPARISON.md).
Historical increments, fixture restrictions and earlier measurements are preserved
in [IMPLEMENTATION_HISTORY.md](reference/IMPLEMENTATION_HISTORY.md); their old
"current"/"remaining" statements describe those earlier versions.

## References

- [HWP 5.0 revision 1.3](https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf)
- [Hancom OWPML model](https://github.com/hancom-io/hwpx-owpml-model)
- [OpenType glyph data](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf)
- [OpenType font tables/checksums](https://learn.microsoft.com/en-us/typography/opentype/spec/otff)
