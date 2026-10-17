# Legacy symbol glyphs and native hyperlink escapes — 2026-10-18

A complete warnings audit found nine missing private-use glyph codes in the
previously rendered document set. The source runs assign ordinary text families
to Microsoft legacy symbol codes (F020–F0FF); those families lack the glyphs.
The renderer now uses the configured font directory's Wingdings face only when
the requested face lacks the glyph and Wingdings supplies it. Existing private
glyphs in requested fonts are preserved. The original Unicode code point stays
in the PDF rather than being rewritten to an approximate Unicode pictogram.

The government notice's pointing hand, circled numbers and check mark are
visible again. Its extracted PDF histogram now exactly matches all 28 source
symbol occurrences. Previously unrelated missing characters shared glyph zero
and acquired incorrect ToUnicode mappings. Therefore its character-multiset
change is a repair, not removed source content. The nested-table checkbox is
also restored, with no new page or geometry change.

Native `%hlk` commands use escaped delimiters. The first unescaped semicolon
ends the target; escaped colon, semicolon and backslash are decoded. Raw HWPX
URLs remain unchanged. The supplied URL's trailing closing parenthesis is
preserved because it is present in the source command; this stage repairs field
interpretation and PDF annotation emission, not the author's URL spelling or
the destination website. URI schemes remain subject to the existing whitelist.

## Validation

201 tests and all-target Clippy pass. All 53 unique documents / 261 pages convert
with zero geometry findings, zero missing-glyph warnings and zero unresolved
link/reference warnings. Source hashes and page/object counts remain unchanged.
The notice's repaired symbol codes are checked independently against the HWP
model. Existing navigation tests pass, including escaped delimiters and literal
URL punctuation. Visual before/after symbol crops were inspected.

Artifacts: `output/roadmap/40-symbol-links/` contains the full conversion set,
all-verification.json, corpus-text-verification.json, symbol-text-verification.json,
tests.log, clippy-final.log and images/symbol-comparison.png.

Image comparison identified one further placement difference in the WMF
reference: a native picture that fits the paper is unnecessarily pulled into
the print margins. That bounded placement correction follows this stage.
