# Justified leading padding — stage 93, 2026-10-20

Remote `main` was fetched and `git pull --ff-only origin main` confirmed the
clean checkout at `e9f017d`, including the Docker CLI packaging work.

The residual horizontal displacement in input 25 was caused by expanding typed
leading spaces during justification. Those spaces are padding before the first
visible item, not gaps between words. The ordinary line painter now distributes
space only between visible items; zero-width controls do not terminate the padding
scan. The selection rule lives in `src/layout/justification.rs` rather than adding
more layout logic to the large text painter. Explicit character distribution is
unchanged, and there are no filename/paragraph-ID rules in production code.

## Independent reference

| Input 25, first-page formula | Hancom | Before | After |
|---|---:|---:|---:|
| `BS` x origin (pt) | 91.741 | 98.118 | 91.690 |
| Left fraction rule x origin (pt) | 271.200 | 274.866 | 271.233 |

The earlier restored fraction rule and OLE contents remain present. This corrects
horizontal positions; it does not claim identical font outlines or fix every
remaining line-break difference. A broader matched-word comparison also records
mixed results where other font/layout differences remain, rather than hiding them.

## Regression

202 currently available SHA-distinct inputs convert to 592 pages. All page counts,
per-page non-whitespace character multisets and raw QA findings match the previous
engine. Twelve PDFs change; 190 are byte-identical. All 161 changed-document pages
were rendered and reviewed in overview sheets, with the reference formula enlarged.

Historical input `202_복학원서.hwp` (an old editor export, SHA-256
`f8adb473ede5e265cf713c5598044274759205bbdfeaa2b2115131ba2e2a3dc7`)
cannot be found by hash and is explicitly excluded from this run, as in stage 92.
The original form and other available form variants remain included. Its two old
PDF pages are historical evidence, not a newly verified output.

337 WSL Rust tests and all-target Clippy pass. The tests verify natural leading
padding under LEFT/JUSTIFY/DISTRIBUTE_SPACE and the independently printed formula
origin. Conditionally guarded external-file tests are not Docker test-suite coverage.

Local evidence: `output/roadmap/93-justified-padding/`, including the available and
unavailable manifests, changed-input list, raw QA comparisons, reference-position
checks, page images and `formula-position-comparison.png`. Docker parity results
are recorded separately under its `container` directory.

The Linux Docker probe and WSL release CLI produce byte-identical PDFs, sidecar
reports and QA for all 202 available inputs (592 pages), also identical to the
current baseline. The versioned local image `hwp-engine:0.1.0-container.2-4f497c0`
contains the same engine binary SHA-256 and passes an additional input-25 smoke
test. Image ID: `sha256:dc83aa11009cc84a238f5b724053c61611b9a83311120f1331928d0024da7c6f`.
No image was published externally and the previous image was retained.
