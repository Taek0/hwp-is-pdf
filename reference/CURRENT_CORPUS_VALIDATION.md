# Current-corpus validation — 2026-10-19, stage 64

Stages 61–64 improve measured typography, equations, brushes, table grids,
form captions, native inline baselines and mixed writing-direction sections.
This document reports observed checks; it does not assert universal HWP
compatibility or pixel identity with Hancom.

| Check | Latest result |
|---|---|
| Source inputs | 73 unique HWP5/HWPX files; source hashes unchanged |
| Conversion | 308 pages, zero failures |
| Automatic geometry | Zero text overlaps, border crossings, outside-page content or unintended foreground-image findings |
| Hancom references | 40 PDFs / 199 pages; all page counts agree |
| Rust checks | 280 tests and all-target Clippy with warnings denied pass |
| Final-stage regression | All 73 PDFs byte-identical to stage 63 |
| Mixed-direction probe | Four pages; text, paper and glyph origins equal isolated outputs (0pt maximum delta); zero QA findings |

## Measured corrections

- Superscript/subscript, vertical origins, explicit offsets and cumulative spaces:
  the supplied reference positions are checked with 0.15pt tolerances.
- The checkbox form remains 10 pages; all per-page Hangul multisets match. Shared
  merged-column boundaries replace inconsistent equal-width guesses. The first
  five caption origins agree within 0.25pt.
- The law exam remains 15 pages. Name and candidate-number tables start at
  199.995/200.0pt versus Hancom's 200.04pt; the type label starts at 188.39pt
  versus 188.4pt. The native 50% and ordinary 85% baseline cases are separately
  covered. The unrelated paragraph-relative floating label still uses its prior
  anchoring path; these fixes do not establish every sub-point/anchor position.
- The report remains 50 pages, with the research table on 46–47 and schedule
  gradient bars on 49. The form remains 2 pages and the math exam 9 pages.
- Scoped math styles, roman functions, Greek/operators, quoted terms, COLOR,
  source whitespace semantics and large-operator limits are implemented. Fallback
  font ink geometry is measured; no reference font subset is used for conversion.
- Gradients emit clipped vector bands. 3D columns read source rotation/depth/gaps.

## Text and image interpretation

The supplied shadow prints contain repeated decorative text; the engine paints
those effects as glyph paths while preserving one searchable copy of the source.
Reference outlines/metafiles/charts can instead produce additional searchable
engine labels. Raw text-count differences are recorded in stage-60/61 comparison
artifacts, not hidden by a universal normalization.

Stage 61 images cover the new typography/math/graphics work. Stage 62 changes
only the checkbox form and law page 1; the other 71 PDFs are identical to stage 61.
Stage 63 changes only law page 1 and notice pages 21–22, without text changes;
71 PDFs are identical to stage 62. Those pages were rendered and inspected.
Stage 64 leaves all corpus PDFs unchanged and adds the mixed-page image check.

## Reproduce and inspect

Manifest: output/roadmap/60-official-probe-calibration/all-inputs.json.
Two early synthetic-cache style probes explicitly use reflow; ordinary inputs
keep the default saved-layout mode. Rejected multi-section files are excluded.

Latest output: output/roadmap/64-mixed-section-directions/regression-final/.
Additional evidence directories: 61-fidelity-completion,
62-table-column-constraints and 63-native-inline-centering under output/roadmap/.
Each has tests, geometry reports, hash verification and comparison images.

Run cargo test --all-targets --no-fail-fast and
cargo clippy --all-targets -- -D warnings with the configured fonts.
Use the reconvert_manifest example for the complete conversion batch.

See TYPOGRAPHY_MATH_FIDELITY.md, TABLE_COLUMN_CONSTRAINTS.md,
NATIVE_INLINE_CENTERING.md and MIXED_SECTION_DIRECTIONS.md for stage details.
FORMAT_COVERAGE.md retains broader supported scope and limits. Generated PDFs,
user documents, font files and the diagnostic reference subset remain outside Git.
