# Pencil — Product quality and release readiness

Track: `PRODUCT_QUALITY_AND_RELEASE_READINESS` · Owner request: 2026-10-15.
This durable parallel track preserves the feature mission, Gemma quality baseline,
1s recommendation / 0.5s Accept / 3s paragraph targets and explicit mutation/privacy
contracts. No recurring automation or new external authority is created.

Statuses: TODO, IN_PROGRESS, LOCAL_VERIFIED, HOSTED_VERIFIED, OWNER_DECISION.
Completion evidence must distinguish implementation, synthetic/local tests, actual
user qualification and product acceptance. A local workflow edit is not hosted CI.
At each major feature/release checkpoint, update priority, dependencies, evidence,
remaining defects and the next executable item here. Do not rewrite historical FAILs.

| ID | Priority / status | Improvement and dependency | Completion criteria |
|---|---|---|---|
| Q01 | P0 / LOCAL_VERIFIED | Main + PR Windows core CI; no model/accounts | Push main and PR-to-main run read-only, unprivileged checks; Rust nonignored regressions, Node runtime/Chromium contracts and TypeScript build; hosted run and failure propagation verified separately. |
| Q02 | P0 / LOCAL_VERIFIED | Obsolete contracts, starts with P3_02_MAIN_GUARD | Reproduce old failure; preserve off-UI/runtime dispatch boundary while allowing mode types; negative guard examples and production runtime tests pass. Audit remaining legacy scripts without deleting real failures. |
| Q17 | P0 / LOCAL_VERIFIED | Intermittent synthetic startup cleanup test, surfaced by Q01 | Preserved full-suite FAILs and isolated/suite passes. Diagnostic identified fixture panic on socket reset (10054); handle abandoned headers/body per connection, keep startup/cleanup assertions. New regression fails old fixture and passes repair; full nonignored Rust suite passed after repair; hosted validation remains pending. |
| Q03 | P0 / OWNER_DECISION | Branch protection, depends on stable Q01 check name | Read current protection/rulesets if accessible; recommend required `source` check and no force/deletion bypass. Owner authorizes changes; enforce only after hosted green and deliberate failed-check validation. |
| Q04 | P0 / TODO | D-drive cold startup + cleanup acknowledgment | Reproduce slow file verification with bounded IO telemetry; retain integrity/locking; cancellation reports actual cleanup. Cold and warm results reported separately. No timeout-only or cache-only claimed fix. |
| Q05 | P0 / TODO | Intermittent desktop Accept input delivery | Capture owned input/geometry evidence; distinguish missing click from rejected mutation; reproduce cause and exact reread regression with zero wrong edits. Recent warm14 pass does not close prior failure. |
| Q06 | P1 / TODO | User-facing Pencil naming | Inventory visible strings; unify UI/docs. Separately map old app IDs, profile/install paths, host names and settings; migration/rollback tests preserve existing installs before any identifier change. |
| Q07 | P1 / TODO | SECURITY.md and reporting | Document safe private reporting route supported by actual repository configuration, supported versions and response expectations; never solicit secrets in public issues. Do not invent monitored email/SLA. |
| Q08 | P1 / TODO | CHANGELOG and current docs | Link prototype changes, limitations and install/remove instructions to source/release; remove obsolete public claims and needless local paths without destroying historical provenance. |
| Q09 | P1 / OWNER_DECISION | License, depends on third-party inventory | Inventory source/model/runtime/dependency terms and redistribution obligations; present choices. Owner selects project license; no blanket rights claim. |
| Q10 | P2 / TODO | Manual text → analysis → Copy | Design explicit non-prompt text entry as unsupported-editor fallback, preserve selection flow; no fabricated capture/direct Apply authority; privacy, draft/cancel tests. |
| Q11 | P2 / TODO | Word/HWP/HWPX/Excel/PowerPoint/LibreOffice adapters | Reuse qualified adapter cores and synthetic fixtures; application/version/range-specific mutation + reread + Undo/stale tests. Never activate general support from a file extension or code-only check. |
| Q12 | P2 / TODO | Provider/API qualification procedure | Synthetic transport/consent/revoke/cancel tests and opt-in live checklist with one selected provider, redacted evidence, budgets, no replay. Account/paid calls require separate authorization. |
| Q13 | P2 / TODO | Ollama/LM Studio feasibility | Compare protocol/model capability and local/cloud UX; threat-review DNS/proxy/redirect, peer/auth, loopback and consent boundaries before HTTP exception. No security relaxation as connectivity fix. |
| Q14 | P3 / TODO | MSI/NSIS packaging, signing, updates/removal | Evaluate reproducible packaging, settings-preserving upgrade/rollback/uninstall with owned data; signing identity/distribution authorization separate. Keep working PowerShell path until replacement qualified. |
| Q15 | P3 / TODO | Incremental responsibility separation | Extract tested seams from App.tsx/main.rs/codex_client.rs one at a time; preserve API and behavior, avoid broad rewrite. |
| Q16 | P3 / TODO | Protocol/tool/evidence/runtime boundaries | Document generation ownership and consistency checks; exclude private evidence/tooling from user package, preserve source-to-artifact traceability. |

## Current slice
Q01/Q02 are parallel to ongoing Q04/Q05 and adapter development. Existing deep
Windows mission workflow remains on its historical branches; core source CI gets
main/PR coverage without silently enabling interactive/native/provider tests.
Fork PRs use `pull_request`, not `pull_request_target`; read-only token, checkout
credentials disabled, no secrets, model downloads, uploads or deployment.

Next executable work: obtain hosted Q01 validation when source publication is authorized;
Q01/Q02/Q17 are locally verified, not hosted-qualified; Q07 security reporting documentation and Q04 startup diagnosis remain independently actionable. Hosted CI and
branch-rule enforcement remain separate evidence/authorization steps. No main
integration, push, release or installation is performed by this local slice.

## Repository settings observation (2026-10-15)
GitHub repository API reports visibility `private`, default branch `main`.
Both protection and rulesets reads returned HTTP403 with plan/public-repository
availability message. This is not proof that no rules exist. No visibility,
account plan, protection or ruleset was changed. Q03 is blocked on platform/access
availability and Owner decision; do not make the repository public as a workaround.

## Q01/Q02 implementation checkpoint
- Q01 workflow now admits main pushes and PRs targeting main, includes TS build,
  Node provider/runtime/capture/Chromium suites and serial Rust nonignored targets.
  `--no-fail-fast` continues other Rust targets while retaining a failed exit code.
  Existing workflow/check identity is retained. Timeout moves from15 to45 minutes
  because full Windows Rust compilation was added (same budget as mission CI), not
  to excuse a measured test deadline. Individual model/runtime deadlines unchanged.
- Q02 old failure reproduced: `FAIL 1 24 P3_02_MAIN_GUARD`. Main only referenced
  engine AnalysisMode variants; legitimate Gemma mode dispatch was forbidden.
  New guard permits mode values, refuses engine imports/direct calls, requires
  dispatch through runtime, with negative examples. Production reducer/Rust tests
  remain required; this source guard alone is not behavior or security proof.
- Hosted workflow execution and a hosted deliberate-failure check remain NOT_RUN.
  The current candidate is local; repository settings and package are unchanged.

Final local check: Rust targets147/282/28/2/14 passed (ignored21/32/1/1/1),
no-fail-fast command exit0; no skipped live test counted as pass. Provider/runtime,
capture, Chromium core227 assertions/worker77/scheduling6, TS/frontend build,
startup disconnect red→green, governance negative checks and YAML semantic checks
passed. Compiler warnings remain; this is not lint-clean or hosted/native acceptance.
Q07 reporting procedure is next independent documentation improvement; Q04/Q05
remain the parallel feature reliability priorities.

## Candidate CI and startup recovery follow-up (2026-10-15)
Owner authorized candidate push and hosted Windows validation. 942043b pushed to
codex/grammar-local-trial-ba58b7d; run38723527175 records that exact snapshot.
Remote main advanced only in README docs (df48758/31a927a); not merged or overwritten.

Q04 partial repair: broker now reports `startup:not_started` for artifact validation
failure before engine creation, after dropping its file pins. Launcher grants only
an explicit generation-bound retry after broker exit and diagnostic drain; it does
not claim runtime cleanup. Existing crash/unconfirmed cleanup remains nonretryable.
Production broker+launcher tests2 and launcher regressions30 PASS; old-launcher
regression fails for absent retry ticket. Startup timeout/model integrity/quality
unchanged. Private installation unchanged; no actual cold HDD speed improvement
or installed retry acceptance claimed. This is independent of Q17 fixture reset fix.

Q05 diagnostic repeat: installed792032b, one owned session, real D-drive Gemma,
14/14 desktop capture/rewrite/translate/Accept+reread PASS; all input clicks observed,
ready30.05s, supervisor PASS/COMPLETE, zero forced/remaining. Failure did not recur;
its original cause is still unknown. No cache flush/pre-read used, but cache state
uncontrolled, so this is not proof of cold performance. Evidence quality-native-20261015.
Final hosted outcome must be tied to the actual follow-up commit, not inherited from
942043b. No main adoption, release or private-install upgrade by this follow-up.

Hosted Q01/Q02/Q17 snapshot942043b: Windows run38723527175 SUCCESS. PR event itself
still NOT_RUN (only push exercised); branch protection unchanged. Final follow-up
also corrects historical-ready → no-runtime-retry exit classification (red reproduced,
launcher31 PASS). No cold-start or click-loss repair claimed from these results.
