# Typography, equations and vector brushes — 2026-10-19 (stage 61)

This stage continues from the user's four-priority Hancom prints. It corrects
flow coordinates and equation interpretation rather than inserting document-name
rules. The source HWP/HWPX files and fonts remain unchanged.

## Typography

Vertical flow keeps the source flow-margin roles when exchanging paper axes.
Glyph centering uses the content em, excluding leading. Script paragraphs retain
the nominal em for leading and spaces, while glyphs use 64% size. Width ratio
still applies to spaces. Explicit vertical offsets move across columns in the
printed direction. The supplied horizontal and upright-vertical script rows now
match measured Hancom origins to 0.15pt; tests include cumulative word spacing.
Horizontal script shifts are -.44/+ .12 nominal em; vertical shifts -.32/+.24.
The baseline measurement separates ink ascent from an explicit baseline shift.

## Equations

Styles are lexical and inherited by fractions, roots, matrices, accents and
limits. Variables default to italic, named functions to roman, and COLOR scopes
do not leak. Quoted terms preserve their text/spaces; normal and quarter spaces
remain distinct. More Greek, relation, set and arrow tokens are recognized.

Hancom's unbraced term extends to whitespace. In particular, the provided
`a^2+b^2` reference nests `b^2` inside the first superscript. This behavior is
preserved; `a^{2}+b^{2}` expresses separate superscripts. Large-operator limits
have their own attachment parsing. Tests protect both interpretations.

Fraction baselines, matrix spacing, searchable case braces and stretched
parentheses are improved. Sum/integral symbols are sized by actual glyph ink
rather than nominal font size; limits use 68% math size. Cambria Math is preferred
for integrals when installed, with a fallback. Body text scripts use 64%, which
is a different rule. The supplied reference font is only a 44-glyph subset and
was used for read-only measurement, never for conversion or distribution.

Math style, symbols, operators and placement are separate modules under src/math.
These changes do not establish every equation command or exact missing-font
outline compatibility.

## Graphics

Gradients now emit bounded, clipped vector bands instead of a 256x256 bitmap.
Linear/radial/conical/square fills preserve source steps and center parameters.
OOXML 3D columns read rotation, depth and gap values; default face colors and
lighting were checked against the supplied print. Projection remains a renderer
approximation, not an implementation of every OOXML camera/style combination.

## Evidence

Artifacts: `output/roadmap/61-fidelity-completion/`. Full Rust suite: 271 tests;
all-target Clippy passes. A final vertical-space adjustment is additionally covered
by the two reference tests and reconversion of the two affected inputs. All 73
inputs convert to 308 pages with no four-class geometry findings. All 40 Hancom
reference PDFs (199 pages) retain matching page counts. Detailed images are
`final-*.png`; counts/hashes are in `verification.json` and `tested-source.json`.

Official grammar consulted: Hancom equation revision 1.3 (local reference copy).
Brush structure: [Hancom HWP 5 revision 1.3](https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf), table 28;
[HWPML revision 1.2](https://cdn.hancom.com/link/docs/한글문서파일형식3.0_HWPML_revision1.2.pdf), tables 29–30.

Follow-on table constraint work is tracked separately; this stage does not claim
universal HWP compatibility or that every reference pixel is identical.
