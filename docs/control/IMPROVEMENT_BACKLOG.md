# Pencil — Product quality and release readiness

Track: `PRODUCT_QUALITY_AND_RELEASE_READINESS` · Owner request: 2026-10-15.
Q19 actual release-browser fixture:6 latest-source quantity/negation/past-date
cases across textarea/contenteditable passed, recommendation805-827ms and
Accept33-37ms. Cache lookup/Accept caused no inference; natural cleanup0.
Small modified-extension fixture only; installed activeTab/IME/long-window tails
remain pending. Debug timing FAIL and build-supervisor failures remain recorded.
Q19 checkpoint 2026-10-20: backend warm supersession verified with actual Gemma
three times, same PID and next request300ms; idle confirmation2.80-2.89s from
request start remains above target. Chromium/native/pipe supersession is now wired
and checked with production actor, Windows pipes and real Gemma service calls.
Installed typing-to-publication/Accept qualification remains pending. Actual pipe
request-to-cancel-ack2898-2934ms still exceeds the proactive1s ceiling; do not
lengthen that ceiling or claim a latency improvement. See HANDOFF for strict supervisor failures.
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
| Q18 | P2 / IN_PROGRESS | Korean rewrite register and wider entity/role coverage; Q11 local Gemma corpus | Preserve 해 주세요 register in natural rewrites without forced paraphrases or weaker meaning/style guards. Add held-out names/compound roles and verify installed HWP/HWPX Accept, full formatting, Undo and reopen. Current three-template synthetic qualification is not general semantic or Hancom acceptance. |
| Q19 | P0 / IN_PROGRESS | Incremental proactive Gemma: current Chromium document core, native local lane and cache | Coalesce unpublished changed lines, cancel pre-admission work, discard stale results, display valid cache without inference. Keep one in-flight/latest pending request. Active token-generation interruption must preserve the warm model and prove idle; current safe drain is not physical cancellation. Actual typing-to-suggestion/Accept and other adapters need separate qualification. |

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


## Korean compound/name checkpoint — 2026-10-18
Q11: the same-clause quoted label/action ambiguity was reproduced with an actual
Gemma response and a red production-alignment regression. Quoted content is now
excluded only after validating the unique nonliteral source occurrence; decoder
quote overlap and exact target mutation fences remain. Q18 records observed
natural rewrite `해 주세요` -> `해 주시기 바랍니다`: meaning and action emphasis
survive, but exact register preservation is not proved. ASCII identifier checks
are conservative ordered-token checks, not general proper-name recognition.
See HANDOFF and rhwp-korean-names-20261018 evidence for final installed outcomes.
Q04 remains deferred per Owner; model, time budgets and integrity checks unchanged.


## Q18 bounded request-register checkpoint — 2026-10-18
The observed natural rewrite 해 주세요 -> 해 주시기 바랍니다 is repaired with
source-derived terminal-ending instructions plus rejection, not postprocessing.
Actual installed before/after proof exists for HWP and HWPX. Four wider synthetic
condition/name templates passed, with prior48 repeated (56 cases180 checks).
Q18 remains IN_PROGRESS: declarative/question endings, mixed sentence registers,
interior honorifics and broader names/roles need held-out model/document tests.
Known narrow ending checks must not be marketed as a complete Korean register
classifier. Q11 editor safety and full save/Undo/reopen checks remain unchanged.
See rhwp-korean-register-20261018/FINAL_RECEIPT.json; Q04 remains deferred.


## Q18 bounded statement/question checkpoint — 2026-10-18
Natural rewrite preserves tested terminal predicates, question punctuation, mixed sentence levels and honorific words. Eight new installed HWP/HWPX cases plus56 regressions passed Accept/full-format/Undo/reopen. Q18 remains IN_PROGRESS for held-out registers, interior honorific morphology, role/negation shifts and legitimate synonym preservation. Exact word guards are intentionally conservative, not general Korean understanding. Follow FINAL_RECEIPT in rhwp-korean-speech-20261018; 2 requests exceed3s and all exceed1s. Q04 remains deferred. Same-integrator review only; prior refused/fixture attempts preserved.


## Q18 closed-equivalence checkpoint — 2026-10-18
The former exact-word limitation is relaxed only for five closed pairs: 드렸고/드렸으며, 말씀하셨고/말씀하셨으며, and 송신/전송 with 했어요, 했나요 or 하셨습니다. Final installed Korean40/132 passed, including2 actual connective and10 actual synonym changes with full saved formatting/Undo/reopen. 말씀 pair remains unit-only. Newly created quote collisions route through unique unquoted anchors; duplicate actions fail closed. Earlier English-source36/114 belongs to an earlier candidate, not the final-binary suite.
Q18 stays IN_PROGRESS: held-out tense/negation/role combinations, broader synonyms and morphology, technical terminology preferences and independent linguistic review. Do not generalize the tested pairs into automatic semantic equivalence. Q04 remains deferred; final requests still miss1s. See HANDOFF and rhwp-korean-equivalence-20261018 receipts.

## Q18 semantic-boundary checkpoint — 2026-10-18

Final same-binary installed Korean60/198 PASS, including20 new cases. Real connector drift and added honorific/request defects repaired; earlier automatic PASS remains semantic-review FAIL. Q18 IN_PROGRESS for broader inflection, role and nested-condition qualification, contextual terminology preferences and independent linguistic review. Conservative refusals remain outside the closed inventory. Q04 deferred; no general Hancom or whole-product acceptance. See HANDOFF and rhwp-korean-semantic-20261018/FINAL_RECEIPT.json.

## Q18 nested roles / measured performance — 2026-10-18

Korean nested-role checkpoint: ordered full surface frames preserve actors, recipients, objects, nested condition attachment, negation, tense, honorifics, punctuation and literal positions within bounded reply/question natural rewrites. Added 확인하면/확인한다면 and 확인했으면/확인했다면 after actual-model diagnosis of a valid form being refused. Final installed actual Gemma HWP/HWPX36 cases/120 checks PASS (8 new,28 regressions), explicit Accept/full saved text+char+paragraph+style/outside selection/Undo/reopen; all8 new styled actions became 회신 while quoted 답신 stayed unchanged. Recommendation1891-2981ms,p50=2421,p95=2960,1s misses36/36,3s misses0/36;Accept37-95ms,p95=83,500ms misses0/36. Host free RAM45.9-49.8GiB during final run, guest free13.6GiB snapshot, GPU aggregate10.6-11.1GiB/15.9GiB. Capacity exhaustion not observed; aggregate/coarse counters do not exclude contention. Raw separate pre-repair diagnostic shows writing1.26-1.34s and alignment1.03-1.17s of server prompt+decode time, not installed acceptance. Stage66.562s and ready12.771s separate. Gemma38/host29 assertions pass but strict native supervisors FAIL_LEFTOVER_CHILDREN (vctip.exe identified in diagnostics), complete cleanup0; focused Korean9 PASS1ignored and unchanged build confirmation PASS with clean supervisors; adapter62PASS. Earlier fixture/oracle failures and partial4 then over-refusal remain FAIL. Single exact Sandbox stopped, app natural0,extension unchanged,guest install/profile/process0,host sampler/owned process/viewer0. Composite unchanged e6ff394 app/extension plus rebuilt broker/host; not a full app rebuild. Host user data untouched, prior AppData recovery unresolved. Same-integrator review; no general Korean/Hancom acceptance, no cloud/CI/main/push/release. Evidence rhwp-korean-nested-20261018/FINAL_RECEIPT.json. Mission INCOMPLETE, Source stale, Q18 IN_PROGRESS.

Next quality-preserving performance experiment: deterministic alignment for exact, unique closed substitutions with unchanged source/revision and all style slots; retain model alignment for ambiguous cases. Require held-out actor/recipient/negation/tense mutations to refuse, full installed HWP/HWPX formatting/Undo/reopen, and measured recommendation p95. Inspect native MSVC vctip fixture cleanup separately; do not relabel strict supervisor failures as PASS. CUDA/FlashAttention/speculation remain compatibility-and-quality A/B work, not enabled optimisations. General editor coverage remains the primary feature track; storage tuning does not become a new blocking gate.

## Q19 installed incremental checkpoint — 2026-10-20

Unmodified installed bf2260a Chromium package: real keyboard opt-in twice,
Gemma6 latest-source supersessions/cache-only lookup/Accept rereads and
navigation/disable PASS in one owned Sandbox at a time. Recommendation816-1330ms,
1s miss1/6; Accept62-80ms,500ms miss0/6. Overall timing FAIL retained; six short
English cases do not qualify general1s behavior. Added optional installed corpus
mode and64-entry native timestamps to diagnose tail latency; default CI unchanged.
Final natural product cleanup and guest uninstall0, Sandbox0. Mapped-asset startup
failure and fixture setup failures retained. Internal staging69/212s and ready20/17s
are separate from inference; D contention remains deferred. No quality reductions,
user data/install, push/main/release. Next: native timing decomposition of the1.33s
case, continuous typing/IME and longer windows. See HANDOFF and external
chromium-installed-incremental-20261020/FINAL_RECEIPT.json. Q19 IN_PROGRESS.

## Q19 input-clock and stress checkpoint — 2026-10-20

Paired real Gemma measurements reproduce1327.9ms automation wait versus872.6ms
layout-visible recommendation: Playwright adaptive polling inflated the old metric.
Page input/rAF timing now separates that overhead, retaining original evidence.
Unchanged installed package, final18 supersession+6 continuous/CDP Korean IME/
196-character changed-line cases PASS: recommend672-969ms, Accept58-74ms.
No model/quality/deadline reduction. Plain-text contenteditable fixture corrected
after retained DIV-markup refusal; rich markup remains unsupported. Physical IME,
long free-form rewriting, low-GPU/cold behavior and other adapters remain open.
One Sandbox at a time; final owned processes/profile/install/Sandbox0. See HANDOFF
and chromium-input-latency-20261020/FINAL_RECEIPT.json. Q19 remains IN_PROGRESS.

## Q18/Q19 paragraph qualification — 2026-10-20

Not all PASS. Installed real Gemma four-sentence EN->KO and English rewrite4
normal cases quality/Accept PASS; KO->EN2 functional cases fail conservative
source-fidelity review (explicit You and future budget). Korean natural textarea
refused and stopped model twice; later editable is unavailable, not independent
quality evidence. Four stale races PASS. Successful functional recommendation
1026-1153ms, Accept55-63ms; paragraph3s target met only for those returned outputs.
Do not weaken checks or hide unavailable cases. Next executable work: capture
bounded internal refusal/raw synthetic output, repair Korean paragraph availability
and translation actor/tense fidelity, repeat identical installed corpus. Newly
strengthened output oracle is offline-verified, not re-run installed. Both sequential
Sandboxes and owned install/profile/processes cleaned0; launcher failure retained.
See HANDOFF and chromium-paragraph-20261020/FINAL_RECEIPT.json. Q18/Q19 IN_PROGRESS.


## Q18/Q19 paragraph repair — 2026-10-20
Closed Korean absence/object-reply equivalences repaired; completed-response
semantic refusal retains warm runtime only after original-deadline idle proof.
Actual raw KO->EN and Korean/English rewrite3 cases pass bounded semantic review;
EN->KO1 fails explicit actor preservation after broad passive guidance. Final code
restores baseline Korean translation instruction and scopes new guidance to English;
final real-model/package rerun pending. Intermediate82c007e ZIP DO_NOT_ADOPT.
Foreground denial and our external fixture path bug prevented installed paragraph
Accept/stale runs; corrected fixture and initial key-injection refusal are recorded.
All sequential Sandboxes/processes cleaned0. No user data or personal install change.
Q18/Q19 remain IN_PROGRESS. Next: fresh final source build, actual semantic review,
8 installed paragraph cases and4 stale races with an active owned browser desktop.
See paragraph-repair-20261020/FINAL_RECEIPT.json; historical FAILs remain FAIL.


## Q18/Q19 final paragraph installed qualification — 2026-10-21
2026-10-21 final paragraph installed qualification: fresh source5cf40a5 package passed8 actual Gemma EN-KO/KO-EN/natural EN/KO textarea+simple-contenteditable cases and4 stale races. Same-integrator output review preserves explicit actors, clause tense, negation, conditions, register and numbers in this bounded corpus. Recommendation1059-1371ms (p50=1152,p95/p99/max=1371), Accept54-66ms (p50=59,p95/p99/max=66); paragraph3s andAccept500ms misses0/8, short1s exceedances8/8. Explicit Review-click timing, not proactive committed-input latency. Model copy284.047s and readiness15.365s separate. Browser permission from real keyboard toolbar/popup, extension unchanged; automatic foreground succeeded, Owner click unnecessary. Natural launcher0, owned install/profile/process/Sandbox0. Build exit0 but strict supervisor FAIL_LEFTOVER_CHILDREN vctip.exe, cleanup0; historical failures preserved. Forced actual-model refusal recovery not exercised, unit evidence only. No user data/personal install/push/main/release. Evidence paragraph-final-20261021/FINAL_RECEIPT.json. Q18/Q19 remain IN_PROGRESS; mission INCOMPLETE, Source stale.


## 2026-10-21 Actual refusal recovery remains unqualified

2026-10-21 actual quality-refusal recovery qualification INCOMPLETE: added owned browser regression asserting typed refusal/no suggestion/no mutation/no replay and model+broker PID/start-time identity before follow-up8 paragraph Accepts/4 stale races. First actual Gemma run6 Korean probes did not refuse; NOT a recovery PASS. Same model4508/broker8732 survived all6; follow-up Accept not reached. Revised8 modality/nested probes were NOT_RUN: second sequential Sandbox foreground activation denied after120s. Both natural launcher0, install/profile/process/Sandbox0. Unchanged5cf40a5 package binaries/extension; external diagnostic launcher copy only redirects stderr and enables timing, not unmodified-launcher qualification. No product patch/quality weakening/user data/personal install/push/main/release. Evidence refusal-recovery-r2-20261021/FINAL_RECEIPT.json and first-run refusal-recovery-20261021. Next reproduce actual refusal without foreground dependency before UI repetition; same-process post-refusal acceptance remains open. Q18/Q19 IN_PROGRESS; mission INCOMPLETE, Source stale.

Reusable test: scripts/mission/gemma-refusal-recovery-browser.mjs. Six original probes and their actual outputs remain in first-run guest/installed-browser/result.json. Final revised probe list has syntax checks only; do not describe it as executed. Observer receives only task-owned process identity and content-free refusal codes, never supplies model output. Existing final paragraph8/4 PASS is separate prior evidence, not post-refusal recovery. Same-integrator review only.


## 2026-10-21 Actual refusal then same-process Accept — bounded PASS

2026-10-21 actual refusal recovery PASS_BOUNDED: installed5cf40a5 native host processed real Gemma request Would you mind sending the report to Mina when you have a moment? and quality guard modality_changed returned analysis_unavailable with empty suggestions. Broker7988/model9000 PID+start times identical before refusal, after refusal and after headless follow-up8 translation/rewrite Accept cases plus4 stale races. Explicit popup/field opt-in and exact reread retained; copied extension changes only localhost manifest permission, production JS unchanged. NOT OS-toolbar/unmodified-launcher qualification. Warm recommendation1058-1371ms p50=1153 p95/p99/max=1371(n8), Accept58-61ms p50=59 p95/p99/max=61;3s/500ms misses0/8, short1s exceedances8/8. Same-integrator semantic review bounded PASS. First automated run FAIL preserved: diagnostic FileStream buffered actual3 refusal codes until closure; repaired external observer with buffer1 WriteThrough, no production/quality change. Model staging301.254s,ready20.481s separate. Both sequential Sandbox natural launcher0,install/profile/process/Sandbox0; user data untouched. Evidence refusal-native-headless-r2-20261021/FINAL_RECEIPT.json corrects overbroad legacy outer receipt label; nested browser scope is explicit modified fixture. New native probe regression added. Q18/Q19 IN_PROGRESS; mission INCOMPLETE, Source stale. Next broaden actual refusal types and browser-visible refusal messaging/physical scope; no Cloud/push/main/release.


## 2026-10-23 Installed extension actual refusal recovery

2026-10-23 installed-extension refusal recovery PASS_BOUNDED: unchanged installed5cf40a5 extension (activeTab/scripting/nativeMessaging/storage, no host_permissions) activated via actual OS keyboard toolbar+popup in one owned Sandbox. Actual browser paraphrase refusal modality_changed returned analysis_unavailable, empty suggestions, no Accept, no mutation/replay. Broker4932/model5664 PID+start times identical before refusal, after refusal and after8 four-sentence translation/rewrite Accept cases plus4 stale races. Exact reread and same-integrator semantic review PASS in bounded EN/KO corpus. Warm recommendation997-1310ms p50=1091 p95/p99/max=1310; Accept54-63ms p50=59 p95/p99/max=63 (nearest rank,n8). Paragraph3s/Accept500ms misses0/8, short1s exceedances7/8; explicit Review click timing, not proactive committed-input latency. Staging242.977s, readiness14.665s separate. Disable/navigation/re-enable checks PASS; extension unchanged; natural launcher0, owned host/install/registration/profile/process/Sandbox0. External diagnostic launcher retains content-free stderr; not unmodified-launcher qualification. No product patch or new build needed, quality/model pins unchanged. Console output encoding is lossy; UTF-8 result JSON used for review. Tray-icon diagnostic remains, shutdown confirmed complete. Evidence refusal-installed-20261023/FINAL_RECEIPT.json. User data untouched; no personal install/Cloud/push/main/release. Q18/Q19 IN_PROGRESS; mission INCOMPLETE, Source stale. Next broaden refusal classes and Korean paragraph quality; physical IME/general profiles remain NOT_RUN.


## 2026-10-23 Korean request repair installed qualification

2026-10-23 Korean paragraph/refusal expansion repaired and PASS_BOUNDED: new source ceb6298 fresh package, unchanged installed extension permissions (no host_permissions), real OS keyboard toolbar+popup. Initial5cf40a5 run passed functionality but manual review found2 polite request translations changed into passive factual questions. Scoped Korean request-question English prompt and modality fence repair this without weakening ordinary omitted-subject guidance or quality/model pins. Final question-en says could you send a reply to Min-su and exact Accept succeeds in both fields. Expanded28rows:20 Accept (including8 immediate post-refusal translation/rewrite Accepts),4 actual refusals across modality_changed/semantic_changed,4 unchanged results NOT Accept proof. Baseline8 Accept+4 stale races pass. Broker6616/model7744 PID+start times unchanged throughout. Same-integrator semantic review bounded PASS; quoted Korean remains verbatim intentionally, not full quote translation. Expanded response observation1090-1568ms p50=1235 p95=1535 p99/max=1568(n20), includes100ms observation wait and is NOT paint/proactive latency; Accept61-71ms p50=64 p95=69 p99/max=71. Baseline visible recommendation966-1279ms,short1s exceedances6/8;3s/500ms misses0. Model staging369.545s/readiness22.855s separate. First Sandbox ExistingLogin0x80070520 before product start preserved; sequential runs only, all stopped. Lib tests46PASS11ignored; initial wrong broker filter ran0, NOT qualification. Test/build child-residue supervisors FAIL_LEFTOVER_CHILDREN1, cleanup0; buildexit0 with source-bound receipt. Final natural launcher0, extension unchanged, owned install/registration/profile/process/Sandbox0; user data untouched. Evidence korean-refusal-final-20261023/FINAL_RECEIPT.json and korean-request-repair-20261023 build receipt. No personal install/Cloud/push/main/release. Mission INCOMPLETE, Q18/Q19 IN_PROGRESS, Source stale; next broader request/quotation translation quality and general-profile/IME boundaries remain open.
