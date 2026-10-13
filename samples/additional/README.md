# Additional public fixture validation — 2026-10-13

Source: https://github.com/mete0r/pyhwp/tree/master/tests/hwp5_tests/fixtures
Only fixture bytes/reference XML were read; pyhwp was not installed or executed.
The local Rust engine performed every conversion. No REA/disassembler was run.
Exact download URLs and Git blob IDs are in sources.json, downloaded SHA-256 in
sha256.json. Original inputs are retained unmodified.

Outputs: ../../output/additional/. All 8 files create PDFs (37 output pages after fixes).
All pages were rasterized with existing Poppler and inspected via contact sheets;
caption, underline, paragraph and page-orientation samples were also viewed alone.
Conversion and zero QA findings do NOT establish faithful format support.

| Input | Pages | Outside | Text overlaps | Rule crossings | Image overlaps | Review |
|---|---:|---:|---:|---:|---:|---|
| aligns.hwp | 16 | 0 | 0 | 0 | 0 | No obvious clipping in overview; full alignment equivalence unverified |
| matrix.hwp | 1 | 0 | 0 | 0 | 0 | Shape transformation fixture, not equation matrix; transform fidelity unverified |
| multicolumns.hwp | 3 | 0 | 0 | 0 | 0 | FIXED: 1→2→3 column definitions applied at paragraph boundaries; nested controls excluded |
| pagedefs.hwp | 2 | 0 | 0 | 0 | 0 | FIXED: section 2 landscape was ignored; now 841.88×595.28 pt, section 1 595.28×841.88 pt |
| parashape.hwp | 1 | 0 | 0 | 0 | 0 | Text visible, no obvious collisions; detailed indent/spacing fidelity unverified |
| sample-5017-pics.hwp | 11 | 0 | 0 | 0 | 0 | FIXED: block pictures use their reserved space before the first text line; empty anchors retain their origin |
| table-caption.hwp | 2 | 0 | 0 | 0 | 0 | FIXED: combined caption/body box, outer margins, vertical alignment, expanded width and sequential pagination |
| underline-styles.hwp | 1 | 0 | 0 | 0 | 0 | FIXED: all 11 styles drawn, including dash/dot and weighted double/triple lines |

## Code change from this validation

HWP PageDef table 132 bit 0 describes orientation separately from nominal paper
dimensions. src/hwp.rs now swaps width/height for landscape. The PDF MediaBox is
checked by tests/additional_regression.rs and pdfinfo. All 26 engine tests pass;
Clippy with -D warnings passes. New tests verify eight separate table bodies,
labels below block pictures, actual column x positions and underline stroke presence.

The original caption clipping (2), text overlaps (49), and image overlaps (14)
are now zero. These before/after counts preserve the original findings. Full Hancom
pixel equivalence, every column transition type and every caption/page combination
are not established by these finite fixtures. Source text/data were not deleted.

Final previews are in output/additional/fixed/. Original raw source XML is retained
beside inputs for reproducing attribute interpretation; it is not conversion input.

Reproduce:

    cargo run --example feature_corpus -- samples/additional output/additional
    cargo test

The machine-readable outcome/QA counts are in output/additional/corpus-report.json.
