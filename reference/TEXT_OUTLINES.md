# Searchable character outlines — 2026-10-19

HWP5 character-shape bits 8–10 (official revision 1.3 table 35) and HWPX
outline/type are normalized to SOLID, DOT, BOLD, DASH, DASH_DOT and
DASH_DOT_DOT. NONE remains ordinary text. Ordinary text, shaped runs and
generated labels use the same scoped PDF stroke-text operation.

The PDF retains its glyphs, Unicode maps and logical text; no duplicate shadow
text, image replacement or loss of searchability is introduced. A transform
scope saves/restores outline state so following ordinary runs remain filled.
Nominal width is 2.5% of text size (6.5% for BOLD); dash metrics are nominal.
Independent Hancom calibration is pending, and reports explicitly say so.
This does not implement shadow, emboss/deboss or vertical writing.

Stage 59 checks: 239 all-target Rust tests and all-target Clippy passed.
All 60 existing inputs / 273 pages reconverted without errors or findings in
the four geometry classes. All source hashes, page counts and per-page extracted
text match the prior corpus. The three drawing-change PDFs remain the only
changed PDFs relative to stage 57; outline support alone changes none of the
existing corpus PDFs. Thus the corpus is regression evidence, not independent
proof of outline appearance.

The new fixture has seven labelled rows; all seven source phrases and seven
following ordinary labels are extracted exactly once. Its raster was reviewed.
A dedicated integration test checks outlined B between ordinary A/C, preserved
positions and balanced output scopes. A unit test checks inactive NONE, thick
outline weight and dash selection.

Artifacts: output/roadmap/59-text-outlines/{verification.json,tests-final.log,
text-outlines.pdf,text-outlines.png,regression,prior-probes}.
Generator: examples/outline_fixture.rs.
User-print copies: D:/ChatGPT/hwp/Hancom-Format-Probes-20261019/01_print.

Remaining priority work is still substantial: arrows/compound drawing rules,
advanced fills, shadows/emboss/deboss, vertical writing, pgct/compatibility,
and broader chart/metafile/equation support. The two probes cover this increment,
not all of those future families. Generated HWPX opening in Hancom is unverified.
