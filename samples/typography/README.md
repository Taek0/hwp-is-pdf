# Typography fixtures

Public source: https://github.com/mete0r/pyhwp/tree/master/tests/hwp5_tests/fixtures
Original bytes and source hashes are recorded in sources.json. No Python converter
was installed or run. Conversion and font interpretation are in the Rust engine.

| Input | Output pages | Missing-glyph warnings | Geometry findings |
|---|---:|---:|---:|
| lists-bullet.hwp | 3 | 0 | 0 |
| facename2.hwp | 1 | 0 | 0 |

PDFs and rendered previews: output/typography/. All four pages visually inspected.
The fixtures exercise filled/open circles and squares, diamonds, triangles,
check marks, stars, and mixed font selections. Exact bullet width/alignment,
font-family fidelity and all script-specific selection remain separate work.

The prior samples/features/numbering.hwp also now displays both legacy F06C
bullet glyphs using Wingdings' Windows Symbol cmap. A missing font ID is reported
as a fallback choice rather than claimed to be an original declared font.
