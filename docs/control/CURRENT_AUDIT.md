# Current Repository Reaudit — product-base snapshot

AUDITED_PRODUCT_BASE=9f54ebd4b7586f7a1658245761ccccf8a77500c6
EVIDENCE_CLASS=CODE_PROVEN + TEST_PROVEN + FIXTURE_PROVEN, LIVE_PROVIDER=NOT_RUN
This document records an immutable product-base audit, not the volatile current docs commit. Export generator measures actual canonical main identity into Source 02. Product source is unchanged by this authority-reset candidate.

## Baseline / inventory
Canonical entry main was tracked clean, tree a1bf70c9a5d215a8ce1f22e360790f0f3b90ad53, parent 1528dc3fca8f373ab02666e72a97738a6937efe2; 19 commits and 395 tracked files. Categories: Rust production 43; dedicated Rust tests 9 (8 src modules + 1 integration target); frontend 12; scripts 32; docs 11; protocol 268; assets/config 20. Every tracked path/hash is in ENTRY_BASELINE.json and TRACKED_PATHS.txt in external evidence. Protocol 268 paths comprise schema snapshots and provenance, not 268 handwritten production modules; path/hash/reference inventory used, not repetitive full schema-body reading.

Existing canonical untracked AUDIT_JOURNAL.md, REPO_AUDIT_REPORT.md and Grammar_PROJECT_CONTROL_EXPORT_20260906T134100Z.zip are preserved. No stash, fetch, push or other worktree mutation. main...origin/main=9 0 is local tracking only. All eight pre-existing non-main branch tips are equal to or ancestors of main; no unmerged committed candidate was found. Registered r2/r5/r6 worktrees have dirty Cargo.toml; historical original P3-03 worktree has 2 modified and 5 untracked files. These are preserved; owner adoption/abandonment unknown, no automatic reuse. New task worktree is separate.

## Actual validation summary
Locked npm ci --offline --ignore-scripts passed (76 packages); existing lockfile versions unchanged. npm run typecheck and actual configured npm run build:frontend passed. Full engine contract with both external self-test roots passed 468/0; 13 other Node scripts passed, personal bundle contract 80/0 and daily-use bundle contract GREEN. Actual TS modules loaded in memory: 9 behavior assertions passed; App AST canApply true before Instant edit and false after (no GUI/backend assertion).

cargo check --locked --offline PASS; full cargo test --locked --offline: lib 3/0, bin 185/0 with 13 ignored, integration 14/0 with 1 ignored (202 passed, 14 ignored). cargo clippy --locked --offline exit0 with 26 bin warnings, not warning-free. cargo fmt --check exit1, pre-existing formatting differences (no product formatting). Historical benchmark contract fails 107 assertions/16 failures: authorized path set drift and CRLF checkout violate its original LF/hash contract. These failures are retained, not relabeled pass. Engine runtime correctness cannot be inferred from benchmark self-tests.

Tauri actual-config native release build --no-bundle --no-sign -- --locked --offline PASS, executable bytes=13102592 SHA256=7f6ead7556e809743fbbc808b60e537ca94ce4f0e3d98eac4bb9ad7c6135b350. Build receipt records the audited base and product diff zero. Tool rewrote isolated Cargo.toml EOL/stat representation; exact HEAD blob equality was checked, canonical physical bytes retained, identical-blob index refresh produced zero staged product diff. Existing dirty worktrees were untouched.

Native store smoke: explicitly executed ignored test `p1_02_windows_live_tests::p1_02_windows_live_store_profile_suggestion_and_import_restart_acceptance`; `native-store-smoke.log` records `1 passed; 0 failed; 0 ignored` (5.04s). `NATIVE_STORE_RESULT.json` records exit=0, classification=FIXTURE_PROVEN_NATIVE_WINDOWS_STORE, provider_or_editor_live=false, and the owned external temporary root. This proves the synthetic Windows terminology store/profile/suggestion/import/restart fixture only; it does not establish live Provider or editor Apply acceptance.

Provider outcomes: Codex NOT_RUN, Claude NOT_RUN, Antigravity NOT_RUN. Existing authenticated app-session availability and safe isolation for live requests were not established; no login/logout or credentials inspected/changed. Synthetic parser/module tests are FIXTURE_PROVEN, not LIVE_SUCCESS. Interactive editor/hotkey/clipboard tests remain NOT_RUN: exclusive safe desktop/clipboard ownership was not established. Full unit tests include Windows ProcessJob synthetic descendant cleanup. Diagnostics residual_count=0 is a code constant and not measured cleanup. An executable-path-scoped process inventory found 0 running processes under this evidence root; it is not a universal zero-residual proof.

## P3-B capability readiness (document-scale target)

This is the single canonical 24-capability readiness matrix. PARTIAL_SEAM_EXISTS means a reusable P3-A capture/draft/cache seam, not an implemented external-document capability: external document identity and revision remain ABSENT. IMPLEMENTED_AND_USED for Instant and Deep is strictly P3-A selection scope; P3-B incremental routing and per-app/per-document cloud policy remain absent. All P3-B adapters, monitoring and annotations are unimplemented.

|Capability|Status|Current evidence / boundary|
|---|---|---|
|stable document identity|PARTIAL_SEAM_EXISTS|captureContract.ts:1 sessionId/generation and p3_03_runtime.rs:119 SourceIdentity are selection identity, no live document epoch|
|document revision|PARTIAL_SEAM_EXISTS|instantSelectionRuntime.ts:15 draftRevision and promptlessContract.ts:20 settings contract; neither tracks external edits|
|changed-range event|ABSENT|App.tsx:578/606 receives candidate/capture events only; no document change subscription in inspected entry graph|
|incremental text snapshot|ABSENT|App.tsx:640 whole selectedText snapshot; p3_03_runtime.rs:129 whole source AnalysisRequest|
|range rebasing|ABSENT|resultReview.ts:30 LCS review diff has no editor range/revision mutation contract|
|per-app adapter interface|ABSENT|main.rs:3-30 module graph lacks editor adapter; provider adapters are different concern|
|editor capability detection|ABSENT|App.tsx:544 prerequisites check CLI commands, not editor capability|
|inline annotation/underline|ABSENT|App.tsx:2273 renders diff inside local result card, not target editor|
|suggestion card anchoring|ABSENT|App.tsx:2132 local result-card only; no editor rect/range anchor|
|cached hover|ABSENT|App.tsx event/state graph and runtime have no hover cache action|
|focus/caret/selection coexistence|PARTIAL_SEAM_EXISTS|App.tsx:511 focuses own heading, windowChromeContract.ts:27 protects interactive window drag; external IME/caret coexistence missing|
|user edit protection|PARTIAL_SEAM_EXISTS|instantSelectionRuntime.ts:46 lateCandidate protects dirty draft; :71 marks user; App.tsx:1434 Apply conflict persists; no external document edit protection|
|local suggestion cache|PARTIAL_SEAM_EXISTS|p3_03_runtime.rs:17 SessionCandidateCache used; selection-scoped, not document incremental cache|
|lifecycle invalidation|PARTIAL_SEAM_EXISTS|p3_03_runtime.rs:180 eleven capture lifecycle reasons; missing document detach/logout revision model|
|per-app opt-in|ABSENT|promptlessContract.ts:20 AppSettings full fields have no app monitor allowlist|
|sensitive field detection|ABSENT|no UIA IsPassword/editor-sensitive guard surfaced in production search/call graph; protected terminology is not secure-field detection|
|app/domain denylist|ABSENT|AppSettings schema and current entry graph have none|
|visible monitoring state|ABSENT|App.tsx:2035 status pill is rewrite status, no monitoring mode exists|
|pause/emergency disable|ABSENT|App.tsx:490 rewrite Cancel and :1399 dismiss are not monitoring teardown|
|accessibility|PARTIAL_SEAM_EXISTS|App.tsx:2135 Escape, :2144 focus heading, :2181 textarea label, :2293+ action labels; annotation/keyboard target editor acceptance absent|
|telemetry/privacy boundary|PARTIAL_SEAM_EXISTS|p3_03_runtime.rs:64 network/persistent-cache flags and App.tsx:1491 local/cloud disclosure; no adapter boundary exists|
|local-only Instant path|IMPLEMENTED_AND_USED|p3_03_runtime.rs:138 InstantSelectionEngine analyze; main.rs:1942 spawn_instant_for_capture; App.tsx:578 listener|
|optional Deep enrichment|IMPLEMENTED_AND_USED|App.tsx:430 rewrite_selected_text, :2315 Run Deep; existing autoRewrite can also trigger at :657; P3B explicit app/document permission absent|
|unsupported-app P3-A fallback|PARTIAL_SEAM_EXISTS|existing selection-scoped flow exists App.tsx:606/1286; no adapter failure→P3A routing because adapter absent|


## Independent runtime findings and flow evidence
## Additional current evidence
- Recapture cancels store, but main.rs:1860 dispatches interruption only when a Codex ActiveTurn exists. CLI has no ActiveTurn binding; CLI recapture thus invalidates stale completion without actively cancelling underlying child.
- App exit main.rs:1524-1540 awaits the same interrupt seam before Codex shutdown, inheriting F01/F02 liveness exposure.
- p3_03_runtime.rs:172-176 store_candidate activates supplied identity before storing; runtime completion main.rs:2014-2020 checks captured source/token but does not recheck terminology revision or mode after analysis. main.rs:2003 supplies constant intent revision 1. This is a P3-A lifecycle seam requiring stabilization review, not document revision support.
- local Instant engine and source-bound cache exist; identity/generation refer to captured selection, not monitored editor document. Rust ReconciliationMachine permits UserEdited Apply but is a library seam, not backend apply authorization.

## Final independent findings
| Finding | Current verdict | Evidence and limit |
|---|---|---|
| F01 | CONFIRMED_CURRENT_DEFECT | main.rs:513-523 holds providers Mutex across CLI rewrite await; :1229-1231 invalidates capture then :1412-1417 waits the same lock. manager.rs:159 clears busy before delayed interrupt can cancel; private cancel fields :23-24 have no independent command handle. Codex main.rs:475-509 uses ActiveTurn separately. capture_session.rs:219-230 and main.rs:531-545 reject stale completion. Resource cancellation/shutdown boundedness is affected; actual cancel latency NOT_RUN. |
| F02 | CONFIRMED_CURRENT_DEFECT | provider/cli.rs:160 ignores Job assignment failure, :168-178 read_to_end has no byte cap, :209-213 joins readers outside the wait timeout. A pipe-holding descendant can block completion; run_version :60-81 and run_args :84-110 have no Job/kill_on_drop. Codex strict Job assignment at codex_client.rs:371-375 differs; spawn-before-assign leaves early-descendant coverage unproven. Codex read_codex_version uses synchronous output without timeout (codex_binary.rs:41-46); Codex stdout/stderr lines (:1016/:1083) and agent delta accumulation (:879) also lack a byte ceiling. Actual orphan/OOM not reproduced. |
| F03 | PARTIALLY_CONFIRMED | Daily-use path build-daily-use-bundle.ps1:2-14 accepts executable and identity, :97-100 writes caller metadata, :108 asserts clean=true; no source-to-build receipt. Legacy build-personal-bundle.ps1:175-183 actually checks clean main, :485 measures HEAD, :543-544 builds, :628-650 records measured commit/hash. Claim is not valid for every bundler. |
| F04 | CONFIRMED_CURRENT_DEFECT | Test coverage gap: test-p3-03-instant-runtime-state.mjs:19-69 duplicates TS behavior; :78-80 checks source string names. test-r5-writing-quality.mjs:18-35/test-r6-provider-transport.mjs:16-25 static regex checks. These cannot prove command/UI/backend integration. Real Rust module tests also exist; whole suite must not be dismissed as fake. |
| F05 | CONFIRMED_CURRENT_DEFECT | App.tsx:1434-1436 requires result or activeKind=instant; editDraft changes activeKind=user (instantSelectionRuntime.ts). Backend main.rs:1338-1339 exact candidate rejects edited Instant-only result. Ready Deep draft binding remains token/intent based; recapture is rejected by token/generation. Current actual TS production-import probe passed 9 behavior assertions and reproduced the canApply AST gate (MODULE_IMPORT_RESULT.json); backend command and real GUI were not executed by that probe. Late-provider dirty-draft protection has a code seam at instantSelectionRuntime.ts:46-68; integrated real UI acceptance remains NOT_RUN. |
| F06 | CONFIRMED_CURRENT_DEFECT | apply_safety.rs:14-25 ApplyPlatform exposes no current-selection read; capture_session.rs:104-109 ApplyContext has target/clipboard but no captured source. execute_apply :109-218 validates HWND/PID/foreground/clipboard and sends paste :202; clipboard read :171 is restore data, not a selection probe. Current-selection bytes are absent from the decision. Real editor mutation negative matrix NOT_RUN. |

D020 verdict: **D020_DOCUMENTATION_OVERCLAIM**. Locally reachable Git history for apply_safety.rs/windows_apply.rs has initial 54aecf5199362023dbf69a0ac696cc415277dba2 and translation update 0d48ca6007493600c9a782fe5fe47c78e962bcac. Initial apply trait and execute_apply also have no bounded copy probe. `git log --all -G 'send_copy|revalid|bounded.*copy' -- src-tauri/src/main.rs src-tauri/src/apply_safety.rs src-tauri/src/windows_apply.rs src-tauri/src/clipboard.rs` finds only 54aecf5 and baseline 0667ede23a3248c1dda952ed936bee4ab6f66575. Current sole production call to capture_selected_text is main.rs:1887 during capture, not Apply. No locally reachable evidence of a removed implementation. This does not prove anything about unavailable external code/history.

Historical authority evidence: ignored `+Chat_Project_Source/Codex_Pencil_Project_Source_2026-08-13/07_DECISION_LOG.md` is older and contains no D020. Existing untracked `Grammar_PROJECT_CONTROL_EXPORT_20260906T134100Z.zip::PROJECT_CONTROL/01_GOALS_AND_ACCEPTANCE.md:14` claims `CaptureSession/HWND/PID/generation/source revalidation` and cites D020. Full D020 bounded probe/exact equality claim is supplied in current Owner task. Distinguish supplied claim from independently recovered original D020 text (not located).

## End-to-end backend route table
Paths below relative to src-tauri/src unless prefixed otherwise. Native delivery/Provider inference were not run by this auditor.
| Flow | Entry / owner | Revision authority | Side effects | Fail-closed / stale / cancellation | Test and unknown |
|---|---|---|---|---|---|
| Shortcut → capture | main.rs:1602-1655, :1850-1943; ShortcutTriggerGate, CaptureSessionStore | session UUID + monotonic checked generation; capture_session.rs:131-166 | shortcut registration, clipboard sentinel/Ctrl+C, widget focus | own HWND/PID rejected windows_target.rs:29-72; modifier/non-text guards clipboard.rs:24-69/:121-154; old session cancelled | p0_02 contract and ignored Windows live tests; current cross-app capture NOT_RUN |
| Instant | main.rs:1946-2024 → p3_03_runtime.rs:110-170 → engine.rs:31-139 | source SHA256, capture generation, analyzer, terminology; runtime intent revision fixed 1 | spawn_blocking local rules; event; RAM cache only | size/span/overlap/identity guards; post-work token/source check. No physical work cancellation; mode/term post-check gap | p3_03_runtime tests + tests/instant*; production UI test by frontend auditor |
| Deep selection | main.rs:370-529 → provider manager / Codex route | saved selected provider; token + BoundRewriteIntent incl terminology revision | explicit provider network/process; no fallback | disclosure per Provider :391-398; mode bound :412; selected-only manager.rs:129-134; stale finish :531-545 | manager fallback tests, Codex duplex fixture; live NOT_RUN |
| Codex Deep/cancel | codex_client.rs:568-684; main.rs:475-509 | exact threadId/turnId ActiveTurn + capture token | isolated persistent app auth home, ephemeral cwd, stdio | tool/server requests rejected; 180s turn timeout; 5s interrupt; strict Job assignment. version probe unbounded | codex_client tests include duplex and ignored live; real account unchanged |
| Claude / Antigravity | provider/claude.rs:136-189; antigravity.rs:59-88; cli.rs:136-227 | manager active/busy, private cancel Arc | official CLI argv, inherited session; temp cwd; env API keys removed | 120s/130s nominal timeout; F01 and F02 prevent bounded overall guarantee | parser/argv unit tests; no real CLI run here |
| Normalize/review | writing_contract.rs:145-207; codex_client.rs:1544-1608; provider parsers | result mode + selected provider; backend bound Ready | returns result to UI | strict structured Codex; tolerant fenced/extracted JSON for CLI; CLI defaults optional fields and omits terminology IDs. content guard applies at final Apply | Rust parser tests; UI diff independent audit |
| User edit / Apply authorize | main.rs:691-729, :1323-1405 | Ready token/bound intent or exact original Instant candidate | no write until authorization | F05 edited Instant-only rejected; Deep edited text accepts Ready binding; final size guard | no integrated Instant draft acceptance test identified |
| Apply / target restore | main.rs:767-778 → apply_safety.rs:59-218 → windows_apply.rs:133-184 | saved token + HWND/PID, current intent+terminology main.rs:664-687 | hide widget, foreground, clipboard write, SendInput Ctrl+V | failed target Copy fallback; clipboard sequence guard; incomplete input cancels; F06 missing external selection compare. Applied proves sent count, not editor content | real ApplyPlatform mock contract tests; ignored native Windows tests; current mutation confirmation NOT_RUN |
| Copy | frontend clipboard API (frontend auditor) | current displayed draft | clipboard write | does not imply external document Apply | independent frontend scope |
| Invalidation / recapture | main.rs:1034-1052/:1116-1156/:1199-1231/:1857-1866 | capture lifecycle, settings intent, terminology store revision | persisted settings/term edits, cache eviction | stale finish bound rejected; CLI recapture no ActiveTurn → no active interruption dispatch. cancel command lacks explicit Instant cache invalidation, though cancelled capture blocks promotion | existing stale token tests; integrated concurrency unknown |
| Shutdown | main.rs:1524-1540 → interrupt → codex.shutdown | shutdown_started atomic; owned process Job | process stop, temp cleanup, app exit | F01/F02 affect progress before shutdown. Codex watcher :1090-1128 bounded wait and Job close | process_job test proves synthetic subtree helper only, not all manager exit paths |
| Terminology | service/store/matcher/import_export; main.rs:827-1053 | store revision, active profile, import base_revision/expiry | local approved dictionary persistence/export; bounded matched subset to cloud | clone→mutate→persist commit (service.rs:410-437); store rollback; 2MiB import, 64 profiles/10k entries; 50 matched/16KiB request | p1_02 contract tests failure injection; no user store accessed |
| Bundle | scripts/build-daily-use-bundle.ps1 / build-personal-bundle.ps1 | two distinct policies (F03) | external package files, executable hashes | daily verifier integrity only; legacy exact historical gate | integrator scripts execution; existing artifact source identity not inferred |

## Test class interpretation / detection matrix
- Static string contracts: R5/R6; cannot detect runtime F01/F02/F05/F06. They guard symbols/config presence.
- Copied behavior fixture: test-p3-03-instant-runtime-state.mjs; cannot demonstrate production import behavior. Current direct TS import and App AST gate probe cover F05 frontend state; full GUI/backend acceptance remains NOT_RUN.
- Actual module Rust unit/contract: p0_02/p1_01/p1_02/p1_03_contract_tests, codex_client duplex, instant library tests. Existing p0_02 tests verify missing HWND/PID/focus/clipboard/partial input, not selection equality absent from trait.
- Native synthetic: process_job terminating_job_releases_descendant_runtime_directory tests helper successful assignment; does not exercise ignored assign error or reader join gap. Need synthetic inherited pipe, byte flood, assign failure and overall deadline for F02; actual manager/IPC lock + slow child for F01.
- Ignored Windows editor tests: p*_windows_live_tests; only explicit successful execution yields LIVE_PROVEN. Native mutation must compare actual synthetic editor content and paste count for F06, not SendInput count alone.
- Ignored Provider tests: codex_client.rs:1972/:2020, provider/manager.rs:403, antigravity.rs:262. Current run status supplied by integrator, not historical PASS inheritance.
- Artifact verification: daily vs legacy personal contracts; hash integrity alone cannot establish source-to-binary provenance F03. Require measured build receipt and adversarial false-metadata test.

No runtime remediation implemented. Exact next task must be P3-A stabilization because F01/F02/F05/F06 remain confirmed. P3-B implementation remains absent.

## Rust scope / read boundary
`runtime-audit.json` contains all 51 tracked Rust paths, line counts, SHA256, production/test category and read mode. Every production module received API/dependency/side-effect review; safety-critical execution bodies received direct reading. Large main/Codex and terminology/settings/rules modules used targeted bodies plus full symbol/guard inventory; repetitive test/validation/rule bodies were sampled, not falsely claimed line-by-line read. Eight dedicated historical contract/live test files were inventoried and relevant assertions sampled. No generated Rust production body exists under src-tauri/src; generated protocol assets are a separate inventory handled by integrator. No synthetic harness was created: native cargo/helper execution is integrator-owned and avoids target contention.

Additional caveat: diagnostics.rs:139 emits owned_residual_process_count as literal 0; it is not measured residual process proof. Code-derived additional lifecycle/stream issues above are stabilization requirements, not invented observed incidents.



## Local authority drift disposition
Old export names R6 5d9db77 and P1_03_ONLY; local main already includes R8 1528dc3/9f54ebd. Old baseline acceptance is historical, not current product acceptance. P3-B direction is newly authorized, implementation is not. Old root audit remains historical untracked evidence. Current semantic authority is control/state.json + docs/control; old docs describe scoped contracts/history, not a competing current-state hub.

## Coverage boundary
Rust production path coverage and call-flow review covers all modules, with deep safety paths and sampled repetitive validation/rules/tests. Frontend: 10 small TS files fully read; App functions/state/events/IPC through line1440 and all JSX action symbols reviewed, rendering bodies/CSS sampled. Full file inventory is not a false claim of line-by-line reading generated or repetitive bodies.

## Evidence location
Run evidence root: D:/dev/grammar-evidence/GRAMMAR-P3B-R0-20260918T084111631Z. Files: ENTRY_BASELINE.json, PROTECTED_BASELINE.json, PATH_CATEGORIES.json, BRANCH_INVENTORY.json, WORKTREE_INVENTORY.json, RUST_RESULTS.json, FRONTEND_RESULTS.json, MODULE_IMPORT_RESULT.json, NATIVE_RELEASE_RESULT.json, TAURI_EOL_RECEIPT.json, OWNED_PROCESS_SNAPSHOT.json, NATIVE_STORE_RESULT.json, native-store-smoke.log, runtime-audit.json, product-plan.md. Paths are audit-run receipts, not assumptions for future agent context; current Repository and result packets are portable authority.
