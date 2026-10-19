# Sparse container compatibility and line endpoints, stage 71

One held drawing document could not open because unused FAT and MiniFAT slots
were zero-filled instead of FREESECT. These slots pointed repeatedly to sector
zero. The stream payloads remained present.

The bounded compatibility path applies only to version-3 containers with an
in-header DIFAT. It walks directory, FAT, MiniFAT, root mini-stream and all active
stream chains. Only unreachable zero links with zero-filled backing storage (or
slots beyond the physical allocation extent) are cleared in a private copy.
The normal CFB parser must then validate the result. Active cycles/cross-links,
nonzero orphan data and truncated payloads are not repaired. Source files remain
unchanged and successful compatibility recovery is reported in warnings.

The recovered document's saved preview exposed a separate missing primitive:
native line start/end coordinates were not used for direction. Native and HWPX
lines now retain their endpoint ordering inside their object bounds, including
rising, reversed and horizontal lines. Arrowheads follow the same directed ends.

Validation: 304 tests and all-target Clippy pass. All 194 frozen byte-distinct
inputs convert to 561 pages with zero failures or zero-page outputs. Source
hashes are unchanged. Relative to stage 68, only the already documented limit
equation and this newly recovered drawing differ; the other 192 PDFs are byte
identical. The new drawing is checked against its saved source preview, not an
independent Hancom print. Previously viewer-rejected probes remain labelled as
such; parser success does not establish viewer acceptance.

Evidence: `output/roadmap/71-compound-compatibility/`, including complete batch
results, source-hash verification, changed-input list, comparison image and
test/Clippy logs. The original document and diagnostic copies are not committed.
