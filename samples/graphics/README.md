# Graphics validation (2026-10-13)

Sources and SHA-256 are in sources.json. Inputs remain unchanged. The public
rhwp corpus supplies image crop and rotation fixtures; pre-existing hwplib curves
and pyhwp matrix fixtures are also used. No viewer/disassembler is invoked.

## Implemented

- HWP curve points and segment flags produce cubic PDF paths. Straight segments
  remain straight. Closed knots wrap their tangents. HWPX pt/seg geometry is read.
- HWP/HWPX shape and picture rotation/flip use the unrotated size, followed by a
  page-space transform. Text in a rotated shape uses the same transform. QA
  transforms glyph bounds, outlines and rules; a negative test verifies detection.
- Image crop reads imgClip/imgDim and uses a PDF clipping path. The original image
  bytes are retained; no raster resize/crop step is part of engine conversion.
- Signed current dimensions of old line records are normalized to size plus flip.
  This avoids huge coordinates and preserves all 15 language-exam page rasters.

## Evidence

- samples/features/curves.hwp: four cubic paths over two pages.
- samples/additional/matrix.hwp: two 30-degree rotated rectangles, one unrotated.
- pic-crop-01.hwp: two pictures of the same banner; second crops the bottom.
- rotated-picture.hwp: 34-degree picture inside a table; label and adjacent picture remain intact.
- shapes.hwpx: reflected/rotated polygon point arrays.
- issue5595_rotated_picture_topbottom.hwpx: 90-degree synthetic picture with an
  intentionally oversized declared footprint. Its blue rectangle reaches beyond
  the paper and PDF page clipping remains; it is not used as a fidelity-pass claim.
- examples/graphics_probe.rs: four-color diagnostic PDF, rasterized at 72 DPI by
  Poppler. Seven exact pixel-color probes verify crop and clockwise rotation.
- 37 Rust tests pass; Clippy -D warnings passes. Existing 33 inputs are re-converted
  separately under output/graphics-regression and compared against prior rasters.

## Limits preserved in reports

The official curve record does not specify a complete interpolation algorithm.
Cardinal tension 0.5 is an explicit renderer choice, informed by the conventional
GDI+ DrawCurve model, not proof of Hancom-identical pixels. See Microsoft's
https://learn.microsoft.com/en-us/windows/win32/gdiplus/-gdiplus-drawing-cardinal-splines-use

Old image records without imgDim use pixel dimensions at 96 DPI, with a report
warning for actual crops; tiny overshoots in inferred full-image edges are absorbed.
General shear and arbitrary cumulative affine transforms still need coverage.
Automatic QA does not establish every graphics-to-graphics collision or pixel fidelity.

Outputs/previews: output/graphics/. The controlled pixel-probe PDF is a synthetic
renderer diagnostic, not a converted user document. Real converted files are separate.
