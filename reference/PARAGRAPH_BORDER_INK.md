# Paragraph rules enclose realized ink — stage 78

The additional editor temporary export `law-probe.hwpx` had 1302 detected rule /
glyph crossings. Its saved line boxes were shallower than the realized glyph
ink, and some empty paragraph bottoms extended into the following line.

For zero/positive padding, paragraph boundary rules now enclose the actual
realized text ink. A saved last-line bottom can also be capped before the next
paragraph's text box when there is room without clipping its own ink. Inline
control markers and floating labels retain this text measurement; inline objects
and tabs keep their existing geometry path. Explicit negative padding is retained.

All 1302 crossings are eliminated. Every character, text position and font size
in the 16-page export is identical before and after; only the paragraph bands
change. This does not repair or validate the sibling editor's old export against
the original native document, whose source properties differ.

Combined stage 77–78 corpus: 202 distinct inputs / 594 pages; no conversion
failures or unknown positioned drawable kinds. Only the native endnote document,
the native notes feature fixture and this temporary export change from stage 76.
The remaining raw QA findings are retained and independently classified in the
held-file audit, rather than removed from the detector.

Artifacts: `output/roadmap/78-paragraph-border-ink`.
