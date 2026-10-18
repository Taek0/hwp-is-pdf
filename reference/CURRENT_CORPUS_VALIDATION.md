# Current-corpus validation — 2026-10-18

Stage 48 converts 53 unique HWP5/HWPX documents into 260 pages. All 218 Rust
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

## Still active

The report remains 50 pages, and its research table remains on pages 46–47.
There are still per-page text differences around pages 4–5, 21–29 and 31–32,
and especially the merged body cells with notes on pages 35–45. Finer line
endings and some cell/footnote placement still differ. Geometry QA does not
catch these. Continue reference comparison and general layout fixes; the user's
request to resolve current document problems is not complete.

Latest artifacts: `output/roadmap/48-table-outside-margin/`, including `regression/`,
`all-verification.json`, `tests.log`, `clippy.log`, `changed-pages.json`,
`page-images/` and `margin-comparison.png`. The last per-page text audit remains
in stage 47.
The FAQ source-cache evidence remains in stage 42.

Hancom explains URC indent units and hanging-indent semantics at:
https://forum.developer.hancom.com/t/hwpml/2464

Earlier completed work: [column bands](NATIVE_COLUMN_BANDS.md),
[embedded graphics](EMBEDDED_GRAPHICS.md), [Korean wrapping](KOREAN_BREAKS_AND_TRACKING.md),
and [legacy symbols and links](LEGACY_SYMBOLS_AND_LINKS.md).
