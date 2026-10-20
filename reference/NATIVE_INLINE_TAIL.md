# Native inline text overflow — stage 89, 2026-10-20

## Independent input and reference

Input 25 is the unchanged native HWP “사용료 산출기준”, downloaded from
`https://www.law.go.kr/LSW/flDownload.do?flSeq=164233033`.
SHA-256: `6BB61D19A68B573900797D5BCD920FE7406405806B93CA13148016CDBE58BC1C`.
The user's viewer print has two pages (178499 bytes).

Two table-cell OLE objects contain version-5 HwpEq5x `Contents` expressions:
`20 / 10,000` and `월간 일평균유입량(㎥/일) / 시설용량(㎥/일)`.
Their `OlePres000` streams are 28-byte stubs. The viewer PDF leaves those
expressions blank; the engine reads and retains the scripts. This is an
incomplete appearance reference, not permission to discard source content.
It does not by itself prove a viewer bug or exact original font appearance.

## Corrected engine defect

The first-page formula built from inline groups and ordinary text occupies
494.886pt, exceeding its 481.88pt saved line. The old renderer compressed the
text to keep the closing brace on the same line. The print moves the brace to
a new line, advancing subsequent paragraphs and the page break.

The new bounded recovery handles a native one-line cache only when all inline
objects remain on the first measured line. It preserves that object's cached
height/baseline and wraps only the trailing text. Tabs, notes, explicit line
breaks, numbering and floating objects are excluded. Growth is measured from
the complete final line box rather than its origin; the following body uses
the recovered page flow. There are no production filename or paragraph-ID rules.

Results:

- Closing-brace line origin: (56.69pt, 326.14pt), no horizontal compression.
- Following `(1) M` paragraph: y=355.88pt; printed glyph top agrees within 0.1pt.
- Final concentration-description line moves to page 2. Its following heading
  starts at y=105.19pt; printed glyph top agrees within 0.15pt.
- Three equations remain; the two source OLE expressions are preserved.
- Both pages have no missing Hangul versus the reference. The only extra Hangul
  is the recovered table equation. All non-whitespace extracted characters are
  unchanged from the engine's pre-fix output.
- Two pages, no outside-page text, text overlaps or border crossings.

## Regression and limits

203 distinct inputs / 456 held copies convert to 594 pages. All previous 202
PDFs are byte-identical to stage 88, and all source-copy hashes are unchanged.
The full 333-test suite and all-target Clippy pass. The added tests cover the independent printed origins, source equation retention,
page continuation, and the negative case where narrower objects fit normally.

Some font shapes, equation/group horizontal positions and second-page line
endings still differ. Missing original fonts are reported; these differences
have not been “fixed” by replacing valid source data with the viewer's omissions.

Local evidence is in `output/roadmap/89-native-ole-reference/`: full conversion
manifest/report, `audit-summary.json`, `reference-check.json`, `after.pdf`,
and two corrected comparison images. Unchanged rasters from stage 88 remain
valid; the new two pages were rendered and visually inspected.
