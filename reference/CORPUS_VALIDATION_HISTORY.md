# Corpus validation history through stage 56

These dated snapshots retain their original findings and unfinished items.
Use [CURRENT_CORPUS_VALIDATION.md](CURRENT_CORPUS_VALIDATION.md) for the latest result.

# Current-corpus validation — 2026-10-18

Stage 56 converts 53 unique HWP5/HWPX documents into 260 pages. All 233 Rust
tests and all-target Clippy pass. Source hashes are unchanged. All four geometry
categories, missing-glyph warnings and unresolved link/reference warnings are
empty. Stage 42 rasterized all 260 pages. Stage 43 changes six PDFs / 80 pages;
those were rasterized again and reviewed in four contact sheets plus selected
full-size Hancom comparisons. The other 47 PDFs were byte-identical. Stage 44 changes four PDFs / 96 pages;
these were rendered and reviewed again, with the final one-page glyph-envelope
change checked separately. Its other 49 PDFs are byte-identical to stage 43. This is not a declaration of complete fidelity.

Native table cell flags now preserve custom margins and explicit header rows.
The report's merged two-row header repeats on pages 33–34, recovering 72 missing
Hangul characters. Its pages 9–16 have the same per-page Hangul multisets as the
reference. Fragment-local padding and alignment avoid repeating short cell text
or manufacturing a header-only final fragment.

Native paragraph margins use URC normalization and preserve hanging indents.
JUSTIFY expands word spaces where possible. Resolved native font metrics can
recover redundant cached lines without changing explicit page boundaries.
Report page 3's 19.2pt displacement is removed: body/caption vertical positions
are within about 0.1pt of the reference. The missing Human Myeongjo face resolves
to HCR Batang, matching the reference's embedded font. This rule does not alter
the math document's independently verified Batang substitution.

The FAQ changes from 14 to 13 pages. Its original line cache has 13 page bands,
including an explicit section-3 start on page 7. The previous output inserted an
extra table continuation before that boundary. The final page retains the full
contact table. No source document was edited.

A nested TOP_AND_BOTTOM table now reserves its measured height before following
text. In inner-table-01, the final “끝” moves below the nested table instead of
crossing its right border, with all three pages retained. Narrow punctuation
with excessive negative tracking remains protected against actual collisions.

## Native list spacing and table packing

HWP paragraph-head bits 2 and 3 now preserve instance-width and automatic
hanging-indent settings for bullets and numbering. Fixed native bullet cells
reserve one nominal em before the configured text offset, independent of glyph
side bearings and horizontal scaling. Continuation lines retain the hanging
inset. This restores two-line “대지조성사업” and “주택건설사업” entries in the
report instead of squeezing them into one line.

The one-dot leader U+2024 selects the OTHER language face. The supplied report's
reference embeds HCR Batang for this character, while the surrounding text uses
Dotum. The corrected width restores the four-line “개발행위허가” entry.

Independent cell packing now accepts positioned glyph runs and uses the same
outside margins as the final vector fragment painter. This avoids planning line
moves against a different page capacity. After stage 43, report page 42's note 18 started about 35pt too high
(218.64pt versus 253.9pt); the per-page Hangul deficit on page 43 dropped from
244 to 121 characters. These characters are
on adjacent pages, not lost. Detailed pagination remains active.

The format evidence is Hancom's HWP 5.0 revision 1.2, table 40 (paragraph-head
properties), and embedded fonts/coordinates in the supplied Hancom PDFs.
https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.2.pdf

## Character borders and solid fills

Native character records now retain border/fill IDs at offset 68 and shading
colors. The optional strikeout color is read at offset 70, after the two-byte
border ID. The shared HWP/HWPX painter joins adjacent character spans, draws
solid backgrounds behind their glyphs, and reserves line/neighbor clearance.
The box uses both the nominal em and actual ink envelope, so Latin descenders
and italic side bearings do not cross its strokes. Planned table-note indices
are adjusted when background operations are inserted.

The report's gray boxes around “부담금” and other terms are restored. After this
change note 18 starts at 230.36pt, reducing the remaining error to about 23.5pt.
The page-43 Hangul deficit drops again, from 121 to 78; the full-report Hangul
multisets match exactly at 31,899 characters. This verifies preservation, not
identical page allocation. All 53 documents retain their stage-43 page counts.

## Body flow after continued tables

Saved single-column body flow now invalidates stale automatic page boundaries
after a continued table shrinks as well as after it grows. Explicit page/column
breaks, page-break-before styles and column-definition changes remain boundaries.
Authored empty paragraphs participate in the repaired vertical flow, preserving
intentional blank pages rather than silently collapsing them.

In the report, page 4 now includes the first “더불어…” line; page 6 is empty except
for its footer, as in Hancom; section 2 still starts on page 7. The title after
the research table returns to page 47. Pages 47–48 now have matching Hangul
multisets. The 50-page report is the only changed PDF; its changed rasters are
pages 4–6, 25–29 and 47–48. Other 52 PDFs are byte-identical to stage 44.

## Named page-number styles

Native DocInfo style records now preserve names and shape references. Generated
page numbers select the document's “Page Number” / “쪽 번호” character style,
including source font, size, spacing and half-em spaces. Styled footer text ends
at the footer area's bottom boundary using the face's descent. The generated
paragraph is isolated from any hyperlink field active in body text.

Report page 3's number baseline moves from 785.115pt to 797.07pt, versus Hancom
797.16pt; its left edge is 285.23pt versus 285.12pt. The supplied endnote and
footnote references independently agree within about 0.1pt in baseline position.
The source HCR Dotum face replaces the previous hardcoded Times face.

Five PDFs / 104 pages change (103 visible automatic numbers; FAQ's cover hides
its number). All other 48 PDFs are byte-identical to stage 45. Every non-number
text coordinate and layout bound in the five changed documents is unchanged.
The focused test also proves that body hyperlink state is preserved without
adding links to generated page numbers.

## Cell line-cache boundaries

Native plain-text cell recovery can now update soft-line boundaries without
increasing the line count, including caches whose Y resets inside a paragraph.
The original document remains immutable; explicit breaks, control-containing
lists and nested columns remain outside this recovery path.

Complete lines stored at the same Y retain their specified leading. A transient
correction rejoins the original coordinate grid instead of repeatedly shifting
later paragraphs. Normal advancing coordinates and parallel horizontal segments
remain unchanged. Report pages 3 and 20 now have matching per-page Hangul
multisets. In the latter, the two “광산피해” lines are no longer compressed to
10pt when the source specifies a 16pt advance.

Four PDFs change, with 30 changed page rasters across them. The other 49 PDFs,
including the original math document and its four supplied variants, are
byte-identical to stage 46. The four changed documents retain page counts and
have zero geometry findings; their changed pages were reviewed as images.

## Outside spacing after continued tables

Floating tables now retain their bottom outside margin when they continue onto
another page. The rule applies both to a following paragraph and to text after
the table in the same paragraph. The regression probe first demonstrated a
missing 15pt gap, then passed both paths after the correction.

In the report, the following text starts at 405.507pt on page 4 (Hancom 405.612),
430.079pt on page 20 (430.092), and 744.099pt on page 47 (744.132). All 53 files
retain page counts and zero geometry findings. Three PDFs change, with eight
changed page rasters; the remaining 50 PDFs are byte-identical to stage 47.

## Merged row bands and stale single lines

Closed vertical-merge bands now use fragment-local pagination when a side-effect-free
preflight proves that every band stays whole. Oversized or boundary-crossing bands
retain the vector path before any page is mutated. Shared complete-cell painting
preserves original column widths, padding and alignment.

Native BREAK cells can recover stale one-line text instead of squeezing its font.
Plain paragraphs and supported automatic hanging bullets retain the original
vertical metrics. Empty text and unusable margin regions decline optional recovery;
a 0.1pt visible-advance tolerance excludes tiny font-realization differences.
The notice remains 30 pages after a 0.085pt near-fit was verified separately.

Report pages 20–23 now match the reference's per-page Hangul multisets. Page 23's
`<시행령 추가>` is restored below the repeated header; the following caption returns
to about 463.0pt. The report remains 50 pages and its research table remains on
46–47. Five PDFs change, but two have identical page rasters; 21 page images change
in the form, report and notice. All changed rasters were reviewed. The other 48
PDFs are byte-identical to stage 48, including the math document and four variants.
All 224 tests, all-target Clippy and 53-file / 260-page geometry regression pass.

## Paragraph-internal cuts in native rows

Native fragment admission now retains half of the following line gap when a cut
falls inside a paragraph. A paragraph end and parallel same-Y segments do not
reserve that gap; painted fragment heights remain unchanged. Applying a full gap
or applying it across paragraph boundaries was tested and rejected because it
shifted the previously matching pages 8–16. The accepted rule agrees with both
those rows and the continuation in the report's page 24.

The report's page-24 final paragraph now has one line on page 24 and three on page
25, matching Hancom. Pages 24–26 and 29 now have identical per-page Hangul
multisets; pages 27–28 differ by two Hangul characters at the body-page boundary.
Only the report PDF changes, on six page rasters (24–29); all were reviewed.
The other 52 PDFs are byte-identical. Page count and geometry findings remain
53 files / 260 pages / zero findings. All 225 tests and all-target Clippy pass;
a focused follow-up also verifies that parallel line segments reserve no gap.
This is an empirically validated native-row rule, not a claim about every writer.

## Note-bearing body paragraphs across cached pages

Same-count line-cache recovery now admits footnote/endnote markers whose widths
are measured by the existing annotation path. Original Y slots, line heights,
baselines and leading are retained. Recovered text must keep every annotation in
its original cached page band; paragraph/line-count changes requiring note
repagination remain outside this conservative path. Explicit breaks, other
controls and parallel cached segments retain their existing handling.

Report page-pair differences at 27–28, 31–32 and 44–45 are removed. Its 94 note
bounds are exactly identical to stage 50; only body line endings change. Eight
page rasters (25,27,28,31,32,43,44,45) were rendered and reviewed. All other 52 PDFs
are byte-identical. The corpus remains 53 files / 260 pages with zero geometry
findings, and 227 tests plus all-target Clippy pass. Thirty-nine of the report's
50 pages now have matching per-page Hangul multisets; this is not a pixel metric.

## Missing legacy Latin font

When HCI Poppy is unavailable, the renderer now uses Batang. The supplied Hancom
PDF embeds Batang for those Latin runs; the previous HCR Dotum substitute made
parentheses and digits too narrow. Installed original faces and explicit font
requests continue to take precedence. The native record's default name repeats
HCI Poppy and does not provide a different installed substitute.

At 12pt the reference's `(2)` glyph widths total about 16.164pt; native Batang
outlines total 16.195pt. The remaining 0.031pt difference is print-width
quantization, rather than a reason to rescale glyphs. This restores page 4's
last word boundary, removing the page-pair difference at 4–5. Forty-one of the
report's 50 pages now have matching per-page Hangul multisets.

Three PDFs change (report, public FAQ and notice), with 57 changed page rasters.
All were rendered and reviewed. The other 50 PDFs are byte-identical to stage 51.
The 53 input hashes still match the original manifest, and the full corpus stays
at 260 pages with zero geometry findings. All 228 tests and all-target Clippy pass.

## Remaining-cell grids for split merged bands

Closed merge bands now track each cell's unconsumed lines and minimum height.
A fragment owns its padding and vertical alignment; the remaining row grid is
remeasured after a cut. A pure preflight clones this state before any page is
painted. Complete rows retain their measured grid heights, preventing small
per-row height differences from accumulating across pages. Native table notes
are deferred once to the table's final fragment; multirow headers with notes
and oversized final note-bearing rows retain the established vector fallback.

The new module is `src/layout/native_table_rows/merged.rs`. Synthetic checks cover
both split fixed-height bands and a long spanning cell across three pages, with
every source label/line appearing once, no geometry findings, and immutable input.
The report's existing tests still require 50 pages, the research table on 46–47,
and note 44's 3+1 lines across pages 44–45.

Ellipsis/two-dot leaders can no longer start a wrapped line alone. Retaining the
preceding character removes the final one-character page-pair difference at
41–42. All 50 report pages now match Hancom's per-page Hangul multisets; this does
not assert identical line endings or glyph metrics within each page.

The full 53-file / 260-page corpus has zero geometry findings. Only the report
and notice change: 25 page rasters were rendered and reviewed, and the other 51
PDFs are byte-identical to stage 52. All 230 tests and all-target Clippy pass.

## CJK punctuation font slots and missing leaders

CJK bracket characters U+3008–U+3011 now select the SYMBOL font slot. Missing
HFT HanYang Junggothic symbols resolve to Batang as in the supplied native PDF,
while its Hangul/Latin fallback remains Dotum. Installed original faces and
explicit font overrides retain precedence. The note's explicitly selected HCR
Batang keeps its half-em brackets. This restores the report's cell-internal line
endings, including the final law-name cell on page 42, without changing pages.

When a requested face lacks U+2024, HCR Batang is tried before unrelated fallback
faces. This corrects the three report leaders that previously used MS Gothic;
U+2024 still belongs to OTHER, so existing font-slot rules are not conflated.

The 231-test suite and all-target Clippy pass. A final leader-fallback follow-up
passes both font-selection tests and a fresh 53-document / 260-page conversion.
There are zero geometry findings. Five PDFs change, across 30 reviewed page
rasters; the other 48 PDFs are byte-identical to stage 54.

Seventeen supplied references were compared again. The original four documents,
math variants, native footnotes/endnotes, comprehensive example, complex tables,
chart and EMF/OLE retain matching per-page Hangul counts. The column reference
renders the large labels as outlines; the engine preserves searchable text, and
its blue-glyph raster extents agree within one pixel. Extra extracted WMF labels
are likewise vector-text preservation, consistent with the earlier full 11-page
raster review. The character-style example exposes one remaining text-extraction
issue: shaping runs have ActualText but insufficient ToUnicode fallback mappings.

## Unicode mappings for shaped runs

Painted shaped runs now register cluster-aware ToUnicode mappings. Single-glyph
clusters retain their complete source sequence, including ligatures and composed
marks. Multi-glyph clusters use unambiguous nominal mappings instead of repeating
the whole cluster for every mark. ActualText remains the authoritative logical
span for ambiguous contextual clusters and bidirectional reading order. Font
Unicode entries now hold strings so multi-character UTF-16 destinations survive
PDF writing. Measurement-only runs do not add unused glyphs to the font subset.

The supplied character-style PDF's extracted text now equals the Hancom reference,
including Hangul, Latin font names and the Arabic character. It is the only
changed PDF; its 1400px page raster is exactly identical to stage 55. The other
52 PDFs are byte-identical. A full 53-document / 260-page extraction scan finds
zero U+FFFD replacements or unresolved CID strings, and all input hashes match.

All 233 tests and Clippy pass. Seventeen references / 147 pages retain matching
page counts and have zero missing Hangul. Fifteen match per-page Hangul multisets;
the two differences are the previously verified outline and WMF text preservation.

## Still active

The current corpus has no conversion, geometry or unmapped-character findings.
The report's page allocation and CJK bracket line endings match the supplied
reference. A final character-style matrix check identifies one further visual
difference: synthetic italic uses 0.21256 shear, while the supplied print uses
0.25651/0.75, approximately 0.34202. Calibrate that transform and verify the corpus
before closing the current-example work. General format compatibility is not
inferred from these checks.

Latest artifacts: `output/roadmap/56-shaped-unicode/`, including `regression/`,
`all-verification.json`, `text-integrity.json`, `reference-comparison.json`,
`reference-summary.json`, `tests.log`, `clippy.log`, `page-images/` and
`character-text-proof.png`.
The FAQ source-cache evidence remains in stage 42.

Hancom explains URC indent units and hanging-indent semantics at:
https://forum.developer.hancom.com/t/hwpml/2464

Earlier completed work: [column bands](NATIVE_COLUMN_BANDS.md),
[embedded graphics](EMBEDDED_GRAPHICS.md), [Korean wrapping](KOREAN_BREAKS_AND_TRACKING.md),
and [legacy symbols and links](LEGACY_SYMBOLS_AND_LINKS.md).
