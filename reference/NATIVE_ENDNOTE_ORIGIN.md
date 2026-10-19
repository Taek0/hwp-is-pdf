# Native endnote origin and rule length — stage 77

Modern 28-byte note-shape records store a signed 32-bit separator length. Reading
only its low 16 bits shortened the supplied endnote rule from the full column to
122.8pt. The adapter now retains all four bytes; legacy 26-byte records retain
their signed 16-bit form. Tests cover a 100000-unit rule and both -1 encodings.

On a native endnote-only page, the final empty explicit-break paragraph keeps its
15pt height but does not contribute its 9pt trailing interline leading. The
configured above/below gaps are also no longer inflated by an invented 1pt rule
allowance. This reduces the supplied 9.93pt text displacement to 0.07pt.

The independent Hancom PDF places the rule at 94.44pt and first note baseline at
107.76pt. The engine now uses 94.37pt and 107.69pt. All six notes remain on page 5;
the rule spans 481.90pt versus the print's 482.04pt. The source document is not
changed. A dedicated reference test checks the origin, line extent and note count.

Artifacts: `output/roadmap/77-note-anchor`. The combined stage 78 regression
converts all 202 held inputs to 594 pages with no conversion failures.
