# Note anchors and additional reference intake — 2026-10-17

The six user-printed note probes and the three new minimal reference PDFs
are available. The latter have 2, 1 and 4 pages respectively. No additional
manual exports are needed for the currently identified note work.

## Binary picture references

Drawing references address the one-based DocInfo BinData table. Each entry
then identifies its physical BINxxxx stream. These numbers need not coincide:
the comprehensive HWP uses table entries 1/2 with storage IDs 2/3. Resolving
directly by storage ID caused a missing-image error. The reader now resolves
through the table, including compression and PCX metadata. A mutation test
renumbers physical streams without changing logical references and requires
identical parsed content and image bytes.

## Body anchors

Paragraph numbering and endnote numbering are independent. Previously an
endnote anchor was omitted whenever a paragraph label was painted on the
same line. Anchors are now measured and painted in all paragraphs, including
white anchors. Both saved and rebuilt layout use the same 75% font-size
factor and 20% baseline raise, preserving horizontal font scaling.

Evidence: Hancom prints 10pt HWPX anchors at 7.56pt, 14pt at 10.56pt, and
15pt native HWP anchors at 11.28pt (printer rounding). The previous engine
used 60% size and 35% raise. Changing the endnote-body superscript flag does
not change the body anchor. Anchor font/position differences are not claimed
eliminated: native font metrics and paragraph justification still differ.

## Validation and limits

185 tests and all-target Clippy pass. Existing 43 documents / 205 pages retain
page, table, image, equation and chart counts; all four geometry QA categories
are empty. 38 PDFs are byte-identical. No extracted characters were removed;
26 math endnote labels and five native endnote labels were restored. Other
note-bearing PDFs change marker appearance. Input SHA256 hashes are unchanged.

Of the 13 additional comparisons, 12 convert: six note probes, three newly
printed examples, complex tables, mixed charts and EMF/OLE preview. The WMF
document fails image decoding and remains unsupported. Successful conversion
and matching page counts alone do not establish visual fidelity.

Math question 13 still has four saved lines versus five in Hancom. Its
continuation indent and line reconstruction are separate remaining work.

Artifacts: `output/roadmap/35-note-markers/` contains manifests, before/after
reference conversions, regression PDFs, validation logs, extracted-character
audit and `images/note-marker-comparison.png`, `images/styles-comparison.png`.
