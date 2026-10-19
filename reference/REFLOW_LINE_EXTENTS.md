# Reflow line extents, tables and endnotes — stages 84–88

The numbering-edited math probe exposed several independent reflow defects.
Changes use format properties and measured geometry, never input names or IDs.

- Apply the independently verified two-leading-empty-line rule after automatic
  column changes in both saved and rebuilt flow. Explicit breaks and disabled
  hiding retain their blanks. All four supplied blank-column cases pass in both modes.
- Separate a line's visible extent from its advance. A final interline gap need
  not fit on the page, and a table cell's last gap must not inflate its minimum
  content height. Generated line caches preserve that distinction as size/spacing.
- Preserve a section-leading non-inline table's absolute width when it spans
  columns within the printable width. Both columns clear the table's bottom.
- HWPX hanging indents retain the declared left margin on the first line.
  The supplied choice labels now agree with Hancom's x positions within 0.1pt.
- Include rebuilt HWPX block tables' outside margins in placement and flow.
- Inter-note spacing replaces the last interline gap. The 26 single-line answers
  now advance by 10pt, matching the supplied print rather than accumulating 6pt
  extra per note. All answers remain on page 9.

## Verification

The primary 202-input corpus converts to 592 pages. Two generated examples lose
one artificially inflated page each; all 63 independent reference page counts
still match (247 pages). The RTL table's 30 declared 22pt rows fit its original
two columns. A separate shorter-paper test retains genuine multi-page coverage.

47 PDFs change; 155 are byte-identical to stage 82. Non-whitespace PDF character
multisets are unchanged for every changed input. All changed pages were rendered
(175 pages, with 14 endnote pages rerendered after the final note-only change).
The remaining 417 page rasters remain valid because their PDFs are byte-identical.
All 454 source-copy hashes are unchanged. Raw QA findings retain the same four
source-authored classifications, with no new geometry findings.

331 tests pass in the full stage-87 run. After the final endnote-only change,
the affected three primary inputs, the alternate math conversion and six relevant
integration targets were rerun successfully. All-target Clippy passes.

The alternate full-reflow math result is now nine pages, with an empty body on
page 8 and all 26 answers on page 9. **This is not pixel equivalence:** some body
paragraph origins and font/rule appearances still differ from Hancom. The default
saved-layout and explicit reflow paths remain separately recorded.

Local evidence:

- `output/roadmap/87-reflow-table-margins/tests.log`
- `output/roadmap/87-reflow-table-margins/changed-inputs.json`
- `output/roadmap/88-endnote-leading/audit-summary.json`
- `output/roadmap/88-endnote-leading/reference-checks.json`
- `output/roadmap/88-endnote-leading/endnote-comparison.png`
- `output/roadmap/88-endnote-leading/index.html`
