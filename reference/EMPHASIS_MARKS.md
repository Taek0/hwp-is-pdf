# Emphasis marks and typography extensions — 2026-10-16

## Supported addition

HWP character-shape emphasis codes 1–6 are decoded into the shared character
model. NONE and reserved codes do not create marks. The HWP CARON/TILDE order is
mapped explicitly rather than assuming the HWPX enumeration has the same order.
HWPX supports the twelve named marks: dot/ring above, tilde, caron, side dot,
colon, grave/acute accent, circumflex, macron, hook above and dot below.

Marks use colored vector paths rather than extra Unicode text. Ordinary glyphs
and their marks form indivisible pagination groups; blank characters have no
mark. Reflow accounts for the mark's vertical extent. Shaped text places marks
on advancing glyph clusters while preserving logical ActualText. A dedicated
layout/text_marks.rs module keeps geometry out of the main text painter.

Mark geometry is font-relative and deterministic. Exact Hancom mark size,
position and shaping behavior have not been verified against viewer exports;
the new inputs are controlled fixtures, not claimed reference renderings.
Outline, shadow, emboss/engrave and full vertical typography remain separate work.

## Evidence

- 164 tests and all-target Clippy pass.
- HWP parsing covers valid/reserved emphasis codes alongside existing italic,
  superscript/subscript and strike properties.
- HWP/HWPX examples preserve logical text; the HWPX PDF's extracted text is
  exactly identical before and after marks are enabled.
- Tests include all named marks, space suppression, RTL shaped clusters and
  a multipage table whose glyphs and marks must remain together.
- All 43 baseline documents / 205 pages are byte-identical to stage 25 and
  retain source hashes, page/object counts and zero geometry findings.
- Both new fixture PDFs and the empty-paragraph PDF were rasterized and reviewed.

Examples: samples/text-style/emphasis-marks.hwpx and emphasis-marks.hwp.
The native fixture copies samples/additional/underline-styles.hwp and changes
only its character-shape emphasis bits; original inputs are not modified.
Generator: examples/typography_fixtures.rs. Local evidence:
output/roadmap/26-emphasis-marks/SUMMARY.html, comparison.png, native.pdf,
tests.log, clippy.log and corpus-text-verification.json.

The preceding empty-paragraph correction is recorded in EMPTY_PARAGRAPH_REFLOW.md.
The math saved-column origin mismatch remains unresolved; the rejected trial
was reverted and its altered PDF is not a deliverable. Existing form/law title
offsets and other reference differences remain as previously measured.

## Primary format sources

- [Hancom HWP 5.0 revision 1.3, table 35 (character-shape flags)](https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf)
- [Hancom OWPML SYMBOLMARKTYPE and name mapping](https://github.com/hancom-io/hwpx-owpml-model/blob/main/OWPML/Class/enumdef.h)
