# Native cumulative group frames — stage 74

Native grouped shapes carry cumulative translation/scale matrices in the root
group coordinate frame. Adding a nested parent's offset again displaced its
children. Cached current sizes can also predate the final scale matrix, so the
old text-box centering used an 85.04pt box instead of its 25.38pt realized height.

Positive axis-aligned matrices now determine geometry; a child's root-frame
origin is converted to its parent's local origin once. Font size stays unchanged.
Non-axis-aligned matrices retain the existing rotation path; this change does not
claim complete arbitrary native affine support.

For supplied print input 24, both title characters now agree with the independent
Hancom print within 0.03pt horizontally and 0.09pt vertically. Previously the
errors were approximately 90.7pt and 29.8pt. The new regression test checks both
characters and the unit test covers scale-matrix translation and truncated data.

All 194 held inputs reconvert to 561 pages. Only input 24's PDF changes from stage
73; all other 193 outputs are byte-identical. Library/checkpoint tests and
all-target Clippy pass. Evidence: `output/roadmap/74-native-group-frames`.
