# Complex WMF playback, stage 66

Input 20 now converts to 15 pages, agreeing with the supplied Hancom page count.
The prior decoder rejected compound polygons, DC save/restore, clipping, pattern
brushes and destination-dependent bitmap operations. A bounded Rust/tiny-skia
software path now handles those operations without Windows APIs. Simpler WMFs
retain the existing vector output path.

The intermediate raster is at three times the WMF window extent, bounded to
4096 pixels per side and 16 million pixels. Intermediate raster dimensions are
not substituted for the original vector picture crop coordinate system. Raster
playback is reported in conversion warnings. Tests exercise compound path holes,
reversible XOR, invalid restore levels and the actual reference document.

## Source damage is separate from renderer support

Five BMP assets in input 20 contain only 2048 bytes while their uncompressed
headers require larger pixel payloads. Original bytes remain available; affected
picture references get an explicitly marked transparent unavailable-content
image, preserving layout. Every affected asset emits a warning. This does not
reconstruct missing pixels and must not be counted as complete content recovery.
Only provably truncated, bounded, uncompressed 24/32-bit BMP payloads use this
path; other invalid image types still fail normally.

## Remaining difference

The overview comparison exposes missing small icons in the WMF fallback. These
metafiles carry complete WMFC embedded EMF chunks, which require a separate path
and transform interpreter. Stage 66 is not a visual-fidelity completion claim.

Evidence: `output/roadmap/66-complex-wmf/`, including source/model inspection,
conversion report, `wmf-comparison.png`, tests and Clippy logs. Source diagrams
and Hancom PDFs are not committed. `tiny-skia` is BSD-3-Clause licensed and is
compiled as part of the standalone engine; no viewer is loaded.

Format references:
- [Microsoft WMF specification](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/)
- [Compound polygon records](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wmf/941630ee-85e6-4a0f-b02f-1c534b3fa9f8)
