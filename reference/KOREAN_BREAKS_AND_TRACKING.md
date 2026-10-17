# Korean wrapping and generated-label tracking — 2026-10-18

The supplied math paragraph now has the same five line starts as Hancom:
UTF-16 positions 0, 38, 70, 95 and 123. This fixes the previous difference where
the line endings included the whole `<보기>` or `[4점]` group on the wrong line.
All 26 question page/start-baseline checks still pass.

Three format rules interact here:

* Hancom HWPX `breakNonLatinWord=KEEP_WORD` permits character breaks;
  `BREAK_WORD` preserves a Korean word. HWP5 bit 7 is normalized to that lexical
  convention. Latin KEEP_WORD semantics are unchanged. The counterintuitive
  HWPX mapping is also documented by the renderer author's independent
  [controlled compatibility measurements](https://github.com/edwardkim/rhwp/issues/2185).
* Closing angle brackets cannot start a line, and opening brackets cannot end
  one. A Latin/Korean boundary can break in character mode, which allows
  `[4` / `점]` while keeping the closing bracket attached.
* Latin characters and spaces use half-em tracking, scaled by horizontal
  character width. Hangul uses a full-em cell. Generated list labels now use
  the same tracking and explicit glyph origins, with logical text retained.

The tracking correction was checked against supplied PDF glyph origins. For
the numbered prefix, the reference advance is about 7.524pt versus glyph width
7.854pt: the -0.33pt separation matches half-em -5% tracking under the printed
horizontal scale. Counting tracking as a full em previously changed line fits.
The official [Hancom spacing help](https://help.hancom.com/hoffice/multi/ko_kr/hwp/format/font/fonts%28spacing%29.htm)
confirms that spaces use the Latin spacing setting; numerical calibration here
comes from the independent supplied PDF, not from the help page alone.

## Font investigation

An HY-family alias was investigated but not retained. The supplied math PDF's
embedded F4 font identifies itself as Batang, despite the source's legacy
Hanyang family name. Automatically selecting installed HY신명조 would move away
from this reference. Existing explicit font-substitution diagnostics remain.
Reference fonts were inspected locally only; they are not bundled or committed.

## Validation

198 full-suite tests pass, plus the subsequently added five-line reference test
(199 total); all-target Clippy passes. All 53 unique source documents / 261 pages
convert with unchanged page/object counts and unchanged source hashes, and zero
findings in the four geometry categories. All 261 pages were rasterized; changed
math lines were visually compared to Hancom. The original 43-document corpus
retains its extracted-character multisets, with 32 PDFs byte-identical.

Artifacts are in `output/roadmap/39-font-and-korean-breaks/`, including the unified
input manifest, all-verification.json, corpus-text-verification.json, page-images,
test logs, wrap-reference-test.log and wrap-comparison.png. The warnings audit
identified remaining legacy private-use symbols and escaped hyperlink targets;
these are handled as the next stage rather than hidden by geometry-only QA.
