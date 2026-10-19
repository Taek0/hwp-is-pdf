# Current-corpus validation — 2026-10-19, stage 60

The four-priority Hancom prints exposed new format/layout issues. The engine now
includes the corrections in [OFFICIAL_PROBE_CALIBRATION.md](OFFICIAL_PROBE_CALIBRATION.md).
The following results describe the supplied corpus, not universal compatibility
or pixel identity. Remaining visible differences are recorded in that document.

| Check | Result |
|---|---|
| Inputs | 73 unique HWP5/HWPX documents; all source SHA256 hashes unchanged |
| Conversion | 308 pages, zero failures |
| Geometry | Zero text-overlap, border-crossing, outside-page or unintended foreground-image findings |
| Hancom references | 40 PDFs / 199 pages; all page counts agree |
| Existing 60 inputs | All page counts and per-page non-whitespace character multisets unchanged; 49 PDFs byte-identical |
| Rust | 263 tests pass; all-target Clippy with warnings denied passes |
| Image review | 74 changed/new pages rasterized; contacts and detailed comparison crops reviewed |
| PDF extraction | No replacement-character or unresolved-CID strings |

The report remains 50 pages, with the research table on 46–47 and the restored
schedule bars on 49. The form remains 2 pages. The new checkbox form is 10 pages
(previously 15); all ten per-page Hangul multisets match its Hancom reference.
The source-driven HWPX hanging-indent repair also improves the math page-6 box;
its four rows and following answer remain within their verified boundaries.

## Text extraction interpretation

Raw reference text counts differ for eleven pairs. Five effect/vertical/parity
prints repeat 368 Hangul characters in decorative shadow layers; the engine
preserves all 72 source Hangul characters once, painting effects as glyph paths.
The outline probe likewise retains all 49 source Hangul characters. Other
extras are searchable engine labels where the reference uses outlines or
metafile/chart drawing (columns, WMF, fields title, charts and outlines).
These differences are documented rather than hidden by a global normalization.
Text counts alone cannot establish correct glyph placement or reading order.

The earlier blank-column references use landscape A4 printing of 400x300pt
source pages; the engine retains source dimensions. Page-count agreement does
not mean that those physical paper sizes are equal.

## Reproduce and inspect

Local artifact directory: `output/roadmap/60-official-probe-calibration/`.

- `all-inputs.json`: the 73-input manifest; only two early synthetic-cache style
  probes explicitly use reflow. Other documents retain normal saved-layout mode.
- `regression-final/`: final PDFs, layout reports, geometry reports and corpus report.
- `verification.json`, `tested-source.json`, `baseline-comparison.json`,
  `reference-pairs.json`, `reference-comparison.json`, `effect-source-text.json`.
- `tests-final.log`, `clippy-final.log`, `changed-pages.json`, `raster-manifest.json`.
- `page-images-final/`, `new-probes-contact-*.jpg`, `changed-existing-contact-*.jpg`,
  `compare-forms-detail.png`, `compare-report-49.png`, `compare-math-hanging.png`,
  and `compare-*.png` for detailed side-by-side examples.

Run `cargo run --example reconvert_manifest -- MANIFEST OUTPUT --keep-going`.
Run `cargo test --all-targets --no-fail-fast` and
`cargo clippy --all-targets -- -D warnings` in the configured font environment.

No input documents, private fonts, diagnostic reference-font subset or generated
PDFs/images are committed. Broader limits remain in [FORMAT_COVERAGE.md](../FORMAT_COVERAGE.md).
Password documents, HWP 2.x/3.x and source editing remain excluded by user choice.
