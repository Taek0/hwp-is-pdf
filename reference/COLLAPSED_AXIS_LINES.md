# Collapsed-axis native lines — stage 90, 2026-10-20

The user identified an overlooked missing fraction rule in input 25's left
grouped fraction. This was an engine defect, separate from the viewer's OLE
omissions documented in stage 89.

The native line has a 100x100 construction box and cumulative transforms that
stretch x while collapsing y to zero. The positive-area group-box resolver
rejected that legitimate line transform, then used the small construction box.
The resulting PDF contained a roughly 1pt diagonal tick instead of the rule.

The resolver now accepts zero scale on an axis for native line records only.
Non-line degenerate boxes and unsupported off-axis/negative transforms retain
the existing guarded behavior. The resolved endpoints produce a horizontal
40.96pt rule; the supplied viewer print measures 40.92pt. The renderer's line
width is 0.28pt versus the print's 0.24pt. Its x origin still differs by about
3.7pt along with the surrounding group/text; exact font/spacing fidelity is
not claimed by this fix.

Tests cover the collapsed-axis transform, rejection for a rectangle, the real
fraction's resolved bounds and the presence of an actual horizontal PDF line
operation. Input 25's characters, glyph positions, sizes and pagination are
unchanged. The two OLE table expressions remain present.

Local evidence: `output/roadmap/90-degenerate-line/after.pdf`,
`fraction-rule-comparison.png`, `line-verification.json`, and the full-corpus
conversion report. The comparison is viewer / before / after. This case also
shows why text/overlap checks alone do not prove that all visible rules exist.
