# Current HWP/HWPX coverage — 2026-10-18

본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.

Scope: standalone Rust HWP 5.x / HWPX visual conversion, including public-format
distribution documents. Password-encrypted documents, HWP 2.x/3.x and editing are
excluded by user decision. **Full format compatibility and pixel identity with
Hancom are not achieved.**

## Verified capabilities and limits

| Area | Implemented | Remaining limits |
|---|---|---|
| Text | Installed families/TTC faces, native/synthetic styles, scripts/offsets, language slots, kerning, half-em Latin tracking, positioned generated labels, Hancom/legacy symbol glyphs and vector emphasis | Missing original fonts, exact synthetic/mark metrics, outline/shadow/emboss and vertical typography |
| Body layout | Saved-cache recovery, supported mixed reflow, floats/contours, native column bands/nested columns, Korean word/character semantics, punctuation and shared-note flows | All writer variants, ambiguous stale caches and unmeasured Hancom line/page cases |
| Tables | Merged/nested cells, structural splitting, repeated headers/captions, mixed pictures/rotations/notes, stale text recovery and single automatic-row height allocation | Ambiguous multiple auto rows and exact row/paragraph composition |
| Notes | Footnotes/endnotes, long-note continuation, source-driven placement and numbering | Some precise page distribution differs |
| Graphics | Curves, rotation/crop/group transforms, static OLE images, bounded WMF raster/vector/text playback, native dual-axis column/line charts, border patterns and alpha-safe effects | Full WMF/EMF/OLE and chart-family coverage, advanced effects/text art, exact pattern/3D metrics |
| Equations | Supported HWP grammar, fractions, scripts, roots, matrices, accents and layout integration | Complete grammar and exact Hancom math metrics |
| Fields/PDF semantics | Cached fields, supported TOC/field evaluation, links/bookmarks, native escaped URL commands, extended lists and page-number side characters | Arbitrary field recalculation, source-authored bad targets and unsupported variants |
| PDF fonts | Stable-ID TrueType subsetting, recursive composites, metrics/hinting/checksums, Unicode mappings | Variable/color/non-glyf/no-subsetting faces retain full embedding; no universal color-font support claimed |

## Final validation

208 Rust tests and all-target Clippy pass. The current unified corpus contains
53 unique documents / 260 pages, with unchanged source hashes. FAQ now has
13 pages, matching its source cache bands rather than the old spurious 14-page output. It has zero findings in the four geometry
classes, zero missing-glyph warnings and zero unresolved link/reference warnings.
All pages were rasterized. The notice's formerly collapsed private-use character
mappings were corrected to match its 28 original symbol occurrences.
The report's research table
fits pages 46–47, matching the reference range. Further merged-table and note-line placement differences remain under investigation.
Saved single-column short notes can continue when their first complete line fits
with the anchor; exact note-line distribution still differs in some cases.
Native plain-text tables can carry staggered cell lines independently
and close fragment bottoms using the source cell rule; transformed objects retain
the shared-ink fallback. Earlier native-symbol repair
separately restores two U+F012B, 100 U+F081C and one
U+329E in place of three lossy "(인)" substitutes.

Four user-exported Hancom references agree in page counts (2/15/9/50). The 76-page
reference set and earlier paired rasters are retained; changed pages have fresh
comparisons. Font subsetting alone preserves every
pixel at 1000px while reducing target PDF sizes by 96.4–98.1%.
Subsequent layout corrections are verified separately from font subsetting.

Centered Latin receipt-label wrapping, native inline-table outside-margin baselines
and text-box vertical alignment are corrected. Selected form/law title Y errors are
0.033/0.008pt. Known differences include internal report table/footnote page boundaries, some typography,
watermark tone/geometry and border pattern details. The empty report TOC page is
also empty in Hancom. QA zero is not a compatibility percentage or proof of reading order.

Latest corpus result: [CURRENT_CORPUS_VALIDATION.md](reference/CURRENT_CORPUS_VALIDATION.md).
Latest additions: [NATIVE_COLUMN_BANDS.md](reference/NATIVE_COLUMN_BANDS.md),
[EMBEDDED_GRAPHICS.md](reference/EMBEDDED_GRAPHICS.md),
[KOREAN_BREAKS_AND_TRACKING.md](reference/KOREAN_BREAKS_AND_TRACKING.md) and
[LEGACY_SYMBOLS_AND_LINKS.md](reference/LEGACY_SYMBOLS_AND_LINKS.md).
Historical increments, fixture restrictions and earlier measurements are preserved
in [IMPLEMENTATION_HISTORY.md](reference/IMPLEMENTATION_HISTORY.md); their old
"current"/"remaining" statements describe those earlier versions.

## References

- [HWP 5.0 revision 1.3](https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf)
- [Hancom OWPML model](https://github.com/hancom-io/hwpx-owpml-model)
- [OpenType glyph data](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf)
- [OpenType font tables/checksums](https://learn.microsoft.com/en-us/typography/opentype/spec/otff)
