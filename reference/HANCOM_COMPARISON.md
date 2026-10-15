# Hancom reference-output comparison — 2026-10-15

Engine revision: 0325153dd44a43cccf63055e2efcc82de6dea68e.
Roadmap stages 1–10 are implemented within the limits in FORMAT_COVERAGE.md.
Stage 10 passes 126 tests, Clippy, and the 43-document / 210-page regression.

## Attempt and observed limitation

Opened the existing installed HwpViewer.exe from Office 2024 Viewer using the
computer-use skill. Opened the unchanged user document D:/ChatGPT/hwp/복학원서.hwp.
The viewer reported that current HIGH document security prevents loading some
picture formats. No security settings were changed. The informational alert was
dismissed, and the Print dialog showed Microsoft Print to PDF selected.

The viewer status displayed 1/2 pages, while this engine's saved-layout PDF has
1 page. This is a preliminary pagination discrepancy, not a completed PDF
comparison: the viewer's picture loading was restricted and no reference PDF
was produced. Font/layout conditions and actual second-page contents remain
unverified. Do not interpret baseline regression equality as Hancom fidelity.

Both the initial Print action and a retry after refreshing window selection failed
with the computer-use runtime error `failed to activate captured window`.
The skill recovery rule requires stopping after failed recovery. No further UI
input was sent. The viewer may remain open at the Print dialog.

## Remaining evidence needed

Hancom-generated PDFs from the unchanged originals, ideally the three user files
(복학원서.hwp, 21_언어_기출_편집가능본.hwp,
2단_수학시험지_변환검증_7826.hwpx), plus the complex footnote corpus example,
are needed for page-by-page comparison. Put reference exports under
output/roadmap/11-hancom/hancom/. No reference images or fidelity percentages
were fabricated. Once exports are available, compare page counts, text order,
object geometry and raster overlays and record font/security differences.

Local evidence: output/roadmap/11-hancom/viewer-print-dialog.png.
