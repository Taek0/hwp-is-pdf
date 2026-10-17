# Remaining work agreed with the user — 2026-10-16

All six priorities remain in scope. A completed subtask does not mark its entire
category complete. Each stage requires focused tests, the existing-document
regression set, rendered images, a commit and a push.

| Priority | Scope | Current state / next concrete work |
|---|---|---|
| 1 | Math document paragraph and column flow | All 26 math question starts match reference pages and text tops within 0.058pt. Native same-page column bands and equal-width nested columns are now corrected against the supplied four-page reference (NATIVE_COLUMN_BANDS.md). Within-paragraph word boundaries still differ. |
| 2 | Font metrics, title and baseline positions | Inline-table margins, drawing-text CENTER/BOTTOM and final-character tracking corrected. Selected form/law title Y errors are 0.033/0.008pt; law title X error is 0.032pt. Generated italic labels remain. |
| 3 | General paragraph line wrapping | Numbered HWPX caches with omitted autoIndent can gain required lines within logical blank-column recovery; explicit breaks and complex objects are excluded. Broader stale-cache recovery, Korean word boundaries and exact justification remain. |
| 4 | Complex table row splitting and vertical alignment | Explicit merged header bands implemented and tested. Independent cell continuation exists; exact reference row-fragment distribution remains. |
| 5 | Watermarks, logos and borders | Grayscale/tone calibrated against four exports; native floating picture margins corrected. Optional PCX visibility compatibility reproduces the form's omitted logo while default conversion preserves it (PICTURE_PLACEMENT.md). Remaining: broader visibility/effects, wider tone parameter coverage and compound-rule fidelity. |
| 6 | Additional formats | BinData index/storage-ID mapping fixed. New WMF reference fails decoding; modern mixed chart uses an unfaithful dense data-table fallback. EMF/OLE preview fixture converts but does not prove general vector support. Outline/shadow/emboss, vertical typography and broader equations remain. |

Keep excluded: password-encrypted documents, HWP 2.x/3.x and original-file editing.
Do not substitute per-file coordinates or page-number conditions for format rules.
