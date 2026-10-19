# Section-specific writing directions — stage 64

Horizontal, vertical and upright-vertical sections now share one rendering
context. Each completed page retains its owning section. Only vertical sections
are prepared in exchanged flow axes, and only their pages/geometry are transformed
back to physical paper coordinates. A direction change invalidates direction-
sensitive glyph metrics; fonts, field state, notes and numbering remain shared.
Horizontal saved layout remains cached unless reflow is requested.

The native HWP adapter reads the documented section text-direction bits (0:
horizontal, 1: vertical); unsupported reserved values fail explicitly instead of
being silently treated as horizontal. Source: [Hancom HWP 5 revision 1.3](https://cdn.hancom.com/link/docs/한글문서파일형식_5.0_revision1.3.pdf), table 130.

Tests compare HORIZONTAL → VERTICAL → VERTICALALL → HORIZONTAL against isolated
sections with identical styles. A separate test checks continuous page numbers
and an explicit restart. The original document nodes remain unchanged.

The provided horizontal and upright-vertical probes were composed in memory into
four pages. Every page has identical extracted text, paper dimensions and glyph
origins to its isolated engine output (maximum delta 0pt). This is a composition
invariant, not a separately printed Hancom mixed-document oracle. The example
rejects conflicting styles/assets and never writes the source documents.

Validation: 280 tests and all-target Clippy pass. All 73 corpus PDFs / 308 pages
are byte-identical to stage 63, with zero conversion or four-class QA findings.
All input/source hashes and all 40 reference page counts match. The additional
four-page mixed probe also has zero geometry findings.

Artifacts: output/roadmap/64-mixed-section-directions/, including mixed.pdf,
mixed-contact.png, composition-check.json and verification.json.
Vertical inline object orientation is still outside this plain-text extension;
this stage does not silently rotate unsupported complex objects.
