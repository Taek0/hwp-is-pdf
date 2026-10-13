# Source and local scope

MIT © 2025–2026 Edward Kim. See LICENSE.
Source: https://github.com/edwardkim/rhwp/tree/main/src/ole_chart
Retrieved 2026-10-13.

Original Git blob identities:
- grid.rs: ac0fcf3de5d4b243f7a113b93eb519d00580cf82
- orientation.rs: ce77bc7ce957a2707aff5203160f9291587c3b4d
- parser.rs: 9a8267eb22f14c707f3e9ef8a17a375a985f4628
- legacy_presentation.rs: 7df8380ae579b76900a4978ef0dafb5fc1bb12d8

This small standalone package preserves the upstream grid/parser/orientation implementation and its tests.
It does not import the upstream editor, native libraries, or online services.
Local rendering is separate; successful grid parsing does not imply original chart styling is fully understood.

Local changes to legacy_presentation.rs: public read API, preserve chart_type and optional auto axes,
accept the documented type range 0–26 instead of restricting parsing to the upstream combo-chart case.
Wire reads, class-version validation, and bounds limits are unchanged.

Local changes to grid.rs and legacy_presentation.rs: decode VtInteger's signed
32-bit value and base VtValue fields, as present in the public hwplib chart.hwp
fixture. Integer cells participate in numeric data and orientation selection.
Integer offsets are deliberately excluded from the upstream f64 editing-offset
API, which is unused by this read-only converter.
