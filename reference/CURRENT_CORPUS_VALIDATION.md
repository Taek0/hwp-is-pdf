# Current-corpus validation — 2026-10-19, stage 82

Engine snapshot: `aeb5c584aac3323e783defa0f49c4e09f68ad4fb`.
This audit distinguishes exercising a file from proving exact Hancom fidelity.
It does not certify a universal 95% format-completion figure.

| Check | Result |
|---|---|
| Workspace inventory | 454 physical HWP/HWPX files, 202 distinct SHA-256 inputs, including sibling editor exports |
| Conversion | 202/202, 594 pages, no failures or zero-page outputs |
| Source preservation | All 454 source-copy hashes unchanged |
| Raster validation | All 594 pages rendered; page inventories and changed-page comparisons retained |
| Drawable inventory | All 202 inputs scanned, no unknown positioned/group-child drawable kinds |
| Independent references | 63 PDFs / 247 pages; all page counts agree; one reference is incomplete |
| Text glyph warnings | Zero missing-glyph warnings |
| Rust checks | Full 326-test suite and all-target Clippy pass |
| Latest change | Only the scaled-picture sample changes from stage 80; other 201 PDFs byte-identical |

## Corrections verified in this increment

- [Source charts and OLE](SOURCE_CHARTS_AND_OLE.md): supported 2D source-backed
  charts, OLE formula/static EMF recovery, HWPX containers and declared font substitution.
- [Native group frames](NATIVE_GROUP_FRAMES.md): title displacement reduced from
  about 90.7pt horizontally / 29.8pt vertically to under 0.1pt.
- [Table grids and space budgets](TABLE_SPACE_CONDENSATION.md): the legal citation
  stays on page 1; merged boundaries and printed header line endings are retained.
- [Native endnotes](NATIVE_ENDNOTE_ORIGIN.md): 32-bit rule lengths, full-width rule,
  and the first-note baseline agree with the supplied print within about 0.15pt.
- [Paragraph borders](PARAGRAPH_BORDER_INK.md): 1302 crossings in the additional
  editor export are eliminated without changing any character or text position.
- [HWPX inline baselines](HWPX_INLINE_BASELINES.md): proved outside-box caches remove
  the press-release/contact table's 4.811pt displacement.
- [Note-bearing table rows](NOTE_TABLE_ROW_TAILS.md): the report's extra page-33
  row band disappears. All 50 per-page Hangul multisets still match Hancom.
- [Empty picture anchors](EMPTY_PICTURE_ANCHORS.md): a scaled block image no longer
  moves downward by its own 225pt height; source geometry and saved preview corroborate it.

## Raw QA is retained, not hidden

| Input | Raw findings | Independent/source evidence |
|---|---|---|
| Fixed-zero-leading sample | 8 text overlaps | Same overlap is visible in Hancom |
| WMF OLE sample | 66 duplicate glyph overlaps, 8 underline contacts | Hancom has the same 66 duplicated positions and touching parentheses |
| Press-release header | 7 foreground-image overlaps | White text intentionally overlays the gray image |
| Grouped title | 2 foreground-image overlaps | Title intentionally overlays the banner; positions match Hancom |

Outside-page findings: zero. Unclassified geometry findings: zero. These are
geometry classifications, not a declaration that every glyph/pixel is identical.

## Explicit validation boundaries

- 139 inputs have no paired independent PDF. They received conversion,
  raster, source/structure and regression checks, not a fabricated visual oracle.
- Seven archived viewer-rejected inputs remain marked as such. Successful tolerant
  conversion does not establish their package/schema validity.
- Input 20 contains five truncated BMP payloads and replacement thumbnail images.
  Missing original pixels are reported and are not reconstructed.
- Input 12's viewer print omits its OLE formula and explanation paragraphs. The
  engine preserves them; a complete print is still needed for exact comparison.
- Four deliberately stale generator caches use explicit `--reflow` in the ledger.
  The numbering-edited counterfactual math probe was also tested separately with
  full reflow: 10 pages versus Hancom's 9. Its primary saved-layout result is 9 pages.
  This remains a known full-reflow fidelity limit, not an untested file.
- Four blank-column probes were printed on different paper from their declared
  400×300pt source. Bottom-origin PDF coordinates are not directly comparable.
- Extractable text differences from shadows, outlines, raster chart labels and
  vector WMF text are recorded. They must not be counted blindly as lost content.
- Saved thumbnails are secondary, potentially stale evidence. 101 were decoded;
  50 without full prints were additionally inspected. Copied template thumbnails
  were not used to overwrite the actual source content.

Portable summary: [verification-stage82.json](verification-stage82.json).
Local complete ledger, CSV, PDF links and 594 page images:
`output/roadmap/81-final-held-audit/index.html`.
The ledger records every input, mode, source hash, output hash and limitation.
Earlier stage numbers are historical in [CORPUS_VALIDATION_HISTORY.md](CORPUS_VALIDATION_HISTORY.md).
