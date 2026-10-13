# REA viewer investigation — 2026-10-13

User-selected repository: `D:\ChatGPT\hwp\rea`, package version 6.4.0.
Read its AGENTS.md, reverse-engineer-anything SKILL.md, connection recovery,
native/managed guide and managed CLI adapter source. The public npm CLI 6.4.0
was used without changing MCP registration or the viewer installation.

## Observed

- `wrapper.json`, Evidence
  `ev_050069c10f7dea5826bc343f972b58063168f2d9eb4d6598e8506ab883ef35ab`:
  HwpEngineWrapper.dll is x86 PE, mixed C++/CLI and native code; ILONLY is clear.
  SHA-256 `d7bae03df7862a9b845fb0c8804829592d31d648d1db771f6af1641249862eec`.
  The admitted CLI header is at file offset 435636, flags at 435652.
- `wrapper-members.json`, Evidence
  `ev_b396bacc28492bdedcce14fd4621d4409a7ae6760eb14dfc590e1f0bbc3bafea`:
  member inspection unavailable because metadata representation exceeds REA's
  67,108,864-byte admission budget. This is a parser diagnostic, not evidence
  of protection, corruption or the file's actual metadata byte length.
- `viewer-members.json`, Evidence
  `ev_9119e378464cacaaec7a59d316d32cb5ba05d0124a89dda101c78e6ef70bfbe0`:
  HwpViewer.exe member inspection encounters the same representation limit.
- `providers.json`: Ghidra is not configured and Java is unavailable to the
  provider; IDA is not configured; Hopper does not support this Windows host.
  The native provider has not analyzed any viewer function.

## Unresolved

Document serialization call chains, native record interpretation and exact
layout algorithms have NOT been recovered by REA. No CIL/native function
decompilation or runtime behavior is claimed. No target code was executed by
these REA commands. The public npm command exposes fewer member-filter options
than the local source adapter; source inspection is not proof of CLI availability.

The Rust converter instead reads HWP/HWPX bytes itself using the public specs and
public fixtures. Its legacy chart parser is separately attributed MIT source.
The static viewer analysis is research evidence only, not a runtime dependency.
