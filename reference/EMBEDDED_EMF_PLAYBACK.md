# Embedded EMF/EMF+ playback, stage 67

The complex WMF examples carry WMFC chunks containing complete EMF streams.
Their small icons are themselves EMF+ only metafiles placed in dual-mode parents.
Playing only the ordinary WMF or ordinary EMF fallback loses these icons.

The engine validates WMFC chunk counts/lengths and implements bounded EMF paths,
Bezier segments, world transforms, object selection, save/restore, clipping and
bitmap transfers. EMF+ nested image placement reads both floating and compact
integer coordinates. Nested EMF+ path images preserve transparent pixels; decoded
bitmap pixels are premultiplied before software compositing. Corresponding GDI
bitmap fallbacks are not painted twice. Recursion, object, point and image sizes
are bounded. Unknown EMF+ only drawing commands fail rather than disappearing.

The measured HWPX metafile crop convention uses a 15000-unit virtual image square
when imgDim is absent and the crop lies in that range. This is a compatibility
inference from the supplied full/cropped examples, not a claim that every writer
uses this convention. Intermediate raster resolution does not change the crop.

Input 20 remains 15 pages. The comparison image shows restored white icons in all
three blue circles and the QPU/network/sensor icons. A regression test detects
white icon pixels inside blue circular components, so blank circles cannot pass
merely by preserving the page count. The five genuinely truncated source BMPs
remain explicitly reported; they are not reconstructed by this work.

Evidence: `output/roadmap/67-embedded-emf/`, including `emf-comparison.png`,
conversion reports and test/Clippy logs. Full EMF/EMF+ command coverage is not
claimed; the implementation is organized into separate ordinary, complex WMF,
EMF and nested EMF+ modules.

References:
- [EMF extended pen](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-emf/d7f51e05-4024-497c-ad4a-8aeca9d34256)
- [EMF+ image parallelogram placement](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-emfplus/9bad3867-71b0-46fb-9b90-f7d8fb37ff76)
- [EMF+ path object](https://learn.microsoft.com/ja-jp/openspecs/windows_protocols/ms-emfplus/b539cf16-6232-4705-9f6e-6f914705145f)
