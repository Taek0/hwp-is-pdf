# Hancom print calibration — 2026-10-19

This increment uses the user's printed four-priority package, its repaired
single-section replacements, two earlier style probes, and the existing corpus.
It is a standalone Rust implementation; no viewer module is loaded or executed.

## Corrections

- Calibrate shape DASH/DOT/chain lengths and arrowheads. Apply hatch, linear,
  radial, conical and square brushes, source gradient steps and closed-shape
  clipping. Decode native HWP gradient/image brushes as well as HWPX brushes.
  The report's page-49 schedule bars are two-step gradients with stepCenter=10;
  they now appear without changing its 50-page text distribution.
- Paint native page image backgrounds once, behind content, and include
  LEFT_ONLY gutter width. The field/memo example retains its cover artwork and
  source horizontal placement. Suppress only unmodified CLICK_HERE prompts.
- Render character outlines, drop/continuous shadows and relief with one
  searchable text copy. BOLD is not a valid HWPX outline enum; THICK is accepted.
  Decorative layers use glyph paths rather than duplicated hidden text.
- Render uniform plain-text vertical sections with upright CJK/all-letter modes,
  rotated ASCII punctuation, vertical font origins and cross-column offsets.
  Horizontal width compression no longer overlaps upright CJK glyphs.
- Preserve ordinary HWPX paragraph baselines and inline object leading. Keep
  saved scripts/offset caches intact; no heuristic invalidates authored caches.
- Apply HWPX hanging indents to full-width zero-origin saved lines and rebuilt
  paragraphs, restoring the form body continuation margin. Existing explicit
  HWPX left margins are not added twice. The math page-6 box was image-compared
  before replacing an obsolete test assumption that all four lines share X.
- Continue supported HWPX CELL tables across pages while preserving the guarded
  fallback for notes, captions and oversized nested objects. Form controls have
  static check appearances, grouped with captions through table fragmentation.
- Preserve non-flow foreground overlays at their source paper/page coordinates
  during reflow, painting them after their anchor text. They consume no body row.
- Rebuild cacheless equation baselines from measured content, preserve available
  reserved height, support REL/BUILDREL/SCALE and column matrices, and stretch
  LEFT/RIGHT glyphs while retaining searchable delimiters. Keep rmAOB/rmcm/bold50.
- Read current OOXML chart data before stale legacy caches: numeric XY scatter
  and clustered 3D bars now use the supplied series. A bounded, structurally
  validated WMF mask/color presentation fallback is available for supported OLE.

Implementation is separated into brush, ellipse, arrow, shadow, vertical, form,
gradient, equation, chart and native fill/OLE modules under src. No document-name
or reference-page coordinate rules were added. Inputs and fonts remain unmodified.

## Limits visible in the supplied comparisons

Passing page counts and geometry checks does not establish pixel identity.
Vertical absolute starts and some script/column spacing still differ. Some form line positions and checkbox spacing remain different even when
per-page content matches. Equation typeface/spacing/size is approximate: installed HancomEQN does
not match the reference's HyhwpEQ; the reference subset is diagnostic output only
and is not an engine font. Full equation grammar and nested style scopes remain
partial. Chart 3D projection, axis styling and unsupported chart families are not
complete. Gradients use a bounded 256x256 RGB raster and can differ from the
printer's vector band edges; nonuniform multi-color stop positions and compound
brush combinations are not fully supported. Exact THICK outline calibration,
non-left gutter modes, mixed-orientation sections and vertical inline objects
still need work. Static checkboxes do not imply interactive PDF form support.

The first drawing-styles and text-outlines probes have fabricated saved line
caches. Their comparison explicitly uses --reflow; ordinary documents keep the
default cache-preserving mode. The rejected multi-section files are not inputs.

See CURRENT_CORPUS_VALIDATION.md for final counts and local artifact paths.
