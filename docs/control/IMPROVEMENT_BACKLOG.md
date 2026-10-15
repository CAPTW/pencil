# Pencil — Product quality and release readiness

Track: `PRODUCT_QUALITY_AND_RELEASE_READINESS` · Owner request: 2026-10-15.
This durable parallel track preserves the feature mission, Gemma quality baseline,
1s recommendation / 0.5s Accept / 3s paragraph targets and explicit mutation/privacy
contracts. No recurring automation or new external authority is created.

Statuses: TODO, IN_PROGRESS, LOCAL_VERIFIED, HOSTED_VERIFIED, OWNER_DECISION, DEFERRED.
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
| Q04 | P3 / DEFERRED | D-drive cold startup + cleanup acknowledgment | Reproduce slow file verification with bounded IO telemetry; retain integrity/locking; cancellation reports actual cleanup. Cold and warm results reported separately. No timeout-only or cache-only claimed fix. |
| Q05 | P0 / TODO | Intermittent desktop Accept input delivery | Capture owned input/geometry evidence; distinguish missing click from rejected mutation; reproduce cause and exact reread regression with zero wrong edits. Recent warm14 pass does not close prior failure. |
| Q06 | P1 / TODO | User-facing Pencil naming | Inventory visible strings; unify UI/docs. Separately map old app IDs, profile/install paths, host names and settings; migration/rollback tests preserve existing installs before any identifier change. |
| Q07 | P1 / TODO | SECURITY.md and reporting | Document safe private reporting route supported by actual repository configuration, supported versions and response expectations; never solicit secrets in public issues. Do not invent monitored email/SLA. |
| Q08 | P1 / TODO | CHANGELOG and current docs | Link prototype changes, limitations and install/remove instructions to source/release; remove obsolete public claims and needless local paths without destroying historical provenance. |
| Q09 | P1 / OWNER_DECISION | License, depends on third-party inventory | Inventory source/model/runtime/dependency terms and redistribution obligations; present choices. Owner selects project license; no blanket rights claim. |
| Q10 | P2 / TODO | Manual text → analysis → Copy | Design explicit non-prompt text entry as unsupported-editor fallback, preserve selection flow; no fabricated capture/direct Apply authority; privacy, draft/cancel tests. |
| Q11 | P0 / IN_PROGRESS | Word/HWP/HWPX/Excel/PowerPoint/LibreOffice adapters | Reuse qualified adapter cores and synthetic fixtures; application/version/range-specific mutation + reread + Undo/stale tests. Never activate general support from a file extension or code-only check. |
| Q12 | P1 / TODO | Provider/API qualification procedure | Synthetic transport/consent/revoke/cancel tests and opt-in live checklist with one selected provider, redacted evidence, budgets, no replay. Account/paid calls require separate authorization. |
| Q13 | P1 / TODO | Ollama/LM Studio feasibility | Compare protocol/model capability and local/cloud UX; threat-review DNS/proxy/redirect, peer/auth, loopback and consent boundaries before HTTP exception. No security relaxation as connectivity fix. |
| Q14 | P3 / TODO | MSI/NSIS packaging, signing, updates/removal | Evaluate reproducible packaging, settings-preserving upgrade/rollback/uninstall with owned data; signing identity/distribution authorization separate. Keep working PowerShell path until replacement qualified. |
| Q15 | P3 / TODO | Incremental responsibility separation | Extract tested seams from App.tsx/main.rs/codex_client.rs one at a time; preserve API and behavior, avoid broad rewrite. |
| Q16 | P3 / TODO | Protocol/tool/evidence/runtime boundaries | Document generation ownership and consistency checks; exclude private evidence/tooling from user package, preserve source-to-artifact traceability. |

## Historical initial slice
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
launcher31: 30PASS/1FAIL, ready observation unresolved). No cold-start or click-loss repair claimed from these results.


Follow-up verification correction: commit05ef854 incorrectly stated launcher31 PASS.
The actual r2 result was30PASS/1FAIL (ready observation); r3 was30PASS/1FAIL
(unconfirmed-cleanup scenario ended with broker-null startup disposition). Both
failures remain recorded in quality-launcher-r2/r3-20261015.log. Focused retries
passed2/2 but do not erase these failures or establish their cause. Added last-phase
and stdout failure diagnostics without changing deadlines/assertions. Hosted final
result must be read for the exact candidate commit; no local full31 PASS claimed.


CI/local launcher flake root cause reproduced in quality-pre-ready-repeat-20261015.log:
Python pathlib.read_text held a Windows handle without FILE_SHARE_DELETE while
Publish-Session attempted atomic Move; original error was Access denied, before
broker creation. Thus retryTicket absence and broker-null disposition were caused
by the observer itself. Test readers now use explicit READ|WRITE|DELETE sharing,
matching production local_model_session.rs File::open and installed Rust API docs.
Added deterministic open-reader/atomic-replace regression. No launcher retry,
permission, timeout, phase assertion or model integrity rule weakened. Historical
failed runs remain FAIL; this correction does not explain original native click loss
or HDD startup latency. Candidate CI must verify this new fixture version.


Further deterministic reproduction corrected the initial fixture-only attribution:
even a production-equivalent shared-delete reader blocks File.Move(overwrite).
ReplaceFile succeeds with that reader retained. Production regression now holds
both session and host-config readers while the real launcher publishes ready;
old launcher fails (quality-status-replace-red-20261015.log). Launcher now uses
File.Replace for existing files, Move only for first session creation; no retry loop
or deadline increase. Test observers retain Rust-equivalent sharing. Initial .NET
PowerShell null binding produced an empty backup path; corrected to NullString.Value.
That intermediate test run is failed, not final validation. This is a startup status
publication race fix, not proof that cold disk latency or native clicks are repaired.

## Installed CI candidate checkpoint (2026-10-15)
Q01/Q02/Q17 hosted snapshot b145ddd: run38728375665 SUCCESS. Installed that source
without model/settings changes. Q04/Q05 remain open: final actual Gemma owned
trial-r3 passed14/14 with natural cleanup/remaining0; ready24.97s with uncontrolled
cache, recommendation555-1225ms, observed Accept click-to-reread77-123ms. Quality
12 golden +2 previously reviewed equivalents. Earlier occlusion/interruption/stale
status failures preserved. Fixture focus repair is not a product input-loss fix.
Next: isolate cold verification/init costs and reproduce original input loss.
See HANDOFF installed checkpoint and personal-b145ddd-20261015/FINAL_RECEIPT.json.

## Phase diagnosis and Chromium requalification (2026-10-15)
Q04/Q05 remain open. Installed broker phase probe measured warm/uncontrolled8.72s:
model pin2.74s, engine health/load4.96s, explicit warmup0.081s. No model-quality,
integrity or deadline change. Inactive-review hypothesis did not reproduce loss:
7 native mouse +7 CDP Accept/reread PASS with cleanup0. New installed extension
fresh-profile real Gemma translation/rewrite14 PASS, cleanup0; see latest HANDOFF.
Fixture path/preparation failures preserved, not product regression or successes.
Next: first-cold phase evidence and original missing-input reproduction.


2026-10-15 Q04/Q05 reproduction checkpoint: cold first-file verification timed out
before GPU initialization (read175s vs hash1.29s). Sequential hint and8MiB reads
failed qualification; no product optimization adopted. Q04 remains open. Q05 has
a controlled test-input cursor race reproduction and a fail-closed helper repair;
42 actual Gemma cases were40PASS/2safe input refusals, not a full PASS. Historical
product loss remains open pending isolated reproduction. See HANDOFF and
cold-input-cause-20261015 evidence; do not conflate fixture repair with product fix.


## Owner sequencing override — 2026-10-15
Current execution priority: (1) document/editor integration, (2) Provider/API
connectivity and stability, (3) startup/performance optimization. The Owner
explicitly deferred D-drive first-start latency and will keep other sessions
running. Do not stop unrelated processes or require them to pause. Q04 is deferred,
not repaired; preserve all timeout failures and existing latency/quality targets.
Q05 remains a correctness regression within each editor qualification, not a
reason to keep broad editor implementation behind repeated performance trials.

Next concrete slice: requalify the existing Writer adapter with the current
Gemma-enabled product, paired native DLL/OXT and owned saved DOCX. Verify selection,
translation/rewrite, edited Accept, complete reread, formatting/Undo, changed-source
refusal, Disable and cleanup. Prior runtime0fc8624 evidence is not current Gemma
qualification. Then complete rHWP trusted admission/local transport and HWP/HWPX
installed flow; progress Calc/XLSX and Impress/PPTX adapters, with actual Microsoft
Office/Hancom application qualification tracked separately. Reuse existing cores.
Manual text -> analyze -> Copy is a useful unsupported-editor fallback, never a
claim of direct Apply or a replacement for the requested adapters.

All-document operation is the product objective, not current support. Activate
only verified application/version/range combinations; file suffixes do not grant
mutation authority. Keep explicit Accept, exact source/revision, user edit/format
preservation, sensitive-field refusal, single owned test session and cleanup.
Provider work follows adapter connectivity: one selected provider, consent,
revocation/cancel, no replay, explicit successful-response and failure validation.
This reprioritization grants no new accounts/costs, security exceptions or release.

2026-10-15 Q11 progress: current Gemma + paired Writer candidate passes scoped saved DOCX translation/rewrite/Accept/format/Undo and stale/Disable checks (9 widget +5 enclosing). General activation and broad DOCX/Office remain open; next reproducible opt-in setup and expanded document coverage/rHWP transport. See HANDOFF and writer-gemma-ssd-20261015 evidence. Q04 remains deferred.

2026-10-15 Q11: reproducible Writer package preflight implemented and actual package
regeneration verified, with disabled-personal-build refusal. No new GUI coverage.
Next: current-Gemma multi-paragraph DOCX qualification; general activation pending.


2026-10-15 current checkpoint: 2026-10-15 Current-Gemma four-paragraph Writer DOCX: paired e6ff394 native/app and paragraph-enabled OXT passed13 widget+5 enclosing checks. Selected range within paragraph2; differently colored/aligned surrounding paragraphs and empty paragraph preserved by whole-body reread and per-portion checks. Actual Gemma EN-KO/natural/polite and four-sentence translation/rewrite, edited Accept, Undo, source-change/Disable refusal. Final recommendation831-1205ms (short1s misses2/3, paragraph3s misses0/2); Accept272-294ms (0/5 misses). No autoSave. Final trial PASS, app/Office natural0, Job remaining0 before/after, owned uninstall complete. Earlier missing-profile and quit-reply failures preserved; fixture quit request now tolerates only closed transport and still requires natural exit0. No product-core change or personal-install activation. Cross-paragraph selection/current-Gemma topology, complex docs, Word/IME/live Provider unqualified. Evidence writer-gemma-paragraphs-20261015/FINAL_RECEIPT.json.


2026-10-15 checkpoint: 2026-10-15 Actual Gemma Writer cross-paragraph DOCX: e6ff394 binaries plus package-verified span-enabled OXT, selection from inside paragraph1 to inside paragraph4 including empty paragraph3. Final10 widget+5 enclosing checks PASS: EN-KO/natural/polite Accept and Undo, full text/portion bold/RGB/alignment preservation, edited span, removed-boundary refusal unchanged, source-change and Disable. Two sequential trials PASS with natural app/Office exit and supervisor0 before/after; owned uninstall complete. Final recommendation1098-1471ms and Accept455-958ms; across6 samples Accept500ms missed5, short1s missed6, no result over3s. Latency unresolved, no relaxed targets. No production defect found or core change. Personal install unchanged; general DOCX/Word/current-Gemma merge-split unqualified. Evidence writer-gemma-span-20261015/FINAL_RECEIPT.json.
