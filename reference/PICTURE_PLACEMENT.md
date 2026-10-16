# Saved picture placement and Hancom visibility — 2026-10-16

## Native floating picture anchors

The saved picture position refers to its outside margin box. The painter used
the image rectangle instead, dropping the left/top margins. It now includes the
outside box for native saved, non-inline, uncaptioned pictures. Center alignment
uses half the difference between opposing margins; right/bottom alignment uses
the trailing margin. Inline, captioned and rebuilt general-float paths retain
their existing separate spacing rules.

The source form's watermark has 852 HWPUNIT outside margins. Including those
8.52pt margins fixes its displacement without a filename or page-number rule.

| Watermark top-left (pt) | Hancom PDF | Before | After |
|---|---:|---:|---:|
| X | 111.720005 | 103.28 | 111.80 |
| Y | 211.200023 | 202.68 | 211.20 |

Remaining X difference is 0.08pt; height differs by 0.04pt. This is not a claim
that the whole page matches. Existing text/border differences remain outside
this stage's scope.

## Why the logo differs

The source logo is `/BinData/BIN0001.PCX`; the watermark is JPEG. The original
and four Hancom probe exports contain the watermark but no logo image. The
engine decodes the PCX successfully and preserves it by default.

[Hancom's official support notice](https://www.hancom.com/support/faqCenter/faq/detail/2709)
states that PCX and several legacy image formats may not be displayed at the
high document-security setting, while the low setting permits them. The
observed omission is consistent with that policy; the user's actual application
setting was not inspected. No per-object hidden flag was established.

The new explicit `--hancom-hide-pcx` option reproduces the PCX visibility
difference. The HWP adapter preserves original-format metadata even after PCX
decoding and effects; rendering omits those picture operations and records each
omission in the report. It preserves the source, layout space and normal JPEG
images. The option currently covers decoded native HWP PCX pictures only, not
all formats mentioned by Hancom. It is a display compatibility option, not a
security feature: parsing and decoding still occur. No application security
setting is changed.

```sh
cargo run -- input.hwp output.pdf /path/to/fonts --hancom-hide-pcx
```

Library callers can use `convert_configured` with
`ConversionOptions { hancom_hide_pcx: true, ..Default::default() }`.
Existing conversion APIs retain source-preserving defaults. Direct layout
callers can set `Engine::hancom_hide_pcx` before rendering.

## Verification

179 tests and all-target Clippy pass.

Tests cover asymmetric margin boxes at left/top, center and right/bottom,
PCX opt-in behavior, ordinary PNG preservation, unchanged text coordinates,
source immutability and the supplied form's parsed source format and anchor.
The default 43-document / 205-page corpus retains page/object counts, character
multisets and source hashes with zero geometry findings. Only the form changes;
the other 42 PDFs are byte-identical to stage 30. Compatibility output keeps
two pages and three table draws, while intentionally reducing pictures from
two to one and recording the omission.

Evidence: output/roadmap/31-picture-placement/ contains both form modes,
page-comparison.png, images/, placement-comparison.json, corpus-text-verification.json,
verification-summary.json, tests.log and clippy.log. User documents and generated
outputs remain outside Git.
