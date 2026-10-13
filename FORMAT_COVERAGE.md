# Official-format implementation ledger

본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.

Reference: https://www.store.hancom.com/support/downloadCenter/hwpOwpml
Primary specification: HWP 5.0 revision 1.3, 2018-11-08 (local research/hwp5-spec.pdf).
Scope: document content and visual conversion to PDF. Embedded executable code is not run.

This ledger distinguishes parsing, rendering and fixture verification. A parsed record alone is not complete support.

| Feature group | Current state / next acceptance |
|---|---|
| CFB, raw DEFLATE, record hierarchy | Implemented; investigate legacy trailing bytes |
| Text, font mapping, paragraph cache | Implemented subset; general reflow remains |
| Tables/merged cells | Implemented; caption and multi-page rows pending |
| Header/footer | Binary adapter pending; common renderer exists |
| Character overlay, ruby | Pending |
| Footnotes/endnotes | HWPX endnotes subset; binary adapter pending |
| Page number, section/column rules | Partial; new numbering/hiding/orientation pending |
| Shape line/rectangle/ellipse | Implemented subset |
| Polygon/arc/curve/group/transforms | Pending |
| Pictures/PCX/effects | Implemented subset; clipping/rotation pending |
| Equation | HWPX subset; binary adapter and grammar extensions pending |
| Bookmarks/fields/hyperlinks | Partial visible-text preservation; PDF links pending |
| Distribution documents | Pending public-format decoder |
| Password-encrypted documents | Pending documented password-based format coverage |
| OLE/chart/text art/form objects | Pending static visual representation |
| Layout without cache, page/column splitting | Pending |

Previously validated fixtures remain regression gates. Failed public fixtures must remain visible in the results; no blanket compatibility claim is made.
