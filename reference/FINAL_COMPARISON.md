# Final independent-engine comparison — 2026-10-15

This closes the requested implementation/verification pass, not a claim of full
HWP compatibility or pixel identity with Hancom. Password-encrypted documents,
HWP 2.x/3.x and original-document editing remain outside the agreed scope.

## Delivered changes

| Stage | Change | Code commit |
|---|---|---|
| 15 | HWPX floating-table margins; native BREAK/SQUEEZE/KEEP and bounded stale-cell recovery | bd2aaca |
| 16 | Ink-based transformed-text pagination and allocated table-height consistency | 5e6618e |
| 17 | Language-specific font properties and optional OpenType Latin pair kerning | cb6037b |
| 18 | Border patterns, alpha-preserving HWP/HWPX image effects, original Hancom PUA glyphs | c281dc7 |
| 19 | Stable-ID TrueType subsetting with composite closure and full-font fallback | 78e29b6 |
| 20 | Single automatic-row height allocation; source page-number side characters; final audit | 7ad6155 |

Implementation modules are separated by responsibility under src/. The largest
Rust source file is 576 lines. No Python implementation or viewer DLL dependency
was added. Generated images/PDFs remain local in ignored output/; fixture generators,
tests and this verification record are committed.

## Verification

- 151 Rust tests pass; all-target Clippy with warnings denied passes.
- 43 baseline inputs / 205 pages reconverted with original SHA-256 hashes intact.
- Page/picture/table/equation/chart counts retained; zero findings in the four
  geometry classes; no new missing-glyph warnings.
- In the last layout/furniture pass, 38 PDFs retain non-whitespace character
  multisets. Five restore only page-number dashes: FAQ 26, notice 60, endnote 10,
  footnote 12, report 100. No PDF character was removed. Thirty-seven PDFs are
  byte-identical to the previous stage.
- Earlier legacy-symbol repair replaced three "(인)" substitutes with the original
  two U+F012B / one U+329E glyphs and restored 100 U+F081C decorative characters.
  These are intentional restorations, not unexpected text loss.
- Four Hancom references / 76 pages were rasterized at 1000px. All final engine
  contact sheets and selected full-size pairs were reviewed; paired images are
  available for every page.
- Font subsetting alone was verified separately: all 76 rendered pages are
  pixel-identical to the full-font stage, with identical extracted text.
  The subsequent automatic-row and page-number changes are intentionally distinct.

## Reference page counts and PDF sizes

Sizes are decimal MB, rounded. Before is the full-font stage 18.

| Document | Hancom pages | Rust pages | Before | Final |
|---|---:|---:|---:|---:|
| 복학원서 | 2 | 2 | 49.14 MB | 0.94 MB |
| 법학적성시험 | 15 | 15 | 52.93 MB | 1.44 MB |
| 내신도깨비 | 9 | 9 | 59.64 MB | 2.13 MB |
| 중간보고서 | 50 | 50 | 101.88 MB | 2.82 MB |

The math page frame differs from the reference by at most approximately 0.05 pt.
The form's first grid rule moved from 104.20 pt to 118.38 pt, versus Hancom
115.92 pt: the error fell from 11.72 pt to 2.46 pt. The correction allocates spare
declared height to the single automatic header row, rather than the final fixed row.
Page-number side characters reproduce the report's "- 50 -" furniture.

## Remaining visible differences

- Form: the receiving-official label is still one English line versus two in
  Hancom. Some title/cell offsets and watermark tone/geometry differ.
- Math/law: some line breaks, column content positions and typography differ.
- Report: intermediate page boundaries differ; the final research table starts
  on Rust page 47 versus Hancom page 46, while both end on page 50.
- Missing original faces and synthetic styles remain approximations. Compound
  border pattern pitch/3D shading are not calibrated to every Hancom version.
- The HWPX PAPER border compatibility interpretation is backed by the supplied
  print; additional independently exported origin variants remain desirable.
- Advanced formats and effects listed in FORMAT_COVERAGE.md remain partial.

The report's page 2 is an empty TOC placeholder in both outputs. The form's source
contains a small logo absent from the reference print; earlier viewer image
security warnings may explain this. The engine preserves source pictures rather
than deleting them to imitate a potentially filtered print.

No overall compatibility percentage is inferred from page counts or QA zero.

## Local evidence

- output/roadmap/20-final/SUMMARY.html — PDF downloads, 76 paired page images,
  contact sheets and links to every stage.
- output/roadmap/20-final/final-verification.json
- output/roadmap/20-final/corpus-text-verification.json
- output/roadmap/20-final/tests.log and clippy.log
- output/roadmap/19-final/pixel-and-size-verification.json
- output/roadmap/15-tables/ through 19-final/ — stage-specific artifacts.

## Format sources

- [Hancom HWP5 revision 1.3](https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf)
- [Hancom OWPML model](https://github.com/hancom-io/hwpx-owpml-model)
- [OpenType glyf](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf),
  [loca](https://learn.microsoft.com/en-us/typography/opentype/spec/loca) and
  [font-file checksums](https://learn.microsoft.com/en-us/typography/opentype/spec/otff).

