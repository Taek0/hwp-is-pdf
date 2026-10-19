# Held-file audit — stage 76 historical snapshot

The earlier engine-input inventory excluded sibling editor outputs. A fresh
no-ignore scan includes them: 454 physical HWP/HWPX files, 202 distinct SHA-256
contents. Git internals, build targets and Rust toolchains are excluded; source
documents and the editor's temporary exports are included. The manifest is
`output/roadmap/76-held-file-audit/complete-inputs.json`.

The first 194 inputs all convert. The intentionally stale `plain-cached` generator
fixture is now also checked in its designed explicit reflow mode, like the three
previous bad-cache probes. This yields 567 pages, not the former compressed
one-page saved-cache rendering. All eight additional editor exports convert to
27 pages. Conversion success alone is not a fidelity or viewer-validity result.

All earlier 561 pages were rasterized and reviewed in 29 contact sheets. Changed
pages are re-rendered separately. The new editor exports have 27 page rasters.
The scan found an actual missing gray image in input 20: its raster had been
replaced by an 8×8 constant-gray bitmap while the old crop coordinates remained.
When crop bounds exceed decoded pixels but fit a retained source image rectangle,
that rectangle now supplies the coordinate frame. Explicit dimensions and native
HWP dimensions remain authoritative. The gray area is restored; the removed
original logo artwork is not reconstructed.

Known evidence classifications, kept separate from raw QA:

- Input 1 fixed-zero-leading overlaps are also visible in Hancom's print.
- Input 21 has exactly 66 duplicate-position characters on page 2 in Hancom too;
  its underlined parentheses touch the underline in both renders.
- Input 24's two title glyphs intentionally overlap the banner picture's bounds;
  both placements now match Hancom within 0.1pt.
- Input 20's seven white header glyphs intentionally overlay the gray source image.
- Four blank-column probes were printed on landscape A4 whereas the source page
  is 400×300pt. Raw PDF bottom-origin Y differences must not be read as flow errors.
- The column-layout reference outlines the large `2단` glyphs: absent extracted
  Unicode text is not absent visible content. All four pages were compared visually.

Outstanding audit items are recorded rather than suppressed: the editor's old
`law-probe` temporary export has 1302 raw border crossings; the native endnote-only
last page has an approximately 10pt origin difference; input 12's viewer print is
incomplete and cannot validate the recovered OLE formula's exact appearance.
No 95% universal compatibility claim is made by this inventory.

Subsequent stages fixed all 1302 export border crossings, the native endnote origin
and separator length, the note-bearing report row bands and the scaled-picture
empty anchor. See [CURRENT_CORPUS_VALIDATION.md](CURRENT_CORPUS_VALIDATION.md) for
the final 202-input / 594-page result and explicit validation limitations.
