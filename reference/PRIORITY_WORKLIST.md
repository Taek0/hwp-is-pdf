# Remaining work agreed with the user — 2026-10-16

All six priorities remain in scope. A completed subtask does not mark its entire
category complete. Each stage requires focused tests, the existing-document
regression set, rendered images, a commit and a push.

| Priority | Scope | Current state / next concrete work |
|---|---|---|
| 1 | Math document paragraph and column flow | All 26 math question starts remain aligned. Question 13's five internal line starts now match Hancom after Korean-break, punctuation and half-em Latin-tracking fixes (KOREAN_BREAKS_AND_TRACKING.md). Native same-page and nested column fixes remain verified. |
| 2 | Font metrics, title and baseline positions | Inline-table margins, drawing-text CENTER/BOTTOM and final-character tracking corrected. Selected form/law title Y errors are 0.033/0.008pt; law title X error is 0.032pt. Generated italic labels remain. |
| 3 | General paragraph line wrapping | Korean HWPX break semantics, mixed Latin/Korean boundaries and angle-bracket prohibitions are corrected. Bounded numbered-cache repair is verified against actual line starts. Broader stale-cache recovery and unmeasured justification cases remain. |
| 4 | Complex table row splitting and vertical alignment | Explicit merged header bands implemented and tested. Independent cell continuation exists; exact reference row-fragment distribution remains. |
| 5 | Watermarks, logos and borders | Grayscale/tone calibrated against four exports; native floating picture margins corrected. Optional PCX visibility compatibility reproduces the form's omitted logo while default conversion preserves it (PICTURE_PLACEMENT.md). Remaining: broader visibility/effects, wider tone parameter coverage and compound-rule fidelity. |
| 6 | Additional formats | BinData mapping, native dual-axis combination chart rendering and supplied WMF record playback are implemented and tested (EMBEDDED_GRAPHICS.md). All 13 new references convert without geometry findings. This is not full WMF/EMF/OLE coverage; outline/shadow/emboss, vertical typography and broader equations remain. |

Keep excluded: password-encrypted documents, HWP 2.x/3.x and original-file editing.
Do not substitute per-file coordinates or page-number conditions for format rules.
