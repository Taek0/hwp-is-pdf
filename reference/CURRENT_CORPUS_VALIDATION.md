# Current-corpus validation — 2026-10-18

## Result

The issues identified in the currently supplied documents have been corrected and
rechecked. This is a statement about this corpus and its available references,
not a percentage of universal HWP compatibility.

| Check | Result |
|---|---|
| Main corpus | 53 unique HWP5/HWPX documents, 260 pages |
| Earlier user-printed probes | 7 additional unique documents, 13 pages |
| Combined conversion | 60 documents, 273 pages; no conversion failures |
| Geometry | No text overlaps, rule crossings, outside-page content or unintended foreground-image overlaps |
| PDF text extraction | No replacement characters or unresolved CID strings |
| Inputs | All 60 SHA256 hashes unchanged |
| Rust validation | 235 tests and all-target Clippy pass |
| Reference PDFs | 24 documents, 160 pages; every page count matches |

The main reference set contains 17 PDFs / 147 pages. Fifteen match per-page Hangul
multisets, including all 50 report pages. Two retain additional searchable text
where the reference uses outlines or embedded metafile drawing. The seven older
probes also match their reference's per-page non-whitespace character multisets.
No reference Hangul is missing. Character counts complement the rendered-page
reviews; they do not replace visual checks or establish reading order by themselves.

## Completed corrections

- Native paragraph margins, hanging indents, word spacing and stale line caches.
  Note-bearing paragraphs preserve their saved page slots and annotation pages.
- Plain and merged table fragments retain cell padding and vertical alignment.
  Closed merge bands track unconsumed lines and remeasure the remaining row grid;
  a pure preflight runs before page painting. Complete rows keep measured heights.
- Repeated headers, outside spacing, table/body repagination and deferred table
  notes. The research table remains on pages 46–47; note 44 retains its 3+1 split.
  The report's empty TOC page 2 and empty page 6 match the supplied print.
- Character borders, native page-number styles, CJK punctuation font slots,
  missing HCI Poppy/Junggothic symbol faces, and U+2024 fallback.
- Ellipsis line-start protection and mixed-language ToUnicode mappings, including
  multi-character ligatures. The character-style sample's extracted text now
  equals the reference while the Unicode-only change preserved its page pixels.
- Synthetic italic uses the measured 0.34202 base shear with character-width and
  line-fit scaling. Native italic faces stay native. Positive authored advances
  are retained across slanted/upright style boundaries; generated labels follow
  the same transform. The supplied style sample's X errors are below 0.065pt.

Stage 57 changes three PDFs, each on its first page: the comprehensive example,
character-style example and law exam. Their per-page text is unchanged; the other
50 main-corpus PDFs are byte-identical to stage 56. Changed pages were rendered
and reviewed. Earlier reviews cover unchanged pages.

## Reference interpretation

- Column labels and several WMF labels are outlines in their reference PDFs;
  searchable engine text is intentional. Column glyph raster extents agree
  within one pixel at the compared resolution. WMF pages retain the prior full
  11-page image review.
- The native PCX logo is preserved by default. `--hancom-hide-pcx` reproduces the
  viewer's omission; no source or application security setting is changed.
- The blank-column probes declare 400×300pt paper. The engine preserves it;
  the reference printer used landscape A4. Their document-area word positions
  agree within 0.241pt horizontally and 0.089pt vertically at physical scale.
- Printer-grid rounding, JPEG recompression and unavailable original faces can
  produce small rendering differences. These checks do not claim pixel identity.
- Broader format limits, including uncalibrated cases outside these examples,
  remain listed in [FORMAT_COVERAGE.md](../FORMAT_COVERAGE.md). Password documents,
  HWP 2.x/3.x and source editing remain excluded by user decision.

## Reproduction and artifacts

Main manifest: `output/roadmap/41-paper-picture-placement/all-inputs.json`.
Probe manifest: `output/roadmap/57-synthetic-italic/prior-probes-inputs.json`.

Latest artifact directory: `output/roadmap/57-synthetic-italic/`:

- `regression-final/`: all 53 main PDFs, layout reports and geometry checks.
- `prior-probes/`: the seven earlier probe conversions and checks.
- `final-verification.json`, `all-verification.json`, `text-integrity.json`.
- `reference-comparison.json`, `reference-summary.json`,
  `prior-probes-verification.json`, `probe-visual-verification.json`.
- `tests-final.log`, `clippy-final.log`, `page-images-final/`,
  `final-report-comparison.png`, `italic-comparison.png`, `contact-final.jpg`,
  and `blank-probes-physical-*.png`.

User documents, fonts and generated artifacts remain outside Git. Earlier
experiments, measurements and their historical unfinished items are preserved in
[CORPUS_VALIDATION_HISTORY.md](CORPUS_VALIDATION_HISTORY.md).
