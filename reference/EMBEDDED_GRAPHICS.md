# Embedded combination charts and WMF — 2026-10-18

The newly supplied references exposed two complete conversion failures: native
combination charts were replaced by an overcrowded table, and WMF image decoding
aborted the document. Both supplied files now render without those failures.

## Combination chart

The bounded VtChart reader retains manual primary/secondary scales, per-series
painter selectors, colors, pen widths, plot extents and axis text styles.
Axis scale-block fonts take precedence over the axis-title defaults; legacy
strings are terminated at their first NUL instead of interpreting padding as
part of the font name. Horizontal label alignment is retained too.

The type-9 column/line renderer uses the two source scales and draws all 102
values in each of the two series. The provided chart retains the 80–110 primary
axis, 0–6 secondary axis, red line and cyan columns. Sparse category labels do
not create a 102-column data table. Geometry findings fall from 208 text overlaps
and 17 rule crossings to zero. Small printer/font/layout differences remain;
plot padding is currently computed from label metrics, not a full VtChart UI
layout engine. Unsupported native painter types still fail explicitly.

## WMF

`src/metafile/` separates bounded record parsing, embedded DIB extraction,
object-table playback and text rendering. It does not execute escapes, OLE,
scripts, GDI, the viewer or external converters. The provided document uses
21 DIB bitmaps inside 13 WMFs, plus polygons, rectangles and positioned text.
Those operations become ordinary image/vector/text PDF operations. Logical
window coordinates, lowest-free object handles, font selection, CP949/ANSI text
and explicit per-byte advances are honored. Internal diagram text is treated
like chart text rather than incorrectly flagged as body text over a photograph.

The reader follows Microsoft's [WMF format](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/),
including [STRETCHDIB](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/7ebae08d-61ee-4d82-9aa5-9217ba2aa8c1)
and [EXTTEXTOUT](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/7d07c44a-a828-4b82-9af0-e0a81cced5a8).
It bounds record count, object count, input size and bitmap pixel count. This is
coverage of the supplied WMF record set, not all of WMF/EMF: unsupported drawing
records, raster operations, rotated/clipped text and palette modes are rejected.

Native inline pictures also include outside margins and vertical captions in
their cached baseline reservation. This removed 14–30pt image displacement and
15 caption/body overlaps in the WMF reference. It applies to ordinary native
pictures as well, with dedicated asymmetric-margin and top/bottom-caption tests.

## Verification

196 tests and all-target Clippy pass. The pre-existing 43-document / 205-page
corpus is byte-identical to stage 37, with unchanged inputs, text and object
counts and zero geometry findings. All 13 additional reference documents now
convert and have zero findings; page counts match the supplied Hancom PDFs.
After deduplication this covers 53 source files / 261 pages. All eleven WMF
pages were raster-compared, including caption boundaries and text-only diagrams.

Artifacts: `output/roadmap/38-embedded-graphics/` contains manifests,
reference-verification.json, verification-summary.json, regression/reference PDFs,
tests.log, clippy-final.log, chart-comparison.png, wmf-comparison.png and full WMF
page images. These checks do not certify universal fidelity. In particular,
legacy family-name resolution and Korean word-breaking semantics remain active
follow-up work against the same supplied references.
