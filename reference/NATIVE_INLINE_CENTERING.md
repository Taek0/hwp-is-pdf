# Native inline object baseline semantics — stage 63

A saved HWP line with baseline 1507 and text height 3015 uses a centered object
baseline. Applying the normal 85% object baseline moved two equal-height nested
input tables to different Y positions because their outside margins differed.
The renderer now passes a 50% baseline only for this saved native-line case;
ordinary 85% lines and rebuilt paragraphs keep their established path.

Native inline rectangles also include outside margins in baseline height, as
native tables and pictures already did. The existing grown-rectangle recovery
remains guarded separately. Neither change uses a filename or object ID in
production code.

The law exam's name and candidate-number boxes now start at 199.995/200.0pt,
versus 200.04pt in Hancom. Its type-label rectangle starts at 188.39pt versus
188.4pt. Real-fixture and synthetic tests cover both ratios, rectangles and
asymmetric margins. The previous plain paragraph-relative floating-object anchor
path is not generalized from these inline cases.

Validation: 277 tests and all-target Clippy pass; 73 inputs / 308 pages convert
without failures or four-class geometry findings. All 40 reference page counts
match. Input/source hashes were checked. Only law page 1 and notice pages 21–22
change relative to stage 62, without text changes; the other 71 PDFs are identical.

Artifacts: output/roadmap/63-native-inline-centering/. Inspection includes
compare-header-baselines.png and compare-notice21.png, with layout reports and
verification.json. These checks are corpus evidence, not universal pixel identity.
