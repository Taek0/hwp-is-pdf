# Checkpoint audit, stage 65 (in progress)

The target percentage is an estimate, not a counted proportion of the HWP specification.
No 95% completion claim is made by this stage.

## Frozen inputs and evidence

`output/roadmap/65-checkpoint-inputs/workspace-input-inventory.json` inventories
346 copies / 194 distinct HWP/HWPX byte streams in the conversion workspace.
Dependency trees, the separate editor project and the unrelated `rea` repository
are excluded. Duplicate copies are associated by SHA-256. Previously rejected
synthetic probes remain visible in this audit; parsing them does not establish
Hancom acceptance. The viewer-rejected input 09 is not a successful reference case.

The existing 40 Hancom PDFs and new 23 PDFs provide 63 reference pairs. New PDFs
contain 48 pages; numeric filenames are mapped in the package manifest. Every
new reference page was parsed and rendered during receipt checking.

## Initial results and corrections

- Every one of the 194 distinct inputs was attempted, with errors retained.
- Initial broad run: 3 errors, 1 zero-page result, 540 rendered pages. Geometry
  findings occur in linespacing, mixed-cached and equation-limits inputs.
- HWPX alternate-content switches had been silently ignored. The known chart
  alternative now uses the complete embedded OLE mask/color presentation. This
  restores all seven newly supplied chart pages without executing OLE code.
- This is appearance recovery, not full OOXML chart regeneration. Printed chart
  titles and raster quality can differ from the saved presentation; the comparison
  image explicitly exposes this remaining difference.
- Static WMF in an OLE Contents stream is no longer mistaken for legacy chart
  data. Vector ellipse and pen style support restores the 4-page OLE example.
- Zero-length WMF headers are accepted only for a complete header+EOF empty
  metafile, not arbitrary truncated records.

## Still open

- Input 20 needs additional WMF state, polygon, clip, brush and raster operations.
- Input 08 is 3 engine pages versus 2 Hancom pages.
- A separate held input has a malformed CFB FAT; do not silently repair it or
  count it as a supported document without validating the original structure.
- A held character-style input produces zero pages and requires inspection.
- Three inputs have automatic geometry findings to investigate.
- Reference page agreement is not content or pixel agreement. All reference
  pages still require a systematic content/position comparison beyond the
  targeted images; non-reference inputs cannot be labelled Hancom-equivalent.

Evidence: `all-held-inputs.json`, `all-held-initial/corpus-report.json`,
`reference-page-audit.json`, `charts-comparison.png`, `ole-comparison.png`, and
the test/Clippy logs under the stage directory. Later stages must update this
ledger rather than dropping a failing case from the denominator.
