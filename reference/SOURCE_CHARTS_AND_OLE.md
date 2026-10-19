# Source charts and recovered OLE content — stages 72–73

The frozen inventory has 346 copies representing 194 byte-distinct HWP5/HWPX
inputs. All 194 convert to 561 pages with no conversion failures. Eleven PDFs
change from stage 71; the other 183 are byte-identical. This is conversion and
regression evidence, not a claim that 194 files have independent Hancom oracles.

## Changes

- Supported default 2D OOXML bar, stacked/percent bar, line/marker and OHLC charts
  use source series, categories and axes instead of stale OLE preview images.
  Printed inputs 13–16 and 19 now have vector marks and searchable labels, without
  the obsolete preview title. Unsupported layouts retain their cached path.
- HWPX OLE references now enter the same bounded, non-executing normalizer as HWP.
  The static EMF `/Contents` envelope restores the customs-service logo. HwpEq5x
  version 5 UTF-16 prefix expressions restore the formula in print input 12.
  Unknown commands and truncated arguments fail explicitly.
- HWPX containers preserve child affine matrices, translating source units once
  and avoiding repeated scaling. The supplied three child images are sanitized
  white placeholders in the source; no replacement artwork is invented.
- Explicit HWPX substitute font declarations are honored when the original face
  is absent. Classic Haansoft Batang ASCII advances are preserved using measured
  1024-em metrics and fitted substitute outlines. HCR Batang is not the same font.
  Exact original outlines and non-ASCII metrics remain dependent on available fonts.

## Evidence and boundaries

`output/roadmap/73-drawable-inventory/after-inventory.json` scans positioned nodes
and group children in all 194 sources: zero unrecognized drawable kinds remain.
This scan does not prove every attribute is implemented. All-source conversion,
QA findings and before/after artifacts are retained in the same directory.
Stage 72 contains cropped Hancom/engine chart comparisons.

The supplied viewer PDF for input 12 omits its formula and following explanation
paragraphs. The engine retains these original contents. A complete independent
print was requested; this comparison must remain explicitly incomplete. The
source-envelope interpretation is also corroborated by the exact-source report
at https://github.com/edwardkim/rhwp/issues/5725.

Raw geometry QA retains eight text overlaps in the source's fixed-zero-leading
sample, seven image overlaps around a header background, and 66 duplicated WMF
glyph overlaps plus eight WMF rule crossings. They have not been suppressed to
manufacture a clean score. Source/ref classification and remaining native group
and table-anchor placement differences are subsequent audit work.

Five truncated BMP payloads and viewer-rejected input 9 retain their existing
damage/rejection status. Successful engine conversion does not validate them as
complete, viewer-readable source documents. No viewer runtime or font binary is
required or included in the engine.
