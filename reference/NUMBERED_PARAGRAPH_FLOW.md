# Saved numbered paragraph flow — 2026-10-17

List text reservations now use the label's horizontal scale in saved layout.
When paraHead.autoIndent is enabled, continuation lines reserve the label
width and text offset too. Existing cached indents larger than that reservation
are preserved; the label is still painted only once.

For HWPX sections already eligible for logical blank-column recovery, a
numbered paragraph whose saved continuation lines omit this reservation can
need additional lines. A separate saved_numbering module rebuilds those line
breaks using measured text/note/equation widths. It only replaces the cache
when more lines are required, retains saved vertical line metrics and positive
gaps, and lets the logical column cursor propagate the added height. It rejects
native HWP caches, explicit line breaks, cache resets, varying widths, tabs,
floating objects, tables and footnotes. Source nodes are never modified.

## Measured outcome

Math question 13 expands from four lines to five. All 26 question starts now
remain on the reference page and their extracted text tops differ by at most
0.058pt (previously seven starts differed by about 16pt). The nine-page document
retains 249 equations, 22 pictures and 17 table draws. This is a paragraph-start
measurement, not a claim of pixel identity: within-line word boundaries,
horizontal positions and justification still differ.

The complete suite passes 187 tests, plus the subsequently added independent
26-question baseline regression passes (188 total). All-target Clippy passes.
The existing 43-document / 205-page corpus retains all page/object counts and
extracted-character multisets, with zero findings in the four geometry classes.
41 PDFs are byte-identical to stage 35. The other changes are the math document
and scaled bullet reservations in report pages 32–42. All 50 report pages were
raster-compared; research-table pages 46–47 remain pixel-identical. Original
input hashes remain unchanged.

Artifacts: output/roadmap/36-numbered-wrap/ contains regression and reference
PDFs, test/Clippy logs, question-positions.json, corpus-text-verification.json,
report-raster-diff.json and wrap-comparison.png. The original supplied reference
and converted math page are used in that comparison, both with white markers.

## New-reference findings still open

The three new user exports are all readable and match page counts (2/1/4).
The comprehensive sample's previously fatal BinData reference issue is fixed.
The character-style sample visibly includes italic/bold and colored/dotted
underlines. These checks do not certify all font effects.

The column-layout sample exposes same-page column-band transitions: three
short lines belonging at the bottom of page 2 overlap at the top of page 3.
Its 12 text-overlap findings are new-reference failures, not counted among
the clean pre-existing 43-document regression corpus. Nested column layout
also differs. Diagnostic images are under output/roadmap/37-column-bands/;
no column-band fix is claimed by this stage.

The additional mixed-chart reference uses an unsupported native chart style
and currently falls back to a dense data table (208 text-overlap and 17 border
crossing findings). This is not faithful chart conversion. The WMF document
still fails decoding. The EMF/OLE fixture converts through its available preview;
that does not establish general EMF vector support. These are concrete next
compatibility targets, with reference PDFs already available.
