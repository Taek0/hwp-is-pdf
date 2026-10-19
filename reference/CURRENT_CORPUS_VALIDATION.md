# Current-corpus validation — 2026-10-20, stage 88

Engine snapshot: `0b16b80925e38a61eb7175d810511104409a53a7`.
Engine changes: [REFLOW_LINE_EXTENTS.md](REFLOW_LINE_EXTENTS.md).
This audit distinguishes exercising a file from proving exact Hancom fidelity.
It does not certify a universal 95% format-completion figure.

| Check | Result |
|---|---|
| Workspace inventory | 454 physical HWP/HWPX files, 202 distinct SHA-256 inputs, including sibling editor exports |
| Conversion | 202/202, 592 pages, no failures or zero-page outputs |
| Source preservation | All 454 source-copy hashes unchanged |
| Raster validation | All 592 pages rendered; page inventories and changed-page comparisons retained |
| Drawable inventory | All 202 inputs scanned, no unknown positioned/group-child drawable kinds |
| Independent references | 63 PDFs / 247 pages; all page counts agree; one reference is incomplete |
| Text glyph warnings | Zero missing-glyph warnings |
| Rust checks | 331-test suite passes; final note-only change also passes targeted integration tests and all-target Clippy |
| Latest change | 47 PDFs change from stage 82 with identical character multisets; other 155 PDFs byte-identical |

## Corrections verified in this increment

- [HWPX property alternates](HWPX_PROPERTY_ALTERNATES.md): header tab branches resolve once.
- [Reflow line extents](REFLOW_LINE_EXTENTS.md): blank-column boundaries, table height/margins, spanning headings, hanging indents and endnote spacing.

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
  full reflow: now 9 pages, matching the primary saved-layout result and Hancom.
  All 26 answers use 10pt leading on page 9. Body origins and some font/rule
  appearances still differ; page-count equality does not certify exact fidelity.
- Four blank-column probes were printed on different paper from their declared
  400×300pt source. Bottom-origin PDF coordinates are not directly comparable.
- Extractable text differences from shadows, outlines, raster chart labels and
  vector WMF text are recorded. They must not be counted blindly as lost content.
- Saved thumbnails are secondary, potentially stale evidence. 101 were decoded;
  50 without full prints were additionally inspected. Copied template thumbnails
  were not used to overwrite the actual source content.

Portable summary: [verification-stage88.json](verification-stage88.json).
Local current PDF index and reference checks:
`output/roadmap/88-endnote-leading/index.html`.
Earlier detailed ledger and unchanged raster evidence remain in stage 81.
The ledger records every input, mode, source hash, output hash and limitation.
Earlier stage numbers are historical in [CORPUS_VALIDATION_HISTORY.md](CORPUS_VALIDATION_HISTORY.md).
