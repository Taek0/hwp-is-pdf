# Tables and notes validation — 2026-10-13

Public corpus: https://github.com/edwardkim/rhwp/tree/main/samples
Exact URLs, Git blob IDs and input hashes are in sources.json/sha256.json.
Conversion uses only the Rust engine. No viewer or external converter is called.

## Changes

- Decode HWP footnote/endnote properties, including the observed 28-byte record
  with an extra two bytes after the 16-bit separator length. Preserve spacing,
  separator width/color, placement and numeric prefix/suffix in the common model.
- Reserve note space before body lines and table bands; notes anchored in cells
  use the document column width rather than the cell width.
- Collect endnotes inside tables as well as the main body. END_OF_SECTION and
  END_OF_DOCUMENT are routed separately. Use actual column count and configured
  spacing instead of an unconditional two-column/8-point assumption.
- Correct table split value 2 (split). Keep merged row bands together when they
  fit a page; split oversized text-only rows at shared line boundaries, continue
  cell borders, and repeat eligible headers. Nested oversized objects remain limited.
- Carry the final table fragment position to subsequent text, including text in
  the same hosting paragraph, and move captions with the relocated table.
- Measure local text flow consistently for cell height and rendering. Recover
  repeated zero paragraph origins only when the source cache is unusable.

## Results

| Input | Before pages | Final pages | Before findings (outside / text / rules) | Final findings |
|---|---:|---:|---|---|
| endnote-01.hwp | 5 | 5 | 0 / 0 / 0 | 0; all 6 endnotes, including table note 3, present |
| footnote-01.hwp | 6 | 6 | 0 / 0 / 0 | 0; all 9 footnotes use body width |
| hwpers_test4_complex_table.hwp | 1 | 1 | 0 / 153 / 0 | 0; see fixture caveat below |
| inner-table-01.hwp | 3 | 3 | 0 / 0 / 0 | 0 |
| issue1937_rowbreak_footnote_overpagination.hwp | 54 | 56 | 120 / 749 / 682 | 0 |
| table-complex.hwp | 4 | 4 | 0 / 0 / 0 | 0 |

All six retain every source text character at least as often in text drawing ops
(headers may repeat). Tests additionally check note collection, numeric suffixes,
body-width note placement and two-section note isolation. The entire 40-test suite
and Clippy -D warnings pass. Changed and stress-document pages were visually reviewed.

Despite its name, hwpers_test4_complex_table.hwp has NO table records. It consists
of plain paragraphs with all cached origins zero. It is a paragraph recovery case,
not evidence of complex-table support. table-complex.hwp and the report fixture
exercise actual table records and merged cells.

## Scope limits

Successful PDF creation and zero geometry findings do not establish Hancom-identical
pagination. Very long single footnotes, complex numbering styles, oversized nested
objects inside split rows, and multi-column note corner cases need more coverage.
The existing missing-symbol warning for U+F0FE in inner-table-01 is retained; this
increment does not claim complete private-use glyph support.

Outputs: output/tables-notes/. Initial results: output/tables-notes-before/.
Old-document regression: output/tables-notes-regression/SUMMARY.md.
