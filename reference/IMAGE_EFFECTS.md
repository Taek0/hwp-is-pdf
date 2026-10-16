# Hancom raster-effect comparison — 2026-10-16

The user exported three diagnostic copies of the form and re-exported its
unchanged original. All four PDFs contain two pages and a 728 x 729 watermark.
The copies change only the selected picture's brightness/contrast bytes.
The original input remains unchanged. The re-exported original confirms the
previous reference's combined-effect pixels.

## Implementation and measured scope

The shared HWP/HWPX effect path now uses encoded RGB luma
`Y = .299 R + .587 G + .114 B` for grayscale. The old image-library grayscale
weights made the red parts too dark before effects were applied.

For brightness `b` and contrast `c` expressed as fractions, the calibrated
linear transfer is `gain = 1+c`, `black = 127.5*(1-gain)`,
`output = gain*Y + black + b*(255-black)`, clamped to 0..255 and truncated.
Brightness shifts levels without changing the contrast slope before clipping.
Previously the engine anchored contrast at white and interpolated each pixel
toward white for positive brightness; both behaviors differ from these exports.
Color images apply the same transfer independently to each channel. Alpha and
the original embedded bytes are preserved; repeated effect application is inert.

This transfer is an empirical model calibrated against the four settings below,
not a formula specified by the public file-format document. Positive contrast,
negative brightness, monochrome threshold fidelity and other viewer versions
still need independent Hancom comparisons. Do not claim universal tone identity.

| Brightness | Contrast | Mean absolute pixel difference (0..255) |
|---:|---:|---:|
| 0 | 0 | 0.657 |
| 70 | 0 | 0.573 |
| 0 | -50 | 0.535 |
| 70 | -50 | 0.538 |

The combined watermark previously differed by 6.805 on average. Measurements
compare decoded PDF image pixels at identical dimensions, avoiding page placement
and rasterizer effects. Hancom's JPEG compression can contribute to the residual;
the measurements do not establish that compression explains every difference.

## Reproduction

`cargo run --example image_effect_probes -- INPUT.hwp OUTPUT_DIR IMAGE_ID`
creates three new copies and refuses to overwrite existing copies. It preserves
compressed-stream CRC/size trailers. Export these copies using Hancom to establish
references. `inspect_image_effects INPUT OUTPUT_DIR` exports decoded source and
derived images without modifying the input and refuses to overwrite images.

Local evidence: output/hancom-effects-probes/ and
output/roadmap/30-image-effects/{watermark-comparison.png,watermark-error.json,
probe-comparison.json,corpus-text-verification.json,tests.log,clippy.log}.
User document contents and Hancom PDF exports are not added to Git.

176 tests and all-target Clippy pass. All 43 baseline documents / 205 pages
retain source hashes, page/object counts and extracted character multisets, with
zero geometry findings. Only the form's image pixels change; the other 42 PDFs
are byte-identical to stage 29. Both form pages and a side-by-side page comparison
were rendered, and the watermark image comparison was visually reviewed.

Watermark position and the source logo's print visibility are separate outstanding
issues; this change does not hide source content to match a reference.
