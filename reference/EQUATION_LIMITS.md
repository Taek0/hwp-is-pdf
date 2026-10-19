# Limit operators and explicit numerators, stage 70

The supplied `equation-lim.hwp` exposed three semantic errors: `rarrow` was
painted literally, a limit condition was attached at the right edge instead of
under the operator, and a final explicit numerator group absorbed the preceding
limit operator. The Hancom print shows the limit outside the fraction.

The parser now preserves a final explicit numerator group independently of its
prefix, while retaining groups owned by scripts/unary constructions. Named `lim`
and `Lim` operators center their upper/lower conditions. The measured lower-case
limit label is 120% of baseUnit and the condition remains 68%; optical baseline
and limit gaps are calibrated against the supplied print. Arrow-name aliases
retain case distinctions for single/double arrows. No filename or object-ID rule
is used in production layout.

The source's original equation face is not installed, so the comparison still
shows fallback-font shape/spacing differences. This stage fixes structure and
collisions; it does not claim full equation grammar or original font identity.

Validation: 299 tests and all-target Clippy pass. All 13 equation-containing
inputs in the frozen inventory were reconverted. Only this limit document's PDF
changed; the other 12 are byte-identical to stage 68. All four geometry finding
lists are empty in the corrected limit document. Evidence and the before/after
comparison are under `output/roadmap/70-equation-limits/`.

Grammar reference: the locally retained official Hancom equation-format revision
1.3 document (`reference/한글문서파일형식_수식_revision1.3.pdf`), including grouping,
fractions, limit operators and explicit spaces.
