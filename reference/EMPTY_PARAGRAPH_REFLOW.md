# Empty paragraph metrics — 2026-10-16

Reflow previously gave every empty paragraph a 12pt default size, even when its
run explicitly selected a different character style. A zero-width layout unit
now preserves that style's size and vertical metrics without inserting text.
At 160% spacing, the controlled 8pt and 20pt empty paragraphs advance 12.8pt and
32pt respectively. A regression checks these advances, source immutability,
unchanged extracted text and geometry.

160 tests and Clippy pass. All 43 existing documents / 205 pages are byte-identical
to stage 24, including the four Hancom targets. Source and output geometry remain
unchanged for those fixtures. The additional sample is
samples/text-style/empty-paragraphs.hwpx; its render and logs are in
output/roadmap/25-saved-column-flow/.

The math document's saved column-origin discrepancy is **not fixed** by this
reflow correction. A trial that carried overflowing blank paragraphs accumulated
position error across later columns and was reverted. Forcing full reflow also
encounters an unsupported table structure in this document. Neither trial is
shipped as a replacement PDF. Further work must distinguish saved column caches,
blank-paragraph flow and mixed-table reflow rather than add document offsets.
