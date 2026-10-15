# Hancom reference-output comparison — 2026-10-15

Engine revision: 0325153dd44a43cccf63055e2efcc82de6dea68e.
Roadmap stages 1–10 are implemented within the limits in FORMAT_COVERAGE.md.
Stage 10 passes 126 tests, Clippy, and the 43-document / 210-page regression.

## Attempt and observed limitation

Opened the existing installed HwpViewer.exe from Office 2024 Viewer using the
computer-use skill. Opened the unchanged user document D:/ChatGPT/hwp/복학원서.hwp.
The viewer reported that current HIGH document security prevents loading some
picture formats. No security settings were changed. The informational alert was
dismissed, and the Print dialog showed Microsoft Print to PDF selected.

The viewer status displayed 1/2 pages, while this engine's saved-layout PDF has
1 page. This is a preliminary pagination discrepancy, not a completed PDF
comparison: the viewer's picture loading was restricted and no reference PDF
was produced. Font/layout conditions and actual second-page contents remain
unverified. Do not interpret baseline regression equality as Hancom fidelity.

Both the initial Print action and a retry after refreshing window selection failed
with the computer-use runtime error `failed to activate captured window`.
The skill recovery rule requires stopping after failed recovery. No further UI
input was sent. The viewer may remain open at the Print dialog.

## Remaining evidence needed

Hancom-generated PDFs from the unchanged originals, ideally the three user files
(복학원서.hwp, 21_언어_기출_편집가능본.hwp,
2단_수학시험지_변환검증_7826.hwpx), plus the complex footnote corpus example,
are needed for page-by-page comparison. Put reference exports under
output/roadmap/11-hancom/hancom/. No reference images or fidelity percentages
were fabricated. Once exports are available, compare page counts, text order,
object geometry and raster overlays and record font/security differences.

Local evidence: output/roadmap/11-hancom/viewer-print-dialog.png.

## User-supplied reference PDFs received — 2026-10-15

The user exported four PDFs into output/roadmap/11-hancom/hancom/temp/.
중간보고서.pdf initially had zero bytes while printing; the completed file is
651,051 bytes and parses successfully as 50 pages. All four files parse and all
157 combined reference/engine pages rasterize successfully with Poppler.

| Reference PDF | Original | Hancom pages | Rust pages |
|---|---|---:|---:|
| 복학원서.pdf | 복학원서.hwp | 2 | 1 |
| 법학적성시험.pdf | 21_언어_기출_편집가능본.hwp | 15 | 15 |
| 내신도깨비.pdf | 2단_수학시험지_변환검증_7826.hwpx | 9 | 9 |
| 중간보고서.pdf | samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp | 50 | 56 |

Reference identities were matched using first/last-page text. Original PDFs remain
unchanged. Hashes, PDF paths and every page's dimensions are in the local
output/roadmap/11-hancom/comparison.json. Reference and engine page PNGs and text
are in compare/{form,law,math,report}/. No new converter behavior was changed.

### Findings from inspected pages

- Reinstatement form: Hancom places the filing receipt on page 2; Rust keeps it
  below the main form on page 1. The receipt text exists in both. Font weights,
  receiving-official cell wrapping, watermark geometry and signature-line/symbol
  appearance differ. The small upper-left logo is absent from the reference;
  the prior viewer run reported restricted image formats, so that absence alone
  is not attributed to an engine defect.
- Law exam, page 1: page count and broad two-column content placement agree, but
  the title weight/spacing, header-box geometry and rounded corners differ.
  Hancom shows passage-box borders missing in the Rust output. A zero geometry-QA
  result cannot detect a missing source border or establish visual fidelity.
- Math exam, page 1: page count agrees; the top header's title wraps differently,
  with different row heights and page-border offsets. Text font weights, line
  spacing and subsequent table/figure vertical positions differ.
- Report: page 2 is a genuinely empty TOC placeholder in the Hancom export too
  (only the TOC title and footer). Rust's absence of entries there is not a lost
  rendered TOC. Footer dashes and text weight still differ.
- Report table continuation differs substantially: page 10 contains different
  row ranges in each output. The final Hancom page 50 corresponds to Rust page 56
  and contains the same concluding section, with different line spacing/weight.
  The six-page increase must be investigated in table/paragraph measurement and
  continuation; it is not evidence by itself of extra or missing source content.

### Comparison scope and next priorities

All page counts, dimensions, parsing and rasterization were checked. Visual review
covered the form's two reference pages and single Rust page; first pages of both
exams; report page 2, paired pages 10/25/50 and Rust page 56. This is an initial
reference comparison, not an exhaustive semantic audit of every line or a measured
compatibility percentage. Text-extraction alignment is only a navigation aid;
private-use math glyphs, reading order and sparse pages limit its reliability.

Priorities from actual reference evidence:
1. Correct form page-break/receipt placement and report table height/continuation.
2. Restore paragraph/passage borders and rounded header-box geometry.
3. Match Korean/Latin font face, bold weight, spacing, and header wrapping.
4. Recompare affected and baseline documents against these reference PDFs.

Local visual report: output/roadmap/11-hancom/SUMMARY.html.
The earlier missing-reference blocker is now resolved by user-supplied exports.

## Pagination repair validated — 2026-10-15

The next implementation matches the reference page counts: 복학원서 2 pages and
중간보고서 50 pages, from previous 1/56. Review artifacts and source-hash/object-count
verification are in output/roadmap/12-pagination/. Full tests: 128 passing; Clippy
passing; 43 baseline documents / 205 pages, zero geometric findings.

The original PDFs in hancom/temp remain unchanged. Captured reference note placement
in the report (18–39 collected after the floating table) informed the saved-layout
policy. Receipt content is on page 2; all report text is retained by source-character
checks, and the concluding section remains on page 50. Some intermediate page
boundaries and typography still differ: page-count agreement alone is not visual
identity. In particular, the final research table begins on Rust page 47 versus
Hancom page 46. Remaining typography/border fidelity is tracked in FORMAT_COVERAGE.md.

## Typography and borders — 2026-10-15

The next comparison uses output/roadmap/13-typography/. Font matching now respects
installed family names, TTC face indices and native bold faces, with synthetic
bold only when needed. Source font-space behavior and terminal paragraph controls
are retained. The math title now wraps into two lines with corresponding nested
cell height growth. Law passage borders and rounded heading boxes are restored.

132 tests and Clippy pass. The 43-document baseline remains 205 pages, with source
hashes and object/page counts unchanged, zero geometric findings and no new missing
glyphs. Four targets retain 2/15/9/50 pages and the previous non-whitespace character
multisets. All 76 target pages were rendered and their contact sheets reviewed;
selected page pairs and enlarged details are in the local HTML report. Paired
images are visual aids, not a pixel-error measurement (report previews may be
resized to the reference preview size).

Absolute positioning, some intermediate page boundaries, receipt wrapping,
watermark/symbol rendering and math page-border offsets still differ. Font
fallback and synthetic weight/italic styles remain approximations. Full source
fonts are embedded, so PDF size is not optimized. Reference PDFs are unchanged.
No missing content can be inferred solely from the different small form logo:
the viewer previously warned of image blocking at its HIGH security setting.

Format references:
- [HWP 5.0 revision 1.3](https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf):
  character space flags, paragraph border flags/offsets, rectangle corner ratio.
- [Hancom paragraph border help](https://help.hancom.com/hoffice130/ko-KR/Hwp/format/paragraph/paragraph(border).htm):
  column/text-area border extent and adjacent paragraph connections.

## Page coordinate correction — 2026-10-15

The math reference's outer frame was measured directly from the supplied PDF:
left/right 28.32/566.88 pt, top/bottom 48.24/779.52 pt. Source text margins and
outward offsets predict 28.35/566.93/48.20/779.51 pt. This establishes a text-relative
interpretation for this HWPX export despite its PAPER wire enum. The renderer now
uses that compatibility interpretation for HWPX; the native HWP5 adapter retains
the origin described by table 136 separately. More independently exported
CONTENT/PAPER examples would strengthen coverage of this format discrepancy.

Correcting the border exposed an existing tall-footer placement defect: negative
bottom-alignment slack pulled the footer back into the body. The footer now starts
at its allocated origin when it exceeds the reserved height. Paragraph-relative
CENTER/BOTTOM object positions also stop using a page-height displacement.

The prior stage's PAPER-offset assertion tested the implementation's assumption,
not the Hancom output. It has been replaced with the reference-derived coordinate
assertion. 137 tests and Clippy pass. Artifacts, corpus checks and remaining
differences are recorded in output/roadmap/14-coordinates/SUMMARY.html.

Sources:
- [Hancom page-border positioning](https://help.hancom.com/hoffice/multi/ko_kr/hwp/format/pageborder/page_border(border).htm)
- [Hancom object position](https://help.hancom.com/hoffice/multi/ko_kr/hwp/insert/objectattribute/objectattribute(position).htm)
- [HWP5 revision 1.3, tables 130 and 135–136](https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf)

Final stage-14 regression: 43 documents / 205 pages, all source hashes and object/
page counts unchanged, zero geometry findings and no new warnings. 42 PDFs are
byte-identical to stage 13. The math document is the only changed baseline PDF;
its nine pages were visually reviewed. The four target PDFs preserve their
non-whitespace character multisets and 2/15/9/50 page counts. Frame edge errors
against the reference are 0.030/0.050/0.040/0.010 pt (left/right/top/bottom).
This does not imply matching table positions, fonts or all page contents.
