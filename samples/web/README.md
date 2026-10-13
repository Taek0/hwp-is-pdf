# Public HWP conversion samples (2026-10-13)

Two successful, deliberately small format samples from the public hwplib repository:

| Local input | Public source | Result |
|---|---|---|
| merged-cells.hwp | https://github.com/neolord0/hwplib/blob/main/sample_hwp/merging-cell.hwp | 1 page; 7×7 table (input before the merging operation) |
| paragraphs.hwp | https://github.com/neolord0/hwplib/blob/main/sample_hwp/changing-paragraph-text.hwp | 1 page; Korean sample paragraphs |

PDF/PNG outputs and geometry audit JSON files are in `../../output/web/`.
Both successful outputs have zero detected page-boundary, text, border, or foreground-image collisions.
These small fixtures do not establish compatibility with arbitrary public documents.

Other attempted inputs are kept for reproducing unsupported cases:

- `government-notice.hwp`: https://github.com/DesignTheHAM/ksconv-samples/blob/main/sample-1-government-notice.hwp — unsupported `tcps` character-overlay control.
- `government-faq.hwp`: https://github.com/DesignTheHAM/ksconv-samples/blob/main/sample-2-government-faq.hwp — unsupported `head` header control in the binary adapter.
- `table.hwp`: https://github.com/neolord0/hwplib/blob/main/sample_hwp/basic/표.hwp — caption list interpreted as table-cell data; adapter rejects the short record.
- `textbox.hwp`: https://github.com/neolord0/hwplib/blob/main/sample_hwp/basic/글상자-압축.hwp — nonmatching compressed stream trailer; conversion rejected.

No contents were uploaded to a conversion service. Successful documents were converted locally by the Rust engine and rasterized with Poppler.
