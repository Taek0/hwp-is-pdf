# Remaining work agreed with the user — 2026-10-16

All six priorities remain in scope. A completed subtask does not mark its entire
category complete. Each stage requires focused tests, the existing-document
regression set, rendered images, a commit and a push.

| Priority | Scope | Current state / next concrete work |
|---|---|---|
| 1 | Math document paragraph and column flow | In progress. Default saved output still has the measured origin mismatch. Full reflow table rejection is being corrected; a different reflow page count must not replace the reference-matched default silently. |
| 2 | Font metrics, title and baseline positions | Inline-table margins, drawing-text CENTER/BOTTOM and final-character tracking corrected. Selected form/law title Y errors are 0.033/0.008pt; law title X error is 0.032pt. Generated italic labels remain. |
| 3 | General paragraph line wrapping | Pending: distinguish stale saved line breaks from authored breaks and verify rebuilt line widths/spacing against reference outputs. |
| 4 | Complex table row splitting and vertical alignment | Explicit merged header bands implemented and tested. Independent cell continuation exists; exact reference row-fragment distribution remains. |
| 5 | Watermarks, logos and borders | Pending: source-driven placement, visibility/print behavior, tone and compound-rule fidelity. |
| 6 | Additional formats | In progress. Emphasis marks implemented. Outline/shadow/emboss, vertical typography, broader equation/chart and WMF/EMF/OLE support remain. |

Keep excluded: password-encrypted documents, HWP 2.x/3.x and original-file editing.
Do not substitute per-file coordinates or page-number conditions for format rules.
