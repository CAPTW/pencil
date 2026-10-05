> Current cross-format checkpoint (2026-10-06): OXT600aa72 with explicit
> saved/uniform/mixed/paragraph/cross-format options, paired app/native0fc8624.
> In owned pinned LibreOffice26.2.4.2, select supported mixed bold/RGB runs within
> ONE Standard paragraph; replacement uses first selected style, Undo restores
> original runs. Policy is shown before Enable. Surrounding paragraphs and empty
> typing styles remain preserved. Native47 (8 saved DOCX cycles) and installed
> widget8/enclosing5 PASS with actual rereads/Undo, source-change and Disable
> preservation, no automatic Save and natural cleanup/uninstall. Cross-paragraph
> selections and complex structures remain refused. Next: paragraph-spanning
> replacement semantics and range Undo. Actual Office/Hancom, IME, normal profiles
> and live Providers remain unqualified/NOT_RUN. Mission INCOMPLETE, no main/push.
> Evidence: grammar-writer-cross-format-20261006. Earlier sections are historical.

> Prior paragraph-aware checkpoint (2026-10-06): OXT9443ff2 with explicit
> saved/uniform/mixed/paragraph options and unchanged app/native0fc8624 qualifies
> one homogeneous selected range within one Standard paragraph of a bounded
> multi-paragraph body in owned pinned LibreOffice26.2.4.2. Surrounding paragraph
> text/alignment, supported script weights/RGB and empty-paragraph typing format
> are preserved. Installed widget8/enclosing5 and native41 including8 saved DOCX
> Apply/Undo/Redo cycles PASS. No automatic Save; natural cleanup/uninstall complete.
> Cross-paragraph/cross-format selections, complex structures, normal profiles/IME,
> actual Office/Hancom and live Provider remain unqualified/NOT_RUN. Mission
> INCOMPLETE. Next: define and implement cross-format/cross-paragraph replacement
> semantics and range Undo. Prior paragraphs below describe historical artifacts;
> current F06 scope is authoritative. Evidence: grammar-writer-paragraphs-20261006.

> Prior mixed-surroundings checkpoint (2026-10-06): product OXTa3b231a with
> explicit saved/uniform/mixed options plus unchanged app/native0fc8624 qualifies
> a homogeneous selected range within one Standard body paragraph with different
> supported bold/RGB formats outside the selection. Pinned LibreOffice26.2.4.2,
> owned profile and synthetic new/local saved DOCX only. Installed widget8/enclosing5
> and final native39 including16 saved Apply/Undo cycles PASS; exact complete text
> and per-character format rereads, Undo/Redo, no implicit Save and natural cleanup.
> Cross-format selections, selected line breaks, multiple formatted paragraphs,
> inconsistent imported color metadata and complex structures remain refused.
> Next work: paragraph-aware preservation, then explicitly defined cross-format
> replacement semantics. General Office/Hancom, IME/profiles/live Provider remain
> unqualified/NOT_RUN. Mission INCOMPLETE; no main adoption or push. Evidence:
> grammar-writer-mixed-20261006. Earlier checkpoint notes below retain historical
> artifact scopes; F06 current qualification_scope is the current union.

> Prior uniform-format checkpoint (2026-10-05): app/native0fc8624 plus product
> OXT2a16115 with explicit saved/uniform options qualifies one Standard paragraph
> with homogeneous script weights, opaque non-theme RGB and alignment in owned
> pinned LibreOffice26.2.4.2. Installed widget8/enclosing5, formatted core32/16 cycles,
> plain core73/32 cycles PASS; explicit synthetic Save/reopen and cleanup confirmed.
> Plain-mode OXT5a339b8 qualification remains separate. Mixed runs, multiple formatted
> paragraphs, structures, normal profiles/IME/other apps/live Providers remain
> unqualified or NOT_RUN. Next work within the authorized global-adapter task is
> mixed-format range preservation and paragraph-aware Undo, not renewed saved-plain
> qualification. Mission INCOMPLETE; no main adoption or push.
> Earlier checkpoint notes below are historical scope statements, superseded by
> this paragraph and current state F06. Evidence: grammar-writer-uniform-format-20261005.

> Prior saved Writer checkpoint (2026-10-05): F06 now includes local saved/reopened
> default-style plain DOCX with pinned26.2.4.2, app/native0fc8624 and product
> OXT5a339b8 built with --enable-saved-docx. Actual installed widget6 plus core73
> and32 cycles PASS; exact Apply/Undo, save revocation, readonly/user-edit/Disable
> boundaries, no automatic disk Save and complete cleanup. Older unsaved-only
> statements below are historical. Complex/rich formatting, other applications,
> physical IME/general profiles/live Providers remain unqualified or NOT_RUN.
> Mission INCOMPLETE. Evidence: grammar-writer-saved-apply-20261005.

> Source semantic correction (2026-10-05): F-06 is an overall scoped summary.
> Its current verdict explicitly includes qualified Edit/RichEdit, Chromium and
> bounded Writer synthetic Accept. `qualification_scope` states each boundary;
> it does not qualify saved/rich Writer documents or arbitrary editors. The old
> OTHER_EDITORS_UNQUALIFIED summary was stale, not evidence invalidation.
> Retained legacy8 Project Sources are historical only. Current00-11 remain the
> authoritative exact12 set, regardless of total Project inventory.

> Current Source replacement decision (2026-10-05)
>
> Owner requests replacement from the local candidate, not main integration.
> Source02 measured branch/HEAD describes the candidate; main remains4e833988.
> Contract1.5.0 authorizes this exact candidate snapshot export. Upload only the
> twelve payload markdown files; do not upload transport ZIP or metadata files.
> UI upload and fresh-chat access/semantic checks remain Owner work. Continue
> authorized development without treating that external step as product acceptance.
> Latest installed Writer runtime0fc8624 is narrowly qualified; the next product
> work is saved synthetic Writer DOCX and formatting-preserving Apply/Undo.

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

## Owner correction — 2026-10-05: restore desktop direct Apply first

The Owner explicitly reaffirmed desktop direct Apply as a core requirement and instructed that fixing it take highest priority. This supersedes the r3/r4 interpretation of Copy-only as a permanent product decision. Existing Copy-only remains temporary safety containment until an actual supported native target is qualified. The R1/R2 evidence stays valid and must not be bypassed. Unrelated Codex spawn-cleanup work is deferred. Work inside the original mission may implement the target-side edit transaction and required desktop integration; no permission is inferred for live Provider/account use, ordinary-profile/system registration, security weakening, push or release.
