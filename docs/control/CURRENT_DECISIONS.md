# Current decisions and authority

Decision set version: 2026-09-18.p3b-r0.1. Exact current task/next task and product states are stored once in `control/state.json`; exported Source 02 receives measured Git identity. This document defines durable decisions, not repeated volatile HEAD claims.

| Decision | Authority / effect |
|---|---|
| R0-01 | Owner explicitly authorizes current repository reaudit, P3-B product-direction freeze, docs/governance/validation tooling and minimal Source replacement. Product implementation is not authorized in this task. |
| R0-02 | Preserve P3-A explicit selection→shortcut→local Instant→optional selected Provider Deep→review/edit→explicit Apply or Copy permanently. Unsupported adapter does not imply unsafe automatic fallback capture. Sensitive/password fields remain denied; Copy-only may use already explicitly supplied safe text, never silently read denied fields. |
| R0-03 | Freeze P3-B proactive inline assist target: opt-in, document/revision/changed range, bounded local analysis and memory cache, annotation, cached hover, explicit user mutation. Hover inference/network count must be zero. This is a target, not current implementation. |
| R0-04 | Exactly Codex, Antigravity, Claude; one selected Provider per request; no silent replay/fallback. P3-B cloud processing requires explicit per-app/per-document enable and disclosure. Existing P3-A autoRewrite behavior is separately documented, not proof of new policy. |
| R0-05 | First adapter: CHROMIUM_TEXTAREA_CONTENTEDITABLE_MV3. Plain textarea then simple contenteditable within the same family; unsupported rich/custom controls denied. See FIRST_ADAPTER_DECISION.md for official API evidence and blockers. No HWP support promise. |
| R0-06 | P3-A confirmed cancel/process/draft/Apply safety defects require stabilization before proactive monitoring work. Exact next task is the singleton in state.json. No automatic continuation. |
| R0-07 | Product acceptance is not renewed by this reset. P1-03 historical acceptance and current implementation/build/tests are distinct; live Provider/editor acceptance remains separately reported. |
| R0-08 | Multi-agent work uses exact-base packets, new worktrees, exclusive path claims and measured result packets. Wrappers reference one digest-pinned contract. No automatic rebase/cherry-pick on stale main. |
| R0-09 | Replace all active Project Sources with exactly 12 generated markdown files. UI application/verification instructions belong outside payload. Generate qualified successor only from canonical main after safe adoption. |

## D-020 corrective disposition

`D020_DOCUMENTATION_OVERCLAIM` is the current verdict for locally inspectable evidence. The Owner's task quotes a historical D-020 claim of foreground restore, bounded current-selection copy probe, exact source equality, and paste-zero on no-selection/timeout/mismatch. The current existing export's `PROJECT_CONTROL/01_GOALS_AND_ACCEPTANCE.md:14` also claims source revalidation and cites D-020. The complete original D-020 source was not recovered; the older ignored 2026-08-13 decision log contains earlier decisions only.

Current `src-tauri/src/apply_safety.rs:14–25` has no selection-read operation; `capture_session.rs:104–109` ApplyContext has no current selection bytes; `apply_safety.rs:192–202` checks target then pastes. Clipboard read at `:171` is for restoration. Stored session/intent checks in `main.rs:675–687` do not implement external selection equality. The introducing commit `54aecf5199362023dbf69a0ac696cc415277dba2` already lacks that probe; later `0d48ca6007493600c9a782fe5fe47c78e962bcac` does not add it. No regression from a locally reachable implemented D-020 was found. Unavailable external commits/source are UNKNOWN, not disproven.

Do not propagate the historical overclaim. Stabilization must prove the bounded current-selection condition using synthetic editor content reread and paste count, or explicitly limit unsupported states to Copy-only. No code repair is performed in R0.

## Historical authority handling

The September 6 export, P0/P1/P2/P3 gate documents and prior root audit remain evidence, not current status authority. Existing ignored/untracked sources and other worktrees are preserved. Local branches whose tips are ancestors of main are historical/merged; dirty contents are not abandoned or owner-adopted by inference. Remote relationship is local tracking only; no fetch/push is authorized.
