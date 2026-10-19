# HWPX property alternates — stage 83, 2026-10-20

Header properties previously retained both `hp:switch` branches. Recursive
margin lookup happened to find the modern branch, but direct tab-stop enumeration
found no `tabItem` children at all. Forced reflow consequently used default stops.

Resolve the header as well as sections/masters, selecting the supported
`2016/HwpUnitChar` branch exactly once. Unknown extensions retain their default.
The regression test checks direct tab access, non-doubled margins and removal of
inactive alternate values.

Validation: 327 tests pass. All 202 primary-mode PDFs (594 pages) are byte-identical
to stage 82. The independently printed numbering-edited math probe has improved
choice columns under forced reflow, but this change alone still produces 10 pages.
The remaining pagination difference is not hidden by the successful primary audit.

Local evidence: `output/roadmap/83-hwpx-tab-branches/`, including
`tabs-comparison.png`, `pdf-differences.json` and the complete conversion report.
