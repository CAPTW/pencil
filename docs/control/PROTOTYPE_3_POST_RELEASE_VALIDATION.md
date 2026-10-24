# Prototype 3 post-release validation — 2026-10-23

Published ZIP/source remain unchanged: `5d43b2b42f50a663644eefae12e6205eb80033a6`,
SHA-256 `4c42f69b9c26c06b59463ac2d200392c19adfbf8f004c5db4423014d9ca95fb8`.
This report adds evidence; it does not replace the original release receipts.

## Installed real Gemma, bounded Chromium scope

Original packaged launcher, app, broker, native host and installed extension were
used in a fresh medium-integrity Windows Sandbox with networking/clipboard disabled.
The pinned model/runtime came from the read-only D-drive asset mapping. Only the
owned guest profile/assets/installation and exact registration were removed.
No extension permission, activation or response injection was used. Real keyboard
toolbar activation succeeded twice; Playwright drove synthetic document controls.

Four existing 3–4-sentence cases (EN→KO, KO→EN, English/Korean natural rewrite),
each in textarea and simple contenteditable: **8 exact Accept rereads passed**.
Four repeated-edit/stale-Accept checks passed; changed-source Accept preserved the
new text. Cache/Accept made no inference request. Disable/navigation/reload,
unchanged installed extension hashes and memory-only storage checks passed.

Explicit Review locally click → visible suggestion next frame: 1089.3–1372ms,
p50 1138.3ms, p95/p99 1372ms. Accept → exact reread: 58.1–63.1ms,
p50 59.7ms, p95/p99 63.1ms. Paragraph3s and Accept500ms misses: 0/8.
All eight exceed 1s; this is not proactive committed-input latency qualification.
Model copy: 98.682s; launcher-to-ready: 12.310s, separately excluded from warm timing.

Same-integrator unblinded output review found the bounded names, quantities,
negation, conditions and Korean endings preserved. English remains stiff
(“is scheduled to”, “It is necessary”). These are reused regression cases,
not unseen evaluation or general semantic/fluency qualification.

Natural launcher exit0; browser/host/model/app residuals0, uninstall complete,
owned profile/install/registration removed, Sandbox inventory0. First preparation
attempt failed before product launch because host-machine.txt was omitted;
its failure and separate cleanup are preserved. Second serial attempt passed.

Desktop capture/Accept on this ZIP, physical IME, normal profiles, Edge, live
Providers and broader editor/hardware qualification remain NOT_RUN. Mission
INCOMPLETE; Project Source remains stale. A tray-icon removal warning was logged
despite launcher exit0 and complete cleanup; not erased or called a crash.

## CI failure and bounded repair

Release source main CI39969950355 and record candidate CI39970645697 passed.
Record main CI39970685154 failed: active-launcher uninstall refusal exceeded the
existing10s test deadline. The exact internal cause of that hosted delay is unknown.
Investigation found tree traversal and Add-Type compilation preceded lock refusal.
The repair checks an existing lock read-only before those operations in owned
cleanup, with no file creation. Full containment/link/tree checks and the final
exclusive mutation lock remain. The test deadline is unchanged.

A deterministic production-script regression holds a real exclusive Windows
handle and makes tree enumeration fail: old code fails at enumeration; repaired
code must refuse at the lock without changing the receipt or executable.
Local first full run had17 UTF-8 decoding errors from inherited console encoding;
preserved separately from product behavior and the hosted timeout.
Final local and hosted results are recorded in the handoff/evidence receipt.

Evidence: `D:/dev/grammar-evidence/prototype3-gemma-20261023/` (r2/guest,
quality-review.json, ci-red, ci-green, ci-green-utf8 and hosted CI logs).
The source uninstaller repair is not retroactively present in the published ZIP.

Local repair: launcher38/38 and owned cleanup boundary17/17 PASS; both supervisors report COMPLETE with0 residual processes. GitHub repair CI is a separate pending result at this commit.


## 2026-10-24 repaired package revision r1

2026-10-24 Prototype3 r1 package revision published from clean5e18309 (main Windows CI39975857078 PASS). ZIP44798a7b584e4a1ee4bfd25e43a163a65aa18324ff9fce93f53189543935a5c8, 654 manifest files verified, repaired uninstaller hash equals source. Existing release tag/original ZIP retained; four distinct r1 assets added and GitHub SHA256 verified. Final owned Sandbox actual Gemma desktop RichEdit EN-KO/KO-EN/natural EN/natural KO4 captures+candidates+Accept+exact rereads/UI confirmations PASS; recommendation1151.8-1443.0ms, Accept75.3-110.4ms. Extra EM_UNDO checks FAIL4, CANUNDO0, original not restored; no all-checks PASS and no general undo claim. Final classification PARTIAL_UNDO_FAILED. Root cause/physical Ctrl+Z remain open. Outside-trial cleanup refused with synthetic sentinel intact; active launcher uninstall refused636.3ms with receipt preserved; normal uninstall complete. App/editor natural exit0, model/owned process/install/profile/registration residual0, all Sandboxes stopped. Build helper vctip explicitly reaped, not natural cleanup. Earlier launcher-path failure, premature UI assertion, Undo failure and read-only Sandbox mapping artifact-validation failure preserved. Final copy309.0s and model-ready14.43s separate from recommendations. No user documents/settings touched. Four reused paragraphs manual meaning review acceptable with stiff wording; not general/unseen quality. r1 Chromium/physical IME/ordinary profiles/Edge/live Cloud/reproducible builds NOT_RUN. Mission INCOMPLETE; Project Source stale. Evidence prototype3-r1-20261024, release assets PROTOTYPE_3_R1_VALIDATION.json and PROTOTYPE_3_R1_INSTALL.md.


## 2026-10-24 RichEdit Undo local repair qualification

2026-10-24 RichEdit Undo repair LOCAL CANDIDATE (runtime 2583623, no main/push/release): qualified RichEdit TSF SetText reproduced CANUNDO0; correction flag did not fix it. Bound same-thread TOM replacement now records editor Undo, with TSF identity/revision/privacy guards retained and actual TOM whole-story length/text and selection/range checked immediately before write and after write. No rollback/replay or automatic Undo. Late binding invalidation and source/selection changes are refused; full-length comparisons cannot accept truncated prefixes. Native transaction17 cases,2 Undo restorations and2 replay refusals PASS (including65534-unit source). Deferred EN_CHANGE was measured after DoEditSession completion; later application edit is preserved, not incorrectly reported as an in-transaction failure. Separate real-binding fixture FAIL18 identically before/after (input-scope VT_EMPTY before capture); this is unresolved and not counted as PASS. Final unmodified installed package actual Gemma4 reused paragraphs (EN-KO,KO-EN,natural EN,natural KO): capture+candidate+Accept+exact reread+UI Applied+EM_CANUNDO1+EM_UNDO exact source restoration all PASS. Recommendation1185.1-1457.2ms,Accept87.4-129.5ms;1s target misses4/4,paragraph3s misses0/4. Deployment217.926s/model-ready15.967s separate. Bounded manual meaning/subject/condition/politeness review acceptable; stiff English remains, not unseen/general quality qualification. First guest focus failed automatically; later activation acquired. Outside-trial deletion refused with synthetic sentinel intact; active uninstall refused713.9ms, final uninstall complete. App/editor/launcher natural exit0, guest process/install/profile/registration0; the same single Sandbox was stopped, remaining0. Host user settings/documents untouched. Final clean full build and654 package hashes verified; ZIP a1544be66032d6abb95a7901e8998a39fb3c4ecace07553ac8c4bdb8c30c1cf5. Two intermediate builds deliberately aborted for review hardening, owned cleanup0; old failures remain. Same-integrator review only. Physical Ctrl+Z/IME, general editors/rich formatting, Chromium, live Cloud, full Rust suite and Windows CI NOT_RUN this checkpoint. Published r1 still has its historical Undo limitation; this candidate has not been published. Mission INCOMPLETE, Project Source stale. Evidence D:/dev/grammar-evidence/richedit-undo-20261024/FINAL_RECEIPT.json.


## 2026-10-24 capture fixture activation and candidate CI

2026-10-24 capture fixture activation repair: the same unmodified fixture passed25 cases as the first owned Sandbox process, while later inactive processes reproduced18 failures with input-scope VT_EMPTY. Diagnostic showed SetForegroundWindow=false but thread focus=true; after actual foreground activation the same process passed25. Production privacy refusal was correct; no production guard or inference changed. Fixture now requires visible owned foreground+focus before capture, bounded30s readiness, and returns ENVIRONMENT_NOT_READY/78 rather than cascading18 product failures. Hidden-window negative returns78 without capture; positive same-process repeat3 gives75 capture scenarios,9 explicit Accept/Undo exact restorations and3 replay refusals PASS. Transaction17 cases also PASS. Helper cursor click failed, but foreground activation was observed; click is not claimed successful. Original failures/compile typo preserved. One Sandbox only, owned process0 at guest completion; supervisor shutdown recorded separately. Windows CI now compiles both C++ probes with W4/WX in addition to existing Rust/Node/TypeScript checks, explicitly build-only for GUI scope. Previously installed actual Gemma4-case qualification remains bound to runtime2583623, not rerun or relabeled here. Outgoing source is for codex/grammar-local-trial-ba58b7d; no main/release/personal install. Hosted outcome must be tied to exact pushed SHA and is recorded separately in capture-fixture-20261024/CI_RECEIPT.json. Mission INCOMPLETE; Project Source stale.
