# Source organization

The public entry points remain `convert*`, `model::Document`, `layout::Engine`, and `hwp::open`.
The module splits are internal; callers do not need to depend on implementation files.

| Location | Responsibility |
| --- | --- |
| `hwp.rs` | Bounded binary readers, record tree, common node builder |
| `hwp/document.rs` | Compound-document streams, DocInfo and document assembly |
| `hwp/controls.rs`, `hwp/columns.rs` | Control records and column definitions |
| `hwp/list_records.rs` | Numbering starts/extended levels and binary bullet records |
| `hwp/tab_records.rs`, `tabs.rs`, `layout/tab_layout.rs` | Tab records, destination advances and leaders |
| `hwp/paragraphs.rs`, `hwp/section.rs` | Paragraph and section records |
| `hwp/object_properties.rs`, `hwp/images.rs` | Object placement and embedded-image decoding |
| `model.rs` | Shared document/node model and HWPX package loading |
| `layout.rs` | Engine state, shared layout types and public rendering entry points |
| `layout/section_render.rs` | Section dispatch and saved-layout rendering |
| `layout/reflow*.rs` | Paragraph, object and table measurement and ordinary flow |
| `layout/reflow_units.rs` | Styled text/control/object metrics passed to the line breaker |
| `layout/equation_layout.rs`, `layout/endnote_flow.rs` | Equation sizing/painting and source-based endnote continuation |
| `layout/caption_layout.rs` | Caption width budgets, nested measurement and painting |
| `layout/shape_paths.rs`, `math_accents.rs`, `math_stacks.rs` | Shape paths and structured equation decorations/stacks |
| `number_format.rs`, `layout/list_labels.rs` | Number sequences, list counters, label geometry and image bullets |
| `layout/complex_text.rs`, `layout/local_flow.rs` | Bidi/OpenType shaping and nested-list flow |
| `layout/column_*.rs` | Column geometry, source continuation, balancing and shared-note reservation |
| `layout/text_*.rs`, `layout/object_paint.rs` | Text styling and drawing operations |
| `layout/table_layout.rs`, `layout/table_measure.rs` | Table/cell rendering and shared grid measurement |
| `layout/table_pagination.rs`, `layout/table_vectors.rs` | Table fragment transitions and measured-vector splitting |
| `layout/notes.rs`, `layout/page_decorations.rs` | Notes, page furniture and finalization |
| `layout/page_furniture.rs` | Header/footer cache recovery, measured space and page variant selection |
| `reflow.rs`, `pagination.rs`, `wrap_region.rs` | Layout algorithms without engine orchestration |
| `pdf.rs`, `qa.rs` | PDF emission/font metrics and geometric output checks |
| `pdf/operations.rs` | Shared drawing operations and logical text spans |
| `hwp/field_records.rs` | Native field commands and bookmark parameter decoding |
| `layout/field_values.rs`, `layout/field_navigation.rs`, `layout/toc.rs` | Deferred values, logical link scopes and outline TOC generation |
| `pdf/navigation.rs` | Link annotations, named destinations and hierarchical outlines |

Add behavior to its owning module. Split by responsibility when a file acquires a second concern;
do not create numbered file fragments or duplicate implementations to meet a line limit.
Keep internal helpers private or `pub(super)` unless a public caller actually needs them.

Fixtures and generators belong in `samples/` and `examples/`, integration checks in `tests/`.
Generated PDFs, page images and audit reports belong in ignored `output/`; build products belong
in ignored `target/`. Do not move source documents into generated-output folders.

Additional typography modules:
- `pdf/fonts.rs`, `pdf/font_catalog.rs`: font bytes, TTC extraction, installed face catalog and weight selection.
- `layout/font_selection.rs`: source-family matching and explicit substitution policy.
- `layout/saved_cell_text.rs`: overflowing plain-text HWPX cell cache recovery and nested height propagation.
- `layout/paragraph_borders.rs`, `layout/rectangles.rs`: connected paragraph borders/fills and rounded rectangle geometry.
- `layout/text_decorations.rs`: underline and strike rendering.

Page coordinate modules:
- `layout/page_borders.rs`: source border-origin normalization, parity, visibility and fill extent.
- `layout/object_position.rs`: shared paragraph/page/paper vertical anchor calculation.
