# HWP/HWPX implementation coverage — 2026-10-13

본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.

Official reference: https://www.store.hancom.com/support/downloadCenter/hwpOwpml
HWP 5.0 rev 1.3 (2018-11-08), equation rev 1.3, chart rev 1.2,
distribution rev 1.2. Supplemental PDFs/text: reference/; main spec:
../engine/research/hwp5-spec.pdf.

Parsing, successful PDF creation, collision checks, and source fidelity are
separate acceptance criteria. **Full format compatibility is not achieved.**

| Feature | Implemented/tested | Remaining limitation |
|---|---|---|
| Container/records | CFB, raw DEFLATE, CRC/size trailers, zero padding, bounded reads | HWP 3.x unsupported |
| Distribution | Public-format XOR/key derivation/AES; two-page fixture | Password-encrypted documents unsupported |
| Text/fonts | UTF-16 positions, fallback fonts, TTC embedding; negative tracking bounded before line fitting | Font selection primarily Hangul/Latin; private-use symbols incomplete; no general reflow |
| Tables | Nested/merged cells, captions, inherited margins, row splitting/repeated headers, single-cell line splitting | Complex merged cells across page breaks need more cases |
| Header/footer/page | Binary head/foot, page numbers, new-number metadata, first-page hiding | General section/column transitions, alternate orientations and advanced number formats |
| Overlay/ruby | Binary overlay and ruby controls | Other overlay borders approximate; typography differs |
| Notes | Binary footnotes/endnotes; HWPX note path retained | General footnote reservation and exact endnote pagination incomplete |
| Numbering/bullets | Seven-level heads, counters, labels and bullet records | Extended formats; fixture private-use bullet F06C lacks a glyph mapping |
| Shapes | Line/rectangle/ellipse/polygon/arc, nested groups | Curves use straight segments; full affine transforms and text art incomplete |
| Images/OLE | PNG/JPEG/BMP/PCX, basic effects; OLE static BMP preview | Crop/rotation, WMF/EMF and arbitrary OLE presentations unsupported |
| Native chart | Bounded VtArchive types/objects/grid; integer/double data, labels and styles; fixture values verified | Simplified 2D rendering; all chart types/3D/axis semantics and OOXML charts incomplete; unknown types use labelled data table |
| Equations | Binary eqed, fractions/scripts, roots, matrices/cases/piles and symbols; bounded recursion | Full grammar, column-oriented matrices, decorations, exact metrics; unknown commands may remain literal |
| Fields/bookmarks | Cached visible text retained; nonprinting metadata admitted | Live field evaluation, PDF links/destinations not implemented |
| PDF/QA | Embedded TrueType/CID/ToUnicode, vector paths/images; outline sampling and rule/image checks | Not pixel comparison; arbitrary shape intersections not exhaustively tested |

## Reproducible evidence

- User fixtures: original HWP 1/15 pages, HWPX 9 pages; regression suites.
- samples/features: 14 files, all convert; page/collision gates in tests/feature_regression.rs.
  Approximation and missing-glyph warnings retained.
- samples/web: 6 files, all convert. Government notice 30 pages, FAQ 14 pages;
  all four current QA finding classes empty. Counts are engine outputs, not verified Hancom counts.
- output/features/corpus-report.json and output/web/corpus-report.json: outcomes,
  warnings and QA. Successful conversion does not establish fidelity.
- MIT chart parser source/modifications: vendor/legacy-chart/UPSTREAM.md.
- REA investigation: reference/viewer-analysis/README.md and raw Evidence.

No viewer executable or DLL is invoked by the converter. No document is uploaded.

## Additional corpus (2026-10-13)

8 pyhwp fixtures in samples/additional now generate 37 pages with zero findings
in the four QA classes. In-section column changes, caption clipping/overprinting,
block-picture anchoring, and eleven underline styles were fixed and regression
tested, in addition to binary landscape PageDef handling. Source-directed visual
review and structural tests supplement QA; zero findings alone are not fidelity.
All 26 engine tests and Clippy pass. Full results:
samples/additional/README.md; output/additional/corpus-report.json.

## Full revalidation

Latest full run: output/revalidation-20261013-164704/SUMMARY.md.
31 inputs (including all three user documents), 127 pages, zero findings in all
four QA classes. Input SHA-256 and per-document page/object counts are preserved.
27 tests and Clippy pass. Full raster comparison also caught an HWPX underline
NONE regression; this and the PAPER-relative page-border coordinate bug were fixed.
Previously absent baseline QA data is marked unavailable, not interpreted as zero.
Known fidelity limits and warnings above remain applicable.
