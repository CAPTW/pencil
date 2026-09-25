> Active mission: see MULTI_AGENT_CONTRACT.md v1.1.0 override and control/state.json. Historical no-execution/one-step approval restrictions below are superseded only inside this mission; acceptance claims are not promoted.

# Acceptance and test matrix

Evidence labels: CODE_PROVEN, TEST_PROVEN, LIVE_PROVEN, FIXTURE_PROVEN, DOCUMENT_REPORTED, INFERRED, NOT_RUN, BLOCKED, UNKNOWN. Static strings, copied functions, real module tests, native fixtures and live product tests are distinct. Read CURRENT_AUDIT.md for the immutable product-base identity and run receipts. Current identity comes from Git, exported once in Source 02.

| Class / exact command or entry | Current evidence | Detects / does not prove |
|---|---|---|
| `npm ci --offline --ignore-scripts --no-audit --no-fund` | TEST_PROVEN exit0, lock versions unchanged | Restore only, no actual Provider |
| `npm run typecheck`; `npm run build:frontend` | TEST_PROVEN exit0 using actual Vite config | TS/build only, no UI Apply acceptance |
| 13 existing Node core/provider/UI contracts | TEST_PROVEN exit0 | Mixed static/behavior, not end-to-end cancel/Apply |
| engine contract with P3_02_NATIVE_SELF_TEST_ROOT and P3_02_UTF8_JSON_SELF_TEST_ROOT | FIXTURE_PROVEN 468/0 | Correct prerequisite runner tests, not live language quality |
| historical benchmark contract | FAIL: 107 assertions/16 failures | Fixed file-set + LF/hash assumptions drifted; not 16 current engine defects |
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
