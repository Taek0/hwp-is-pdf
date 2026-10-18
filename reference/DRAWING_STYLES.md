# Drawing strokes and fills — 2026-10-19

HWP drawing line kinds 2–6 are retained as dash/dot/chain/long-dash styles,
instead of being normalized to solid. HWPX uses the same painter. Dash phase
continues through polygon vertices and flattened curve segments. Straight,
rectangular, rounded, elliptical, polygonal and curve outlines share this path.
Exact dash lengths remain nominal until independent Hancom printing is available.

Explicit NONE removes drawing outlines while retaining fills. A zero-width PDF
path now uses fill-only/no-paint rather than a device hairline. Ellipses retain
their solid face fill. Existing solid outlines retain their prior operation path.

Two unit tests cover cross-vertex dash phase, degenerate edges, and retaining fill
without an outline. Stage 58 all-target tests passed. The 53-document / 260-page
corpus reconverted with no conversion or four-class geometry findings. Only
features/shapes, web/textbox and new-references/06_wmf_document PDFs changed;
their eight changed page streams were rendered and reviewed. Page counts stayed
unchanged. This is not complete arrow, compound drawing stroke or advanced fill
support.

Artifacts: output/roadmap/58-drawing-styles, including comparison.json and
comparisons/. Reproducible source: examples/drawing_style_fixture.rs.
