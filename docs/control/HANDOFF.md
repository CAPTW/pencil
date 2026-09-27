# Grammar autonomous mission candidate handoff

Mission: GRAMMAR-AUTONOMOUS-P3A-STABILIZATION-TO-P3B-CHROMIUM-PERSONAL-USE-R1. This is an incomplete mission checkpoint, not P3-A/P3-B or personal-use acceptance. Read AGENTS.md, the v1.1 contract override, control/mission.json and control/state.json. The Owner has already authorized internal continuation; no fresh per-step approval is needed.

## Cloud session continuation (2026-09-27, branch claude/eloquent-faraday-hh62qc)

Entry 099baea by fast-forward (no rewrite). The Owner approved Windows CI for this session branch (contract 1.3.0); codex/grammar-autonomous-r1 is untouched and path claims moved to claude-cloud on this branch only. CI evidence lives in the workflow run named below (artifact `grammar-synthetic-evidence-<sha>-<attempt>`, 7-day retention); no local D:/ evidence exists for this session.

Step 1 - independent cloud native tests. `cloud_owned_desktop_qualification` is split into `cloud_owned_desktop_input_environment` and `cloud_owned_desktop_copy_only_boundary`, each its own CI step that runs after earlier failures (`!cancelled`); every other check group is now a separate step too. The synthetic editor starts suspended inside a kill-on-close Job; teardown (including a failing test's unwind) records process exit, zero Job processes and destroyed windows. Each test writes `native/<test>.json` (FAIL unless completed; checks, cleanups, GITHUB_SHA, run id/attempt) and every step tees its log into the artifact; `summary.json` checks that receipts name the checked-out commit. Run 36299835063 on 4bd9a73: input environment PASS (3 runs; each editor cleanup exited/Job 0/windows destroyed), Copy-only boundary PASS (cleanup verified), receipts_bound_to_source=true, all other steps PASS. A workflow_dispatch `fault_injection=input_environment` run deliberately fails run 2 with a live editor to exercise the FAIL receipt, unwind cleanup and step isolation. Run 36301495139 on 4b6705a did exactly that: the input step failed with a FAIL receipt (last check `run1_sendinput_delivered_and_readback`; both editors exited with Job 0 and destroyed windows), and every later owned-desktop, build, host, runtime and Chromium step still ran and passed. The built-app step failed on its own unrelated defect, fixed below.

Step 2 - standard Edit capture (`src-tauri/src/native_edit.rs`). The withheld reader is replaced. The hotkey reads only the focused control of the foreground window, and only when that control is admitted. Admission requires class exactly `Edit`, Unicode, visible, enabled, not `ES_PASSWORD` and no password character. The control must belong to the top-level window's process, that process must not be on the password-manager/credential UI list, and the field must be under 65535 UTF-16 units. Everything else is denied before any text-bearing message: RichEdit, WinForms/WPF, browsers, Windows 11 Notepad and elevated apps (UIPI). Empty selections, split surrogates and identity/focus/range/length changes during the read are denied. Capture never uses the clipboard or keys. Owned-desktop receipt `cloud_owned_desktop_native_capture` PASS (run 36301083212 on 0fdd2f6 and every later run) covers:
- Korean/CRLF/emoji exact selection, whole field and reselection.
- Zero text messages to password, superclassed and ANSI fields, and on a window switch.
- A read-only field captured as Copy-only; clipboard untouched.

Step 3 - native Apply. A standard-Edit capture keeps a binding: HWND, pid, UTF-16 range, full-text SHA-256 and read-only flag. Apply sequence:
- Lock the control with `EM_SETREADONLY` against typing.
- Re-verify the text hash and the exact selection under the lock.
- Replace with one undoable `EM_REPLACESEL`, then re-read the exact expected text before unlocking.

Cursor, source, editor and target changes, read-only fields and text limits end in Copy-only with a specific reason. Unverifiable outcomes cancel the capture without retry. The applied path never touches the clipboard. Run 36302014460 exposed a real-Windows fact: a standard Edit refuses `EM_UNDO` while read-only, so the original "undo a displaced replacement" design could leave a moved replacement behind. Since b808fc9, when the re-read proves that only our replacement moved, the verified original is restored under the lock with one atomic `WM_SETTEXT`. The modification flag and the user's moved selection are then restored and Apply reports Copy-only. This rare recovery clears that control's single-level undo. The injected race now runs for both flag states. Receipt `cloud_owned_desktop_native_apply`: PASS in run 36302014460 up to the injected race; the restore fix is PENDING in CI run 36303467689 on 013ca67 (in progress when this was written).

Step 4 - built-app user flow (`scripts/test-app-flow-e2e.mjs`). It drives the release build from the installed package with the real global shortcut on the owned desktop, against the synthetic Edit harness. The widget no longer blocks local Instant behind cloud consent: the Deep disclosure is offered beside the draft and declining sends nothing. The primary shortcut is a documented toggle, so each capture starts with the widget hidden and reselection uses the toggle (close and cancel, then capture). Covered checks:
- Exact capture; an edited Instant-only draft applied with document reread and clipboard untouched.
- Copy writes only the clipboard.
- Cancel: the token fails with `invalid_session_state`.
- Reselection changes only the new range.
- Stale tokens are `rejected_stale` and never change a newer document in another window.
- Both process trees exit.

Run 36302014460 and earlier failed before any flow because the test attached before the widget document committed; this is fixed in 09f4656. Result: PENDING in CI run 36303467689 on 013ca67 (in progress when this was written).

Step 5 - runtime endurance (`src-tauri/tests/runtime_mission/endurance.rs`, own CI step). The test runs 8 rounds of 9 synthetic Provider cases after a warm-up: success, cancel after spawn, pipe-holding and early descendants, workspace file lock, stdout/stderr/dual floods and invalid UTF-8. It adds 20 rounds of 8-way reservation contention. Every run needs:
- Its expected outcome within the request plus teardown budget.
- Teardown phase evidence: reader drain/drop, Job zero and filesystem cleanup.
- No fixture PID and no runtime root afterwards.

Run 36302014460: 72 runs, 0 failures. Handles 146 to 146, threads 6 to 3, private bytes +0.23 MiB, 0 children, 0 roots against a settled baseline. The loaded reproduction (every CPU saturated) reproduced the historical 750ms success-fixture timeout in 6/6 runs as `timeout_during_io` (I/O about 750ms). Those runs ended in 955-1062ms total, versus the historical 1937ms, with cleanup 204-298ms, Job zero, reader drain, filesystem cleanup and 0 PIDs. They are recorded as reproductions, not passes: F-02's loaded timeout is a budget exceeded under saturation, now with bounded and complete teardown. Run 36303467689 repeats it on 013ca67.

Step 6 - Chromium regression (`adapters/chromium/tests/browser.test.mjs`, synthetic localhost, isolated profile). Additions:
- Zero page or service-worker network requests while cached suggestions are hovered and opened.
- Explicit Copy writes the clipboard only.
- A keyboard-only pass reaches and activates Accept, Apply edit, Copy and Dismiss with document rereads.

Run 36302014460 failed the new Copy check because it refilled text that Dismiss had just closed; unchanged text is intentionally not re-analyzed. The test now changes the text first. Existing checks cover opt-in, changed-line analysis, cache/annotation, Accept/Edit/Dismiss/Ignore, stale/beforeinput/user-edit rejection, navigation, disable, sensitive fields and both editors. Result: checks before the Copy step PASS in run 36302014460; the remainder is PENDING in CI run 36303467689 on 013ca67 (in progress when this was written).

Step 7 - synthetic Deep boundary. `runtime_mission::deep_boundary` drives production `ProviderManager::rewrite` with production terminology matching. The store also holds unmatched, suggested and disabled entries. The fixture records argv and each root spawn. It verified:
- Only the selected Provider starts, once, with its privacy flags.
- The selected text is sent once, plus only matched approved terms.
- A non-selected Provider is refused before any spawn.
- A failure and a cancellation each leave exactly one spawn: no replay, no fallback.

The host Deep test checks the browser path sends the field text once with an empty terminology block. The browser test checks Provider spawns never exceed explicit Deep requests. PASS in run 36302014460 (`DEEP_BOUNDARY_PASS`, host Deep receipt). No live or paid Provider was called.

Step 8 - personal package (`scripts/package`, `scripts/test-personal-package.ps1`). The release app and host come from the receipt-bound build: clean source, fresh target, `--locked --offline`. The package fixes the extension ID with a per-package public key. `MANIFEST.json` binds the source commit/tree/digest, every file hash, the extension ID and the qualified scope. `Install-Grammar.ps1`:
- Verifies every file before copying and refuses an existing root.
- Installs for the current user only.
- Registers the host for the packaged origin.

`Uninstall-Grammar.ps1` stops only processes running from the root and removes only the recorded registration. It deletes the root and, with `-RemoveUserData`, the app's settings and WebView data. CI installs the extracted ZIP into a fresh root and checks file hashes, the registry, the host manifest origin and a protocol call through the registered host. Steps 4 and 6 then run against the installed files, and removal is followed by an independent residue check. Executables are never uploaded. Result: first execution PENDING in CI run 36303467689 on 013ca67 (in progress when this was written).

### PASS / FAIL / NOT_RUN (interim)

Status as of 2026-09-27T07:40Z. PENDING rows are in CI run 36303467689 on 013ca67 and will be replaced by its outcome.

| Step | Check | Result | Evidence |
|---|---|---|---|
| 1 | Independent owned-desktop steps, FAIL receipt and unwind cleanup under injected fault | PASS | 36299835063 (4bd9a73), 36301495139 (4b6705a) |
| 2 | Standard Edit capture: exact text, denials without text messages, clipboard untouched | PASS | 36301083212 (0fdd2f6), 36302014460 (4d493ba) |
| 3 | Locked verified Apply, undo by user, cursor/source/editor/read-only/closed-target Copy-only, typing and selection races | PASS | 36302014460 (4d493ba) |
| 3 | Injected selection race restored under the lock (WM_SETTEXT), flag and selection kept | PENDING | 36303467689 (013ca67) |
| 4 | Built app: shortcut, Instant, edit, Apply, Copy, cancel, reselect, stale | PENDING | 36303467689 (013ca67) |
| 5 | Endurance 72 runs, contention, resources flat | PASS | 36302014460 (4d493ba) |
| 5 | Loaded 750ms timeout reproduction with bounded complete cleanup | REPRODUCED 6/6 (recorded, not a pass) | 36302014460 (4d493ba) |
| 6 | Chromium opt-in, analysis, cache without inference/network, Accept/Edit/Dismiss, stale/user-edit/sensitive/navigation | PASS up to Dismiss | 36302014460 (4d493ba) |
| 6 | Chromium Copy, keyboard-only actions, Ignore, contenteditable, composition, Deep | PENDING | 36303467689 (013ca67) |
| 7 | Deep boundary: selected Provider only, matched terms only, no fallback or replay | PASS | 36302014460 (4d493ba) |
| 8 | Receipt-bound build, package, fresh-root install, registered host, removal without residue | PENDING | 36303467689 (013ca67) |
| - | Live/paid Provider, accounts, physical keyboard/IME/zoom, toolbar activeTab, normal-profile install, Edge | NOT_RUN | outside the authorized scope |

### Personal use: scope, how to run, residual limits

Build locally (Windows, from a clean checkout of the evidence commit): `npm ci`, `cargo fetch --locked --manifest-path src-tauri/Cargo.toml`, then `pwsh -File scripts/mission/build-receipt.ps1 -SourceRoot . -ReceiptPath <outside-repo>\build-receipt.json` and `pwsh -File scripts/mission/package.ps1 -ReceiptPath <that receipt> -OutputRoot <fresh dir>`. Install with `pwsh -File <package>\Install-Grammar.ps1` (default `%LOCALAPPDATA%\GrammarPersonal`, `-SkipBrowserHost` without the extension). Load `<root>\extension` unpacked in Chrome; its ID must equal `MANIFEST.json` `extensionId`. Remove with `<root>\Uninstall-Grammar.ps1 -RemoveUserData`. `PERSONAL_USE.md` in the package is the Korean user guide. CI builds the same package per run but never uploads executables. Per-run evidence is in the job log, where the summary step prints every receipt. That includes `personal/package-build.json` (source commit/tree/digest, app/host/ZIP SHA-256, extension ID, file hashes), `package-install.json` and `package-remove.json`.

Enabled scope (synthetic qualification only):
- Standard Windows Edit capture.
- Locked verified Apply on those controls; Copy-only everywhere else.
- Local Instant in the desktop widget.
- Chromium local Instant on explicitly enabled textarea and simple contenteditable fields.
- Deep only for the selected Provider after explicit consent.

Residual limits:
- NOT_RUN: live/paid Provider inference and accounts; physical keyboard, IME, zoom, multiple monitors and long human sessions; production toolbar `activeTab` gesture; normal-profile or Web Store installation; Edge.
- Antigravity Deep stays `provider_privacy_unqualified`. Claude bare mode cannot use OAuth sign-in.
- Chromium DOM replacement has no guaranteed browser undo.
- A native Apply briefly makes the field read-only, so keys typed in that instant are dropped. If the app is killed at that moment, the field can stay read-only until the target app is reopened.
- The selection-race recovery clears that field's undo buffer.
- Main integration, release, signing and acceptance of the whole product are not claimed.

## GitHub cloud validation request (2026-09-27)

Owner authorized source commit/push to existing public CAPTW/pencil candidate branch and standard Windows Actions tests. Contract1.2 pins and exact task/state github_validation scope record this narrow amendment. Main integration, public releases/deployment, paid larger runners and live Providers remain unauthorized. Workflow evidence is separate from local package603764c and native interactive acceptance; exact remote SHA/run results are external in cloud-r1. No local evidence/VM files are included in Git.

## Activation lifecycle continuation (2026-09-27)

Entry a76b811; evidence `D:/dev/grammar-evidence/grammar-autonomous-r1/resume-r6`. Failed content enable delivery previously retained worker permission/capacity. The enable catch now closes only its still-current generation and exact epoch; an old rejection cannot revoke a newer session. Disable messages carry the saved documentId and epoch, and content ignores stale epochs. Two deterministic production-worker regressions fail before repair (worker-before.log44/46) and pass afterward46/46. Core215/typecheck pass; independent native_host read-only review found no actionable regression.

Browser-wSEvxV is a retained FAIL: the new lifecycle loop reused the fixture's explicitly sensitive-marked textarea, so analysis was correctly denied. The harness restores only that task-owned marker and returns its own tab to front. Browser-5Bcj4p29 local checks PASS, including four re-enable cycles: stale disable does not clear fresh UI, field opt-in is never restored, explicit Apply content rereads succeed and disable removes the UI. This bounded sequence is not prolonged endurance acceptance. No timeout, privacy gate, Provider or native capture activation changed. Exact-source package and full synthetic Deep results must be read from resume-r6 external receipts if produced; old a76b811 artifacts omit this repair.

## Viewport and scroll continuation (2026-09-26)

Entry9818425; new evidence `D:/dev/grammar-evidence/grammar-autonomous-r1/resume-r5`. A real synthetic resize to320x240 exposed the panel remaining at its old offscreen coordinate (browser-aOq955, retained FAIL). Geometry-only, frame-coalesced resize/scroll updates now clamp measured panel bounds; border-box panel height reserves space for the visible toggle. A ResizeObserver handles content size changes. Disable disconnects observer/listeners and cancels the pending frame. No text read, inference, draft replacement or safety-budget change is part of layout.

The intermediate browser-jhHm0e run passed27 checks before the scroll regression was added. Final source browser-GsF8z2 passes28 local native-host checks, including resized bounds, actual page scroll anchoring, draft retention and no added inference. Worker44/core215/typecheck PASS. Independent read-only review found no actionable issue; arbitrarily tiny viewports, physical zoom/mobile visual viewport remain unqualified. Expanded page overlap is still possible and the explicit collapse control releases it. Fresh source/package/Source preview receipts, if completed, are external in resume-r5; do not reuse9818425 artifacts for this new layout change. Overall mission and main adoption remain incomplete, with external acceptance boundaries below unchanged.

## Panel accessibility continuation (2026-09-26)

Entry2267e24; evidence `D:/dev/grammar-evidence/grammar-autonomous-r1/resume-r4`. The overlay interception retained in browser-AOK4c5 now has an explicit escape: Hide Grammar panel or Escape within its controls preserves the draft/cache while releasing covered page controls. A visible Show panel control remains; hiding is not pausing. Existing revision, expiry, sensitivity and disable checks continue while hidden. Keyboard reopening does not trigger inference. Wide-element overlap while expanded and native browser undo remain limitations.

Production browser-O6WhWQ passes26 synthetic native-host checks, including an actual covered page button click, preserved edited draft with unchanged inference count, source edits while hidden invalidating old mutation authority, and keyboard reopening. Worker44 and core215 also pass. Read-only independent review by native_host found no actionable issue in the UI change; parent ran the tests. No performance threshold or safety gate changed. Fresh committed-source build/package/Source preview and extracted browser receipts, if completed, are recorded externally under resume-r4; old2267e24 artifacts do not contain this UI fix. Main is not adopted and the overall mission remains incomplete. Native capture/clipboard/physical IME/toolbar and live Provider boundaries remain as recorded below.

## Cleanup recovery continuation (2026-09-26)

Entry for this continuation is3fa0018 (the previous clean-source package is in `resume-r2`). New evidence is `D:/dev/grammar-evidence/grammar-autonomous-r1/resume-r3`. This section supersedes the earlier statement that restart reconciliation is absent. Main adoption and A/B/C acceptance remain withheld.

The browser now offers **Check completed cleanup**. It sends only opaque control records, never document text or a Provider request. Before native reservation it persists an opaque intent; the host has four fixed installation-bound receipt slots with generation counters. Deep consumes RESERVED exactly once into PENDING. Only checked runtime teardown records COMPLETE; query may retire an unstarted RESERVED atomically, so a delayed Deep cannot start. Worker persists ACK intent before host release and serializes reserve/persist against acknowledge/erase. Lost reserve replies are found by the persisted token. Permissions still require fresh explicit enable/consent after restart.

Hard-crash PENDING, missing native records, legacy markers, damaged storage and failed disk persistence stay blocked. New runtime shutdown, PID absence, age, and missing files are not predecessor-cleanup evidence. Windows-only receipt files are content-free and bounded (ledger, lock, one temporary file); no automatic reset or orphan eviction. Power-loss/storage rollback and malicious same-account modification are not qualified. The documented Windows API flags are checked against Microsoft's MoveFileExW documentation; this is not evidence of power-loss durability.

Independent worker review found lost reservation ownership, acknowledgment/reuse ordering and retired-admission capacity defects. Durable intent/find, lifecycle serialization, and exact-owner retirement fix these; deterministic regressions cover them. A later disable-during-ACK regression also prevents a stale cancellation timer from relatching a cleaned owner. Production worker44 tests, core215 assertions, host16 tests, Provider contracts and typecheck pass. Real host synthetic Deep4/4 covers success/cancel/EOF and denied admission; admitted fixture PIDs exited, runtime directories0, fresh-host find/query/ack, wrong-ticket/PENDING refusal and cross-process lock contention are checked. Provider live remains NOT_RUN.

Two initial Chromium recovery harness attempts are retained in browser-9g2OwY and browser-EE6eMA: runtime.reload unloaded the command-line extension, producing blocked popup navigation and then no replacement-worker event. The harness now closes/reopens only its task-owned profile to test actual persistent restart state. Final browser/build/Source receipts must be read from this continuation's external evidence, not inferred from the previous package.

The resumed editor tests also exposed two concrete composition defects: input rearmed analysis while composing, treating the unreadable preedit as an unsupported field; and a cleared simple contenteditable contains a Chromium-generated bare BR placeholder. The adapter now defers scheduling/pumping/publication through composition, validates committed text before publishing a bounded deferred Instant response, restores unchanged-text cached suggestions, and accepts only the single bare empty BR with optional zero-length text nodes in addition to text-only fields. Late native replies never read preedit or authorize mutation. Browser-ANiUh2 and browser-YvBENQ retain the pre-fix textarea/contenteditable failures; browser-xuxJr9 exposed zero-length text nodes around the empty BR after a prior Range replacement. A focused synthetic DOM probe reproduced that exact structure before widening the empty-only predicate. Physical Windows IME remains NOT_RUN; protocol synthesis is not that acceptance.

Browser-lDeJ1A passed composition checks but failed Deep because the fixture output directory did not exist. A direct invocation of the same synthetic executable reproduced exit101/NotFound (`missing-fixture-dir-repro.log`); the harness now creates a fresh task-owned case directory before launch. This is a harness setup correction, not a raised timeout. The production-permission diagnostic browser-POyZAY confirmed pre-action injection denial but `Extensions.triggerAction` returned Method not allowed. No security/debugging switch was added. Production toolbar activeTab acceptance remains NOT_RUN; the standard harness still adds only its synthetic localhost permission.

The final local-Instant browser run browser-GCylBI passes24 checks, including actual task-profile restart, both editor composition flows, delayed real-native response publication, empty-placeholder negative cases and content rereads. The preceding browser-AOK4c5 run passed29 checks including synthetic Deep for both editors, then failed the new negative fixture's pointer click because the annotation card overlapped its center. That negative case now uses ordinary keyboard Tab focus, retaining the click failure as a known overlay usability limitation. Final full-Deep package acceptance must be taken from the extracted-artifact run; these two partial runs are not merged into one PASS.

Next internal work remains supported-editor qualification and exact-source packaging. Native positive capture/clipboard cannot be accepted without an isolated owned desktop session; Provider live/account/cost and normal-profile installation remain external boundaries. No permission or activation is restored by a successful cleanup check.

## Resumed candidate state (supersedes the previous checkpoint plan below)

Entry candidate9cd0cb1 remains the historical package source. New logs are under `D:/dev/grammar-evidence/grammar-autonomous-r1/resume-r2`; browser receipts remain in the evidence root browser-* directories and library checks in shared-deep-library-final; do not reuse the old package receipt for these changes. Main remains unadopted.

The new shared `provider/executor.rs` is used by desktop manager and library host. A consent-bound headless facade admits one request, uses only the selected Provider and awaits checked cleanup. Native host input/cancel/EOF runs concurrently with Deep and retains the future until completion. Browser popup grants per-document consent, Send transmits only the enabled field, and returned epoch/revision/source SHA/Provider are checked before a cached review. Applying remains explicit and revalidates current content. Hover is cache-only. User edits and policy revocation cancel or reject late Deep. Dirty review drafts block another Deep and survive passive hover/expiry as Copy-only.

The worker persists only opaque unfinished-Deep markers before admission, removes them after affirmative cleanup, and retains cancelled owners in the four-document cap. Restart with an unfinished marker blocks fresh Deep; automatic reconciliation is not implemented. No text/history is persisted by the extension. Antigravity headless Deep is `provider_privacy_unqualified` before probes. Claude uses no-session-persistence/bare/all-tools denial; installed2.1.282 recognizes flags but bare excludes OAuth/keychain authentication, including the shared desktop Claude path. No credentials were changed; no live Provider request was made.

Runtime diagnostics separate workspace, spawn/Job, I/O and cleanup. `runtime/native-pre-cap-repair.log` preserves a new failed750ms matrix: success entered I/O at955.438ms after205.616ms workspace and749.031ms spawn/Job; cleanup141.224ms, root directories0. This locates the new timeout, not the old1937ms failure. A spawn PID was observed but fixture root.pid absent, so fixture execution is not claimed. The early post-workspace cleanup path is repaired to use the original request deadline+5s, without renewed budget; Codex cleanup failure also latches against a second teardown budget. Separate existing120s Claude-caller coverage does not replace the750ms stress test.

Current completed checks:26 production-worker tests,215 core assertions,53 Source tests, Provider frontend contracts, typecheck, frontend build and governance checks pass. Shared Rust build passed; desktop206 passed/21 ignored, library80 passed/7 ignored, repaired native21 passed, latest host actor10 passed. Actual-host protocol and synthetic Deep4/4 passed (`host-deep-r1/receipt.json`), including flags, PID exit and roots0. Isolated Chromium25 checks passed in `browser-6WAo4C/result.json`: textarea and simple-contenteditable Deep mutation rereads, keyboard Accept, dirty draft/expiry, revocation and sensitivity boundaries. The browser user-edit case proves cancellation/discard across the request lifecycle, not independently that the child was still active; the actual-host cancel test waits for a live fixture PID. A prior browser failure `browser-R7W6S1/failure.json` exposed missing snapshot reconciliation after page beforeinput mutation and is preserved; the production fix is covered by the subsequent original-text return. The unchanged750ms matrix passed on this later run; earlier failures remain failures. Actual120s Claude-caller fixture completed497ms with one observed exited PID and roots0; cancellation acknowledgment0ms/cleanup248ms. Passing repeats are not attributed to the deadline-cap change alone. Review found and fixed cancellation before asynchronous admission and dirty-draft expiry. Exact final results must be recorded before acceptance. Clippy all-targets exits0 with warnings (shared library module exposure adds dead-code warnings); format check remains FAIL in baseline formatting regions. Historical formatting/benchmark failures below are not erased. Source tests now reject contradictory completion/adoption/activation fields as well as whole-product claims.

Next: rebuild exact-source artifacts and run the same isolated browser checks against the extracted package, preserving all failed attempts. Then continue the remaining editor/native qualification or return at the explicit external installation/account boundary. Production activeTab toolbar gesture, clipboard/IME/undo/endurance and native capture remain separate qualification work. Main adoption and full A/B/C completion are not claimed.

## Candidate and preserved baseline

Worktree: `D:/dev/worktrees/Grammar/grammar-autonomous-r1`; branch `codex/grammar-autonomous-r1`. Entry main: `4e833988ed27086dd6d4acb78d39fc3e9c98c707`, tree `d74a2ae8a75c0bf7dcbfae6eafd730ba00e0bc79`. Measure current candidate HEAD/tree and diff before resuming; an external handoff receipt carries exact final identity. Main is not adopted in this checkpoint. Canonical untracked audit reports/export and both failed predecessor worktrees remain untouched. Their FAIL/DO_NOT_ADOPT judgments remain historical failures.

## Previous checkpoint implemented behavior and evidence

- F01/F02: short manager locks, capture-bound operation reservations, independent cancellation, suspended child Job assignment, bounded readers/protocol buffers, first-cause errors, checked process/job/reader/workspace teardown and admission latches. Codex startup/probes and uncertain rewrite failure cleanup are included. Shutdown retains client/receipt ownership across cancelled waits and repeated errors. Repeated exit requests cannot bypass teardown; cleanup failure keeps the app open with a content-free error.
- F05: actual TS candidate origin/draft proof and backend capture/source/candidate/edit-revision validation allow edited Instant drafts to reach explicit review/Copy-only. The unchanged intent/terminology gates still apply. Late results cannot replace the dirty draft.
- F06: HWND/PID alone never authorizes mutation. Native Windows Apply is Copy-only before foreground/paste. Standard Edit metadata denial is implemented/tested, but positive native capture and semantic sensitivity authority are not qualified. Native capture activation is explicitly withheld; the selection/Instant/Deep/review flow implementation is retained for qualification, not claimed working end-to-end on native editors.
- Chromium: current-document then current-field opt-in; sensitive/capability checks before field-value reads; bounded UTF-16 revision/delta core; existing Rust Instant engine via actual MV3/native messaging; adjacent cached annotation cards; Accept/Edit/Dismiss/Ignore/Copy; pause/disable/emergency/denylist/navigation teardown. Plain textarea and text-node-only simple contenteditable have synthetic browser mutation reread evidence. No rich editors/frames/shadow editors. The default host configuration is unconfigured; no user profile was installed.
- F03: daily bundler now rejects missing/mismatched build receipts before output creation. Fixed build pipeline measures clean physical tracked bytes plus Git identity before/after and hashes desktop+host binaries from a fresh target. Negative tests cover fake identity, dirty source, source/desktop/host mismatch. A receipt is local reproducible provenance, not signed anti-forgery attestation.

Evidence root: `D:/dev/grammar-evidence/grammar-autonomous-r1`. `runtime/HANDOFF.md` and logs contain native fixture commands and per-case PID/root receipts. `full-rust.log`: 217 passed, 21 ignored at pre-final-check checkpoint; final checks use separately named logs. `frontend-contracts.log`: Provider scripts passed; `scripts/test-p3-03-instant-runtime-state.mjs` now imports production TS (27 assertions; dirty-draft mutation fails five assertions). `adapters/chromium/tests/core.test.mjs`: 214 assertions. `worker.test.mjs`: 9 mocked-Chrome tests, including revoke/late-enable/old-port races. `browser-pMIDis/result.json`: 15 actual isolated Chromium+native-host synthetic checks, including both editor rereads, changed-line-only analysis, preserved unaffected annotations, cached hover count unchanged, isolated-world sensitive-value reads0, navigation revoke. Earlier browser failures are preserved and drove fixes (own-edit snapshot, stale refill, field selection, card geometry).

The browser fixture adds ONLY its local test origin host permission to the copied extension. Production activeTab toolbar gesture acceptance is NOT_RUN. Real Provider inference/accounts, normal-profile installation, native positive capture/clipboard/Apply acceptance, IME/device/zoom matrix, browser undo semantics and prolonged endurance are NOT_RUN. Direct DOM replacement has no guaranteed native undo transaction; do not claim it. No screenshots/OCR/keylogging, credential access, real user documents or cloud requests were used. Task-owned native registrations are removed by the browser harness finally block.

## Budgets and changed assumptions

Historical 200ms failure is not reclassified. Cancel-handle acknowledgement target100ms; existing caller request timeouts preserved; one 5s teardown budget shared across child/Job/readers/workspace. Windows synchronous filesystem calls can overrun and are reported as late/error; waiting cancellation is not actual cleanup. No global default3s workspace age deadline. Failed cleanup blocks new admission; no silent success.

Chromium implementation limits are 8192 UTF-16 units/document, 64 cached suggestions, 60s TTL, 250ms debounce, four documents, one host request per document, 5s local host timeout; native frames128KiB. Changed complete lines are analyzed; unchanged suggestions rebase only through exact unaffected ranges/source equality. These are candidate resource contracts, not broad performance-quality claims. Suggested local responsiveness target is2s from field/input to annotation including debounce; final native timeout is5s. Deep requires a separate request budget, not an enlarged Instant timeout.

## Final checkpoint caveats

`final-rust.log` records219 passed/21 ignored; `final-frontend.log` passes Provider contracts and typecheck. Core now215 assertions; `browser-BpsSOU/result.json` records16 synthetic checks and zero sensitive-field value reads, including explicit sensitive attributes. Later worker policy fixes invalidate pending enables synchronously, block concurrent policy changes/new enables, and recheck the four-document limit after injection; final external receipts record their tests and exact commit.

`runtime/final-checkpoint.log` is a retained failure:19 passed/1 failed. While release compilation was also active, the success fixture exceeded its unchanged750ms request deadline and returned provider_timeout (elapsed1937ms); workspace roots were removed. This failed case recorded no root.pid, so its owned_pids_exited=true field is vacuous and does not establish child execution. Phase timing is absent; timeout can occur after workspace creation before spawn, or after spawn during the I/O deadline. Total1937ms is within the combined5750ms budget, but the expected success failed. Concurrency is an observation, not a proven cause. A same-budget repeat without compilation must be recorded separately and cannot erase this failure. F02 overall acceptance remains unresolved. `final-format.log` fails formatting; `final-clippy.log` exits0 with43 warnings. These are not clean lint/format acceptance.

Release attempts whose source changed during security fixes are superseded and must not emit qualified receipts. Only the final clean-source receipt, if successfully produced, authorizes a candidate package; the package remains UNQUALIFIED_MISSION_CANDIDATE. Exact source/build/package identity, final Source preview, test additions and residual checks live in the external final result/handoff packets.

## Previous checkpoint plan (superseded by resumed state above)

1. Recheck final runtime review/tests and preserve all ownership/cleanup evidence. Qualify native capture only in an isolated input desktop and with defensible sensitivity authority. Do not remove the fail-closed gate just to revive the UI. A native supported Apply path needs exact document/range authority and mutation reread; generic native editors stay Copy-only.
2. Implement `GRAMMAR-P3B-P1-SHARED-DEEP-RUNTIME-AND-CONSENT-BOUND-HOST`. Extract one shared headless Provider facade into the library; avoid copies of private CLI logic. Keep P3-A capture reservations in its caller. Isolate pure mode/request/terminology DTOs from Tauri settings coupling. Both desktop and host must use the same bounded provider runtime.
3. Host is currently synchronous local Instant only. Add bounded concurrent cancel/EOF reception and await checked cleanup for Deep; keep source/epoch/revision identity. Add explicit per-document consent/disclosure and selected codex/antigravity/claude. No hover inference, silent fallback, replay, or entire terminology store. Initially empty approved constraints are safer than unapproved terms. Use owned synthetic executables/AppData only; live inference still needs separate Owner permission.
4. Complete browser keyboard/Copy/IME/undo/revoke-inflight/endurance and production activeTab gesture acceptance. Harden known unsupported UI layouts. Rebuild/verify source-linked candidate artifacts, then assess safe local main integration. Current local Instant successes do not waive these gates.
5. Regenerate exact12 Source from canonical main only after adoption; this checkpoint can emit PREVIEW_NOT_QUALIFIED. Manual Project UI application remains Owner work and does not block internal development.

No background continuation is implied by this handoff. On resume reuse this mission worktree and its authorization; record reviewed path claim transfer, rather than creating another recovery mission. No remote push/release or account/system changes are authorized.
