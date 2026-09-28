> Active mission: see MULTI_AGENT_CONTRACT.md v1.1.0 override and control/state.json. Historical no-execution/one-step approval restrictions below are superseded only inside this mission; acceptance claims are not promoted.

# Acceptance and test matrix

Evidence labels: CODE_PROVEN, TEST_PROVEN, LIVE_PROVEN, FIXTURE_PROVEN, DOCUMENT_REPORTED, INFERRED, NOT_RUN, BLOCKED, UNKNOWN. Static strings, copied functions, real module tests, native fixtures and live product tests are distinct. Read CURRENT_AUDIT.md for the immutable product-base identity and run receipts. Current identity comes from Git, exported once in Source 02.

| Class / exact command or entry | Current evidence | Detects / does not prove |
|---|---|---|
| `npm ci --offline --ignore-scripts --no-audit --no-fund` | TEST_PROVEN exit0, lock versions unchanged | Restore only, no actual Provider |
| `npm run typecheck`; `npm run build:frontend` | TEST_PROVEN exit0 using actual Vite config | TS/build only, no UI Apply acceptance |
| 13 existing Node core/provider/UI contracts | TEST_PROVEN exit0 | Mixed static/behavior, not end-to-end cancel/Apply |
| engine contract with P3_02_NATIVE_SELF_TEST_ROOT and P3_02_UTF8_JSON_SELF_TEST_ROOT | Windows CI step "Instant engine and benchmark contracts" from an LF worktree: exit 0 (GREEN requires no failure and no NOT_RUN). Without Windows PowerShell 5.1 and the roots: NOT_RUN_PREREQUISITES, exit 2, never GREEN (Linux 452 pass, 17 NOT_RUN) | Correct prerequisite runner tests, not live language quality. MAIN_RUNTIME_SOURCE_CHANGED retired as stale policy (HANDOFF pre-use round) |
| benchmark contract | TEST_PROVEN 136/136 GREEN (same CI step) | The cdf8553 failure was the P3-01 closed file set predating P3-02's two files (stale policy); the measurement never failed |
| personal/daily bundle contracts | TEST_PROVEN 80/0 and GREEN | Legacy static contract + shortcut fixture; source-to-exe provenance still separately audited |
| actual TS import and actual App canApply AST | FIXTURE_PROVEN 9 assertions; Instant edit true→false gate | F05 frontend condition; not native Apply |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked --offline` | TEST_PROVEN exit0 | Current Rust compiles |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | TEST_PROVEN 202 pass, 0 fail, 14 ignored | lib3/bin185/integration14; ignored Provider/editor tests not passed |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline` | TEST_PROVEN exit0, bin26 warnings | Lint command pass, not zero warnings |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | FAIL: pre-existing format drift | No auto-format authorized |
| Tauri `build --no-bundle --no-sign -- --locked --offline` | TEST_PROVEN native release receipt and hash | Build only, not installer/publication or product acceptance |
| ProcessJob native unit fixture | FIXTURE_PROVEN | Owned successful-job cleanup path only; F02 assign failure/pipe holder unproven |
| explicit ignored native local-store test | FIXTURE_PROVEN native Windows store test 1 pass, 0 fail; NATIVE_STORE_RESULT.json | Native fixture; no Provider/editor/clipboard interaction |
| Provider runtime | Codex NOT_RUN; Claude NOT_RUN; Antigravity NOT_RUN | Existing auth availability and safe isolated live inference not established; no credentials/login mutations |
| Windows editor/hotkey/clipboard live suite | NOT_RUN | Exclusive safe desktop/clipboard ownership not established; no claim from input count |

## R0 governance acceptance versus product baseline

R0 must preserve every existing product/config/dependency/protocol/packaging script blob. New governance/schema/wrapper/source validators, meaningful negative tests, actual module probe, typecheck/build and relevant retained contracts must pass. Pre-existing formatter and historical immutable benchmark contract failures are baseline product/tooling limitations, recorded visibly; no code change is made to turn them green. They are not silently counted as passing tests.

Fast-forward requires independent review to decide whether these baseline failures are unrelated to the docs/governance diff. If a failed check is relevant to changed behavior, any new validator fails, product diff is nonzero, or main advances, adoption is blocked. The reviewer must explicitly record the relevance decision; author self-approval is insufficient. All required product acceptance gates in roadmap remain unexecuted until their own authorized task. This interpretation does not waive current product acceptance failures.

## Defect-to-test requirements for next authorized stabilization

| Defect | Required new evidence (future, not run by R0) |
|---|---|
| F01 | Real manager/IPC holding a slow synthetic child; Cancel and shutdown deadline; stale completion unable to overwrite new capture |
| F02 | Job assign failure, early descendant, inherited pipe after parent exit, byte flood, version/auth probe timeout, measured owned PID subtree cleanup |
| F03 | Daily-bundle false metadata/exe mismatch negative case + measured receipt. Legacy personal bundler is a different path |
| F04 | Import actual production functions; mutation of production behavior must fail test; avoid duplicated reducers |
| F05 | Instant-only edit→review→explicit Apply; late Deep preserved dirty draft; backend draft/capture/intent/revision binding |
| F06 | Synthetic same-HWND selection/position/source change, no selection, copy timeout; paste0 or Copy-only, successful mutation reread |

## P3-B acceptance invariants

Document epoch/revision and unicode ranges must match before mutation; stale/ambiguous range fails closed. Opt-in/denylist/sensitive-field checks occur before reading; visible state, pause and emergency disable precede monitoring. Hover/click cached surface emits inference/network count0. Local analysis is bounded and memory-only. Cloud requires separate consent and one selected Provider. Accept/Edit/Apply is explicit, mutation is reread, unsupported states preserve P3-A without reading secure fields. Each ROADMAP step inherits these checks and declares its own unit/integration/native/privacy gate.

No threshold here is claimed measured for P3-B. Budgets and editor support must be frozen and verified in their bounded design/implementation steps. P3-B implementation and product acceptance are not R0 outcomes.

## Autonomous mission candidate checkpoint (2026-09-25)

Runtime/draft regressions and isolated Chromium local-Instant mutation rereads have synthetic evidence in HANDOFF.md. Native capture is activation-withheld and native Apply is Copy-only. The resumed candidate implements consent-bound Deep through a shared executor and concurrent cancellation host; synthetic runtime/host/browser checks pass, with live Providers NOT_RUN. Therefore A/B/C as a whole are NOT_ACCEPTED. Actual build receipts and Source package verification are separate evidence, not upgrades of these product verdicts. Historical audit/FAIL evidence is immutable.


## Bounded cleanup receipt candidate (2026-09-26)

Worker44 deterministic tests and native host16 tests pass for exact installation/token/generation receipt ownership, bounded four-slot storage, no-history persistence, cancellation races and restart reconciliation. Actual synthetic host4/4 verifies fresh-process query/ack after checked cleanup and rejects PENDING/wrong tickets. Unknown states remain blocked. These results do not qualify power loss, live Providers, native capture/clipboard, or whole-product A/B/C acceptance. Current browser and source/artifact receipts are in the resume-r3 external evidence directory referenced by HANDOFF.md.

Actual isolated Chromium24 local checks also pass for restart reconciliation, both supported editors' protocol composition, delayed Instant response and explicit mutation rereads. A separate29-check partial run reached both editors' synthetic Deep success before a later fixture pointer interception; it is preserved as FAIL, not a complete Deep/browser PASS. The production activeTab protocol action was denied by Chromium and remains NOT_RUN. Read final extracted-package receipts independently.

## Pre-use inspection round (2026-09-28)

Findings, reproductions, fixes and residual limits are in HANDOFF.md ("Pre-use inspection round"). Evidence classes:

- New Rust unit tests: TEST_PROVEN.
  - Capture attempts, Cancel, busy reservation, open-capture Instant, batch launchers, Claude sign-in wording and the Codex per-turn output cap.
  - Run under Wine locally and in the Windows CI job.
- Built-app flow: FIXTURE_PROVEN on the owned synthetic desktop through the synthetic executable. It covers:
  - Instant visible during Deep, Cancel, capture-ended and a held clipboard;
  - Unicode/CRLF, password and empty selection;
  - shortcut spam and restart.
  - Not a live Provider.
- Chromium: FIXTURE_PROVEN.
  - The modified isolated extension runs in CI, and locally with the real host and synthetic child under Wine. It covers field choice incl. sensitive refusal, card identity, the Deep reply held while a card is open, and repeated Accepts.
  - The unmodified installed package is covered only on the keyboard path (R3).
- Install and uninstall failure paths: FIXTURE_PROVEN with task-owned roots, registry tree, stand-in processes and user-data folder.
- No automated test: W2, C2, C6, C12, C13, C14, D9, I8 and P4 (code review only).
- NOT_RUN: physical keyboard/IME, live Providers, normal profiles and real documents, the mouse toolbar path, Edge.
