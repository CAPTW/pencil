# Grammar autonomous mission candidate handoff

Mission: GRAMMAR-AUTONOMOUS-P3A-STABILIZATION-TO-P3B-CHROMIUM-PERSONAL-USE-R1. This is an incomplete mission checkpoint, not P3-A/P3-B or personal-use acceptance. Read AGENTS.md, the v1.1 contract override, control/mission.json and control/state.json. The Owner has already authorized internal continuation; no fresh per-step approval is needed.

## Pre-use inspection round (2026-09-27/28): review and fixes before a limited personal trial

Mission GRAMMAR-AUTONOMOUS-P3A-STABILIZATION-TO-P3B-CHROMIUM-PERSONAL-USE-R1. Base candidate cdf8553 (round r4). Goal: find, reproduce and fix what a person would meet in a limited personal trial (confusion, lost drafts, stuck installs, restart problems), not only re-run CI. Run 36339738039 stays evidence for cdf8553's scope only; it is not reused for code changed after it. Nothing here approves a personal install, a main merge or a release. `mission_completion` stays INCOMPLETE. This section supersedes the r4 verification figures for everything changed after cdf8553.

Start checks: read AGENTS.md, contract 1.3.0, control/mission.json, control/state.json, this handoff and ACCEPTANCE_MATRIX. `python scripts/validate_governance.py --root .` PASS. HEAD cdf8553, clean, equal to the remote branch.

Method:
- Four read-only risk reviews (desktop lifecycle, widget, Chromium, install/runtime) set the risk list.
- Each finding was reproduced before fixing. Evidence was one of: a failing unit test on the old code; the previous scripts on a stub package; headless Chromium with a copied extension and a synthetic host or the real engine (the Windows host under Wine); or a CI run.
- Fixes got a regression test where one could be automated. Windows-only checks were batched into the mission CI.
- Two fresh-context review passes then covered all of cdf8553..HEAD. Their findings are fixed below (review section).
- Each record: cause → user impact → reproduction → fix → regression test → residual.

### Findings fixed

Install and package (13a3f13, 75a1dab):
- I1 BLOCKER. Registration used New-Item on `HKCU\Software\Google\Chrome\NativeMessagingHosts\<name>`. Chrome creates that parent only when some host registers, so on a clean profile the default install stopped with "registry key ... does not exist". Fix: create each missing level, never `-Force` (which would replace existing hosts); `registered` is written only after the key points at host.json. Test: `missingParentKeysCreatedAndRegistered`.
- I2 HIGH. A failure after the root was created left a folder with no receipt, so reinstall said "already exists" and uninstall said "Not a Grammar installation". Reproduced with the previous scripts on a stub package. Fix: the uninstaller and an `incomplete` receipt are written first, the registration is recorded before anything else can fail, and any error rolls back; an incomplete rollback names the uninstaller; an existing root is refused with upgrade steps. Tests: `registrationFailureRolledBack`, `existingRootRefused`.
- I3 MEDIUM. Uninstall relied on host-tools it deleted early and could not resume after a lock. Fix: it removes its recorded key itself (only if the key still points at its manifest), deletes the receipt and itself last, and reports `problems/notes/complete/next`. Tests: `lockedUninstallIncompleteAndRepeatable`, `repeatedUninstallCompleted`, `uninstallRemovedOnlyItsRegistration`, `runningOwnCopyStoppedAndRemoved`.
- I4 MEDIUM. `-RemoveUserData` deleted the settings, dictionary and Codex sign-in that every Grammar copy of the user shares, even while another copy ran. Fix: refused then; the guide no longer uses it by default. Test: `removeUserDataRefusedWhileAnotherCopyRuns`.
- I5 Integrity. Files not in MANIFEST.json were installed unchecked, hidden files were not listed, and packaging accepted ignored or untracked files and assume-unchanged/skip-worktree edits. All are refused now. Tests: `changedFileRefused`, `unlistedFileRefused`; local packaging negatives.
- I6 LOW. The uninstaller did not default to its own folder, and a trailing separator broke the receipt match. Covered only by local stub-package runs (Linux pwsh).
- I7 MAJOR (review). With `-RemoveUserData` the root, including the receipt and uninstaller, was deleted before the user data. A locked user-data folder then left the data, including the app's Codex sign-in, with no uninstaller to rerun, although the result said to rerun it. Fix: user data goes first, after a fresh check for another running copy; the receipt and uninstaller always go last. Tests: `lockedUserDataKeepsReceiptAndUninstaller`, `repeatedRemoveUserDataCompleted` (task-owned folder, not the runner's AppData).
- I8 MINOR (review). When removing the key failed, the fallback record `host/registration.json` was still deleted, so a rerun reported complete while the registration stayed. Fix: the record is kept. No automated test (the key must refuse removal).
- I9 MINOR (review). A key that now points to another manifest made every run incomplete, and a rerun could never clear it. Fix: it is left untouched and reported as a note. Test: `foreignRegistrationLeftAndUninstallCompleted`.
- I10 MINOR (review). Under Windows PowerShell 5.1 the installer failed with a .NET method error, and its rollback could delete the receipt before other files. Fix: `#Requires -Version 7.0`; the rollback removes the receipt and uninstaller last. The rollback path was run locally on Linux pwsh.

Desktop capture lifecycle (1c7f1a3, 95c44cf):
- D1 HIGH. Cancel (or Escape while Deep ran) ended the whole capture. The user saw the raw `invalid_session_state`, and the Instant draft, possibly edited, could no longer be copied. Fix: `cancel_rewrite` returns the capture to Captured. Tests: `cancel_deep_keeps_the_capture_and_its_instant_draft_usable` (fails on the old code); built-app step 7.
- D2 HIGH. A busy Provider (an earlier request finishing, the startup probe, Refresh, Test connection) left the capture in Rewriting, so Deep and Copy failed until a new capture. Fix: `reserve_rewrite` rolls back to Captured and answers `provider_busy`; the widget waits up to 5 s when a request it stopped is still finishing (nothing was sent). Test: `busy_provider_reservation_returns_the_capture_to_captured` (fails on the old code).
- D3. The late end of a cancelled request could end a newer Deep request. Fix: numbered attempts, checked when the request finishes and, after review, also at reservation. Tests: D1's test and `a_cancelled_attempt_cannot_reserve_or_undo_a_newer_one`.
- D4 MEDIUM. Shortcut toggle, tray Hide and window close ended the capture silently, and the widget kept offering Copy. Fix: a `capture-ended` event, sent only when a capture actually ended (after review, so shutdown and shortcut warnings stay); `dismiss_window` hides even when the capture already ended. Tests: `closing_the_widget_still_ends_the_capture`; built-app step 8.
- D5. Copy of an Instant draft made outdated by a mode or dictionary change reported `invalid_session_state` and ended a usable capture. Fix: `draft_outdated`, capture kept, message "A setting changed after this draft was made". Test: capture contract.
- D6 HIGH (CI 36361143740). While Deep ran, the widget replaced the draft box with a spinner, so the local Instant draft could not be read, edited or copied until Deep ended (up to two minutes with a real Provider). Fix: the draft box stays, with a progress line naming Cancel for copying now. Test: built-app step 7 (the draft must appear while the slow synthetic Deep runs).
- D7 HIGH. The backend emitted the Instant result only while the capture was Captured, so with auto rewrite it was lost whenever Deep started first. Fix: kept for an open capture (Captured, Rewriting or Ready). Test: `the_instant_draft_belongs_to_an_open_capture_while_deep_runs`; built-app steps 7-12b.
- D8 MEDIUM (review). Run Deep on a ready result said "This capture is no longer active" although Copy still worked. Fix: the button is disabled while a result is ready, and the command answers `rewrite_result_ready`. Tests: mvp contract, the D7 test (`is_ready`).
- D9 MEDIUM (review). A language change in Grammar mode cleared the Instant cache, so an edited draft could not be copied and the message blamed the mode or dictionary. Grammar Instant depends on no language setting, and a language change that matters changes the intent. Fix: the separate language invalidation is gone. No automated test.
- D10 LOW (review). The Cancel status promised a draft that did not exist (modes without Instant, or nothing to change). Fix: it says "Deep cancelled" then.
- D11 (CI 36364343895). A shortcut press right after a closing press started no capture: no error, nothing in 15 s.
  - The shortcut gate ignored every press until it saw the previous press's release. The hotkey crate only polls for the release (every 50 ms) and can report it late or never, and with MOD_NOREPEAT there is no auto-repeat to filter.
  - Fix: a press is ignored only within 250 ms of the previous accepted one.
  - The root cause is not proven: this is the only silent drop path found. On such a timeout the built-app flow now presses once more and reports whether the first press was dropped (still a FAIL).
  - Test: `a_missed_release_never_swallows_the_next_press`.

Widget (a2e56d2, 01da908):
- W1 HIGH. After a mode, target or reference language change the new Deep result never appeared: there were two draft stores and only one was cleared. Fix: both reset together once per intent change. Test: built-app step 6.
- W2 MEDIUM. Dictionary, profile, import, "Save suggested", mode and language changes silently discarded an edited draft. Fix: a confirmation first; declining changes nothing. No automated test (window.confirm path); it is on the Owner's manual list.
- W3. A late Deep result or error could reset the rewriting state of a newer request. Fix: only the latest call may change the widget. Built-app step 7.
- W4. Capture errors, Dismiss and stale rejections left the Instant runtime behind. Fix: one `clearCapture`.
- W5. Raw codes reached users (`invalid_session_state`, `code: sentence`). `codex_unavailable` showed "Codex missing" even with another Provider selected (seen in 36357375075). Fix: sentences only. Tests: mvp and capture contracts.
- W6. The error row is announced (role=alert), and focus moves to Dismiss after Copy.

Chromium adapter (2c309bd, 766f7ae; reproduced in headless Chromium before fixing):
- C1 HIGH. Moving through the suggestion list (Tab or hover) replaced the open card, so Accept applied a suggestion the user had not chosen. Fix: a card opens only on choice, and every action is bound to that suggestion object. Tests: "keyboard Accept applies the chosen suggestion and keeps the others", "pointer hover never changes the open card".
- C2 MEDIUM. After a page edit without an input event, Deep then Accept overwrote a different range and reported "content verified". Fix: identity binding closes the stale card. Local harness only.
- C3 HIGH. Accept or Apply edit emptied the whole list and never re-checked. Fix: the other suggestions stay (shifted) and the changed lines are re-checked. Browser test above.
- C4 HIGH (keyboard). "Enable this field" enabled the last focused element, so passing links or buttons made a keyboard user enable, and possibly send to Deep, another field. Fix: links and buttons do not change the choice, the panel names the field ("Field to enable: ..."), and Alt+Shift+E enables the focused field. Browser test "Alt+Shift+E ...".
- C5 MEDIUM. Each contenteditable Accept added two text nodes, so the field was disabled after about 8 Accepts. Fix: edit the nodes in place. Browser test: nine repeated Accepts.
- C6 MEDIUM. A discarded reply, host error or expiry emptied every suggestion and left a stale status. Fix: only the affected parts are dropped, the whole field is re-checked after a discard or expiry, and each item expires on its own. Local harness only.
- C7 LOW. Typing during Deep showed "Local engine unavailable". Fix: wait and re-check, bounded to 150 s after review.
- C8 (worker). After "Check completed cleanup" succeeded, grants from before the failure still worked. Fix: recovery revokes them. Test: worker.test (fails on the old worker).
- C9 (worker). A `cleanup_busy` reply to cleanup-reserve left a "reserving" marker that blocked Deep. Fix: the marker is dropped and the send reports busy, and after review the retiring slot is freed as well. Tests: worker.test, including "a document closed during a busy reserve frees its slot" (fails on the old worker).
- C10 HIGH (CI 36361143740; a regression from the C4 fix). Clicking into a password field and pressing "Enable this field" enabled the previously chosen supported field and read it. Fix: any text field the user is in becomes the choice; an unsupported one is named "(not supported)" and refused. Reproduced locally with the real host (one extra analysis). Tests: the fixture's "sensitive label rejected" and "explicit sensitive metadata denied".
- C11 MAJOR (review). A Deep reply replaced a card the user had opened while waiting, so a click aimed at its Accept could apply the whole-field rewrite. Fix: the result waits in the list. Test: "a Deep reply never replaces a card opened while waiting" (Deep reply held in the content world; fails on the old content.js).
- C12 MAJOR (review). While Deep ran, expiry or re-render said "no cloud" or "Suggestions expired". Fix: the status keeps saying the field is being sent. No automated test.
- C13 MINOR (review). Alt+Shift+E ignored Ctrl (AltGr) and composition, so typing Ę or Ē could enable and read a field. Fix: it requires no Ctrl or Meta and no composition, and refuses unsupported fields. No automated test.
- C14 MINOR (review). A list button could open a different suggestion that had moved into its position. Fix: it opens only its own suggestion. The outcome of Accept, Apply edit, Copy or a refused action now stays until the user acts again. No automated test.

Providers and runtime (7c41a11, 01da908):
- P1 HIGH. A `.cmd`/`.bat` launcher (npm's `claude.cmd`) showed Ready, but Rust refuses multi-line arguments to batch files ("batch file arguments are invalid", reproduced with a probe under Wine), so every Deep request failed at spawn. Fix: reported unavailable with the remedy, and `claude.exe` is preferred. Test: `batch_launchers_are_unavailable_and_native_executables_are_not`.
- P2 MEDIUM. A successful Claude rewrite whose text mentions logging in was reported as signed out. Fix: sign-in wording counts only in failed runs. Test: `successful_result_mentioning_login_is_not_signed_out`.
- P3 MEDIUM. The Codex app-server stdout cap (64 MiB) counted the client's whole life, so a long session eventually failed a healthy request. Fix: it resets at `turn/completed`. Tests: `output_cap_covers_one_turn_not_the_client_lifetime` (fails on the old code), `output_cap_still_stops_a_runaway_turn`.
- P4. A partial `remove_dir_all` could delete a runtime folder's ownership marker, and the deadline path did not restore it, so the folder was never reclaimed. Fix: every give-up path restores it. No deterministic test (needs a sharing violation mid-removal).

### Contract failures at cdf8553 (63f07c9)

- Engine contract, 17 checks: environment gap, not a product regression. They need Windows PowerShell 5.1 and the task-owned self-test roots. They are now NOT_RUN where these are missing (exit 2, never GREEN) and run for real in a CI step from an LF worktree. MAIN_RUNTIME_SOURCE_CHANGED was the P3-02 task's no-touch rule for main.rs (stale policy) and is retired; the engine boundary checks stay. Linux: 452 pass, 17 NOT_RUN.
- Benchmark contract, 1 failure: stale policy. The P3-01 closed file set predates P3-02's two files; they joined the set under the same text checks. 136/136. The measurement itself never failed.

### Failures during the round (kept as failures)

- Endurance, run 36357375075: FAIL. Loaded run 2 started the synthetic child in 4.9 s under 8 spinning threads, timed out and missed the cleanup deadline, leaving one runtime folder; runs 3–6 were refused (`provider_cleanup_unresolved_restart_required`, fail closed). No budget or case was changed. Runs 36358305471, 36358600996 and 36361143740 passed (72 clean runs, 6/6 loaded timeouts reproduced). Classification: load variance on the hosted runner plus the P4 marker weakness (fixed).
- Test defects, not product:
  - The built-app step 6 assumed the widget was hidden (01da908).
  - It also expected `source\n\n(translation)` for a one-line source, which the product joins as `source (translation)` (fac6038).
  - The Chromium repeated-Accept loop accepted the engine's correct repeated-word suggestion instead of the misspelling, and plain End stopped at a wrapped line. Reproduced with the real engine (fac6038).
- Product defects found by CI: D6/D7 (built-app step 7) and C10 (fixture), fixed in 95c44cf and 766f7ae.
- Built-app flow, run 36364343895: FAIL at section 9's capture (see D11).
  - Commit 971525a changes only docs relative to 75a1dab, which passed in 36364165671. The failure depends on timing.
  - It is kept as a FAIL; the gate is fixed, and the test gained a diagnostic press.

### Residual limits (not fixed in this round)

- Claude and Antigravity receive the prompt (the selection plus matched terms) as a command-line argument, which other processes of the same user can read while Deep runs. Codex uses stdin. Changing it needs live Provider verification (out of scope).
- The desktop Provider acknowledgement is stored per Provider and survives restart. With Auto rewrite on (the default) every capture is sent to that Provider. No screen withdraws it; `-RemoveUserData` resets it.
- Copy is disabled while Deep runs (Cancel first). Escape does not close the desktop widget.
- `cancel_cli_operation` still identifies an operation by token and intent, not attempt. In a one-hand-off window a stale operation may run on; its result is discarded, and the newer request waits via the busy retry.
- History API or hash navigation removes the Chromium panel without a stated reason.
- `%TEMP%\codex-pencil-runtime-v1` survives uninstall. What a real Provider writes there is NOT_RUN.
- A stale Chromium cleanup temporary file keeps browser Deep blocked (fail closed); recovery is a reinstall.
- Provider executable lookup checks the current directory first. The engine descriptor's `runtime_wired` flag is stale.
- No automated regression test: W2, C2, C6, C12, C13, C14, D9, I8 and P4.
- NOT_RUN:
  - physical keyboard and IME;
  - live Providers (Codex, Claude, Antigravity);
  - normal user profiles and real documents;
  - the mouse toolbar path, Edge, zoom/multi-monitor and long sessions.
- Build reproducibility is not verified.

### Review pass

Two fresh-context review agents read cdf8553..HEAD read-only, split into desktop runtime, and Chromium plus package. They are the same model family as the author and had no shared conversation, so this is not an independent human review.
- Desktop: no BLOCKER, 1 MAJOR (D8), 5 MINOR (D10, D9, the busy re-run, D4's unconditional event, attempts at reservation). All are fixed, except `cancel_cli_operation`'s identification (residual above).
- Chromium and package: no BLOCKER, 3 MAJOR (C11, C12, I7), 7 MINOR (C13, C9's slot, C7's bound, C14 twice, I8/I9, I10). All are fixed.

### Verification

- Local:
  - Wine: 217 Rust tests passed, 0 failed, 26 ignored (Windows-only live tests).
  - Frontend typecheck, and every local Node contract.
  - worker.test 49/49.
  - The whole Chromium fixture, including synthetic Deep, in headless Chromium with the real host and synthetic child under Wine.
  - Stub-package install, refusal, `-RemoveUserData` and rollback runs.
- CI:
  - 36364165671 on 75a1dab passed every step, including built-app sections 1-13 with shortcut spam and the fixture with synthetic Deep.
  - 36364343895 on 971525a (docs only) failed built-app section 9 (D11).
  - The run on the final commit is named in the Owner report; earlier failures are listed above.

### Owner's minimal manual checks (not run by this round)

1. Check the ZIP hash, extract it into an empty folder, and install with `pwsh` (PowerShell 7) into a fresh folder. Confirm the Chrome extension ID matches `MANIFEST.json`.
2. Desktop, in a standard Windows Edit field (for example a classic dialog's single-line box; the new Windows 11 Notepad is not a standard Edit and is refused):
   - select Korean and English text, press Ctrl+Shift+G, edit the Instant draft, Copy, and paste it yourself;
   - check that the original text did not change.
3. Choose a Provider and allow Deep once with a short non-sensitive sentence, on your own account and at your own cost:
   - the Instant draft stays editable while Deep runs;
   - Cancel keeps it;
   - changing the mode after editing asks first.
4. Chromium, in a fresh profile with the action pinned:
   - enable the document by keyboard (Alt+Shift+T);
   - Alt+Shift+E in a non-sensitive textarea, then Accept one suggestion;
   - a password field is refused;
   - type Korean with the IME.
5. Quit from the tray, restart, and confirm no capture or draft returns.
6. Uninstall. Add `-RemoveUserData` only if the settings and dictionary should go, then confirm the folder and registration are gone.

## Review round r4 (2026-09-27): Owner decisions after round r3

This section supersedes the "Kept, as Owner decisions" list and the Apply wording of round r3 below. `mission_completion` stays INCOMPLETE.

Owner decisions:
1. Native Copy-only stays. No new editor or adapter is added to bring back replacement in desktop editors.
2. The desktop Apply button is merged into Copy.
3. The unreachable legacy paste code is deleted; the regression tests stay.
4. R3 is accepted only for the verified keyboard toolbar path. The mouse path through the Extensions menu or the popup, a normal profile and Edge are not verified and must not be claimed.
5. main merge, personal installation and release remain separate decisions.

Changes:
- One Copy button (`copy-result`). It calls the session-bound backend command (IPC name `apply_replacement` kept). The command checks the capture, intent, terminology binding and Instant draft, formats a translation locally, writes the clipboard, counts terminology usage and ends the capture. The widget shows `Copied — paste it into the field`, keeps the draft visible and disables Copy until the next capture; Dismiss and the header Close close it. The former plain Copy (browser clipboard API, no session check) and the `applied` outcome are gone.
- Deleted:
  - the clipboard-and-`SendInput` paste path: foreground request, widget hide, clipboard snapshot/restore, input injection;
  - its ignored interactive harness `windows_live_target_bound_apply_acceptance` and the paste probes;
  - the paste-era reasons `target_not_foreground`, `target_changed_before_paste`, `widget_hide_failed`, `clipboard_ownership_lost` and `input_injection_failed`;
  - the capture's clipboard fields.
- The `ApplyPlatform` trait now has only `is_window`, `window_pid` and `write_clipboard_text`. It cannot send input, change focus or message the captured editor.
- Kept:
  - the R1/R2 regression tests through `apply_current_session`;
  - the live application-edit scenarios and the Copy-only boundary;
  - the read-only Win32 port test;
  - a new unit test: a capture without the native binding is Copy-only too.
- The stored `restoreClipboard` preference stays so existing settings files remain valid; nothing reads it.
- Tests were updated for the new UI, not loosened:
  - frontend contract: only Copy outcomes parse; `applied` and the paste-era reasons are rejected;
  - window chrome: no `applied` branch, and the widget stays open after Copy;
  - built-app flow: the single Copy button for edited and unedited drafts; a cancelled, closed or stale token returns its rejection and never writes the clipboard.

R3 scope (decision 4): accepted for the path verified in 36331068541 (and again in 36337934296) only. That path is Chromium, a fresh task-owned profile with the action pinned, and real keyboard input: Alt+Shift+T, the Right arrow to the action (Tab is the helper's fallback), Tab to "Enable this document" in the popup, and Space only on the focused target. The installed-package test still tries the mouse and UI Automation first and records which path worked; those attempts are not qualification.

Verification:
- Local: Wine 208 passed, 0 failed, 26 ignored (Windows-only live tests); frontend typecheck and build; node contract tests.
- CI run 36337934296 on 8e81fc8, the three decision commits. All 34 steps passed, and every owned-desktop receipt is PASS and bound to 8e81fc8.
  - Built-app flow: PASS, 10 checks, `result_delivery` COPY_ONLY. It covers Copy of an edited and of an unedited draft, cancel, toggle-reselect and stale tokens. All 12 per-editor step records show 0 reads and 0 state-changing messages.
  - Native receipts, all PASS: Copy-only boundary (8 checks), input environment (11), capture (14), native Copy (10), application edits (7).
  - Unmodified installed-package browser: PASS, 11 checks, both activations by keyboard. The mouse click opened the popup but "Enable this document" was not found, and UI Automation Invoke failed; the receipt records both.
  - Modified isolated extension fixture: PASS, 39 checks. The synthetic Deep boundary (4 checks), the host Deep lifecycle (4 synthetic cases, not live) and endurance (72 residue-free runs, 6 loaded timeouts reproduced, no failures) passed.
  - Personal package from 8e81fc8, tree c53155d5…, tracked bytes 476e5e2d…:
    - hashes: app 88ae09e6…1512, host ba7f3697…d83c, ZIP 8e78f250…8d80;
    - extension ID `lhcfloaegpmljmlldiacljbgohccohfo`;
    - manifest `nativeApply` NONE_DESKTOP_COPY_BUTTON_ONLY_NATIVE_EDITORS_NEVER_MUTATED;
    - the fresh-root install verified 5 files and one local Instant suggestion, and removal left no residue.
- Separate review pass: a fresh-context review agent reviewed 959473e..8e81fc8. It was the same model family, read-only and without the shared conversation, so it is not an independent human review. It found no blocking issue and no reachable path that sends input, a focus change or a state-changing message to a captured editor. e7b573f fixes its findings:
  - should-fix: "never changes text in other apps" had been widened past the desktop app, but the Chromium extension does edit enabled browser fields. The wording now names the desktop app and the extension exception.
  - minor:
    - no "window changed" note for a capture without the native binding;
    - clipboard checks for the closed and stale tokens in the built-app flow;
    - Dismiss stays enabled for a copied draft, as the header Close already did;
    - toolbar claims limited to the keyboard path, with its keys in PERSONAL_USE;
    - CI step names say Copy;
    - the ignored interactive P1-03 test expected an error code from the clipboard classifier that capture stopped using in Step 2. It now asserts only that capture leaves an image-only clipboard untouched.
- e7b573f and this docs commit are verified by the run on the final commit, reported in the Owner report.

## Review round r3 (2026-09-27): R1/R2/R3 corrections

This section supersedes the Step 3 and Step 4 Apply results and the Step 6 "installed host and extension" classification in the continuation below. `mission_completion` stays INCOMPLETE.

Verdicts:
- R1 fixed by removal: the locked verify-replace-reread Apply overwrote an application edit made after its verification read and reported Applied.
- R2 fixed by removal: a moved selection led to a wrong-range replacement plus a `WM_SETTEXT` restore that was reported as Copy-only.
- Grammar now never mutates a captured native editor. Apply returns Copy-only (`target_mutation_disabled`), puts the result on the clipboard and sends the editor no text, selection or state-changing message.
- Status: risky path blocked; the native Apply feature is not implemented.
- R3: the 39-check browser test is a modified isolated extension fixture. A new installed-package smoke test verifies the unmodified installed package through a real toolbar gesture.

### Counterexamples: failing first, then fixed

Commit 4400974 added the regressions without changing production code. Run 36322314553 (4400974) failed as intended:
- Rust regression step: 4 new production-path unit tests failed. Local Wine at the same commit gave:
  - R1: Applied with "hello fixed";
  - R2 and restore: 7 editor state changes including `WM_SETTEXT` and `EM_SETMODIFY`;
  - plain Apply: Applied.
- Real synthetic editor, application edits (receipt `cloud_owned_desktop_native_apply_against_application_edits`: FAIL):
  - R1: `outcome=Applied state_changes=4 foreign_change_notices=2 app_edits=1 final_is_application_text=false clipboard_has_result=false`.
  - R2: `outcome=CopiedFallback { reason: TargetSelectionChanged } state_changes=7 foreign_change_notices=2 app_edits=0`. The final text matched only because of the whole-text restore.
  - Edit right before the restore: Copy-only reported after the restore. The receipt truncated this line; later receipts drop quoted values instead of truncating.

Fix dc05cb5 (run 36323285356):
- The Rust regression step passed.
- The same live scenarios passed with 0 state changes, the application's own text kept and the result on the clipboard. They are R1, R2, the edit before a restore and 12 application-timer iterations.
- The Copy-only live matrix passed: clean capture, cursor move, text change, read-only, another foreground window, user typing during Apply, closed target.

### Evidence by category (final verification run: see the Owner report for the run on the final commit)

| Category | Evidence | Result |
|---|---|---|
| Deterministic production-core tests | `native_edit` tests through `apply_current_session`: R1, R2, restore, Copy-only, selection authority, read-only Win32 port; Wine 218 passed | red 4400974, PASS from dc05cb5 |
| Real Windows synthetic editor | application edits (R1/R2/restore/timer), Copy-only matrix, capture | FAIL 4400974, PASS from dc05cb5 (36323285356 onward) |
| Installed app flow | built app from the installed package: shortcut, Instant, edit, Copy-only Apply, Copy, cancel, reselect, stale | PASS from 64e0301 (36324511882). Every step rereads both editors' text and selection; 0 reads and 0 state-changing messages during Apply, Copy and stale Apply |
| Modified Chromium fixture | 39 checks, `MODIFIED_ISOLATED_EXTENSION_FIXTURE` (copied extension: new key/ID, fixture host permission, own host-config and registration) | PASS (36324511882 onward). In 36331068541 its stale-source check raced the 1 s reconciliation that also prevents mutation; it now changes the value at Accept mousedown |
| Unmodified installed-package browser | `installed-package.test.mjs`, 11 checks | PASS in 36331068541 (ce89642); earlier runs FAIL while the gesture helper was fixed |
| Synthetic Provider | host Deep lifecycle, Deep boundary | PASS in every run |
| Live Provider | none | NOT_RUN (not authorized) |

Installed-package smoke test in 36331068541, all unmodified:
- the installed extension directory with the package key/ID `kibjpddbcgbdfflnnhcmpnnhlalaklld` (that package);
- the installed permissions (activeTab, no host permission);
- the installer-written `host-config.js`;
- the installer's HKCU registration and host binary.

Browser setup: a fresh task-owned profile, the browser sandbox on and Playwright's `--disable-extensions` removed. The profile pins the action to the toolbar, which is a user UI preference.

Results:
- Before the gesture: no page access and no UI.
- The action was invoked through the real toolbar. The OS mouse click on the pinned action opened the popup, but UI Automation could not locate the popup's button by name. UI Automation Invoke on the action was refused. Real keyboard input then worked: Alt+Shift+T, arrow keys, and Space on the focused action and on the focused "Enable this document". It granted page access both times. All fallbacks are recorded in the receipt.
- Textarea and simple contenteditable: analysis ran in the installer-registered host process; Accept and Apply edit were each reread.
- Disable removed the UI and analysis; storage held no text. A cross-origin navigation revoked access. The second gesture re-enabled the document and required a fresh field opt-in. A reload removed the opt-in.
- Cleanup: browser closed, installed host exited, profile removed, installed files unchanged. Removal then left no registration, files, app data or processes.
- Observation: Chromium keeps activeTab after a same-origin reload. The Grammar UI still needs a new popup Enable, which needs a toolbar gesture.

When the Extensions menu itself was used, with Playwright's `--disable-extensions` present, neither an OS mouse click nor UI Automation Expand opened it. This is recorded, not qualified.

### Separate review pass

A fresh-context review agent (same model family, read-only, no shared conversation; not an independent human review) reviewed 91c18c0..64e0301. It found no reachable production path that mutates a native editor.

Fixed:
- 1 blocking finding in the gesture helper.
- Should-fix findings:
  - narrow NOT_RUN;
  - a meaningful second activation;
  - Copy-only statements scoped to native editors;
  - a unit test that selection authority cannot re-open paste for native captures, and comment fixes;
  - a step timeout.
- Minor findings:
  - counters for every change notification and text/selection rereads in the built-app flow;
  - quoted values removed from receipts;
  - `Win32EditPort::send` limited to the capture's read messages;
  - stale comments;
  - receipt source binding;
  - hit test limited to the target or its container.

Kept at the time, as Owner decisions (round r4 deleted the paste path and merged Apply into Copy):
- the compiled but unreachable paste path (gated and tested);
- the in-memory capture binding fields;
- the desktop "Apply" label, which now copies.

### Package and reproducibility

The package is rebuilt by every CI run from that run's commit. The package-build, install and remove receipts in that run's log link source commit, tree and tracked-bytes digest to the app, host and ZIP SHA-256 and to the install/remove results. A document inside the package cannot contain its own package's hash. The final package hashes are therefore reported for the final commit's run.

The extension key, and so the extension ID and ZIP hash, is random per package by design. Bit-for-bit reproducibility of the app and host binaries is unverified: the same commit was not rebuilt in the same environment. Different hashes across commits say nothing about it.

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

Cursor, source, editor and target changes, read-only fields and text limits end in Copy-only with a specific reason. Unverifiable outcomes cancel the capture without retry. The applied path never touches the clipboard. Run 36302014460 exposed a real-Windows fact: a standard Edit refuses `EM_UNDO` while read-only, so the original "undo a displaced replacement" design could leave a moved replacement behind. Since b808fc9, when the re-read proves that only our replacement moved, the verified original is restored under the lock with one atomic `WM_SETTEXT`. The modification flag and the user's moved selection are then restored and Apply reports Copy-only. This rare recovery clears that control's single-level undo. The injected race now runs for both flag states. Receipt `cloud_owned_desktop_native_apply` PASS with 13 checks, including the injected race for both flag states, in runs 36303467689 (013ca67), 36303738766, 36304934814 and 36306114335 (dbc4fca). The randomized selection races ended either in a clean Apply or a pre-lock refusal (`restored0`); the restore path is covered by the deterministic injection.

Step 4 - built-app user flow (`scripts/test-app-flow-e2e.mjs`). It drives the release build from the installed package with the real global shortcut on the owned desktop, against the synthetic Edit harness. The widget no longer blocks local Instant behind cloud consent: the Deep disclosure is offered beside the draft and declining sends nothing. The primary shortcut is a documented toggle, so each capture starts with the widget hidden and reselection uses the toggle (close and cancel, then capture). Covered checks:
- Exact capture; an edited Instant-only draft applied with document reread and clipboard untouched.
- Copy writes only the clipboard.
- Cancel: the token fails with `invalid_session_state`.
- Reselection changes only the new range.
- Stale tokens are `rejected_stale` and never change a newer document in another window.
- Both process trees exit.

Earlier failures, all fixed:
- Runs 36302014460 and before: the test attached before the widget document committed (fixed in 09f4656).
- Run 36304934814: it expected the result pane before any capture, but with Codex signed out the idle widget shows the sign-in pane. A capture always opens the result pane, so local Instant and Apply need no cloud account (fixed in dbc4fca).

Result: PASS, 10/10 checks, against the installed personal package (`app_kind: installed-personal-package`, idle pane `codex-sign-in`) in run 36306114335 on dbc4fca.

Step 5 - runtime endurance (`src-tauri/tests/runtime_mission/endurance.rs`, own CI step). The test runs 8 rounds of 9 synthetic Provider cases after a warm-up: success, cancel after spawn, pipe-holding and early descendants, workspace file lock, stdout/stderr/dual floods and invalid UTF-8. It adds 20 rounds of 8-way reservation contention. Every run needs:
- Its expected outcome within the request plus teardown budget.
- Teardown phase evidence: reader drain/drop, Job zero and filesystem cleanup.
- No fixture PID and no runtime root afterwards.

Run 36302014460: 72 runs, 0 failures. Handles 146 to 146, threads 6 to 3, private bytes +0.23 MiB, 0 children, 0 roots against a settled baseline. The loaded reproduction (every CPU saturated) reproduced the historical 750ms success-fixture timeout in 6/6 runs as `timeout_during_io` (I/O about 750ms). Those runs ended in 955-1062ms total, versus the historical 1937ms, with cleanup 204-298ms, Job zero, reader drain, filesystem cleanup and 0 PIDs. They are recorded as reproductions, not passes: F-02's loaded timeout is a budget exceeded under saturation, now with bounded and complete teardown. Runs 36303467689 and run 36306114335 on dbc4fca repeated it: 72/72 each; resources flat (handles 146 to 146, at most 149; threads 6 to 3; private bytes under +0.5 MiB; 0 children and roots). Loaded reproductions were 6/6 each: 1001-1189ms total, cleanup 236-438ms, Job zero, 0 PIDs.

Step 6 - Chromium regression (`adapters/chromium/tests/browser.test.mjs`, synthetic localhost, isolated profile). Additions:
- Zero page or service-worker network requests while cached suggestions are hovered and opened.
- Explicit Copy writes the clipboard only.
- A keyboard-only pass reaches and activates Accept, Apply edit, Copy and Dismiss with document rereads.

Run 36302014460 failed the new Copy check because it refilled text that Dismiss had just closed; unchanged text is intentionally not re-analyzed. The test now changes the text first. Existing checks cover opt-in, changed-line analysis, cache/annotation, Accept/Edit/Dismiss/Ignore, stale/beforeinput/user-edit rejection, navigation, disable, sensitive fields and both editors. Result: PASS with 39 checks in runs 36303467689, 36303738766, 36304934814 and run 36306114335 on dbc4fca. The last run used the installed package's host (SHA-256 `ef4b26c7…7762`, equal to the packaged host) and its extension files.

Step 7 - synthetic Deep boundary. `runtime_mission::deep_boundary` drives production `ProviderManager::rewrite` with production terminology matching. The store also holds unmatched, suggested and disabled entries. The fixture records argv and each root spawn. It verified:
- Only the selected Provider starts, once, with its privacy flags.
- The selected text is sent once, plus only matched approved terms.
- A non-selected Provider is refused before any spawn.
- A failure and a cancellation each leave exactly one spawn: no replay, no fallback.

The host Deep test checks the browser path sends the field text once with an empty terminology block. The browser test checks Provider spawns never exceed explicit Deep requests. PASS in run 36302014460 and every later run, including run 36306114335 on dbc4fca. The boundary receipt records 2 constraints sent, both matched approved terms. The host Deep receipt covers denied, success, cancel and EOF, and verifies the cross-process lock. No live or paid Provider was called.

Step 8 - personal package (`scripts/package`, `scripts/test-personal-package.ps1`). The release app and host come from the receipt-bound build: clean source, fresh target, `--locked --offline`. The package fixes the extension ID with a per-package public key. `MANIFEST.json` binds the source commit/tree/digest, every file hash, the extension ID and the qualified scope. `Install-Grammar.ps1`:
- Verifies every file before copying and refuses an existing root.
- Installs for the current user only.
- Registers the host for the packaged origin.

`Uninstall-Grammar.ps1` stops only processes running from the root and removes only the recorded registration. It deletes the root and, with `-RemoveUserData`, the app's settings and WebView data. CI installs the extracted ZIP into a fresh root and checks file hashes, the registry, the host manifest origin and a protocol call through the registered host. Steps 4 and 6 then run against the installed files, and removal is followed by an independent residue check. Executables are never uploaded. Runs 36303467689 through 36304934814 failed the build receipt. The failure receipt added in f1b00fd named the cause: on a `core.autocrlf=true` checkout the Tauri CLI rewrites `src-tauri/Cargo.toml` with LF endings, so the post-build source measurement changed. `src-tauri/.gitattributes` now checks that file out with LF everywhere (932f687).

Result in run 36306114335 on dbc4fca: build, package, install and removal all PASS.
- Source: commit dbc4fca, tree `0f5bc1eb3ec9f3176ad69f0a75464de0a6d4bd85`, tracked-bytes SHA-256 `3a5f2415f7529b00e6d52868656eb84105ac08fd4d5eef195c7aed213fa1cd3f`.
- Receipt build: fresh target, exit 0.
- App SHA-256 `77fcbf26789f04d5a58355c549a5517f79dc13c980ebb41d38918007a747e20f` (13234176 bytes).
- Host SHA-256 `ef4b26c742eecea8ac546fb589f2f510844f4349ddec42e1221b2dfda96a7762` (1870336 bytes).
- Package: 20 files, ZIP SHA-256 `d05f6810c8a13ebca604f3e80b5cc24deddb89d597a6342c3ea0645da168a9d1`, extension ID `djibhkenbanknlolafdopkoigjboloae`.
- Install: fresh root; 5 key files verified against `MANIFEST.json`; host registered for the packaged origin; local Instant answered through the registered host (1 suggestion, no Provider).
- Removal: uninstaller complete, and the independent check found no install root, registry key, app data or installed-binary process.

These hashes belong to that run's build. A later commit, including a documentation-only one, is a different package with its own receipts.

### PASS / FAIL / NOT_RUN (final evidence run 36306114335 on dbc4fca)

Run 36306114335 on dbc4fca repeated every row below with the same result. The earlier runs listed first established each result.

| Step | Check | Result | Evidence |
|---|---|---|---|
| 1 | Independent owned-desktop steps, FAIL receipt and unwind cleanup under injected fault | PASS | 36299835063 (4bd9a73), 36301495139 (4b6705a) |
| 2 | Standard Edit capture: exact text, denials without text messages, clipboard untouched | PASS | 36301083212 (0fdd2f6), 36302014460 (4d493ba) |
| 3 | Locked verified Apply, undo by user, cursor/source/editor/read-only/closed-target Copy-only, typing and selection races | WITHDRAWN (review R1: application edits were overwritten) | 36302014460 (4d493ba) |
| 3 | Injected selection race restored under the lock (WM_SETTEXT), flag and selection kept | WITHDRAWN (review R2: mutate-then-restore is not safe) | 36303467689 (013ca67), 36306114335 (dbc4fca) |
| 4 | Built app from the installed package: shortcut, Instant, edit, Apply, Copy, cancel, reselect, stale | PASS (10/10) with the withdrawn mutating Apply; superseded by the Copy-only flow | 36306114335 (dbc4fca) |
| 5 | Endurance 72 runs, contention, resources flat | PASS | 36302014460 (4d493ba) |
| 5 | Loaded 750ms timeout reproduction with bounded complete cleanup | REPRODUCED 6/6 (recorded, not a pass) | 36302014460 (4d493ba) |
| 6 | Chromium opt-in, analysis, cache without inference/network, Accept/Edit/Dismiss, stale/user-edit/sensitive/navigation | PASS | 36302014460 (4d493ba), 36306114335 (dbc4fca) |
| 6 | Chromium Copy, keyboard-only actions, Ignore, contenteditable, composition, Deep (39 checks, installed host + modified isolated extension fixture) | PASS | 36303467689 (013ca67), 36306114335 (dbc4fca) |
| 7 | Deep boundary: selected Provider only, matched terms only, no fallback or replay | PASS | 36302014460 (4d493ba) |
| 8 | Receipt-bound build, package, fresh-root install, registered host, removal without residue | PASS | 36306114335 (dbc4fca) |
| - | Live/paid Provider, accounts, physical keyboard/IME/zoom, normal-profile install, Edge | NOT_RUN | outside the authorized scope |
| - | Toolbar activeTab on the unmodified installed package (round r3: real keyboard input on the pinned action in a fresh profile) | PASS | 36331068541 (ce89642) |

### Personal use: scope, how to run, residual limits

Build locally on Windows from a clean checkout of the evidence commit. A clone made before 932f687 must re-check out `src-tauri/Cargo.toml` once, by deleting it and running `git checkout -- src-tauri/Cargo.toml`. Steps: `npm ci`, `cargo fetch --locked --manifest-path src-tauri/Cargo.toml`, then `pwsh -File scripts/mission/build-receipt.ps1 -SourceRoot . -ReceiptPath <outside-repo>\build-receipt.json` and `pwsh -File scripts/mission/package.ps1 -ReceiptPath <that receipt> -OutputRoot <fresh dir>`. Install with `pwsh -File <package>\Install-Grammar.ps1` (default `%LOCALAPPDATA%\GrammarPersonal`, `-SkipBrowserHost` without the extension). Load `<root>\extension` unpacked in Chrome; its ID must equal `MANIFEST.json` `extensionId`. Remove with `<root>\Uninstall-Grammar.ps1 -RemoveUserData`. `PERSONAL_USE.md` in the package is the Korean user guide. CI builds the same package per run but never uploads executables. Per-run evidence is in the job log, where the summary step prints every receipt. That includes `personal/package-build.json` (source commit/tree/digest, app/host/ZIP SHA-256, extension ID, file hashes), `package-install.json` and `package-remove.json`.

Enabled scope (synthetic qualification only):
- Standard Windows Edit capture.
- Desktop Apply is Copy-only: Grammar never changes text inside another application's native editor (native Apply not implemented). The Chromium extension still replaces text in explicitly enabled textarea and simple contenteditable fields on explicit Accept or Apply edit, with reread.
- Local Instant in the desktop widget.
- Chromium local Instant on explicitly enabled textarea and simple contenteditable fields.
- Deep only for the selected Provider after explicit consent.

Residual limits:
- NOT_RUN: live/paid Provider inference and accounts; physical keyboard, IME, zoom, multiple monitors and long human sessions; normal-profile or Web Store installation; Edge. The toolbar `activeTab` gesture passed in round r3 through real keyboard input on a pinned action in a fresh profile; the mouse path through the Extensions menu is not qualified.
- Antigravity Deep stays `provider_privacy_unqualified`. Claude bare mode cannot use OAuth sign-in.
- Chromium DOM replacement has no guaranteed browser undo.
- Desktop Apply never inserts the result; the user pastes it from the clipboard.
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

## Local prerequisite repair — 2026-10-03

Owner requested local continuation after the ba58b7d Sandbox trial. Only this local branch takes over the inherited cloud claim; remote and main are unchanged. Clean-guest evidence is preserved outside the repo at `D:/dev/grammar-evidence/grammar-local-trial-ba58b7d/LOCAL_TRIAL_20261003.md`. Original host exited 0xC0000135 until a test-only VC runtime was supplied; the desktop required an explicit WebView2 runtime path. Conditional browser 11-check and bounded UIA Copy evidence do not constitute unconditional installation acceptance or the full 18-check app suite.

The host release command now explicitly enables static CRT on Windows MSVC and rejects caller Rust flag overrides; the exact configuration is part of receipt verification. No dependency versions changed. The installer runs the packaged app's `--check-webview2` mode (Tauri/Wry loader) before filesystem/registry installation, with 30-second wait and bounded termination of its owned probe. That mode exits before app state, settings, Provider initialization or GUI construction. It checks discovery only, not complete browser initialization. Missing runtime leads to an actionable message without automatic installation.

Validation before release rebuild: governance and eight negative provenance cases passed. Production-installer synthetic probe cases cover available, missing, abnormal exit and timeout. Fresh committed build, PE imports and new Sandbox results will be recorded separately; no new product acceptance is asserted here. Evidence root: `D:/dev/grammar-evidence/grammar-local-fix-20261003`. No live Provider, main adoption, remote push or release.

### Local prerequisite repair validation result

Product commit `b0814c098ac99c0c7246d356b0e8ec7439a2672b` was built from a clean tree into a new target. The host has no VCRUNTIME140 import and passed local Instant in a clean Sandbox without a supplemental DLL/PATH. Production WebView2 preflight rejected absent runtime before install directory or registry creation. Default guest discovery returned 78; explicit guest runtime discovery returned 0. This remains a conditional runtime trial, not an Evergreen installation test.

Installed unmodified extension: 11 keyboard-path checks PASS. Bounded actual desktop UIA: onboarding, capture/hotkey/Instant, edited Copy PASS; clipboard equals draft, source and selection unchanged, all editor counters zero. Full 18-check app CDP suite remains BLOCKED, not replaced by UIA. Both installations removed with no reported root/registration/process/user-data residue; guest stopped. Four synthetic production-installer probe cases and eight provenance rejection cases PASS. The first new Sandbox attempt stopped at an overbroad harness PATH check before product execution; preserved as FAIL, then rerun with the inherited portable PowerShell path removed. No security settings changed. Separate review pass by same integrator only.

Evidence and complete artifact hashes: `D:/dev/grammar-evidence/grammar-local-fix-20261003/RESULT.md`. Package is bound to b0814c0, even if this record later advances HEAD. Existing cloud evidence and historical audit base are unchanged. Local worktree is `C:/Users/USER/.codex/worktrees/grammar-local-trial/Grammar`, branch `codex/grammar-local-trial-ba58b7d`. Continue isolated full-app harness diagnosis; live Provider, general profile, physical IME, main adoption and release are not accepted by this result. Source materiality changed locally; exact-12 canonical Project Source generation and Owner UI application were not performed.

## Local full app flow and delayed capture termination — 2026-10-03

The b0814c0 packaged app exposed 127.0.0.1:9361 and the IPC bridge under a Medium, non-administrator restricted version of the guest account. Earlier High-integrity logs and Microsoft's elevated-host hardening explanation support elevation as the CDP blocker, but this is an inference: an optional same-binary High-integrity control command was rejected by automatic tool approval policy, without a detailed reason, and did not execute. No debugging policy or security setting changed. Evidence: `D:/dev/grammar-evidence/grammar-app-flow-20261003/output/medium-cdp-diagnostic.json`.

The full flow then found a product failure after shortcut spam: Dismiss failed with a lost capture. `end_capture_and_notify` awaited Provider interruption before sending an identity-free termination event; a late event could clear a newer capture. A deterministic injected delayed event reproduced an erased edited draft on b0814c0 (`red-fresh/red/app-result.json`). Another attempt failed earlier on foreground acquisition and remains a separate harness/environment failure.

The repair snapshots the ended session token under the capture lock and carries it in the notification; the frontend clears only the matching capture. Dismiss no longer discards its token/draft before backend success, and preserves a newer selection. The same full app-flow script now supports an explicitly guarded Medium-integrity Sandbox, with installed app hash and test-script hash binding, plus the regression. `scripts/local` records the guest-only lowering helper and runner. New package acceptance is pending the fresh build and unchanged-assertion run; no complete mission or live Provider acceptance is claimed.

The 4a219f0 package passed the delayed-event regression but still failed shortcut spam / Dismiss (`green/app-result.json`); it is not a full-flow-qualified package. Further inspection found that every quick press independently spawned capture/hide work, allowing old capture errors/selections to race newer store state. The next repair coalesces presses during one admitted operation with an RAII guard; release events still update the key latch, no unbounded queue or delayed foreground capture is created, and visibility is rechecked after admission. Tests cover contention and guard release after failure. Full flow must be rerun without assertion changes.

### Full app result (supersedes pending validation above)

Clean product commit `d85f3234494ebb8deeae22c289d2038f5005270e`, tree `61701d7fe907f83492bbbe12e352e66abe4644e6`, was rebuilt into a fresh target and packaged. All 19 checks passed in EACH of two fresh Sandboxes (`green2`, `repeat`): selection/Instant, edited and unedited Copy, stale and cancelled tokens, synthetic Deep/Cancel, clipboard contention, Unicode, password/empty refusal, delayed old termination, shortcut spam/next capture, restart and process exit. Across 52 editor counter records, state changes were zero. Document contents were reread by the harness. Both production uninstall receipts report complete with no installation/registration/process/user-data residue. Sandbox inventory is empty.

Capture Rust tests: 2 passed. Shortcut Rust tests: 10 passed, 1 interactive test ignored; the two new admission tests are included in those ten. Frontend capture/draft contracts, typecheck, release build and governance self-test passed. The full Rust suite, physical keyboard/IME, ordinary profiles, actual Provider and build reproducibility were not rerun/proven by this checkpoint. Current-package Chromium smoke results follow below. Earlier b0814c0 browser evidence remains separately scoped. No test budgets or assertions were weakened; four earlier app-flow failures remain in FAILURE_INDEX.json. Review was a separate pass by this same integrator, not independent review.

Artifact ZIP: `D:/dev/grammar-evidence/grammar-app-flow-20261003/package2.zip`, SHA-256 `996088d7bf64bd2c7b41f1ed16a38b273a92f01248538cb64b4b8dca3650eafc`. Full source/app/host/script hashes, receipts and limitations are in that evidence root's RESULT.md. The external runner differs from the checked-in runner only by selecting package-flow2; the E2E script is byte-identical. Explicit guest WebView2 selection remains conditional. The package stays bound to d85f323 even when this documentation advances HEAD. Local Source materiality changed; exact-12 canonical export and Owner Project UI application were not performed. Mission INCOMPLETE, main NOT_ADOPTED. Local synthetic checkpoint is complete; Owner-assisted physical keyboard/IME and personal installation need their appropriate scope, and real Provider/account use remains separately authorized.

Current-package browser follow-up (`browser2/browser-result.json`): 11/11 PASS on d85f323, installed unmodified extension and host, sandboxed fresh Chromium profile, two keyboard toolbar activations. textarea Accept/Apply edit and simple contenteditable Accept were reread exactly. Disable, cross-origin navigation and reload removed the applicable access/UI. Browser and host exited, profile was deleted, extension hashes unchanged, production uninstall complete. Final Sandbox inventory empty. The initial browser attempt was stopped before any receipt when the reused harness was found to have an old hardcoded source SHA; preserved as INTERRUPTED, then replaced with manifest-derived identity. Mouse/Invoke popup activation remained unsuccessful and is not accepted.

## User-assisted input session prepared — 2026-10-03

Unchanged product d85f323 is running in owned Sandbox `f1088846-38f5-4fb5-bd71-842c7b4cab6f` for physical input, using the new guarded scripts/local/Guest-App-HandsOn.ps1 and Guest-Trial-Guide.ps1. READY_AWAITING_OWNER_INPUT and GUIDE_SHOWN receipts exist; Auto rewrite false and all Deep acknowledgements zero verified. No physical keyboard/IME or real Provider acceptance claimed. Guest language enumeration was empty (unknown IME availability). Session expires after 60 minutes or task-owned finish.request, stops only owned children and uninstalls; verify actual cleanup later. Intentionally keep this guest open for Owner input. If it has already closed, do not report it as still ready. Initial preparation failed solely because guest Notepad was absent; installer cleanup completed and guest stopped. Dedicated guide removes that dependency. Evidence and resume instructions: D:/dev/grammar-evidence/grammar-hands-on-20261003/STATUS.md. Canonical main, remote and user profiles remain unchanged.

## Additional automated regression — 2026-10-03

Owner asked whether Codex login is needed and requested agent-run tests. Login is not required for local Instant/Copy; idle sign-in pane is distinct from selection-bound local review. The pending manual guest was ended through finish.request, uninstalled completely and stopped; physical keyboard/IME remain NOT_RUN. No credentials or live Provider calls.

Current app binary Rust suite: 221 PASS, 26 ignored (not all Cargo targets). Core: 215 assertions PASS. Worker: 49 PASS. Package d85f323 installed-host Chromium modified isolated fixture: 44/44 PASS in a fresh Medium Sandbox with browser sandbox enabled, covering protocol composition (not physical IME), cache zero inference/network, stale mutation boundaries, exact rereads, synthetic Deep cancellation/revocation, sensitive-field denial and cleanup. Host SHA matches the clean package receipt. Registration removed, host count zero, profile removed, package uninstall complete, guest stopped and final inventory empty. No new product defect or product source change. Unmodified installed-extension 11-case test remains separately classified. External harness adaptations and receipts are preserved at D:/dev/grammar-evidence/grammar-auto-regression-20261003/RESULT.md. No source export, main adoption, remote push or release. Next physical input and live Provider acceptance require their actual evidence; do not require login for local-only testing.

## Account-free start screen repair — validation pending

Owner encountered the idle Codex sign-in pane and inferred login was required for local testing. The idle signed-out Codex pane now leads with local Instant instructions and a Close and use local Instant action, using the existing safe dismiss path. Optional Codex Deep login is collapsed; missing CLI wording explicitly applies to Deep. No Provider, consent, capture, Apply or runtime contract changed. The full installed-app driver adds a start-button check proving no settings/consent change before the existing real capture/Instant/Copy flow. Typecheck and capture contract passed. Fresh package validation is pending; previous package acceptance remains separately bound to d85f323.

### Account-free start validation completed

Clean product/package a305f1574e5fc5e10c78bb78177d48250061e0ab passed 20 installed-app checks in a fresh Medium Sandbox, including the new visible local start action (no settings/Deep consent changes), then real synthetic-editor capture/Instant/Copy, stale boundaries, cancellation, shortcut spam and restart. Test-only d5a3865 selects the visible local heading instead of the now-collapsed login heading; product code and packaged identity remain a305f15. Current-package installed unmodified Chromium keyboard smoke passed 11 checks, with exact textarea/contenteditable rereads and revocation. Both uninstalls complete, browser profile removed, host count zero, final Sandbox inventory empty. Typecheck, capture/window contracts, governance, clean build and package provenance passed. Full Rust/core/worker/44-case fixture were not repeated for this UI-only change; their prior d85f323 evidence remains separate. No physical IME, real login/Provider or ordinary profile acceptance.

ZIP: D:/dev/grammar-evidence/grammar-local-start-20261003/package.zip; SHA-256 5b13c1a2af2bb4a79db81563985ba43c670fc9d0e35f934499d6cf8964426e58. Full receipt/source/app/host/test identities and limits: same root RESULT.md. Local helper requires 20 app checks; manual helper pins this package in package-start, but it was not launched for physical input in this checkpoint. Same-integrator review only. No main adoption, push, release or canonical Source export. Whole mission remains INCOMPLETE.

## Desktop Deep consent revocation — validation pending

The settings pane lacked consent status/revocation; generic settings writes could lower consent without stopping an admitted Deep request. The candidate adds visible per-Provider status and Revoke all Deep consent, disables Auto rewrite and preserves the local draft. Generic settings saves cannot raise consent without explicit selected-Provider review, so a stale settings save cannot restore a revoked acknowledgement. Frontend saves are serialized; revocation suppresses late publications and earlier review completions. Backend settings persistence and capture cancellation share admission serialization, and rewrite rechecks current consent under the capture lock. Persistence failure remains an error with best-effort cancellation, never successful revocation.

Rust app suite 223 PASS/26 ignored, typecheck, capture/window contracts and governance passed. Installed-app regression adds active synthetic Deep revocation, edited draft Copy, backend denial of new requests and stale settings, cleanup wait and revoked settings across app restart. Fresh build/guest result pending; no live Provider or complete acceptance claim. Evidence: D:/dev/grammar-evidence/grammar-consent-revoke-20261003.

### Desktop consent validation and outstanding shortcut failure

Clean product/package 427e5cb4df540857a46f1680311f285e2efe0d10 passed the installed-app 21-check flow in `app`, including synthetic active Deep revocation, retained edited Copy draft, denied new Deep/stale settings restoration, Provider busy clearance, and restart persistence. Driver-only d084cfd made all three zero consent values and Auto rewrite=false explicit before restart. That repeat FAILED at the existing shortcut-spam assertion (`repeat/app-result.json`, "the first press captured"); it had passed revocation but did not reach restart. Preserve this failure, not a full-pass repeat.

Driver-only b886ca8 adds pre/post spam diagnostics without changing assertions or timeouts. The diagnostic fresh guest passed 21/21, including explicit revoked settings and restart. Before spam: widget hidden, correct foreground, selection 12..33, source matched; after spam: one exact capture. This pass does not resolve the intermittent failure. Installed global-hotkey 0.8.0 registers MOD_NOREPEAT and polls release asynchronously (50ms); Grammar additionally suppresses presses for 250ms without an observed release. Suppression is a plausible cause, NOT measured proof for this failed run. Next internal work: instrument/reproduce production admission and release ordering; preserve operation coalescing and target safety. Do not increase waits or declare the failure environmental without evidence.

Current-package unmodified installed Chromium keyboard flow passed 11/11, including exact textarea/contenteditable mutation rereads, disable/navigation/reload, no access before gesture, unchanged extension and host cleanup. Mouse/Invoke popup activation remains unqualified. All four installs (`app`, `repeat`, `diagnostic`, `browser`) uninstalled completely; each owned guest was stopped; final inventory empty. Across the three app runs, all 80 editor counter records had zero state-changing messages (including the failed run, not 80 independent tests).

ZIP SHA-256 c5e289d63a3bea9f146040d396994c8b5b74605972978e2821180e0fb11aa827; full hashes, driver identities and results: D:/dev/grammar-evidence/grammar-consent-revoke-20261003/RESULT.md. Test-only commits do not change the package's 427e5cb identity. Desktop revocation does not revoke the extension's separate document consent. Persistence-write failure and rapid overlapping UI consent actions received code review only, no installed fault injection. Same-integrator review only; actual Provider/account operations, physical keyboard/IME, ordinary profiles, Edge and build reproducibility NOT_RUN. Explicit guest WebView2 remains conditional. Manual helper pins the new package but no hands-on guest was launched here. Mission INCOMPLETE; no main adoption/push/release or canonical exact-12 Source export/Owner UI application. Source materiality updated locally only.

## Native shortcut release ordering repair — validation pending

Installed global-hotkey 0.8.0 source confirms registration uses MOD_NOREPEAT and every WM_HOTKEY emits Pressed, with Released polled on a separate thread every 50ms. c885247 adds a production-router regression: two distinct Pressed activations without an intervening polled release. The second was incorrectly rejected (red.log); an initial wrongly qualified --exact filter selected zero tests and is not red evidence. Removing Grammar's redundant 250ms latch fixes this deterministic input-loss path. It does not prove the exact cause of the earlier uninstrumented Sandbox failure. Inactive shortcuts and releases remain denied; existing RAII operation coalescing still prevents queued capture/hide races. No text/capture/consent authority expanded.

Two older unit tests assumed two native Pressed events within 100ms were the same physical press. Replaced that invalid assumption with distinct-activation and late-release tests; the independent red assertion remains unchanged. App Rust suite 224 PASS/26 ignored after repair. The native editor fixture adds repeated key-downs without key-up; installed-app checks add native one-dispatch behavior and six successive close/recapture cycles, with exact Copy and unchanged editor assertions. Existing spam assertions and time budgets are unchanged. Clean release/package and fresh guest verification pending. Evidence root D:/dev/grammar-evidence/grammar-shortcut-native-20261003. Same-integrator review, no independent reviewer or live Provider/account use.

### Native shortcut validation completed

Clean product 8e2e0f0270870b6704d263fc3a7e771329e5e94c, tree 4f28a65521346f2187f4c98e3da4486501d206f4, was rebuilt into a fresh target and packaged. Both fresh Medium Sandbox app runs passed 23/23 with identical driver: five-press spam, native repeated key-down dispatch once, six successive close/recapture cycles, precise Copy, consent revocation, stale boundaries and restart. All 88 editor counter records showed zero state-changing messages; content/range rereads passed. Current installed unmodified Chromium keyboard flow passed 11/11 in browser2; actual textarea/contenteditable edits reread, revocation and cleanup passed. All three installs removed completely, all owned guests stopped, final inventory empty.

Initial browser startup failed before installation because the external runner filename lacked the Medium launcher's required Guest-App- prefix. Preserved in browser/restricted-launch-error.txt; name corrected in a fresh guest, guard unchanged. This was not a product test. The earlier consent-checkpoint intermittent failure remains historical; repaired input-loss mechanism is proven independently, its identity with that earlier event sequence is unproven. No blanket claim of zero intermittent failures in physical use.

App Rust 224 PASS/26 ignored; final focused shortcut subset 10 PASS/1 ignored, typecheck/window contract, syntax/governance/diff checks and clean build PASS. Existing warnings remain; no broad lint acceptance. Full hashes, red/green classification and review: D:/dev/grammar-evidence/grammar-shortcut-native-20261003/RESULT.md. ZIP SHA-256 2f43d6415725d73f60bf03513ce31e90b4ab1de291428eb4f50856835826f4ab. Helper-only follow-up pins manual package-shortcut; no hands-on guest launched. Package source remains 8e2e0f0 when later records advance HEAD. Same-integrator review only; physical keyboard/IME, actual Provider, normal profiles, Edge and reproducible builds unverified. Next authorized internal work is desktop consent persistence-failure fault injection (previously code-review only); no live Provider authorization inferred. Mission INCOMPLETE, main/remote unchanged, local Source materiality updated, no canonical exact-12 export or Owner Project UI application.

## Consent persistence failure — installed reproduction, repair pending validation

Evidence D:/dev/grammar-evidence/grammar-consent-save-failure-20261003. Driver-only 1be8be0 locks the task-owned guest settings file with a read-sharing Windows FileStream, causing actual production rename failure without an app test hook or policy/ACL change. Original 8e2e0f0 package: cancellation settles, edited draft and exact disk settings survive, Copy and retry succeed (24 checks PASS in red, despite that directory's preliminary name). It exposed raw settings_previous_stage_failed UI text. Copy itself cleared the error, so that probe did not prove direct-retry error clearing.

Stronger driver-only 4daa5dc retries revocation directly without Copy first. red-direct FAIL confirms successful revocation status displayed alongside the old storage error. Both installations removed completely and guests stopped. Repair clears the stale error after successful revocation, maps the stage failure to actionable wording, and explicitly says previous permissions remain when persistence fails. No backend consent or persistence semantics changed: failed save is NOT revoked consent; retry is required. Typecheck, capture/window contracts and Node syntax PASS. Fresh build/installed 24-check flow pending; unchanged assertions will run on the new package. Same-integrator review only; no real Provider, credentials or user-profile data.

### Consent save-failure validation completed

Clean product/package 175aafac59efef957a21a7209ae227fa46e11c2a, tree 51fb049dd2af3e9572c4794344a11f8bc3dc657d, passed all 24 installed-app checks in green. Test-only 12a7b77 retained the red-direct assertions and additionally verified the synthetic Provider remained active immediately before the faulted revocation. Real file-lock save failure kept exact disk bytes/settings and edited draft, Provider busy cleared, direct retry removed the stale error and set all three acknowledgements to zero with Auto rewrite false, then exact Copy succeeded. All 46 editor counter records showed zero state changes and content/range rereads passed. Current unmodified installed Chromium keyboard flow passed 11/11; host hash matched package, mutation rereads and revocation passed.

The initial probe in red is PASS, not a red regression; red-direct is the preserved actual FAIL. All four installations (probe, red-direct, green, browser) uninstalled completely and all guests stopped, final inventory empty. No backend code changed. Typecheck, capture/window contracts, syntax/governance/diff checks and clean build passed; full Rust suite not repeated for this frontend-only repair, prior results remain separate. Broader storage faults and rapid overlapping consent actions remain unqualified. Same-integrator review only, no real Provider/accounts, physical keyboard/IME, general-profile installation, main adoption or push.

Full hashes, classifications and review: D:/dev/grammar-evidence/grammar-consent-save-failure-20261003/RESULT.md. ZIP SHA-256 aad26d50876a996e8a85025601a5ab1b33886785ae6bc825c34b0c4cfa518adf. Package identity stays 175aafa despite test-only and record commits. Manual helper points to package-consent-save but was not launched for physical input. Next internal work: consent UI overlap qualification (previously code-review only). Mission INCOMPLETE. Local Source materiality updated; canonical exact-12 export and Owner Project UI application not performed.


## Pending review cancellation and consent overlap — local validation in progress

Evidence root D:/dev/grammar-evidence/grammar-consent-overlap-20261003. Added two installed-app schedules: queued grants then revoke, and a real saved grant whose delayed IPC response arrives after review Cancel. The driver delays real IPC fetch responses without fabricating settings or changing the installed binary. Initial attempts failed test setup (disabled Run Deep and frozen Tauri invoke); these are not product red evidence. Production change increments the existing action epoch on review Cancel so late grant continuation cannot submit Deep; already stored permission remains unchanged and requires explicit Revoke to remove. Typecheck and capture/draft/window contracts passed. Fresh binary and installed red/green results pending. No real Provider, login, canonical main adoption or push.


### Consent overlap validation completed — 2026-10-03

Product/package 61c8d9313ad95889b6420243b45ac365d9bd2637 passed 26/26 installed-app checks with unchanged red assertions and 11/11 unmodified installed Chromium keyboard checks. Old 175aafa red3 reproduced pending-review Cancel followed by late Deep submission (two IPC transport requests; synthetic Provider only). Two queued UI grants followed by Revoke already passed. Fix increments the existing consent action epoch on Cancel: late grant continuation submits zero Deep requests, edited draft copies exactly; already stored permission remains and is removed only by explicit Revoke. All 50 editor counter records have zero state changes, exact content/range checks passed.

Test-only driver 9653cf5 holds actual IPC fetch responses in the owned page, preserving backend results. Installed binary remains unchanged; controlled scheduling is not uninstrumented real-world timing. Initial run1/red disabled-button setup and red2 frozen-invoke interception failures are preserved as harness failures, not product reproduction. All six installations removed completely and guests stopped; inventory empty. Same-integrator separate review only. Typecheck, capture/draft/window contracts, syntax/governance self-test, build/package and diff checks passed. Full Rust suite not rerun for this frontend-only fix; existing warnings retained.

Evidence and complete hashes: D:/dev/grammar-evidence/grammar-consent-overlap-20261003/RESULT.md. ZIP SHA256 b80dbbd57d393c9d7007d20ec9cdd3eb1cb16fb9ba1c51441536cb0c3982d8fe. Package remains bound to 61c8d93 when record commits advance HEAD. Updated manual helper was not launched. Next internal work is pending-consent capture/provider binding qualification. Actual Provider/accounts, physical keyboard/IME, ordinary profiles and Edge remain NOT_RUN. Mission INCOMPLETE; canonical main and remote unchanged. Local Source materiality updated, no canonical exact-12 export or Owner Project UI application.


## Pending consent capture/provider binding — validation in progress

Old installed 61c8d93 reproduced capture replacement while a real grant response is delayed: its late continuation submitted Deep for the new capture. Initial provider case was a driver check() assumption failure because selection is queued, corrected to click with later actual settings readback. Evidence root D:/dev/grammar-evidence/grammar-consent-binding-20261003. Fix pins the reviewed session/generation before saving and compares it after completion; provider change invalidates the action before queuing its save, with a provider-ref check at continuation. New explicit consent on the new capture must still run synthetic Deep and retain edited draft. Typecheck, capture/draft/window contracts and syntax passed. Installed red/green validation pending; no real Provider/account use, no main adoption or push.


### Pending consent binding validation completed — 2026-10-03

Product/package 910730d70edaeb1a2abc1823505b3c68375c4ee9 passed installed-app 28/28 and installed unmodified Chromium keyboard 11/11. Old 61c8d93 red3 reproduced both changed-context continuations (one rewrite IPC request each); provider-switch attempt was backend-denied by disclosure, not proof of live data leakage. Green used identical driver/assertions: zero stale dispatch, new capture disclosure retained, new explicit consent successfully ran synthetic Deep only for that new session/generation and preserved edited Copy. All 54 editor counter records showed zero state changes and exact content/range rereads passed.

Initial red provider check() assumption failure is separate from its valid capture reproduction. Red2 failed the prior four-second synthetic child's active precondition before new cases under concurrent build; retained FAIL, exact timing cause unproven. Red3/green ran after build with unchanged budgets/assertions, not a load-robustness claim. All five installations uninstalled completely and guests stopped, final inventory empty. Typecheck, capture/draft/window contracts, syntax/governance self-test, fresh build/package and diff checks passed. Full Rust not rerun for frontend-only repair; existing warnings retained. Same-integrator separate review only.

Evidence and full hashes: D:/dev/grammar-evidence/grammar-consent-binding-20261003/RESULT.md. ZIP SHA256 947ac7feb9b3dc7576965c9009b92b70ed1eecc154cf3fb793cb7a49065ac9df. Package remains bound to 910730d when records advance HEAD. Manual helper updated but not launched. Next internal work: pending-consent intent changes and page reload qualification. Physical keyboard/IME, live Provider/accounts, normal profiles, Edge remain NOT_RUN. Mission INCOMPLETE; main/remote unchanged. Local Source materiality updated, no canonical exact-12 export or Owner Project UI application.


## Pending consent intent/reload — validation in progress

Evidence root D:/dev/grammar-evidence/grammar-consent-intent-20261003. Installed 910730d reproduced late grammar and translate requests after the user changed mode while a real grant response was held. Four schedules cover mode, target language, auto reference language, and WebView reload. Fix invalidates pending consent before active intent changes are queued, compares the reviewed intent after response, and reads the actual saved acknowledgment before an immediate auto continuation rather than waiting for a React effect. A new explicit review must still submit the exact current intent; reload must not replay the pending request or restore a draft. Typecheck, capture/draft/window contracts and syntax passed. Installed validation pending; no actual Provider/login, main adoption or push.
Reload also reproduced normal Close failure because the frontend had no token while the backend capture remained active. Added a read-only identity command for explicit Close only, strict identity parser, and unchanged exact-token Dismiss validation; a delayed identity-response test requires a new capture to survive. Rust app suite 225 PASS/26 ignored after fixing the initial Tauri async-command return-type compile error. Response serialization is identity-only. Red3 stages the final driver before launch and reproduces all three stale-intent dispatches plus reload Close failure; red2 has an inconclusive loaded-script identity from a staging error and is not used for source-bound validation. Full installed 33-check qualification pending.


### Intent/reload validation completed — 2026-10-03

Product/package 419ee3c847a0a5f6ddd6ed9044da0e12657934f2 passed installed app 33/33 and unmodified installed Chromium keyboard 11/11 in fresh Medium-integrity Windows Sandboxes. All 62 native editor counter records showed zero state changes; exact content/range and Copy assertions passed. Red3 on 910730d reproduced stale mode, target and auto-reference continuations (two rewrite transport requests each) and reload Close failure. Green uses the identical f2340b28692504ae9c37b6c6cb69d5f4eab00a14ef7e02f823e77bb37861c044 driver, zero old continuation requests, successful newly consented current-intent synthetic Deep, no reload replay/draft restoration, normal explicit Close, and preservation of a newer capture during a delayed identity read.

The fix invalidates pending grants on active intent changes before queued saves, binds completion to reviewed intent, and reads the actual saved acknowledgment before auto continuation. Explicit Close after reload retrieves only opaque session/generation, then uses unchanged exact-token Dismiss validation. Newer frontend capture aborts that Close. This is controlled real IPC response scheduling on an unchanged installed binary, not uninstrumented timing or live Provider evidence. Persisted consent survives reload; reload is not revocation.

Rust app 225 PASS/26 ignored; typecheck, capture/draft/window contracts, syntax, governance/self-test, clean release build and package passed. Initial Tauri return-type compile failure is retained. Red initial label-selector errors are harness failures; red2 staging after launch makes loaded-script identity inconclusive and is excluded from exact-source proof. Red3 is the authoritative reproduction; no assertions or budgets relaxed. Same-integrator separate review only.

All five guest installs removed completely; all five owned guests stopped; final inventory empty. Browser sandbox enabled, installed host matches build hash, profile removed and installed extension unchanged. Only keyboard toolbar activation qualified; Mouse/Invoke fallback failures remain recorded. No real Provider/login, physical keyboard/IME, ordinary profiles, Edge, main adoption, push, release or canonical exact-12 export/Owner Project UI application. Mission remains INCOMPLETE.

Evidence and full hashes: D:/dev/grammar-evidence/grammar-consent-intent-20261003/RESULT.md. Package remains bound to 419ee3c when record commits advance HEAD. Local Source materiality updated. Next is limited Owner-assisted physical keyboard/IME trial; manual helper pins package-consent-intent but was not launched.


### Owner input session prepared — 2026-10-03 14:01 UTC

Owned Sandbox 19f3ae29-4bc2-49f3-84b9-77df7ade0579 is intentionally left open for Owner input. Product/package 419ee3c847a0a5f6ddd6ed9044da0e12657934f2, Medium launcher and explicit guest WebView2 154.0.4258.53. READY_AWAITING_OWNER_INPUT and GUIDE_SHOWN verified. Auto rewrite false; all Deep consent absent. App/editor started. Networking and host clipboard redirection disabled; read-only payload and fresh task-owned output only. No actual Provider/account use.

Physical keyboard and IME remain NOT_RUN. Empty guestLanguageTags is unknown IME availability, not a successful test. User should use only the synthetic editor, select a sentence, Ctrl+Shift+G, edit draft, Copy, and manually paste into Synthetic Other Editor; original source must remain unchanged. Try Korean composition only if already available. Repeat close/capture three times. Do not install input features or log in.

Runner waits at most 60 minutes from 2026-10-03T14:01:02Z, then terminates its owned children and attempts production uninstall. finish.request in output ends the wait early. No claim of future cleanup completion until uninstall.json is inspected. Closing the VM early may prevent an uninstall receipt. Agent is not continuing background work; the bounded guest runner remains active. Inspect receipt and stop this exact owned Sandbox at next follow-up. Main/remote unchanged; mission INCOMPLETE.

Evidence: D:/dev/grammar-evidence/grammar-hands-on-419ee3c-20261003/STATUS.md and output receipts.


## Physical trial closed; additional automated cancellation audit

The prepared physical-input guest was ended using its finish.request. Production uninstall receipt reports complete, no installation/registration/user-data/process remnants. Exact owned Sandbox 19f3ae29-4bc2-49f3-84b9-77df7ade0579 stopped; inventory empty. No physical-input observations were supplied, so keyboard/IME remain NOT_RUN.

Additional agent-run tests: Provider contracts seven groups PASS (mixed static wiring and behavior, not live Provider acceptance); Chromium production core 215 assertions and worker 49 tests PASS; host Rust 16 PASS; app Rust 227 PASS, 26 ignored. Two new tests invoke the real app cancel_cli_operation helper: valid bound rewrite remains protected, invalidated rewrite cancellation reaches the executor pre-dispatch guard, reservation remains busy until finish, stale finish cannot remove a newer reservation, unbound cancellation sets its owned flag without claiming completion, and empty cancellation is inert. Only in-memory AppState and synthetic text used; no real CLI, account, profile, clipboard or desktop accessed. Existing compiler warnings retained.

Same-integrator review only. These tests qualify the exercised deterministic orderings, not all cancellation interleavings or native cleanup timing. No new production defect established or product change made in this round. Existing package/source 419ee3c and its prior 33 app/11 browser receipts remain the installed qualification; test-only commits do not relabel that binary. No rebuild needed for test/document-only changes.

Physical keyboard/IME, real Provider, ordinary profiles, Edge and whole-product acceptance remain unqualified. Main/push/release unchanged. Next optional external evidence is Owner physical input or separately authorized live Provider; no repeated manual-session launch without an expressed desire to test. Internal safe development remains authorized if a concrete defect or gap is selected.

Evidence: D:/dev/grammar-evidence/grammar-post-trial-audit-20261003/RESULT.md.


## Same-intent cancellation repair — 2026-10-04

Same-intent retry cancellation defect reproduced on test commit 00b5d0a: after invalidation and another begin with the same token/intent, the old Provider reservation was mistaken for a valid new operation and cancellation was skipped. Product 6444140c3986b61ffd7493fee7a626fc28c80cee carries the existing rewrite attempt into ProviderOperation and requires matching attempt plus intent before protecting it. No timeout, consent or cleanup contract changed.

Unchanged regression assertion passes after repair; it checks old operation identity, preservation of the new capture attempt and protection after the new operation is reserved. This is deterministic production-helper execution, not measured installed-UI reproduction or live Provider inference. Full app Rust 228 PASS/26 ignored; host Rust 16 PASS; typecheck and seven Provider contract groups PASS. Existing warnings retained. Same-integrator separate review recorded in REVIEW.md; not independent review.

Clean build and package passed. Installed app 33/33, installed unmodified Chromium keyboard 11/11 PASS in separate Medium-integrity Sandboxes. All 62 editor counter records show zero state changes. Existing driver f2340b28692504ae9c37b6c6cb69d5f4eab00a14ef7e02f823e77bb37861c044 unchanged. Browser enabled-field mutation rereads pass; browser sandbox on, host hash matches build, profile removed, installed extension unchanged. Both uninstalls complete; both guests stopped and inventory empty. Explicit guest WebView2 runtime and synthetic Provider remain test conditions. Mouse/Invoke fallback does not qualify mouse activation.

Physical keyboard/IME, real Provider/accounts, ordinary profiles, Edge, reproducible build and whole-product acceptance remain NOT_RUN/unqualified. Main, remote and release unchanged; mission INCOMPLETE. Manual helper updated to this package but not launched. No canonical exact-12 Source export or Owner Project UI application. Next internal option: cancellation/shutdown admission review; physical input remains optional and requires actual observations, not repeated automatic session launch.

Evidence/full hashes: D:/dev/grammar-evidence/grammar-cancel-attempt-20261004/RESULT.md. Record commits do not relabel the 6444140 package.


## Managed shutdown admission — 2026-10-04

Managed Provider shutdown admission defect reproduced in 057811e: shutdown_providers returned successfully, but the manager still admitted a late probe. Product 34aae4fd8a2bfc74693f5df4200f3a36d20a0da1 closes manager admission under its existing lock, cancels and retains the current operation until completion, and permanently closes desktop Codex cache admission. Temporary headless cache cleanup retains its original semantics. Failed/abandoned desktop teardown does not reopen admission or count as cleanup completion; Provider work then requires restart. No budget or consent policy was relaxed.

Actual production-helper tests cover successful shutdown, late probe/rewrite denial, cache get rejection, waiting for existing ownership, abandoned wait and repeated teardown. App Rust 231 PASS/26 ignored; host Rust 16 PASS; typecheck and seven Provider contract groups PASS. Same-integrator review only, in REVIEW.md. Existing warnings retained. The failure log remains in red.log.

Clean build/package source hashes match before/after. Fresh Medium Windows Sandbox installed app 33/33 and unmodified installed Chromium keyboard 11/11 PASS. Native editor counter records 62, state changes zero; exact Copy and source/range readback pass. Browser host matches receipt, sandbox enabled, profile removed and installed extension unchanged. Both uninstalls complete and both guests stopped; inventory empty. The app guest started slowly; cause not established. One host read preceded app-result publication and failed; completed receipts were subsequently verified without changing/retrying the test or budgets.

Shutdown defect proof is deterministic backend execution; the installed suite is general regression, not an interactive tray-exit race qualification. Independent account/login subprocess paths, real Providers/accounts, physical keyboard/IME, ordinary profiles and Edge remain unqualified. No main adoption, remote push, release or canonical exact-12 export/Owner Project UI application. Mission INCOMPLETE. Next internal qualification: installed synthetic active-Provider tray-exit and cleanup flow; do not infer authority for login or live inference. Manual helper updated but not launched.

Full evidence/hashes: D:/dev/grammar-evidence/grammar-shutdown-admission-20261004/RESULT.md. Records after 34aae4f do not relabel its package.


## Installed tray-exit qualification — target unavailable

Installed natural tray-exit qualification remains NOT_RUN/TARGET_UNAVAILABLE. All six attempts have FAIL receipts; none proves normal exit. Product/package is unchanged 34aae4fd8a2bfc74693f5df4200f3a36d20a0da1. Existing prior app33/browser11 qualification is retained independently.

Run1 failed before the new case: newly compiled synthetic child used dynamic CRT and exited -1073741515. Recompiled fixture with rustc -C target-feature=+crt-static; no runtime/security installation. Runs2-6 completed the existing 33 checks but could not identify/activate the owned tray icon. Forced cleanup is not counted as normal exit. All six production uninstall receipts complete; all six guests stopped, inventory empty. Failed-case runtime session deletion before VM disposal was NOT_PROVEN; disposable guest teardown removed the environment.

Run3 shell UIA diagnostics exposed only shell container nodes. Run4 ExistingLogin helper did not improve discovery, so a pure integrity-level explanation is unproven. Run5 focused XAML element had UIA ProcessId 0; helper now resolves actual hit-test window owner and revalidates it before clicks. Run6 confirmed owner was the guest shell but Win+B/right navigation stayed on the Start element. Root cause of unavailable notification-area target remains unknown. No screenshots/OCR, arbitrary coordinate clicking, credential use, user profiles, host clipboard or security changes. Product runs Medium; later gesture broker uses the guest existing-login context, a qualification condition rather than general user behavior.

New optional GRAMMAR_TRAY_EXIT_TEST=1 test is Sandbox-only. It starts a synthetic 90-second finite child through production rewrite IPC, verifies child PID and owned runtime session, requests an actual shell icon/app Quit menu click, then requires natural app exit code0, child exit, session deletion and unchanged synthetic editor. Existing final forced-cleanup checks remain separate. Helper uses exact installed app path/PID, named UIA target and actual hit-test owner. Broker waits at most10min; driver waits30s for gesture,20s for natural exit. No thresholds relaxed. Failure diagnostics retained. These are test additions, not a new product capability. Final artifact-hash receipt fields were syntax checked only, not re-executed; actual tested driver hashes are in runs.json.

Node syntax, PowerShell parse, fixture compilation, governance and diff checks PASS. No full Rust rerun for test-driver-only changes; no product rebuild. Review is same-integrator only. Mission INCOMPLETE; main/push/release unchanged. Next: establish an accessible owned tray target or a separately qualified application exit UI; do not repeat the same blind navigation. No user manual trial required for other autonomous work. Live Provider/physical keyboard/IME remain NOT_RUN.

Evidence: D:/dev/grammar-evidence/grammar-tray-exit-20261004/RESULT.md.

## Confirmed window Quit and owned registration diagnostic — 2026-10-04

Product f90f3d58719bfe2f0a230a680f3495c63dbf069e adds a named, keyboard-focusable power button (Quit Grammar) to the app header, including signed-out/onboarding/settings views. Confirmation describes draft loss and stopping active Deep; cancelling leaves the draft and operation intact. Close/Dismiss still hides the widget. The command requests AppHandle.exit(0), using the existing ExitRequested cancellation/admission/cleanup gate. Cleanup failure still leaves the app open with the existing error; no direct termination, elevation, consent or deadline changes.

Fresh restricted-Medium Sandbox 6ca76969-99fc-4837-8705-d3ea2ad6cf14: installed unchanged package passed all35 app checks. Actual Quit button confirmation was dismissed first, preserving edited draft and live synthetic child; then accepted. Natural app exit0, child gone and owned runtime session absent were observed434ms after the request (single observation, not a performance bound). Child was about353ms into its90-second finite lifetime at Quit, so this cannot be natural fixture expiry. All64 editor counter records have zero state changes and exact text/range rereads passed. Uninstall complete with no installation/registration/user-data/process remnants; guest stopped and inventory empty. Explicit guest WebView2 runtime and synthetic CLI remain test conditions. Fixture/process cleanup after proof is not counted as the normal exit itself.

Full app Rust231 PASS/26 ignored; typecheck, window contract, seven Provider contract groups, governance negative checks and syntax/diff checks PASS. Clean fresh app/host build and package before/after source identity match. Test-only successor07fc7c1 adds a30-second remaining-lifetime guard and waits for dialog acknowledgement; its driver hash is179f79c717438d815bde21c78766c766b372394c619ef7a003aac4d76a55c014. Installed binary remains f90f3d5, not the later test/record commits. Existing warnings retained. Same-integrator review only. Chromium source is unchanged; browser tests were not repeated for this desktop-only change, and prior34aae4f browser11 qualification is not relabelled as a new-package browser run.

Tray route remains TARGET_UNAVAILABLE. A separate bounded registration comparison found Shell_NotifyIconW(NIM_ADD) false in MediumLaunch's restricted token and true in ExistingLogin High; raw UIA saw shell system icons and an overflow button for a separate High control icon. Medium registration recorded last-error5, but that API does not document a reliable GetLastError contract. Restriction changes groups/privileges as well as integrity; actual Grammar API return was not instrumented, so exact Windows cause is not established. This explains why promoting only the gesture observer did not necessarily help. No app elevation, shell replacement or security setting change. First diagnostic guest uninstalled fully; both diagnostic guests stopped. Detailed limitations, duplicate helper invocation and overwritten baseline diagnostic are preserved in grammar-tray-discovery-20261004/RESULT.md. Do not call the new window route a tray PASS.

Evidence: D:/dev/grammar-evidence/grammar-window-exit-20261004/RESULT.md, build-receipt.json, package/MANIFEST.json, output/app-result.json and output/uninstall.json. Registration comparison: D:/dev/grammar-evidence/grammar-tray-discovery-20261004/RESULT.md.

Next internal qualification: controlled cleanup failure and retry through the installed window Quit path, retaining explicit fail-closed evidence. Real Provider/accounts, physical keyboard/IME, normal profiles, Edge, independent login subprocess lifetime, tray UI and whole-product acceptance remain unqualified. No Owner input needed for the completed automated tests. Mission INCOMPLETE. Main4e83398 and its three pre-existing untracked files are preserved; no push/adoption/release. Source materiality: new candidate user-facing Quit capability; current handoff/state updated, canonical-main-only exact12 Source package and Owner Project UI unchanged. The packaged HANDOFF is the pre-qualification source snapshot; this later record does not modify/relabel that package.

## CLI filesystem cleanup retry — 2026-10-04

Product26b30453243efb814a1ad686ad23120b5b3fccb2 repairs an installed shutdown retry defect. Old package f90f3d5 plus test c53817d retained the app and showed Shutdown incomplete while a synthetic runtime file was locked, but still could not exit after the lock was released and Quit was confirmed again. Red FAIL is preserved. Failed CLI workspaces now retain exact in-memory path/marker ownership; new requests stay blocked while they remain. Explicit desktop shutdown can retry the pending filesystem cleanup. Process/reap failure remains terminal and is never cleared by successful file deletion. Manager/Codex admission remains closed; no silent replay, new Provider selection or consent change.

The retry snapshot is taken at shutdown_providers entry; work enqueued after that snapshot is reported rather than retried inside that helper call. The outer exit handler may already have requested cancellation before helper entry. The same existing absolute8-second shutdown deadline spans waits and retry; the original request/cleanup budgets and8 filesystem attempts are unchanged. A blocking filesystem syscall is not physically preemptible. Independent Codex persistent-workspace/login cleanup is not newly qualified; headless recovery policy remains unchanged. Retained work is limited by blocked new admissions and callers' existing bounded concurrency.

Evidence: D:/dev/grammar-evidence/grammar-shutdown-retry-20261004. Red installed retry FAIL; first green attempt also FAIL before the new test, because the4-second synthetic slow child had finished before the failed-consent-save precondition. Test-only846bbf5 moves PowerShell settings-lock preparation before starting that finite Deep. Lifetime, active-operation assertion and shutdown assertions unchanged. Original and revised driver hashes recorded separately; do not call the entire final driver byte-identical to red.

Final unchanged product package: app36 PASS (green2), modified isolated extension + installed-host synthetic Deep44 PASS (fixture), unmodified installed extension real keyboard toolbar11 PASS (browser), all in fresh restricted-Medium guests. App first held open with child exited and session retained, then actual Quit retry after lock release produced natural exit0 and session removal before forced test cleanup.64 editor records: zero state changes, exact source/range rereads.1740ms is one aggregate first-Quit/failure/retry observation, not a latency bound; synthetic child age at first request about3176ms of90seconds. Both browser host hashes match the build; browser sandbox on. Fixture registration/host/profile removed; installed extension unchanged in the unmodified run. All five uninstall receipts complete and all five guests stopped; final inventory empty. Red failed runtime-session deletion before VM disposal remains NOT_PROVEN, distinct from successful final cleanup.

App Rust233 PASS/26 ignored, including actual Windows file lock and invalid-marker/deadline retention tests; host16 PASS; seven Provider contract groups, frontend build, governance+self-test, syntax/diff and clean fresh build/package/provenance PASS. Existing warnings remain plus two desktop-only retry helper dead-code warnings in the library target; nothing suppressed. Same-integrator separate review only (REVIEW.md).20 package files verified against manifest and build receipt; later test/record commits do not relabel product26b3045. Package HANDOFF is the pre-qualification source snapshot.

Next safe internal work: persistent Codex cleanup failure/retry review using synthetic fixtures only. Tray, real Provider/accounts, physical keyboard/IME, ordinary profiles, Edge, independent login subprocess lifetime and full product acceptance remain unqualified. Mission INCOMPLETE. Main4e83398 and its three existing untracked files preserved; no adoption/push/release. Candidate-source materiality: cleanup ownership/retry behavior changed; current state/handoff/ledger updated, canonical-main-only exact12 Source export and Owner Project UI unchanged.

## Persistent Codex workspace cleanup ownership — 2026-10-04

Product0848a3eb6204e272c242650ae90bc5752d07b973 repairs a separately reproduced persistent Codex watcher defect. Test-only98c8660 failed on Windows because a locked runtime workspace had no retained owner after teardown. The watcher had consumed close_before and cached its filesystem failure permanently. Now watcher and spawn-failure paths retain failed filesystem ownership in the existing shared queue. Cache admission checks that queue; process/reap/reader receipts remain separate and terminal. Desktop shutdown waits for process receipts, retries previously pending filesystem work under its unchanged absolute8-second deadline, and verifies both results. Headless shutdown_checked still returns unresolved filesystem cleanup; headless recovery policy is unchanged. No new child, inference replay or reopened admission.

Actual Windows process-only regression: original assertions PASS after repair, with an owned finite PowerShell child, real FileShare.Read lock, production Job/watcher/readers/cache and actual folder deletion. Separate isolated test calls the real desktop shutdown_providers: locked retry fails; releasing the handle permits cleanup; admission remains closed. Both deliberately run alone because the queue is process-wide. App Rust233 PASS/28 ignored (two new ignored tests explicitly executed separately and passed); host16 PASS; seven Provider contract groups, governance negative tests and diff checks PASS. Red log and its synthetic residual folder are intentionally retained as evidence, not counted as product cleanup. No real Codex executable, account or app-server protocol handshake was used by the new regression.

Clean fresh source/build/package verified; all20 package files match manifest hashes and lengths. Three fresh restricted-Medium Windows Sandboxes passed installed app36, modified isolated Chromium + installed-host synthetic Deep44, and unmodified installed Chromium keyboard11.64 desktop editor records show zero state changes with exact source/range rereads. App natural exit0, child gone and session absent proved before forced test cleanup;1422ms is one aggregate first-Quit/failure/retry observation, not a performance limit. Synthetic child age about2348ms of90seconds at first Quit. Both browser host hashes match the build; sandbox on, registration/profile/process cleanup confirmed, installed extension unchanged in unmodified run. All three production uninstalls complete, all exact guests stopped, inventory empty. App driver unchanged; modified Chromium fixture source pin updated only to the new package commit with existing guards preserved.

Same-integrator separate review only (REVIEW.md), no independent reviewer. Installed app regression uses synthetic CLI: do not call it installed persistent Codex protocol or live acceptance. Process-spawn-error retention is code-reviewed, not fault-injected this round. Blocking filesystem syscalls remain non-preemptible; no timeout/threshold relaxation. Existing warnings/formatter drift not suppressed. Tray, physical keyboard/IME, real Provider/accounts, ordinary profiles, Edge and reproducible-build/full-product acceptance remain unqualified.

Evidence/full hashes: D:/dev/grammar-evidence/grammar-codex-cleanup-20261004/RESULT.md and RESULT.json. Product/package0848a3e is not relabelled by later record commits; packaged HANDOFF remains the pre-qualification snapshot. Mission INCOMPLETE. Main4e83398 and its three existing untracked files unchanged; no adoption/push/release or canonical-main-only exact12 Source/Owner Project UI application. Next safe internal work: synthetic persistent Codex protocol connection/handshake failure and client-drop ownership qualification; no login/live Provider authority inferred.

## Synthetic Codex protocol client-drop qualification — 2026-10-04

Added an explicitly isolated Windows stdio regression with three cases: successful initialize/initialized exchange (server receipt observed), invalid JSON during initialization, and dropping an unfinished handshake after the synthetic server received initialize. Actual production from_io, perform_handshake, ChildShutdown Drop, watcher, Job and shared workspace queue are exercised. The finite task-owned PowerShell server is not Codex; no executable resolver/version probe, CodexHome, credentials, account or live Provider is used.

All three cases PASS with real FileShare.Read locks. After dropping the sole client without a shutdown waiter, the retained Windows process handle is signaled, reader ownership retired and the locked workspace remains explicitly owned in the queue. Releasing the lock then retrying removes the exact session; queue is empty before and after every case and task-owned fixture roots are removed. The test is ignored in the parallel default suite because it mutates the process-wide queue, and was explicitly run in its own process. Initial pass and strengthened server-notification-receipt pass both preserved. Actual elapsed test time is not a performance bound; production budgets unchanged.

Full app Rust233 PASS/29 ignored, plus the new isolated test PASS (three scenarios); governance+self-test and diff checks PASS. Existing warnings retained. Separate same-integrator review found no new product defect. Test-only change; no rebuild or Sandbox rerun required, no new product or live acceptance claim. Installed product/package remains0848a3e with its previous app36/Chromium44/unmodified keyboard11 evidence. No active Sandbox, main/remote/release unchanged, mission INCOMPLETE. Next internal review is process-spawn-failure cleanup ownership, which this test does not exercise. Evidence: D:/dev/grammar-evidence/grammar-codex-protocol-20261004/RESULT.md. Current candidate records updated; canonical-main-only exact12 Source and Owner Project UI unchanged.

## Desktop direct Apply restored as highest priority — 2026-10-05

Owner correction: desktop Apply is a core product requirement, not permanently replaced by Copy. Current status explicitly says restoration in progress; unrelated runtime review is deferred. Historical r3/r4 removal reports and R1/R2 failures remain preserved. Existing installed product0848a3e is still Copy-only; no restored-UI claim.

Implemented src-tauri/native/tsf_apply.hpp, an experimental target-thread TSF write-session component, and scripts/mission/native-apply-tsf-probe.cpp. It checks a single-use grant, expiry, revision, eligibility, full source, exact UTF-16 selection extent and composition/read-only state inside the edit session, replaces once and rereads. It never restores a document or silently retries after a possibly mutating call. The component is not yet loaded, registered, packaged or wired to Tauri/capture. Grant/revision/eligibility are fixture inputs; real target-side identity/revision/privacy binding remains required. This is implementation plus native feasibility, not desktop feature completion.

Actual Medium Windows Sandbox RichEdit50W: final13 checks PASS (normal replacement, replay, source/selection changes, revoked/expired/ineligible/stale grants, edits/selection/revocation/revision changes after asynchronous request admission, and same-text wrong-offset selection). Native content rereads match; only the normal case calls SetText. Standard Win32 Edit: normal case FAIL without mutation; TSF full-text read returns S_OK with zero units while native control contains synthetic text. Cause of that TSF exposure difference not established; retain failure, do not bypass source validation or claim standard Edit support. Prior exploratory3/7/13 RichEdit passes and failed standard runs are separate evidence. Field-eligibility flag rejection is not real password privacy qualification; readonly/composition, Unicode/IME/undo and malformed-provider outcomes remain unqualified.

C++ compiled against installed Windows SDK with /MT /W4 /WX; final build succeeds without warnings. Initial link failed due to unnecessary msctf.lib, corrected to COM/uuid/user32 linkage; failure retained. Existing native_edit Rust safety12 PASS; product Rust/frontend unchanged so no package rebuild or unrelated full regression. Same-integrator review only. Six owned guest runs completed and all exact guests stopped, inventory empty. No service registration, injection, clipboard/network/credential use, installation or security changes; only owned synthetic controls. Original package and its old installation receipts are not relabelled.

Next is still desktop Apply restoration: complete opt-in target-side TSF capture/revision/eligibility and authenticated bounded local bridge, connect the reviewed draft and explicit Apply to the exact target context, then qualify actual supported external editor scenarios in Sandbox. Investigate standard Edit's empty TSF exposure separately; do not infer all editors work from RichEdit. See src-tauri/native/README.md and D:/dev/grammar-evidence/grammar-desktop-apply-20261005/RESULT.md. Mission INCOMPLETE; main/push/release unchanged; candidate state/handoff updated, canonical exact12 Source/Owner UI untouched.

## Actual TSF capture and revision binding — 2026-10-05

Continuation from6d2c5bc implements tsf_capture.hpp. Actual TSF read sessions now obtain source and exact selection, then issue a single-use grant tied to context, native window, process/thread and revision. Explicit opt-in is required to construct an enabled binding. It checks plain input scope, password/read-only state, TOM protected/hidden formatting, visibility and context/document identity before reading. Sink and grant references are weak; destroying or disabling the binding revokes pending work and removes subscriptions. The original write-session component now requires a binding callback and checks it again before SetText. C++ exceptions are contained at edit-session COM entry points, preserving an uncertain outcome after a possibly mutating call; allocation-failure injection remains NOT_RUN.

Native investigation found and repaired a new reentrancy failure: RichEdit executes a queued TSF write inside EM_SETPASSWORDCHAR before updating its password state. Earlier qualified/privacy runs record one unwanted mutation in that scenario. The target-thread component now revokes at entry to password/read-only/format/style/destruction messages using a local window subclass before forwarding. It neither reads keys/text from messages nor injects into another process. Every TSF write completion advances revision, including input-scope-only changes; changing a scope to password and back cannot revive a grant. The final regression preserves the zero-mutation assertion. Earlier first/diagnostic failures were caused by a hidden synthetic window; the fixture explicitly shows its own guest window, without removing visibility checks.

Owned restricted-Medium Sandbox capture/binding: 23 scenarios,40 reported checks PASS, including actual Korean/emoji source preservation, normal Apply, text/selection/scope changes and restoration, post-admission changes, wrong window, opt-out, protected/hidden/password fields, replacement capture, destroyed binding, disable and replay. Negative Apply cases require zero GetText and zero SetText; actual editor contents are reread and the second editor remains unchanged. The transaction-only13 checks also pass. Scope of proof remains the owned in-process RichEdit component; no installed-widget or external-editor acceptance. Same-integrator review only. Both probes compile with /MT /W4 /WX against the installed SDK. Governance and negative validator tests pass. No Rust/frontend product path changed, so no new app package or unrelated full-suite run; installed0848a3e and its receipts remain unchanged.

Prototype capture admission5s/grant30s are conservative expiry bounds, not product performance or Deep budgets. Remaining work: authenticated bounded cross-process bridge, opt-in TSF service activation/lifetime, focus/navigation semantics, real widget capture/Apply wiring, then external supported-editor qualification. Standard Edit's empty TSF exposure is still unresolved. Composition/physical IME, undo, live Provider and general profiles remain unqualified. Evidence and exact source/binary hashes: D:/dev/grammar-evidence/grammar-tsf-binding-20261005/RESULT.md and hashes.json. Mission INCOMPLETE, Copy-only temporary containment, no main adoption/push/release/Source UI application.

## Local Apply channel admission — 2026-10-05

Continuation from ffc3932 adds apply_channel.hpp and a real child-process/pipe native probe. A local first-instance/single-instance pipe uses an explicit logon-SID DACL and rejects remote clients. Admission pins a trusted caller-supplied process handle, checks token identity/session/Medium integrity metadata and kernel pipe-client PID, and requires a random256-bit capability plus exact sequence. Header validation precedes body allocation, text is bounded UTF-16LE without NUL/unpaired surrogates, and header/body share one absolute deadline. Invalid requests revoke admission and clear old/partial content; returned requests do not retain the capability. Pipe I/O is nonblocking on one owning worker, without detached readers or pending-overlapped cleanup. Only token identity metadata is queried, not account credentials.

Owned restricted-Medium Sandbox final28 cases PASS with actual child processes. Wrong peer/capability, replay, wrong sequence/version/command, revocation/disable, dead peer, wrong server PID, duplicate pipe server, malformed/truncated/oversized payload, blocking-mode refusal and UTF-16 negatives passed. Maximum131068-byte body received in531ms in the final unloaded fixture; delayed header plus partial body rejected at157ms under one150ms deadline (fixture assertion150<=elapsed<225, not a product performance guarantee). Every child exited naturally; final probe exit0. First23-case run failed maximum-body transfer because the nonblocking fixture writer attempted one oversized write; fixed by bounded chunks, without deadline extension. A later28-case run failed two stale-buffer assertions because the fixture skipped the receiver when a child exited before connection observation; it now invokes the same fail-closed receiver and preserves the original assertions. Earlier failure logs remain separate.

This is inbound transport admission, not a completed/authenticated deployed bridge or desktop feature. The fixture creates/pins its own peer and transfers the capability through exactly one inherited anonymous-pipe handle. Trusted Grammar binary discovery/bootstrap, bidirectional replies, capture-ID/revision mapping, STA dispatch, queued-work disconnect revocation, service activation/focus lifecycle and widget integration are not implemented. The channel alone must never authorize a document mutation. Different-user/logon/integrity rejection and allocation faults are implemented checks or untested paths, not negative-environment qualification. Existing TSF native tests were not relabelled as cross-process Apply evidence.

Build /MT /W4 /WX and governance+self-tests PASS; same-integrator review only. No existing Rust/frontend path changed or new app package built. Installed0848a3e stays Copy-only; mission INCOMPLETE. Four exact owned Sandboxes stopped, all observed child processes exited, final inventory empty. Main/untracked owner files, remote, accounts, normal profiles and canonical exact12 Source/UI unchanged. Evidence/source/binary hashes: D:/dev/grammar-evidence/grammar-apply-channel-20261005/RESULT.md and hashes.json. Next priority remains the trusted opt-in activation and target dispatcher connecting this admission to actual TSF captures and explicit Apply, then widget/external-editor qualification.

## Target dispatcher and cross-process Apply component — 2026-10-05

Continuation from fc60227 connects real TSF capture and Apply to the admitted local pipe in an owned two-process fixture. Wire version2 adds capture ID; target STA issues IDs, retains full source/range, returns selected text only, and consumes grants for explicit Apply. One pending operation bounds COM ownership. A connection lease pins process/pipe handles and denies reads/mutations after peer death/disconnection. Explicit wire cancellation checks the capture ID. Synchronously executed failed callbacks retain their actual result, including mutation_unconfirmed. This is still experimental native code, not installed-widget integration.

Final restricted-Medium Sandbox: cross-process12 PASS with actual text rereads; channel33 PASS; binding40 checks/23 scenarios and transaction13 PASS. Normal mutation is exactly once, and stale IDs, source/selection ABA, queued cancellation, password transition, disconnected peer and replay never mutate. First/diagnostic runs preserved nine capture rejections: default RichEdit lacks an explicitly allowed input scope. Qualified fixture sets IS_TEXT; unknown scope is retained as a zero-read denial, with no production check relaxed. Separate same-integrator review only. C++17 /MT /W4 /WX builds pass.

Deployment investigation: generic TSF RegisterProfile returns80004005 in restricted Medium, while TF_RP_LOCALPROCESS succeeds. Exact cause is not proved; machine TIP write access is denied. Random task GUID leaves no HKCU/HKLM key, no service/keyboard activated, and probe exits naturally. Unregister returns E_FAIL and process-local state ends with that process. This is unavailable general registration, not a successful installation or grounds to escalate. Six owned guests stopped; final inventory empty. All failure records and source/binary hashes: D:/dev/grammar-evidence/grammar-apply-dispatch-20261005/RESULT.md and hashes.json.

Next: actual TSF service loading/activation, trusted deployed broker bootstrap, worker-to-STA dispatch, focus/lifetime policy, then backend/widget Apply and installed-app acceptance. Standard Edit empty TSF exposure remains unresolved. Unknown fields remain denied. Installed0848a3e stays Copy-only; no new product package or claim of external-editor support. Mission INCOMPLETE, local main/owner files unchanged, no push/adoption/release or Source UI application.

## TSF service loading diagnosis — 2026-10-05

Diagnostic EXE/DLL establishes a possible non-administrator target-loading route. Actual task-only HKCU COM registration/CoCreateInstance succeeds. General TSF RegisterProfile still returns80004005; process-local registration/activation succeeds with a real ActivateEx callback and unchanged current keyboard layout. A nonzero exact-thread WH_CALLWNDPROC hook loads that same diagnostic DLL in a newly created/pinned owned RichEdit child, and handles only its registered bootstrap message without inspecting keys/text. Start/stop produce active counts1/0; hook removal, unchanged source reread, natural child exit0 and parent DLL unloadability pass. This is OWNED_PROCESS_LOADING_ONLY, not production authorization, capture or Apply integration.

First child run failed because the top window was discovered before child initialization; explicit fixture readiness now waits under the same3-second deadline. Thread-ready and exact final-source runs PASS; failure retained. Local-profile unregister returns E_FAIL, while deactivate/release succeed and process-local state ends at natural exit. Task COM key is removed and task TIP GUID absent in both hives; no existing keys overwritten, keyboard defaults changed or elevation attempted. Four owned guests stopped, final inventory empty. Both binaries build C++17 /MT /W4 /WX plus linker /WX. Same-integrator review only. Evidence/source/binary hashes: D:/dev/grammar-evidence/grammar-tsf-service-load-20261005/RESULT.md and final-hashes.json.

Continue by implementing trusted installed-binary bootstrap, bounded worker/STA queue and DLL callback lifetime before joining this loading path to target capture/Apply. Window messages alone must not authorize reads or carry document text. Then wire actual backend/widget explicit Apply with separate Copy and qualify a fresh receipt-bound installed package. Current installed0848a3e remains Copy-only; mission INCOMPLETE and no general editor acceptance, main adoption, push or release.

## Trusted Apply peer image verification — 2026-10-05

Experimental apply_peer.hpp pins a live process under the existing same-user/logon/session/Medium checks, opens its executable without write/delete sharing, and compares canonical path plus SHA256 against trusted build/install inputs. The returned owner retains process/image handles; its constructor is private to the verifier. It does not grant document authority. Deadline and256MiB read ceiling bound admission checks; synchronous filesystem calls are not physically preempted.

Restricted-Medium native fixture: existing33 channel scenarios PASS plus live-image admission, wrong hash, same-byte wrong path, self/zero PID, expired deadline and dead-peer checks. Verified process handle feeds actual channel admission; temporary image copy removed, all children naturally exit. First and final reviewed runs pass; two guests stopped, inventory empty. C++17 /MT /W4 /WX plus linker /WX and same-integrator review. Evidence/hash binding: D:/dev/grammar-evidence/grammar-apply-peer-20261005/RESULT.md and hashes.json.

Trusted expected image distribution is still missing from the installed product; fixture uses its own known binary. Next join mutual-PID/capability enrollment, exact intended-field agreement, worker/STA queue and DLL callback lifetime to the target dispatcher, then real backend/widget Apply and installed package qualification. Consider embedding the already-built app hash in the later-built DLL, with installed sibling path validation; never accept caller-supplied expected hashes. Installed0848a3e unchanged, mission INCOMPLETE, no main/push/release. Prototype component evidence does not restore the user-facing feature by itself.


## Target service to widget integration candidate — 2026-10-05

Native commit8077945 joins authenticated target DLL loading, target-issued capture IDs, worker/STA dispatch, TSF conditional mutation and callback-safe cleanup. No COM/profile registration is required by this integrated route. Registered window message is bootstrap metadata only; exact-thread call-window hook does not inspect keys/text. DLL embeds the already-built sibling app hash and pins its process/image before admitting the local channel. Final owned Medium native13 scenarios PASS through direct client and C ABI, plus existing transaction13/binding40/channel33. All three guests stopped. Evidence and source/binary hashes: D:/dev/grammar-evidence/grammar-target-service-20261005/RESULT.md and native-hashes.json.

Rust backend/widget candidate now binds this capability to the existing capture session, draft proof, translation formatting and current terminology predicate. Apply exists only for a qualified target capture; Copy remains independent. Target I/O owns no capture/configuration/terminology mutex. Wrong token, cancellation, replay and cleanup-pending are covered by five production-runtime unit tests. Physical applied/indeterminate outcomes survive a later UI cancellation; no rollback, automatic Copy or replay. Whole desktop Rust238 PASS/29 ignored, frontend build/typecheck/capture/provider contracts PASS. Historical no-Apply UI assertion was replaced to match the Owner's explicit restoration decision; failure log retained. Native DLL compile C++17 /MT /W4 /WX, with no added dependency beyond an existing windows-sys feature.

New release receipt schema2 builds target-apply.dll AFTER the app, embeds the app SHA256 and filename, records DLL hash/size, then remeasures source. Schema1 historical verification remains available. Package and installer include that DLL. Builder tested with a renamed synthetic native probe only; first D8036 output-directory quoting failure fixed and preserved. Eight provenance negative tests PASS. Actual new app release/install/widget mutation NOT_RUN at this checkpoint; existing installed0848a3e remains unchanged. This is not restored-product acceptance.

Next: clean source release build, isolated installation, actual hotkey/Instant/edited-draft/Apply reread, stale source/selection/privacy and cancellation, separate Copy and complete uninstall. Current qualification is owned RichEdit50W explicitly exposing text/default input scope, not arbitrary apps. Standard Edit still Copy-only; its TSF empty-source failure remains unresolved. Current grant30s/connection180s are conservative component limits; long Deep/review and focus/IME behavior still require product qualification. Cleanup unknown/pending retains one owner and blocks a new native slot, never counts as complete. No main integration, push, account/login, Provider live, public release or Project UI application. Same-integrator review only, mission INCOMPLETE.

## Installed Apply focus failure and repair candidate — 2026-10-05

Clean608a173 package built/installed with exact app614e0b063272fae6d277f145091f66da6844daa91e5876c01fdcc43510331570 and target DLL30d42920b648ee143af744e4242659593e1771365023a1f1dd12687ce11167f5. The real widget captured/edited an Instant draft but normal Apply was rejected. This is a failed installed acceptance, not restored functionality. Uninstall completed with no root left. Evidence: D:/dev/grammar-evidence/grammar-target-app-608a173/compiled-controller2. Earlier first/controller setup failures and interrupted run remain preserved.

A native cross-process review-window reproduction also failed. Binding diagnostics retained revision1, valid grant and zero change notifications but rejected eligibility stage9 (input-scope metadata shape) after the target lost activation. Focus repair returns to the exact captured target only on explicit authenticated Apply, only when the target or verified broker is foreground; it never selects a range. Current target-thread focus must not name another field. All privacy/source/revision/range checks still run under the TSF write session. Wrong capture ID is checked before activation and again after it. Moving between controls in the target process revokes the grant even if the original field is later restored. No Ctrl+V/clipboard rollback or global hook is reintroduced.

Native focus-fix14/binding24/transaction13/channel33 passed. Added wrong-ID foreground preservation and other-field return/unrelated foreground counterexamples; first focus-guard run had one aggregate fixture failure despite applied/reread/cleanup success in case13, preserved for diagnosis. Instrumented actual foreground state rather than relying only on activation return value. Installed acceptance for this repair remains pending a new clean source package. Reproducible owned fixture/controller and installed-widget test are now scripts/mission/target-app-e2e.mjs, target-editor.cs, build-target-editor.ps1, Guest-App-TargetApply.ps1 and TARGET_APPLY_TEST.md. No Provider live/account or host desktop input. Same-integrator review only; mission incomplete and general editor support not claimed.

The instrumented focus-reviewed run confirmed case13 never established review foreground (activation_return=0, actual_foreground=0). The owned target fixture now explicitly permits its broker PID to activate the review window; no product permission change. Final focus-final integrated14 PASS, including actual review foreground, wrong-ID foreground preservation, exact mutation reread and worker/child exit. Binding25 passes other-control/return and unrelated foreground denial with zero mutation; prior failed setup runs remain failures. New installed package is the next acceptance gate.

## Installed direct Apply restored for the owned RichEdit scope — 2026-10-05

Product commit40a7d26bde6f809ad0beaddd65716266e06ecbd0 built clean, app394bf11de6981eab8b45325950181e675cfeff052ef3b8798b6d06f94b999dd0, DLL44c0025ea3f61c3594f3715792af246dd3552d931f5f10fda5d60ac6b4240955, ZIPd78a9b0fa59fc4f3ac52f3de99c08e52e58b90e6d043c62232543914173f6c2e. Receipt schema2 binds the DLL to that exact app. Package content stays immutable, including pre-qualification pending notes; this later record qualifies only the measured artifact. Test/document-only successors do not relabel its product SHA.

Installed-first run performed successful edited Instant Apply/reread, rejected replay and preserved application edits, then FAILED on recapture with native_unsupported_editor. The controller had confirmed only the top-level foreground window, not the child input field; the owned test fixture now explicitly focuses and confirms its RichEdit before sending the shortcut. Product capture checks were not relaxed. installed-focus then passed11 checks. installed-final passes12: actual widget normal Apply/reread; consumed token rejection; app-source/range/privacy changes rejected; explicit Copy after rejected Apply preserves the app edit; Dismiss followed by an actual old Apply request rejects; separate edited Copy matches clipboard and leaves source; target DLL absent while the target process is still alive; app and editor natural exit. No capture errors. Uninstall independently reports root/registration/processes/user-data0, problems empty, complete true. All three app guests stopped. Source, fixture and exact artifact evidence: D:/dev/grammar-evidence/grammar-target-app-40a7d26/RESULT.md and hashes.json; retain installed/ failure, installed-focus/ and installed-final/ separately.

This restores a real installed desktop direct Apply flow for owned x64 RichEdit50W exposing explicit non-sensitive TSF scope, not a mock-only Apply and not broad editor qualification. Standard Edit remains Copy; Windows11 Notepad/Office/general custom editors, physical IME, long review beyond30s, live Providers and real documents remain unqualified. R1/R2 external rollback/paste paths stay deleted. No automatic Apply/fallback/retry or whole-source restoration. Native focus14/binding25/transaction13/channel33 and same-integrator review support this boundary; review is not independent human review. Product mission A/B/C overall stays INCOMPLETE. No main adoption, push, general-user installation, release, account operation or Source UI application.

Source materiality: user-visible desktop capability changed, so current semantic records and guide are updated. Generate only an exact12 PREVIEW from the dedicated candidate; canonical export/adoption and Owner Project UI application remain separate. Next development remains external editor qualification/30s review usability within the existing mission, without weakening privacy or promoting unsupported editors.

Source preview initially failed STATE_SEMANTIC_DRIFT:p3a_status because the generator only admitted an old Deep/native-unqualified mission profile. Added one exact owned-installed-Apply candidate profile, preserving historical/reset profiles and audit base; mixed F-06 qualification and all-editors overclaim negative tests added. State/roadmap now agree on DESKTOP_APPLY_EDITOR_QUALIFICATION. Generator tests pass; package preview is separate from the installed product acceptance and is not canonical/Owner UI application.

## Global Accept / standard Edit development checkpoint (2026-10-05)

Owner's latest instruction explicitly expands development to Text/DOCX/XLSX/HWP/HWPX/PPTX/webpage editors. Contract1.4.0 and roadmap now record that scope (local commit cd727c8). Office/Hancom adapters are not implemented or accepted. Do not reinterpret file extensions as capture/mutation authority. Prior installed40a7d26 RichEdit qualification remains historical and valid for its exact artifact.

New standard Edit candidate executes in the target UI thread, binds the real control's source/range/revision and permits only explicit Accept. It rejects pre-existing custom control subclasses, non-sensitive-scope uncertainty at read/write admission, protected/disabled fields, changed source/selection, old token, cancelled lease and composition. Original system Edit replacement is used with bounded deferred parent notifications and scoped redraw suspension. Application edits during subsequent notification/paint delivery win; no whole-source restoration, synthetic paste or automatic retry was reintroduced. Hooks retain DLL callback ownership if an unknown subclass wraps them, until safe detachment/window destruction.

Actual Sandbox failures shaped the implementation: parent color callback overwrote its own app edit before the redraw guard; ordinary review focus temporarily clears/re-publishes TSF scope. Scope metadata now has a separate observer from document revision. Only a valid explicit Accept can enter a synchronous activation guard. During a temporary VT_EMPTY inside that guard there are no text reads/writes; the exact original plain scope must return before activation ends. Known forbidden scope always revokes permanently. Outside the guard absent/changed scope revokes as before. Native write admission still verifies current context, scope, full source and range. This is not permission to infer plain text from missing metadata.

Evidence root: D:/dev/grammar-evidence/grammar-standard-edit-20261005. `activation-guard-service` passed14 real separate-process/C-ABI cases including review activation, actual replacement/reread and natural target/worker cleanup. `activation-guard-dispatcher` passed17 cases including unknown-token activation, password ABA, missing scope not restored and lease revocation within activation. A final reentrant-operation guard and release-configuration native checks follow; do not relabel the earlier receipts as later builds. Native probe builds carry before/after source byte hashes, dirty/HEAD identity, and DLL-to-broker hash binding. Diagnostic builds are explicitly marked.

Multiline standard Edit remains excluded from service/broker activation: the native component's two full-original-Undo assertions still fail, and the same result was reproduced without Grammar on an unmodified multiline Windows Edit. Single-line native Undo restores the source. The tests/failed logs have not been changed to PASS. The prototype's replacement/privacy/reentrancy tests are not installed-app acceptance.

Rust focused6 and full239 passed (29 interactive/live tests ignored); frontend typecheck/capture/MVP contracts and governance passed. There are pre-existing library/unused warnings. The new candidate attempts the direct route for one-line Edit; unsupported/custom/unknown-scope fields fail capture rather than falling through a less-informed reader. This is a compatibility restriction pending application-specific qualification, not general Edit/Notepad support. Multi-line legacy Copy is unchanged. UI label is Accept. Installed candidate E2E and packaging are still pending at this checkpoint. No live Provider/account, host clipboard/input, user document/profile, main adoption, push or release.

Next: finish native negative/release checks, build the clean candidate, install into owned restricted-Medium Sandbox and exercise actual Instant -> edited draft -> Accept/Copy, refusals and complete teardown; then address multiline Undo/support and named application adapters. Source materiality changed: the old exact12 preview remains a previous checkpoint; regenerate with the existing generator only after the new candidate's precise acceptance state is recorded. Do not present that preview as current or as canonical adoption.

Final release-configuration native checkpoint: final-dispatcher18/18 and final-standard-service14/14 PASS. Boundary build adds OEM conversion zero-read refusal and drag-selection invalidation; both PASS. Component run remains FAIL with exactly the same two multiline Undo assertions; no acceptance assertion was removed. Source/binary receipts and output are in final-native-build, boundary-native-build and boundary-component. Review by the integrator checked scope/activation, callback lifetime, source/range/expiry and no rollback; no independent reviewer used. Installed qualification follows this clean source checkpoint.

## Installed standard Edit Accept checkpoint — 2026-10-05

Product f9d13f24bf20304b75a6aa5e9fca936693894bd5 was built from a clean tracked source, packaged with its receipt and installed in restricted-Medium Windows Sandbox. App SHA256 865501cc23e257dbc9e4595e156dd4e0947fa40ee6dcbd822a100d0fe694b3f8; target DLL 8fdc4b25800de9a4b0ffbc959d3b9f7cdea6a26afc0be469747627a8466c58a5; ZIP 85b556d4856e550c0de4dbf4fdb02d07e4eb6d624b25d45a2ec55a72d289fa9b. Package receipt and manifest remain immutable and honestly say acceptance was pending when built. Later acceptance is bound here to these bytes, not a later docs/test commit.

`installed-standard` FAIL is preserved: the normal edited Instant Accept/source reread and stale/source/range rejection succeeded; the controller timed out externally clearing a standard password field. The fixture now asks its owned target UI thread to change the password marker and verifies EM_GETPASSWORDCHAR internally. Product privacy admission was not changed. `installed-standard-privacy` PASS12; `installed-standard-unicode` PASS13. The additional installed test uses Korean/emoji in both retained prefix/suffix and reviewed replacement, checks exact resulting full source and leaves unselected content unchanged. This is synthetic Unicode input, not physical IME qualification.

Final standard flow also verifies consumed token, app edit, moved selection, sensitive transition and Dismiss rejection; explicit Copy only; module unload while editor remains alive; natural app/controller exit and complete uninstall (root/host registration/process/user-data residues zero). No account/Provider calls, no host clipboard or input, no user documents/profile. Evidence root D:/dev/grammar-evidence/grammar-standard-edit-20261005, installed-build/build-receipt.json, package-f9d13f2/MANIFEST.json, installed-standard-unicode/app-target-result.json and uninstall.json. Final fixture hashes are outside the package in installed-unicode-fixture-hashes.json; its controller/service fixture source changes are tests only. The product DLL is bound to the real installed app hash, not the native probe.

Supported by this evidence: owned x64 single-line Unicode system Edit, uncustomized window procedure, explicit plain TSF scope, no composition/protection and a valid reviewed capture within30s. Missing scope/custom subclasses fail before text capture and do not fall through to a weaker reader. It is not general Notepad/Text/Office/Hancom acceptance. Existing RichEdit capability is retained; current package regression is recorded separately. Multiline original-Undo tests still FAIL2 and multiline direct Accept remains disabled. Ordinary Word/Excel/PowerPoint/Hangeul adapters are NOT_IMPLEMENTED, their isolated app tests NOT_RUN. Existing Chromium inline Accept is separate from desktop-global routing; Edge/global review routing remain unqualified.

Next approved work: fix/qualify multiline Edit without hiding Undo failures, then implement editor-specific document/range/revision authority for Word/DOCX, Excel/XLSX, PowerPoint/PPTX, Hancom/HWP/HWPX and connect Chromium global review routing. Use only isolated owned synthetic docs. An Office/Hancom test VM was requested; no answer has yet supplied one. Do not use host installed apps/open documents as a substitute, infer authority from filenames, add generic paste/whole-source rollback, or claim all-format support. No main adoption/push/release. Whole mission remains INCOMPLETE. Source preview must use the new exact profile and preserve the historical audit base; it does not constitute canonical adoption or Project UI application.
`installed-rich-regression` on the same immutable f9d13f2 package also PASS13, including Unicode partial selection, all refusal/Copy checks, natural exits, DLL unload and complete uninstall with no residues. All owned guests are stopped. This extends current-package regression evidence, not general RichEdit application qualification.

## Multiline subtype diagnosis and candidate (2026-10-05)
After preserving the installed single-line checkpoint, a stock-control12-case diagnostic isolated the Undo failure to the no-ES_AUTOVSCROLL multiline configuration. A control created with ES_AUTOVSCROLL restores the full original source with unchanged EM_REPLACESEL(TRUE)/EM_UNDO; EM_EMPTYUNDOBUFFER, WM_UNDO, repaint changes and equal-length replacement did not repair the no-auto-scroll case. This is an empirical compatibility condition, not a claim about Windows internals. No application style is changed by Grammar. Microsoft's Edit Control Styles documents ES_AUTOVSCROLL as a creation style; normal control appearance/configuration must be preserved.

The existing native component assertions were run unchanged in a separate --autovscroll mode: all26 acceptance scenarios PASS (plus2 stock diagnostic rows), including the two formerly failing full-source Undo assertions. The original no-auto-scroll mode remains separately available and FAIL2. Real TSF dispatcher18 PASS in the autovscroll multiline mode. Candidate service/broker admission now permits only the additional pre-existing ES_MULTILINE+ES_AUTOVSCROLL combination; other multiline fields remain legacy Copy-only. Focused Rust6 PASS and the prior full239 baseline is preserved. Separate-process/native14 and fresh installed multiline source/Unicode/line-boundary/Undo testing are still pending; current accepted product remains f9d13f2 until those finish. Evidence: multiline-diagnostic, multiline-component, multiline-dispatcher and multiline-service-build under the same standard-edit evidence root. Office/Hancom/global webpage routing remain unfinished.
Multiline autovscroll service14/14 PASS through the actual authenticated separate-process/C-ABI path, including review focus, stale/privacy/cancel and natural cleanup. Installed qualification follows this candidate commit.

## Installed multiline autovscroll Accept — 2026-10-05

Clean product330ab3bdbcc08a4cf46661a8bc82740e162cfc3c is now installed-synthetic qualified for the additional standard Edit subtype with pre-existing ES_MULTILINE+ES_AUTOVSCROLL and explicit plain TSF scope. `installed-multiline` PASS15 includes actual widget Instant -> edited review -> Accept -> full-source reread; Korean/emoji partial selection; replacement spanning new lines with exact surrounding CRLF source preserved; one native Undo restores the entire original source. All token/source/range/privacy/Dismiss/Copy, live-target DLL unload, natural process exit and uninstall checks also pass. Uninstall reports root/registration/process/user-data residues0 and no problems. The owned guest has been stopped. This is not physical keyboard/IME or arbitrary application qualification.

App e237970293cc345082ec70dd1bb7f3702624855d0af4064c12b9ad064ea2c75c; target DLL0f7aebf006c285fa3a89d36699c36d185abff6ed8367c70e9c05fae6ef05a97a; ZIP5e82dd62a2eedb72f2365a9db24b4dc9a7080d0bf0bf9100dede2cd143d2d51e. Evidence: multiline-installed-build/build-receipt.json, package-330ab3b/MANIFEST.json, installed-multiline-fixture-hashes.json, installed-multiline/app-target-result.json and uninstall.json under D:/dev/grammar-evidence/grammar-standard-edit-20261005. The source/package manifest's pending wording was true at build time; this later receipt supplies acceptance for those immutable bytes. f9d13f2 single-line/RichEdit13+13 receipts remain distinct from330ab3b multiline15.

Prior blanket multiline exclusion is narrowed only to the measured creation-style subtype. No style is enabled by Grammar, and the no-auto-scroll full-original-Undo failures remain FAIL2. Do not generalize the diagnostic into an OS-internal root cause. Official reference: https://learn.microsoft.com/en-us/windows/win32/controls/edit-control-styles . Broad Text application coverage still depends on explicit scope/control qualification. Word/DOCX, Excel/XLSX, PowerPoint/PPTX, Hancom/HWP/HWPX integration and Chromium desktop-global routing remain unfinished. Current source work is preserved as a clean local candidate; no main/push/release/Provider/account/user documents. Same-integrator review only. Continue the approved remaining adapters with isolated app prerequisites; do not turn this checkpoint into whole-mission completion.


## Chromium selected-source review checkpoint — 2026-10-05

Evidence root: D:/dev/grammar-evidence/grammar-browser-selection-20261005 (RESULT.md, per-run receipts and verified-source/SHA256.json). Source materiality changed; the previous exact12 multiline preview is historical and does not include this new browser feature. New preview/export remains separate from product qualification and canonical adoption.

Explicit document + field opt-in, then selected text + Alt+Shift+R opens selected source and an editable replacement formed only from cached local suggestions. Accept checks field/source/revision/range and a 30-second authority before and after synchronous beforeinput callbacks, consumes authority before mutation, and rereads actual contents. Copy/Dismiss retain the expired draft. No new inference/Deep from review opening. CE review-focus ownership requires a trusted handover and exact shadow-host selection boundary; another page selection or replaced text node cannot reuse it.

## Evidence
- core227 assertions and worker49 tests PASS; node syntax, typecheck, diff check, governance PASS.
- browser-first FAIL: CE focus transfer invalidated normal review. focus-guard FAIL: pointer/focusin was too late; metadata identified HTML boundary retargeting. focusout-guard focused10 PASS after checking selection before trusted focusout. Full-review54 PASS after additional keyboard and DOM counterexamples.
- expiry-red FAIL: cached suggestion mutated after expiry during synchronous beforeinput (actual separate, expected seperate). No assertion changed. One-shot isolated-world Date.now advancement makes this deterministic; the existing actual61-second expiry check is retained separately.
- expiry-green full56 PASS, including both cached/selected review expiry after callback, Unicode prefix/suffix, edited explicit Accept, range changes, source ABA, callback reentrancy, exact CE handover, same-text DOM replacement, script-generated shortcut refusal, navigation and all previous Deep/privacy/IME protocol regressions.
- final medium-result PASS; fixture registration/profile absent and host processes0. Complete uninstall: root/registration/process/user-data residues0, problems empty. All guests stopped.

## Provenance / limits
Final result.json contains tested extension byte hashes. verified-source/ preserves files, tests and guest runners with SHA256.json. expiry-red-content.js was reconstructed using original CRLF and matched the immutable red receipt SHA256; an initial LF-only reconstruction did not match and was not used. Source HEAD was b4209de8b82d042905745dd56e281f15f25d1b41 plus recorded dirty changes. Native host is unchanged installed330ab3b, SHA256519690136b1989188485a3ab080fa0f8bd123e401919bfb3d5d074672aa7d6f2. Extension key/ID/fixture-origin permissions/host-config were modified in a copied fixture. Final fixtureBase explicitly identifies hash-bound source extension. Earlier full-review fixtureBase inherited misleading installed-extension wording; its source receipt is authoritative and it is not unmodified-installed-extension acceptance.

Network/clipboard redirection disabled, owned restricted-Medium Sandbox, synthetic docs/profile and synthetic Provider only. No host user input/docs/profiles/accounts/live Provider. Same-integrator review only: checked cancellation, permission gates, one-shot grant, synchronous callback revalidation, focus handover, mutation reread and cleanup. No independent reviewer.

Next: authenticated desktop/native-host/browser review routing using target-side selection authority; app-aware Word/Excel/PowerPoint/Hancom adapters and isolated synthetic app qualification. No all-format support claim, native browser Undo guarantee, physical IME, Edge or installed new-extension acceptance. No main adoption, push or release.


## Browser process route component — 2026-10-05

Evidence: D:/dev/grammar-evidence/grammar-browser-route-20261005/RESULT.md and build-shim/build-receipt.json.

Chrome documents that a native host started from a service worker receives --parent-window=0 (https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging). It cannot supply target window identity. The desktop must observe foreground and independently bind its browser process to the installed host.

The first actual headed Chromium test rejected the direct-parent assumption: first/ FAIL. Diagnostic/ FAIL preserved exact metadata: host -> C:\Windows\System32\cmd.exe -> owned Chromium -> test Node. No document, clipboard, command-line arguments or credentials were read. The new route allows a direct child or exactly one canonical Windows system cmd.exe shim, with image path/hash and process handles pinned. Arbitrary ancestors and executable-name-only trust remain forbidden.

## Implementation and verification
browser_route.hpp reuses VerifiedApplyPeer and same-logon/Medium-integrity admission. Caller must supply a trusted installed host image/hash, not values received from an extension. Snapshot lookup is bounded by an absolute deadline and a65536-entry bound. Process creation ordering rejects reused-parent PID explanations. The route retains browser, exact host and optional launcher lifetime handles; validates exact root window/foreground at capture admission; review focus may leave a still-live process route but is not new capture authority. Window class alone never authorizes capture, provider work or mutation. No new library/dependency or system setting.

MSVC /W4 /WX build PASS. build-shim/build-receipt.json binds native input bytes to browser-route-probe.exe. tested-inputs/SHA256.json records browser harness and guest runner separately. Probe/runners only execute in owned restricted-Medium Sandbox.

shim/ PASS: actual headed Chromium and installed330ab3b host copy;11 native checks including good route, wrong hash/path/zero/expired/self/browser PID, authentic host launched under wrong parent, review-focus separation and unrelated owned window. The test-created wrong-parent host starts suspended, is resumed without a valid origin, exits naturally, and never performs protocol/provider work.18 focused browser checks also PASS with selected-source review and post-beforeinput expiry regressions. Full56 remains prior15e5cab evidence, not rerun here because no existing extension production bytes changed.

medium-result PASS: fixture profile/registration absent, host processes0. Complete uninstall: root/registration/process/user-data residues0 and no problems. All guests stopped. No live Provider, user data/profile, host clipboard/input, account change, main adoption, push or release. This is a modified isolated extension fixture and component qualification, not new installed-product acceptance. Real Chromium used the cmd shim; direct-launch Chromium mode is not separately qualified.

Same-integrator review only: checked trusted input provenance, bounded process scan, pinning/PID lifetime, canonical system image, same identity/integrity, source-free metadata, exact window, complete guest cleanup. No independent reviewer. Next: connect a dedicated authenticated desktop/native-host review channel, worker documentId/epoch routing, one-shot content selection authority and actual desktop widget Accept; qualify before activation.


## Desktop/browser IPC component — 2026-10-05

Whole mission INCOMPLETE. This component is not yet exported from the installed target DLL or called by Tauri/native host/MV3. No new product activation or installed-app acceptance is claimed. BrowserProcessRoute and the target documentId/epoch/range grant remain additional mandatory gates.

browser_bridge.hpp implements a dedicated one-review duplex local named-pipe connection using existing logon-ACL/remote-refusal/first-instance, pinned executable path/hash/OS peer PID, random capability, sequence, bounded framing and Unicode validation code. Server authenticates the trusted desktop image; client accepts a VerifiedApplyPeer for the trusted host. Trusted inputs must come from the installation, never a remote request. It reuses no target DLL hooks and reads no editor/clipboard/provider data itself.

Protocol: one capture -> source reply -> one explicit Apply -> physical outcome. Application UTF-16 limit8192; generic frame reader has its existing65534-unit allocation cap and the lower application cap is checked before dispatch. One request has a3-second absolute header/body/response budget; a connection has a35-second maximum from connection start. This transport budget does not extend the target's30-second selection authority. Idle preconnection ownership is one handle and must be tied to the owning host port. Synchronous Win32 calls are not claimed forcibly preemptible by the deadline. No detached reader, unowned thread or persistent source file is introduced.

Apply authority is consumed before forwarding. Missing/malformed/lost Apply reply remains indeterminate; no retry or automatic Copy. Successful terminal bytes stay in the pipe until the client disconnects or the terminal deadline expires; immediate DisconnectNamedPipe would discard unread replies. Caller closes a cancelled idle owner; a concurrent cancel sets only the atomic flag so the I/O owner closes safely. Disconnect, stale sequence/capability and invalid frames revoke the connection.

Validation: MSVC /W4 /WX PASS; native-1 real separate-process11/11 PASS. Final lifetime run13/13 PASS with exact Unicode capture/replacement, wrong client/server hash, wrong capability/version/sequence, invalid UTF-16, oversized apply, cancellation after an explicit server-observed capture event, lost outcome, explicit indeterminate reply, actual35-second lifetime expiry, request-response stall expiry, no replay and natural child exit. Negative raw clients stay connected until actual server rejection; disconnect is not substituted for header validation. These are synthetic backend responses, not actual browser document writes. Initial first/ was NOT_RUN because MediumLaunch correctly rejected an incorrectly named runner; launcher restrictions were preserved and the owned runner was renamed.

build-lifetime/build-receipt.json input hashes match current sources. Probe SHA2569e0458affcbbc991926ae9ceb6c8570b9596666f091f347b18ef25eb23cc727a matches the guest receipt. tested-source/SHA256.json also covers the guest runner. Final result statusPASS, exit0, residual probe processes0. Guest stopped; inventory empty. No installation/registry change, live Provider/account, user documents/profiles, host clipboard/input, main adoption, push or release.

Same-integrator review checked trust provenance, bounded ownership, partial-response handling, Unicode lifetime, sequence/one-shot semantics, cancellation and result uncertainty. No independent reviewer. Next: C ABI integration in the existing hash-bound native module; dedicated review native-host actor and MV3 document/selection routing; Tauri capture/Accept binding and real installed UI validation. Office/Hancom app adapters and isolated acceptance still remain, without narrowing the full goal.

Evidence root: D:/dev/grammar-evidence/grammar-browser-bridge-20261005.

## Browser native-host relay integration — 2026-10-05

Mission remains INCOMPLETE. The production target DLL now exports a versioned browser server C ABI and both DLL builders include it. Native host accepts review-listen only as its first request and runs a dedicated actor with no Provider dispatch. The actor owns one native cookie, awaits all native operations and closes it before return. Strict replies bind id/sequence and a single captured opaque ticket, reject unknown fields/NUL/oversize text, and classify missing Apply replies as indeterminate. Trust remains installation-compiled image path/hash. MV3 document routing and desktop widget/client wiring remain incomplete and are not activated by this change.

Evidence root: D:/dev/grammar-evidence/grammar-browser-bridge-api-20261005. MSVC /W4 /WX builds PASS. Actual host Rust tests20 PASS (including4 new protocol tests); pre-existing library warnings remain. native/: actual DLL C ABI6 separate-process cases PASS, including unknown/stale owner, duplicate owner, invalid output buffer/status, cancellation, lost/indeterminate outcome, actual35s lifetime and stalled response. relay-static/: actual statically linked release host plus production-built DLL, synthetic desktop peer and MV3 replies5 PASS: normal capture/Apply, stale sequence, wrong ticket, EOF during capture and forbidden Provider field. All host/client processes exited naturally; remaining0. This does not test a Chromium document or installed Tauri widget.

Preserved failures: relay/ failed before review-ready using a developer debug host without the established package static-CRT flags. That run did not record an exact startup exit code; a missing runtime is a hypothesis, not a measured loader code. A subsequent debug-start diagnostic stalled before regression execution (target-regression/: NOT_RUN), so the owned guest was stopped. target-regression-clean/ used an incorrect DLL location: the probe loads C:/GrammarPayload/target-apply.dll, not its executable sibling; that interrupted run is invalid qualification. Correctly staged target-regression-bound/ tested the new DLL and its exact compiled trusted probe:14/14 PASS, actual text reread, target natural exit and worker close, remaining0. No assertions or product budgets were weakened.

relay-static-receipt.json binds host/DLL/client/test and product source hashes. The host uses the same static CRT flag as existing personal-package builds. relay-dll-receipt.json contains formatted external PowerShell output, not a structured verified receipt; actual stage hashes and build input receipts are the provenance evidence. Host hash e5acc957801867a6e655c7a172a4480b3165b409da59711d3a860df260715a78; relay DLL ce8183b079fef7658948ce29cbc037615efec1d6e9e0a4c83ca5fb6671ac7631. Different probe/DLL trust bindings are intentional and are not installed-product binaries.

Same-integrator review checked first-request separation, one-shot sequencing, strict reply shapes, module/process lifetime, serialized native ownership and compiled trust; no independent reviewer. All guests stopped with empty inventory. No live Provider/account, user document/profile, host clipboard/input, main adoption, push or release. Next: exact MV3 document/selection routing and desktop capture/Accept binding, then actual browser/widget validation; named Office/Hancom adapters and isolated app acceptance remain required.

## Browser document endpoint for desktop Accept — 2026-10-05

content.js now exposes internal desktop-capture/apply/cancel messages to this extension only, requiring its current non-null opt-in epoch. Capture additionally requires the enabled field to be focused in the focused document; it cannot enable a field or substitute the previous field. A memory-only random ticket owns one selected source/range/revision for30s. Input, composition, changed selection/focus, disable/navigation and observed DOM/privacy changes revoke it. A bounded dedicated MutationObserver also checks queued records after synchronous beforeinput callbacks, rejecting DOM/attribute ABA before a write. It observes at most32 ancestors and one bounded supported field; cancellation disconnects it and clears source/grant/timer. No Provider/inference request is added.

The shared DOM commit function now returns rejected/verified/indeterminate outcomes after actual reread. Desktop Apply consumes authority before writes, does not focus the editor away from the desktop review window, and never restores or retries when a page input callback changes the result. Existing page-local Accept still restores field focus. The worker does not yet route real native-host requests to these endpoints; the desktop client/widget is also unfinished. These endpoints do not establish all-format, installed-extension or global shortcut acceptance.

Evidence D:/dev/grammar-evidence/grammar-browser-content-20261005: core227 and worker49 PASS. Owned restricted-Medium headed Chromium, browser sandbox on, modified source extension and unchanged installed330ab3b host: first/27 focused PASS; full/66 PASS including existing Instant/synthetic Deep/permission regressions; supplemental/29 focused PASS with two additional tests and identical product bytes. New checks cover textarea/contenteditable Unicode replacement and actual reread, replay, range/source/focus/ticket/cancel refusal, beforeinput expiry and attribute ABA, same-text node replacement/reselection, post-write application changes reported indeterminate with no restore/retry, and sensitive focused-field refusal. Messages in these endpoint tests are generated by the synthetic worker test harness, not the desktop widget; no real Provider was called.

verified-source/SHA256.json binds copied extension/test/runner bytes. full/browser.test.mjs preserves the earlier full-suite driver before the two supplemental assertions were added. Extension hashes match source-receipt.json and each guest result. All three fixtures removed profiles/registrations/hosts; all package uninstalls complete with residue0. All guests stopped. Same-integrator review only, no independent reviewer. No host input/clipboard, user documents/profile, credentials, main adoption, push or release. Next: worker documentId/epoch/port lifecycle routing, desktop authenticated client and actual review-window Accept; Office/Hancom editor-specific work remains in the full goal.

## Browser worker/native/document integration — 2026-10-05

worker.js now admits desktop-arm only through the existing exact extension/frame0/origin/documentId/epoch permission path. One dedicated review native port is distinct from Instant/Deep ports. Arming and capture recheck active tab/last-focused-window; requests and replies bind id/sequence/ticket and exact documentId. Different tab/browser window, navigation, disable, disconnect and deadline revoke that owner; WINDOW_ID_NONE permits the desktop review window to take focus. No Provider fields or persistent document data are used. First handshake5s, idle listener5min, forwarded operation3s, captured ticket30s and terminal drain5s are bounded independently of the native transport's already bounded lifetime. Native/client/target shorter deadlines still apply.

Same-integrator review found two ownership gaps in the initial worker: timing out an unsettled document operation or merely queuing its cancellation allowed another owner. ownership-red.log reproduces both failures against preserved first/tested-source production worker bytes. Final code retains one in-flight dispatch and one cleanup Promise until the actual Chrome sends settle; new owners return busy during that interval. Assertions were unchanged; no timeout was increased. Restart does not recover permission: existing memory-only epoch policy still requires a fresh user grant.

Evidence D:/dev/grammar-evidence/grammar-browser-worker-20261005. worker64 PASS; core227 PASS; MSVC x64 static CRT /W4 /WX probe and production DLL builder PASS. first/32 focused browser checks PASS before the ownership repair; full/71 PASS with final worker, existing synthetic Deep regressions and actual headed Chromium/native host/DLL/worker/content integration. The authenticated synthetic desktop peer discovers exactly one review pipe under the foreground browser route, captures the source, waits for an explicit fixture Accept signal, then applies once. Textarea and contenteditable preserve prefix/suffix and reread exact Unicode results; changing the source after capture yields rejection and preserves the application edit. Replay is refused. Native host and worker review ports close naturally after each exchange.

Final receipt hashes: host e5acc957801867a6e655c7a172a4480b3165b409da59711d3a860df260715a78; synthetic peer e242c9c71cf7b6c9b2c6bfed4be32c031f541b0e3755c0fc2c9999666c90eebb; DLL c46d9a42df263a50fd4dfcef46ce44b1602b134c9cf537b5cd4f071de7657699. first/ inherited an inaccurate installed-host fixtureBase label; full/ corrects it to source host/DLL, synthetic peer and modified source extension. This is not installed Tauri/global-hotkey acceptance. desktop-arm is invoked by a real isolated-world content sender through test injection; production field activation and the desktop widget do not yet invoke this route. The next work is the desktop client C ABI, existing capture/session runtime integration, field arming lifecycle and actual widget Accept tests.

Both fixture/profile/registration/host cleanups and old-package uninstalls complete, residue0, all guests stopped. No live Provider/accounts/user documents/profiles, host clipboard/input, main adoption, push or release. Same-integrator review only; no independent reviewer. Full named Office/Hancom scope remains required and unimplemented; mission INCOMPLETE.

## Desktop browser client ABI and capture runtime — 2026-10-05

The shared target DLL now exports GrammarBrowserClientCreate/Capture/Apply/Cancel/Close. A single monotonic cookie owns a foreground root window, pinned browser/host route and one-shot transport. Host path is the DLL sibling grammar-chromium-host.exe; its expected hash is compiled by build-target-apply.ps1, from the explicit host argument or the actual built broker's sibling. Missing host binding leaves client Create unavailable. The existing native-host server still authenticates the compiled broker hash. Neither direction accepts a protocol-supplied trusted path/hash.

Create allocates only. Capture discovers exactly one ready review pipe under the actual foreground browser process and shares one3s deadline across route/hash verification, connect and capture response. It rechecks foreground after source receipt. Apply requires the retained live route and consumes the capture first; lost results remain indeterminate. Cancellation sets atomic flags and closes idle owners without waiting; a busy native call retains ownership until its bounded operation returns. Close never reports success while that local operation owns the slot. Native filesystem/OS calls are not claimed forcibly preemptible.

Rust uses the existing TargetApplyRuntime registry/session-token/epoch/cancellation path with the browser ABI variant and8192-unit limit. The hotkey source now classifies a foreground Chromium root before considering native field capture; browser failure propagates and cannot fall through to clipboard/native editor capture. Successful browser source binds to the same prepared capture and explicit desktop Apply path. This source integration is compiled and unit-tested, not yet installed-widget acceptance: field arming/rearming and the actual Tauri/browser user flow remain next work.

Evidence D:/dev/grammar-evidence/grammar-browser-client-20261005. MSVC x64 static CRT /W4 /WX builds PASS. first/32 focused actual Chromium checks PASS with the probe compiled in GRAMMAR_BROWSER_CLIENT_ABI mode: production DLL client/server, actual native host/worker/content, exact textarea/contenteditable replacement, source-change refusal and actual reread. The client also checks duplicate allocation, wrong/unknown cookie, undersized capture buffer, premature/repeated Apply and repeated Close. Final broker a1bd72e39fe765311cc0e8287bdf059a2c2806fe74356c3f5737d24b6f38bd1a and DLL6944222f3fa047cb46575a01b956ac4c6a5d150363efa43ae39e54e7a5290c15; host remains e5acc957801867a6e655c7a172a4480b3165b409da59711d3a860df260715a78. Probe is synthetic desktop code, not the Tauri application.

native-regression/: rebuilt shared DLL with its exact standard native probe binding,14/14 PASS with target reread/natural exit/worker close, remaining0. rust-target-tests-final.log:6 production runtime ownership tests PASS; main and target module compiled; existing unrelated warnings remain. Current helper source removes only an ending blank line after the tested build; original tested-native-browser-document-probe.cpp and client-qualification.json preserve exact helper bytes/mode. Product native inputs and Rust inputs are in verified-source/SHA256.json. No performance threshold/assertion was relaxed.

Browser fixture/profile/registration/host cleanup and uninstall complete; native regression processes0; all guests stopped. Same-integrator review checked fixed trust provenance, lifetime pinning, one absolute capture deadline, cancellation ownership, no browser fallback and exact Rust token reuse. No independent reviewer, actual new Tauri widget flow, live Provider/account, user documents/profiles, host input/clipboard, main adoption, push or release. Full global/editor-format mission remains INCOMPLETE.


## Browser field arming and installed-flow preparation — 2026-10-05

Explicitly enabled, supported focused fields now prepare a control-only desktop connection on focus/enable/heartbeat. This sends no source and performs no inference; Ctrl+Shift+G initiates capture. One outstanding preparation promise is retained until settlement, failed startup backs off five seconds, existing exact-document/revocation and unsettled cleanup admission remain. Client route discovery can wait for normal rearming only within its original three-second capture deadline, with foreground and cancellation checks. Browser-unavailable UI now explains the document/field opt-in prerequisite.

Evidence D:/dev/grammar-evidence/grammar-browser-arming-20261005: full/ PASS71 headed Chromium checks, including actual native host + C ABI desktop test peer selected Unicode replacement and rearming for textarea/contenteditable without injected arm commands. Synthetic Deep only; modified isolated extension fixture, not installed Tauri acceptance. Full medium cleanup records no profile, registration or host processes; uninstall complete, guest stopped. Rust target ownership6 PASS, worker64 PASS, native DLL/hotkey helper MSVC /W4 /WX PASS, frontend/Tauri build PASS. Existing warnings retained.

New browser-app-e2e.mjs drives the actual installed app and unmodified installed extension with physical global shortcut in the owned Medium Sandbox, keyboard toolbar grant, edited Accept and rereads, stale/source/range/cancel/privacy counterexamples. Helper refuses outside the exact task browser path/title and WDAG/Medium identity. This new harness is syntax checked but NOT_RUN at this commit. Next: fresh receipt-bound package and installed execution, repair failures; Office/Hancom adapters still incomplete. Same-integrator code review only; no independent review, live Provider, user document/profile, host input/clipboard, main adoption or push. Mission remains INCOMPLETE.


## Installed browser relay layout repair — 2026-10-05

Fresh fbebb08 package is built and preserved at D:/dev/grammar-evidence/grammar-browser-installed-20261005/package-fbebb08. First installed attempt refused ungranted capture correctly but its toolbar test helper could not execute as a PS5 file under guest policy (FAIL preserved). Harness now executes the exact repository helper as an inline command with synthetic arguments in environment; no policy changes. Second/ and diagnostic/ reached real keyboard document opt-in but capture failed browser_review_unavailable. All three uninstall receipts complete; guests stopped.

Cause: Prepare-Host copied the native executable to install/host without its DLL, while authenticated desktop routing and native loading require the broker, host and DLL in one runtime directory. Prior source fixture colocation did not expose the installed topology defect. Personal installer now explicitly requests UseInstalledRuntime: the registered executable stays at the verified install root, origin/cleanup sidecars are beside that executable, host/ holds registration metadata. Standalone Prepare-Host remains isolated-copy mode. New mode accepts only canonical root/host with app/host/DLL present and refuses existing sidecars before changes; uninstall/rollback already owns the complete fresh install root. No arbitrary path fallback or trust weakening.

Twelve nonregistry inert-file layout tests PASS; earlier test attempt against pre-fix API failed on the missing parameter (not counted as an assertion-level product reproduction). Actual installed failures above are the product reproduction. Installed package test now asserts registered executable/DLL colocation. Browser global acceptance is still pending a fresh repaired package; exact document/range/refusal checks remain unchanged. Mission INCOMPLETE, no Office/Hancom implementation or acceptance yet.


## Installed desktop-global browser Accept qualified — 2026-10-05

Product84df647eee56541e06a8aca92e9245db8ce6753d, evidence D:/dev/grammar-evidence/grammar-browser-installed-20261005/RESULT.json. Repaired clean build + package preserves the source-to-app/host/DLL relationship. Exact installed hashes: app0faba4a8df146a59dfd65c41efd23ed23ee5556743046ae4c10e484f2a974c2e, host1463a0e16f23f78d0040c30ae1a7e84a96a470bf1f59172cefb9bfc54c7b4fa4, DLL3d95df207b7cf73bd536dc2e9136062550a9d7c2f8dde68b5a9470136110c3ab, ZIPb0e05e54a1e78711d1b75e822acb4c7bb6bb8ca16b75e79c6f365068173b0962, extension lecokjlmfolgfpgkbcdpimikdaceinee. These belong to product84df647, not a later records commit. Packaged guides/Handoff are prequalification snapshots and may contain superseded scope text; current repository guides and this later receipt describe qualification. Do not relabel or edit the measured ZIP.

repaired/: PASS10 actual installed Tauri + unmodified extension checks, keyboard-pinned toolbar opt-in, explicit field opt-in, physical guest Ctrl+Shift+G, local Instant and edited Accept for textarea/simple contenteditable. Exact Korean/emoji selected replacement preserves prefix/suffix, consumed-token replay refuses mutation, app source edit/range change reject Accept, Dismiss revokes, normal focus/heartbeat rearms, password-labelled field refuses capture without prior-field fallback. No injected desktop-arm or document enable. The browser is headed/sandboxed with automation/accessibility flags in a fresh task profile. This is automated owned-guest input, NOT human keyboard/IME or general-profile qualification. Native host is now the registered root binary, not an untested copy. Registration failure after sidecar creation also rolls back the complete fresh root. App exits naturally, extension bytes unchanged, host count zero, profile removed; uninstall complete with zero root/registration/process/user-data residues.

native/: same package PASS15 real widget/multiline ES_AUTOVSCROLL Edit checks, including exact full content reread, Unicode/outside selection preservation, native Undo full-source restoration, source/range/privacy/cancel/stale refusal, explicit Copy and module unload while editor remains alive. App and editor natural exit; complete uninstall. Both guests stopped. Source app Rust239 PASS/29 ignored, host20 PASS, worker64 PASS, core227 PASS, inert layout12 PASS, governed clean build/native compiler/package PASS. Earlier app build warnings remain, ignored tests are not passed live tests.

Same-integrator review only: inspected runtime colocation/trusted hash, no arbitrary path fallback, canonical fresh-root/sidecar refusal, one-shot exact-document authority, source-free preparation/backoff, actual cleanup and package identity. No independent reviewer. Preserved first harness failure and second/diagnostic installed product failures are not relabelled. Canonical main4e833988 and its three pre-existing untracked files are untouched; no push/adoption/release, user documents/profiles, host input/clipboard, real Provider/account activity.

Whole goal remains INCOMPLETE: Word/DOCX, Excel/XLSX, PowerPoint/PPTX and Hancom HWP/HWPX app-aware adapters are NOT_IMPLEMENTED; their isolated licensed applications/acceptance are not available here. Next work is their target-side integration design and implementation, retaining per-app qualification. Standard Edit/RichEdit support remains its exact qualified scope, not every Text app. Physical IME, arbitrary sites/editors, general profiles and Edge remain NOT_RUN. Source materiality changed by actual desktop-global browser capability: update current state/acceptance/guides and generate only candidate exact12 PREVIEW. Canonical export and Owner Project UI application remain separate.


Source-preview follow-up: first generation at72f3124 failed STATE_SEMANTIC_DRIFT:exact_next_task because the generator and roadmap marker still named the historical multiline task. Added only the explicitly authorized desktop-browser/application internal task for the already-qualified Edit profile, preserving the prior task and all acceptance/privacy/main/audit checks. Behavioral Source tests62 PASS, including exact new task, historical task, arbitrary-task rejection, whole-product/main/live overclaim rejection and archive integrity. This is Source-package validation, not product qualification. Historical audit base is unchanged. Generate the exact12 candidate preview from this record commit; no canonical export or Project UI application.


## Word privacy metadata prerequisite — 2026-10-05

Next named-app work started: src-tauri/native/word_privacy.hpp implements only read-only Word protection metadata inspection through IDispatch. Explicit request and owning-thread checks precede all COM calls. HasPassword, WriteReserved, ReadOnly, ProtectionType and Permission.Enabled are queried with strict VARIANT types; unknown/error/missing/noncanonical values refuse. No Password/WritePassword, file path, text or document activation/ROT calls, setters, protection removal or event suppression. A clear metadata result is explicitly NOT capture/mutation authority. Existing native admission remains unchanged; no Word editor is enabled.

MSVC /W4 /WX and35 inert COM checks PASS: protection cases, missing members, COM exceptions, bad variants, null Permission, opt-out, actual wrong-thread call without COM access, reference balance and property-get-only allowlist. Evidence D:/dev/grammar-evidence/grammar-word-privacy-20261005/final-build/build-receipt.json and final-probe.log bind the exact new helper source. This never loaded Office or read a real document; Word application integration and live acceptance NOT_RUN. No third-party dependency. Same-integrator review only.

Official API basis checked: [Word HasPassword](https://learn.microsoft.com/en-us/office/vba/api/word.document.haspassword), [WriteReserved](https://learn.microsoft.com/en-us/office/vba/api/word.document.writereserved), [ReadOnly](https://learn.microsoft.com/en-us/office/vba/api/word.document.readonly), [WdProtectionType](https://learn.microsoft.com/en-us/office/vba/api/word.wdprotectiontype), [Document.Permission](https://learn.microsoft.com/en-us/office/vba/api/word.document.permission), [Permission.Enabled](https://learn.microsoft.com/en-us/office/vba/api/office.permission.enabled). These documents specify metadata, not an atomic compare-and-replace guarantee. Still required: authenticated in-app object/context acquisition, exact document/field/source/range/revision under target-side lock, privacy transitions, cancellation and installed Word tests. Do not connect an out-of-process read-then-write workaround. Excel/PowerPoint/Hancom remain unimplemented.

Host App Paths read-only metadata confirms Word/Excel/PowerPoint16.0.20430.20092; no application was opened. No Hancom entry was found in that single inspected native App Paths location (not an exhaustive absence claim). Asked Owner for optional owned VM/install-media information because actual Office/Hancom qualification needs isolated licensed apps; this does not gate independent code work and authorizes no host document or credential use. All Sandbox guests remain stopped, local branch continues, whole goal ACTIVE/INCOMPLETE.

## Word selection identity and selected-range TSF component — 2026-10-05

Local commits789e1f7 (Word metadata) andfa81024 (selected-range transaction). Product package/installed qualification remains84df647; neither new component changes native class admission or enables an Office/Hancom editor. Whole mission INCOMPLETE. Same-integrator code review only; no independent reviewer used this turn.

WordSelectionIdentity requires the owning thread, explicit enabled state, fixed deadline and exact revision; compares canonical COM identities for active window/document/selection and repeated range metadata; refuses wrong HWND/document, non-normal selection, non-main story, hidden/mixed font, fields/content controls/tables/inline shapes, malformed values, privacy changes and expiry. It exposes no text or COM pointers.59 inert IDispatch checks PASS; no Office process, text/password property, real document or credentials accessed. Evidence D:/dev/grammar-evidence/grammar-word-selection-20261005/final-build/build-receipt.json and final-probe.log. Actual Word interfaces may have stricter/different identity behavior: do not infer application support from the inert graph.

TsfSelectedSnapshot is a dormant target-thread component for selected-only reads and writes under caller-owned TSF read/write sessions. Cloned opaque range anchors are compared in the same context; no Word offset/TSF ACP equivalence is assumed. It reads only within the selection in bounded chunks (max8192 UTF-16 units), validates Unicode/embedded-object refusal, keeps a single-use shared grant, and rechecks exact selection/source/revision/privacy binding before one SetText(TF_ST_CORRECTION). Explicit gravity preserves a verification range around the replacement; actual reread confirms it. Failed/uncertain mutation is not retried or rolled back. Exceptions after a mutation attempt yield mutation_unconfirmed. The request owner must revoke authority on admission failure or deadline; an uncalled callback is not completion.

MSVC static-CRT /W4 /WX PASS for both new probes. Restricted-Medium Windows Sandbox final-guest19/19 PASS against a real owned RichEdit TSF store: longer/shorter Korean+emoji replacement; a70,000-unit prefix outside the selected text;8192-unit selection;8193 refusal; full synthetic document reread preserving both sides; changed source, equal-text different selection, cancel, revision/expiry, readonly request denial, invalid Unicode/object/NUL, missing privacy authority, reentrant revocation/revision and consumed grant replay. Positive transactions call SetText once, all negatives zero. These are real store operations with deliberately synthetic adapter authority, NOT actual Word or installed product qualification. Source capture checks and applied-text rereads are separate from fixture-wide validation reads.

Evidence D:/dev/grammar-evidence/grammar-selected-tsf-20261005/final-build/build-receipt.json and final-guest/result.json/probe.log; exe SHA256 matches the build receipt; process exited naturally, all guests stopped, clipboard/network/audio/video/printer disabled, no profile or registration changes. Earlier guest/ FAIL was a harness wait for an uninvoked callback; guest-r3/ exposed actual TF_E_READONLY request rejection before callback. The driver had incorrectly treated every successful RequestEditSession HRESULT as queued; it now waits only for TF_S_ASYNC and explicitly revokes a readonly-rejected request. Those failures remain; guest-r4/ and final-guest/ passed. Assertions still require unchanged complete fixture text, zero writes and rejected replay after readonly clears.

Official basis: https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfrange-comparestart ; https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfrange-clone ; https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfrange-settext . These APIs describe TSF ranges/locks, not automatic qualification of any application's store.

Next: bind authenticated in-process Word Application/window/document to the exact real TSF context and actual revision/privacy invalidation owner; retain bounded callback/DLL lifetime and refusal on unavailable/ambiguous coupling. Do not activate Word by widening RichEdit class checks or external read-then-write automation. Need isolated licensed Word before claiming its provider/undo/privacy behavior; Excel, PowerPoint and Hancom still require their own selection/privacy/capability implementations and real app qualification. Existing optional Owner question about owned VM/install media is pending; it does not authorize installing/activating products or touching host user documents. Continue independent adapter work; no main adoption/push/release/live Provider.

## Word native object acquisition — 2026-10-05

Code1819a8b adds dormant WordTargetIdentity/WordNativeWindowWin32. From an already authenticated target STA, the concrete port checks exact owning process/thread, visible/enabled _WwG field and OpusApp root/ancestry plus WINWORD.EXE image basename before AccessibleObjectFromWindow(OBJID_NATIVEOM, IID_IDispatch). Basename is only a prerequisite, not process authentication. Native Word Window.Hwnd must equal the expected root, Window.Application.ActiveWindow must have the same COM identity, and the prior document/privacy/selection token must still match. Acquisition and revalidation recheck window eligibility, including after reentrant metadata calls. No ROT enumeration, app activation, document/file/text/password reads, focus/clipboard changes or mutation. No class whitelist or installed product activation changed.

Official API inspected: https://learn.microsoft.com/en-us/windows/win32/api/oleacc/nf-oleacc-accessibleobjectfromwindow documents _WwG -> Word Window; https://learn.microsoft.com/en-us/office/vba/api/word.window.application documents Application. Actual Word versions/window topology are NOT qualified by these references or by the inert tests.

Evidence D:/dev/grammar-evidence/grammar-word-target-20261005/final-build/build-receipt.json and final-probe.log: MSVC static CRT /W4 /WX and48 inert native-port/COM checks PASS (wrong or missing native object, failed HRESULT with returned pointer cleanup, active-window mismatch, protected document, wrong thread, expired/revoked/revised authority, reentrant selection/eligibility changes, balanced COM references and zero text/secret getters). Concrete Win32 null-target rejection is checked; real Native OM acquisition is NOT_RUN. Existing59 selection checks PASS unchanged after extracting shared inert fixture; both build receipts include that fixture header. First build/ failure was a test assignment to an immutable guard; preserved, repaired in the test construction, no product condition weakened. Same-integrator review only. No Office or GUI process launched this turn.

Next integration remains actual Word/TSF context, document and selection correlation, not merely matching HWNDs or numerically equating Word positions with ACP offsets. Need target-owned revision/privacy revocation and bounded request ownership before enabling capture/mutation. The application must be qualified in an isolated licensed environment. Excel/PowerPoint/Hancom still pending, whole mission INCOMPLETE. Installed qualification remains84df647; these dormant source changes are not a new installed package. Main/push/release/live Provider unchanged. Previous goal turn and this turn both classify as progress (new tested source components), not verified waits or blocked turns.

## Word/TSF selected-source correlation — 2026-10-05

Code8055af5 adds dormant word_tsf.hpp. WordTsfRevisionBinding subscribes to the real context's ITfTextEditSink and invalidates outstanding grants on each write-session notification, disable or newer capture. WordCorrelationLease requires owning thread, explicit enabled state, actual cookie/selection, exact context/top context/active view HWND and WordTargetIdentity before any source read. It compares the selected Word Range.Text (getter only, no setter) with the selected TSF source, rechecks Word document/range identity and privacy around the getter, then uses the existing single-use selected-range TSF transaction for explicit Accept. Word numerical range positions are checked only against their own captured Word positions, never mapped to ACP offsets. Grant has one original30s lease budget; no retry/rollback/document-wide reads. Previous metadata-only APIs still do not themselves read text; this separate correlation component is the only new selected Text reader.

D:/dev/grammar-evidence/grammar-word-tsf-20261005/build/build-receipt.json binds all source inputs and probe binary; rehashed inputs/output match current code and guest result. guest/result.json and probe.log:14/14 PASS with real owned RichEdit TSF store plus deliberately INERT Word Native Object Model. This is not a real Word provider test. Normal reviewed replacement changed exactly the selected word, full fixture reread preserved prefix/suffix and replay rejected. Mismatched source, protected document, wrong view, invalid cookie, actual edit-and-restore revision change, changed Word range/source, disable, cancel, newer capture and equal-text different selection reject without writes. Protected/wrong-view/invalid-cookie capture had zero TSF/Word text reads. Metadata changing inside the synthetic Word Text getter invalidates/discards the capture after that one getter; this is not a zero-read guarantee for a privacy transition during a provider call. Real Word privacy-event ordering must be qualified before activation.

Concrete TSF edit-and-restore produced revision4 from capture revision1; no direct test assignment substitutes for that notification. The synthetic Word graph is not evidence that Word emits every necessary selection/privacy/revision event. Wrong-thread, unknown metadata and reference balance coverage retained:48 target and59 selection regressions PASS after extending the fixture with an explicit selected-Text-only capability; default fixtures continue forbidding all source reads. MSVC static CRT /W4 /WX PASS. Guest process exited naturally, executable SHA256 matches build receipt, guest stopped, no host clipboard/user docs/accounts/Provider access. No actual Office/Native OM process attached. Same-integrator review only; new builder has a harmless extra blank EOF line reported by cached diff whitespace check, not a runtime test failure.

Official semantics checked: https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itftexteditsink-onendedit (write-session completion notification), https://learn.microsoft.com/en-us/office/vba/api/word.range.text (selected plain text getter; setter deliberately unused). These references do not prove Word/TSF cross-provider coupling, arbitrary COM call latency or the safety of all Word events.

Next: app-specific privacy/selection event invalidation, retain module/callback/subscription ownership through rejected or pending requests, route a qualified Word target through the existing authenticated broker, and validate real Word object model, TSF context, Undo, IME and protection transitions in an owned licensed isolated environment. Unadvise outcome/lifetime handling and same-thread synchronous COM stalls need qualification before deployment; a deadline does not preempt an in-flight COM getter. Existing package84df647 and its Edit/browser acceptance remain unchanged. Word runtime admission stays OFF; Excel/PowerPoint/Hancom still incomplete. Whole goal remains active/INCOMPLETE; this turn is progress, not a blocked audit turn.

## Word TSF subscription ownership — 2026-10-05

Codef78fa39 replaces the Word binding's ignored UnadviseSink outcome with TsfSinkSubscription. A failed close retains the exact source/cookie, reports false and does not mark disconnected; later caller-controlled retirement can retry that same subscription. Only S_OK or documented CONNECT_E_NOCONNECTION resolves a known cookie. A nominally successful advise with INVALID_COOKIE remains unresolved, not silently cleaned. One attach attempt per owner; wrong-thread/reentrant close never calls the provider. Destructor remains best effort, explicitly not cleanup evidence. Runtime integration must inspect close() and retain the actor/module rather than rely on destruction or restart a second actor while cleanup is unresolved.

WordTsfRevisionSink now requires and retains the runtime callback-lifetime token; binding refuses missing ownership. The token remains held if a provider keeps a sink after failed unsubscribe; late events hold only weak state and are harmless after state destruction. This plugs into the existing target worker's callback-owner tracking model, but the Word actor has NOT yet been wired into that worker. These tests establish component reference/token behavior, not actual DLL unload or full Word runtime retirement. In-flight synchronous COM calls are still not preempted by the request deadline.

Evidence D:/dev/grammar-evidence/grammar-word-subscription-20261005: build-r2/build-receipt.json + probe-r2.log contain54 PASS against an inert ITfSource calling production subscription and Word sink code, covering failed close, invalid-cookie/unknown outcomes, missing connection, repeated close, wrong thread, nested close and sink lifetime after failed destruction. First build/ failed because CONNECT_E_NOCONNECTION needs the SDK olectl.h include; fixed without changing assertions. MSVC /W4 /WX PASS.

word-build/build-receipt.json + guest/result.json/probe.log contain14 PASS in restricted-Medium Sandbox using real RichEdit TSF plus inert Word object model. Every scenario now requires successful explicit binding.close(), disconnected state and expired callback token, in addition to unchanged prior source/range/refusal assertions. Binary SHA256 matches the guest, every recorded input hash matches committed code, probe exited naturally and guest was stopped; no active Sandbox remains. Real Office/native OM/DLL lifecycle and Provider live NOT_RUN; no host user documents/clipboard accessed. Same-integrator review only. Official close semantics: https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfsource-unadvisesink ; advise cookie semantics: https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfsource-advisesink .

Next work: complete Word actor/request ownership and application-specific selection/privacy invalidation, then isolated actual Word qualification before admission. Excel/PowerPoint/Hancom still pending. Product84df647 installed qualification is unchanged; current new source remains dormant. Whole goal active/INCOMPLETE; previous and current goal turns are progress, not blocker repeats. No main adoption, push, release or live account operations.

## Word asynchronous request component — 2026-10-05

Code2b2719a adds WordEditSession for owning-STA capture and reviewed Accept through real RequestEditSession callbacks. Each request retains its runtime token and fixed absolute operation deadline. Cancellation revokes its gate; it does not report callback completion. The gate is checked inside correlation/source validation before mutation as well as on entry/publication. Capture's short request gate is released only on successful capture; the original30s review lease remains separate. Binding now requires a live authority predicate for connection admission/revalidation. Caller must retain sessions until completed, revoke rejected requests and retire bindings with checked close; the owning broker dispatcher is still to be wired. No native Word admission change.

Evidence D:/dev/grammar-evidence/grammar-word-request-20261005/build-r2 + guest-r2/result.json/probe.log:17 PASS with actual queued TSF callbacks and inert Word objects. Normal Accept/full-source reread, cancelled pending capture/Accept, expired requests, revoked connection authority, reentrant cancellation during Word selected Text getter, changed source, changed-back source revision, moved equal-text selection, replay, missing runtime token and cancel-after-capture are covered. Negative mutations0; disconnected subscriptions and expired callback owners checked.54 subscription regressions PASS; existing direct correlation probe compiles with required authority predicate (not rerun this turn). Source-input hashes and guest binary hash match; natural process exit, guest stopped, no active Sandbox, no actual Office/account/host clipboard/user-document access. Same-integrator review only.

First guest/ FAIL is preserved: case10 expected a notification from an EMPTY write callback and therefore rejected a still-valid capture, but RichEdit did not notify that empty callback. This was an incorrect test assumption, not proof of missed actual edits. Replacement case10 really changes the selected TSF text to other and back to world inside a write session, verifies full text restored and observed revision advanced, then requires the old request to perform zero writes. It passes without weakening source/range/revision/cancellation assertions. Earlier language about every write callback must be read as every delivered change notification; empty write-callback delivery is not guaranteed by this observed store behavior. Actual Word event behavior is still unqualified.

Next: owning Word broker dispatcher with exact capture ids, pending/rejected/completed outcomes and checked retirement; add application-specific selection/privacy event invalidation and qualify actual isolated Word before enabling it. Excel/PowerPoint/Hancom remain pending. Installed84df647 product evidence unchanged; source component tests do not qualify a new installed package. Goal active/INCOMPLETE; this and preceding turn are progress, not repeated blockers.

## Word owning dispatcher — 2026-10-05

Code8a66ebf adds WordTargetDispatcher around the correlated capture/Accept sessions. Monotonic per-actor capture IDs, exact-ID result access/cancel/Accept, single pending request, single-use Accept, original operation deadlines and live authority checks are enforced. A new capture cannot replace an unsettled callback. cancel revokes permission while retaining callback/state; close returns false until actual completion and successful subscription retirement. Synchronous callback outcomes are preserved, readonly request rejection is not treated as queued work, and completed mutation results survive a later cancel. Runtime retirement must inspect close rather than rely on the destructor. This is a target-side component; target_apply bootstrap/native admission and authenticated Word routing are not enabled yet.

D:/dev/grammar-evidence/grammar-word-dispatcher-20261005/build/build-receipt.json + guest/result.json/probe.log:15 PASS in restricted-Medium Sandbox with real RichEdit TSF and inert Word object model. Includes pending capture/Accept cancellation with close=false and recapture rejection until drain; wrong/stale IDs preserving the current capture; newer capture ID growth; source/authority/eligibility changes; readonly admission rejection without phantom pending; initial/Accept expiry; wrong-thread refusal; close-before-Accept and duplicate/replay rejection. Positive cases exactly one write, negatives zero; complete synthetic editor content reread, checked disconnect and expired callback ownership required. Source inputs/output match receipts, process exited naturally and guest stopped; no active Sandbox. MSVC /W4 /WX and governance PASS. Same-integrator review only. No actual Word, broker-wire Word route, user document/profile/account or Provider live test; installed84df647 qualification remains unchanged.

Next: Word application events for document/selection/privacy invalidation and broker routing using the existing authenticated connection lease, with pending module ownership retained through cleanup. Actual isolated licensed Word qualification is required before native admission; Excel/PowerPoint/Hancom remain pending. Whole goal active/INCOMPLETE. This and preceding turn are progress, not blocker repeats.

## Word application events and actor reentry — 2026-10-05

Code7758ffb adds dormant ApplicationEvents4 invalidation and requires its exact Application identity in WordTargetDispatcher. All events, including unknown IDs, revoke the current capture; event document/selection arguments and ByRef Cancel are never read or written. Wrong-thread callbacks only revoke a separate atomic signal and return RPC_E_WRONG_THREAD, avoiding release of STA COM state on that thread. Native connection-point cookies use nonzero values (including UINT_MAX), distinct from ITfSource INVALID_COOKIE; exact cookies survive unsuccessful close. Both event and TSF subscriptions must close before actor retirement. This does not establish that Word emits all necessary privacy/revision events.

Review identified a reentry defect: FindConnectionPoint could reenter close/new capture before callback pending state existed. Unchanged production-path case19 against the pre-fix dispatcher caused stack overflow (exit -1073741571) in red-guest; previous cases0..18 passed. The failing executable/receipt/log remain under D:/dev/grammar-evidence/grammar-word-events-20261005/red-build and red-guest. Transition guards now preserve executing objects; a cancellation epoch prevents resumed startup from restoring revoked authority. Result observation and Accept preparation also guard authority callbacks. Additional case20 verifies close/recapture during selected-text observation are refused, the old capture is revoked and no source is published.

Final evidence: observation-build/build-receipt.json and observation-guest/result.json/probe.log contain21 PASS using actual owned RichEdit TSF with INERT Word objects; observation-event-build/build-receipt.json and observation-event.log contain54 inert event checks PASS. Earlier final-guest contains20 PASS before the result-observation regression was added. All positive mutations reread exact fixture content; negative mutations0, checked subscriptions/COM reference balance and expired callback ownership required. Final probe SHA256 dbebc5d2a44e28db590832327783c0757026e74aacfd7c2060ab87a147890476 matches guest/build receipt. Every final receipt input/output was rehashed before commit. Probe exited naturally, Sandbox stopped, inventory empty. Target48/selection59/subscription54 regression checks passed earlier in this checkpoint; MSVC /W4 /WX and governance PASS. First build failure from a reused variable name is retained in build/. Same-integrator review; no independent reviewer.

Official contract references: https://learn.microsoft.com/en-us/dotnet/api/microsoft.office.interop.word.applicationevents4?view=word-pia ; https://learn.microsoft.com/en-us/windows/win32/api/ocidl/nf-ocidl-iconnectionpoint-advise ; https://learn.microsoft.com/en-us/office/vba/api/word.application.windowselectionchange . The selection event does not fire for a document embedded in another document; application event coverage is not a universal revision guarantee. Real Word topology, event ordering, protection transitions, own-write events, Undo/IME, synchronous COM latency and deployment remain unqualified. Native Word admission remains OFF and authenticated broker routing is not yet wired. No actual Office/host clipboard/user documents/accounts/Provider access. Installed84df647 acceptance remains the latest installed product evidence, not this dormant source commit. Excel/PowerPoint/Hancom remain pending; whole goal active/INCOMPLETE, no main adoption/push/release. Next: connect the dormant Word actor to the authenticated target service with checked retirement, then qualify actual licensed isolated Word before activation.

## Word channel authority component — 2026-10-05

Codefcf318b adds WordConnectionDispatcher using the existing kernel-backed ApplyConnectionLease for capture and every later source/Accept validation. A dead pipe, terminated/revoked peer lease or explicit cancel removes mutation authority. Absolute request deadlines are passed through unchanged. Failed subscription retirement retains the previous actor and lease and prevents a new capture; a reentrant close revokes authority without destroying executing objects. Destructor remains best effort, never cleanup proof. The constructor and native port are internal trusted-target inputs; neither comes from wire fields. This component is not yet selected by DesktopTargetDispatcher/target_apply bootstrap: production Word admission remains disabled. Existing installed Edit/browser routes are unchanged.

D:/dev/grammar-evidence/grammar-word-connection-20261005/final-build/build-receipt.json and final-guest/result.json/probe.log:15 scenarios PASS with actual owned RichEdit TSF and real local pipe/process handles, but INERT Word objects and synthetic same-process peer. This does not test broker authentication handshake or actual Office. Normal Accept/full-fixture reread, revoked pending capture/Accept, disconnected pipe, cancellation, missing authority/opt-in, expired deadline, app event revocation, wrong/stale ids, replay and failed close retaining exact lease are covered. Successful cases one write; all negative cases zero. Callback token, lease and COM references released; fixture process exits naturally. Final executable SHA256 2fd4e2a51ca1ea78526b73ccf2617c87b8ab620bb1ec0239bf79ca3d62e3194a matches guest and receipt; all final inputs rehashed against committed source. Both guests stopped, inventory empty. build/ preserves initial fixture compile failure (noncopyable ChannelHandle assignment); build-r2/guest passes before script whitespace cleanup; final-build/final-guest reconfirms final source-bound artifact. MSVC /W4 /WX and governance PASS, same-integrator review only. No host clipboard, user documents, actual Office or Provider access.

Next: target-service routing with exact request deadline propagation and checked actor retirement, then actual isolated licensed Word topology/privacy/event/Undo qualification before admission. Excel/PowerPoint/Hancom remain pending; whole goal INCOMPLETE and active. This and previous goal turn made implementation/evidence progress, not repeated blocking waits. Main/push/release remain untouched; installed product qualification still84df647.

## Dormant Word desktop dispatch integration — 2026-10-05

Code8e162d3 connects the Word connection component to DesktopTargetDispatcher through an explicit internal trusted-root/native-port constructor. The existing Edit/RichEdit constructor is unchanged. Word capture/Accept require caller-supplied absolute deadlines; old overloads reject Word instead of inventing a fresh budget. Activation requires the originally captured field/root and still-current capture before/after focus activation. target_apply finish now waits for dispatcher.close() rather than assuming destruction completes cleanup. Word bootstrap remains rejected, so actual Word is not activated and target wire request deadline propagation remains work before that route can be enabled.

Evidence root D:/dev/grammar-evidence/grammar-word-route-20261005: final-build/build-receipt.json and final-guest/result.json/probe.log contain15 PASS through the common desktop dispatcher using real owned TSF/local pipe plus inert Word metadata. Exact full fixture contents checked after Accept/refusal; activation wrong-target refusal and missing-deadline refusal added. First guest and diagnostic-guest FAIL records remain: the hidden-launch fixture could capture but failed production visibility admission. Explicit ShowWindow setup, already used by the existing native fixtures, makes final visibility/activation checks pass without changing the product gate. Final probe1343f73eae556dd411f79218cc144e10bb16d1df7d8119e336c363a1429e9e37 matches source-bound receipt and guest. target-build separately compiles the actual DLL against the preserved84df647 app/host hashes; build-only, not installed acceptance.

IMPORTANT regression result is MIXED, not full PASS. native-regression-build plus final-guest/native runs all9 native groups. Six groups pass: standard-scope, standard-dispatcher, rich-service, standard-service, review-service, multiline-edit. The default no-autoscroll standard-edit component reproduces known Undo FAIL2 (cases0/17, stock diagnostic18 also fails; test and implementation unchanged). In addition multiline-dispatcher case10 fails at capture, and multiline-service14 cases fail. These are not reclassified as expected. Same exact binaries in a fresh dedicated multiline-guest pass service14 and dispatcher18 with natural exits, without modifying assertions/source/time budgets. This narrows the issue to order/environment/timing sensitivity but does NOT establish its cause or resolve the combined failure. Both records are retained; final-guest/combined.json remains FAIL. Existing installed84df647 qualification is historical, not evidence that this new DLL is installed-qualified.

All final source inputs/probe outputs rehashed against committed code; MSVC /W4 /WX and governance pass. Every owned guest stopped, inventory empty. No host clipboard/user docs/accounts/Provider/actual Office used. Same-integrator review only. Next: diagnose the combined native regression before promoting this candidate; continue absolute-deadline service routing and actual isolated Word qualification, then Excel/PowerPoint/Hancom. Whole goal active/INCOMPLETE; this and previous turn produced implementation/evidence progress. No main adoption/push/release.

## Native sequence diagnosis, unresolved — 2026-10-05

Code8e06a2c adds diagnostic-only ready/focus-reply/actual-foreground output before service capture. Existing short-circuit admission and assertions remain equivalent. D:/dev/grammar-evidence/grammar-native-sequence-20261005/stage-build/build-receipt.json matches every current source input after discarding the later unsuccessful experiments. guest/native logs reproduce sequence failures: review-service and multiline-service report ready=1, focus_reply=1, focused=0, foreground_target=0; the actual target service reports bound=0,pending=0. Multiline dispatcher also reports bound=0. Thus these specific service failures occur before a valid foreground/TSF setup, not during Word source replacement. This does not explain every intermittent dispatcher rejection or establish an OS-internal cause. Combined regression remains FAIL; no installed qualification/promotion.

Unadopted experiments are preserved: focus-build/final-build/final-guest use a guest-only caption-input helper, physical-build/physical-guest add temporary thread DPI awareness with restoration. The first helper still reports sent=3 without obtaining foreground. Physical-coordinate run passes7 of9 groups, but multiline dispatcher capture case6 still fails and the next fixture cannot establish foreground (exit9); the known no-autoscroll Undo2 also remains FAIL. No slow/failed cases or assertions were removed, no product thresholds changed. Helper code and all inclusion/call/build changes were removed from the candidate because improvement was not reliably demonstrated; preserved unadopted-focus.patch SHA2569cd7a765956735ade79e2822e61ec63916386e4c5b54d22b451a27094753f3fd and unadopted-owned-fixture-focus.hpp SHA25676db056496b5f61c7c45b271ea46282be470615457bb740da7c54fe6ee279c5b. No simulated-input helper enters product or adopted test code.

After failed final-guest, read-only ExistingLogin diagnostics show session1 Active, input/thread desktop Default and an available foreground HWND. These are after-failure snapshots, not proof of the state at failure. query/session-process tool exit1 with produced files is retained; desktop-state command exits0. No credential/lock/security setting operation, screenshot, host input or user document access. Every probe process reached a terminal receipt; all guests stopped, inventory empty. Microsoft reference semantics consulted: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow ; https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-mouseinput ; https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setthreaddpiawarenesscontext . No claim that these documents prove the observed cause.

Next: establish verifiable owned fixture foreground/TSF readiness and distinguish missing setup from product-path failures without counting either as PASS. Do not restore the discarded helper or repeat unchanged retries. Word wire/bootstrap remains disabled, actual licensed Office/Hancom still NOT_RUN, existing installed84df647 evidence unchanged. Whole goal active/INCOMPLETE; previous and current turns are progress through implementation/reproduction/evidence that changes the next action, not repeated blocked waits. Main/push/release untouched.

## Owned interactive desktop regression environment — 2026-10-05

Code9fb5503 adds a guest-only native runner that creates a uniquely named task-owned desktop, switches input to it, runs the existing suite in a job, then verifies natural child exit, zero active job processes, original input/thread desktop restoration and both desktop-handle closes. No default desktop ACL, registry/security setting, product gate, assertion or per-test deadline changes. Outer330s and child300s bounds stop only the owned process tree; forced cleanup would not qualify a pass. Requires WDAGUtilityAccount/Medium and the existing task-owned PowerShell7 payload. This isolates the suite from shell windows; it is not evidence of the internal cause of default-desktop failures and does not promote normal-profile/installed/Office acceptance.

D:/dev/grammar-evidence/grammar-owned-desktop-20261005/build-r2/build-receipt.json and guest-r2/: launcher-result.json reports natural exit0, launcher.log reports job_empty=1,forced=0,restored=1,thread_restored=1,desktop_closed=1,original_closed=1. Native result.json records8 groups exit0 (standard-scope, standard-dispatcher, rich-service, standard-service, review-service, multiline-edit, multiline-dispatcher, multiline-service). The ninth default/no-autoscroll standard-edit group remains exit1 with original Undo FAIL2; it is not reclassified PASS. All existing C++ assertions and diagnostic binaries are exactly the earlier native-sequence/stage-build artifacts, including the former failing multiline scenarios. Runner SHA2569323dd6f242f65d37d74a8b5db22798cd9dee822f90189c76019aa3b076ad6a7 matches guest; all build inputs/output rehashed. scripts-receipt.json separately matches both deployed PS scripts to source. The first build/guest uses OS PowerShell5.1 and has null per-test exit codes: launcher cleanup succeeds and logs exist, but that run is NOT qualified product-test success. build-r2 corrects runtime selection to the same task-owned PS7 used by MediumLaunch. Both guests stopped; inventory empty. MSVC /W4 /WX, PS parse and governance pass. Copied suite script has a harmless extra blank EOF line reported by cached diff whitespace check, not a hidden PASS. Same-integrator review only.

Reproduction mapping: deploy guest-owned-desktop-suite.ps1 as C:\GrammarPayload\Guest-WordRoute-Native.ps1 (the tested basename), guest-owned-desktop-probe.ps1 as Guest-App-OwnedDesktop.ps1, and the runner executable alongside the unchanged stage-build native probes/DLL; MediumLaunch starts the outer guest script. Existing mapped output contains native case receipts separately from launcher-result.json. Official API references: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createdesktopw and https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-switchdesktop . No host UI/input/clipboard or user docs/accounts used. This establishes a working isolated regression path; old default-desktop failures remain preserved and unresolved. Next: continue Word service deadline propagation and real app qualification; isolated licensed Office/Hancom VM/install-media information has been requested and is pending. No activation beyond qualified Edit/browser; whole goal active/INCOMPLETE, no main/push/release. Previous/current turns are evidence/implementation progress, not blocker repeats.

## Native receiver deadline propagation — 2026-10-05

Code e6745f5 forwards one explicit absolute request deadline through common Desktop dispatch to Standard Edit/RichEdit (Word already required it). Target receive, queue and edit callbacks now share the receiver's initial5s execution budget instead of refreshing it after receive/STA scheduling; response is bounded by receiver start+6.5s and connection expiry. Already-expired connection refuses before unsigned subtraction. Source reads and the final mutation boundary recheck expiry; already-performed mutation remains applied/uncertain, never relabelled safely rejected. Existing30s review authority stays separate. No assertion/performance threshold was raised to pass a failure.

Evidence D:/dev/grammar-evidence/grammar-native-deadline-20261005: build-r2 MSVC/W4/WX and all32 input hashes match code commit via commit-link.json; original dirty-build parent625e9eb remains in build receipt. Both owned Medium guests have8 supported groups PASS, standard and multiline dispatcher22 each including expired admission and delayed callback refusal. Known no-autoscroll Edit Undo0/17 remain FAIL2 in the ninth group. Natural exit, active job0, original input/thread desktop restore and handle closure verified; guests stopped. RESULT.md and REVIEW.md distinguish source/native qualification from installed84df647 evidence. Governance/self-test PASS; same-integrator review only.

Remaining: wirev2 has no sender absolute expiry, so receiver/client clocks and deadlines are not yet an end-to-end contract. Blocking COM is not preempted; do not claim actual cancellation from timeout. RichEdit service regression passes but dedicated queued-expiry RichEdit cases remain. Word bootstrap disabled; licensed isolated Office/Hancom qualification remains pending, named other adapters incomplete. Whole goal active/INCOMPLETE; main/push/release unchanged. Next: sender-expiry transport and callback tests, then application deployment qualification. This turn made implementation/test progress, not a blocked wait.

## Sender expiry transport — 2026-10-05

Code9e1db2b upgrades the shared native/browser request header to v3 (72 bytes) with required absolute same-machine GetTickCount64 expiry. Missing/expired/more-than6500ms future/oldv2 requests revoke admission. Body receive, native server execution/reply caps and browser pending reply clamp to sender expiry. Both clients send the same deadline used for their response wait; no replay/fallback or timeout-budget increase. This is not a claim of preempting COM calls. Official API basis: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64 ; timer resolution is coarse, not submillisecond.

Evidence D:/dev/grammar-evidence/grammar-channel-expiry-20261005: three source-bound MSVC/W4/WX builds; all input/deployed output hashes verified and linked to code commit while retaining original dirty-build parent7f0fa2f. guest-r2 real channel37 PASS (4 new expiry cases), browser bridge13 PASS,8 supported native groups PASS incl Standard/multiline dispatcher22 each. Known ninth no-autoscroll Edit Undo2 FAIL remains unchanged. Natural child exit/job active0/original desktop+thread restore/handle closure verified, both guests stopped. First guest NOT_RUN due existing MediumLaunch filename allowlist; guest-r2 uses existing allowed entrypoint, no guard weakening. scripts-receipt.json records coordinator/owned-wrapper/suite mapping; RESULT.md and same-integrator REVIEW.md carry limits. Governance and PS parse PASS; cached diff check found harmless extra EOF blank line, recorded. No independent review.

Next: exact raw tick expiry at callback mutation boundary (one-time steady_clock conversion remains a precision ambiguity), RichEdit queued-expiry counterexamples and DOM deadline propagation audit. Actual isolated licensed Office/Hancom is still pending, Word bootstrap disabled and other named adapters incomplete. Installed84df647 and protected main4e833988 unchanged; no new installed acceptance/push/release. Source preview is package-only. Whole goal active/INCOMPLETE; current and previous turns made code/evidence progress, not blocked waits.

## Native callback raw expiry — 2026-10-05

Code e20783d keeps original wire-domain operation ticks through native Command/common dispatch/Standard session/RichEdit capture and ApplyGrant. Admission, source reads and final pre-mutation authority check raw GetTickCount64 as well as steady deadline; time conversion is no longer the only safeguard. Capture request expiry is separate from30s review grant, and Accept supplies its own bounded expiry. Post-write outcome remains applied/indeterminate, not falsely no-write; no COM preemption claim. Dormant Word route refuses finite wire deadlines until it adopts this authority; Word bootstrap remains disabled.

Evidence D:/dev/grammar-evidence/grammar-callback-expiry-20261005: corrected guest-r2 has9 supported groups PASS, Standard/multiline26 each and new real RichEdit9. Rich deferred cases assert pending before sleep, expired captures have reads0, expired Applies mutations0 and actual source unchanged. Positive case reads1/mutations1 and exact expected replacement. Tenth noautoscroll Edit keeps known Undo FAIL2. Original first guest deliberately interrupted for accidental suite argument-array comparison (NOT qualified); same binaries rerun with original comparison restored. No assertion/budget weakening. Final natural exit/job0/original input+thread desktop restore/handle close verified; both guests stopped. W4/WX/governance/hash links PASS; all guest scripts match source. Original dirty-build metadata retained; commit-link.json binds inputs to code. Same-integrator review only, no new installed84df647/main/push/release acceptance.

Next evidence-driven task: worker.js desktop-apply message around176 has no operation expiry; content.js applyDesktopReview around435 checks only review grant/current source, so a delayed message could precede cancellation after the worker3s budget. Reproduce before claiming defect, then repair end-to-end worker/document expiry and test no late mutation. This is separate from native tests and not yet a live failure. Continue named application adapters/real isolated Office-Hancom qualification thereafter. Whole goal active/INCOMPLETE; previous/current turns are implementation and evidence progress.

## Chromium worker-to-document expiry — 2026-10-05

Code9aa4135 fixes delayed desktop Apply: worker issues one3s monotonic deadline before async lookup, sends it to content, refuses expired dispatch; content requires finite/bounded/current deadline at capture/Apply and post-beforeinput mutation check. Review30s authority remains separate. Timestamp translation uses performance.timeOrigin+performance.now (https://www.w3.org/TR/hr-time-3/), not wall-clock adjustment. Missing/old messages fail closed.

D:/dev/grammar-evidence/grammar-dom-expiry-20261005 preserves red-source/red-guest: new delayed-delivery test on old production returns Applied3 instead of rejected0 after3100ms. Green-source/green-guest full modified isolated Chromium fixture 76 PASS, including exact unchanged textarea/CE rereads, missing/malformed/unbounded deadlines and beforeinput operation expiry. Node worker/core66 PASS including expired async tab lookup with timer callback not fired; JS syntax/governance PASS. Source hashes match9aa4135. Both guest fixtures clean registrations/profiles/host processes and uninstall complete, guests stopped. Actual installed330ab3b host copy+modified source extension/synthetic Deep child, not unmodified installed/native widget acceptance. Same-integrator review only; REVIEW.md/RESULT.md preserve scope. Installed84df647/main unchanged, no push/release.

Next verified code boundary: GrammarBrowserServerPoll in browser_bridge_api.cpp returns command/id/text but drops wire deadline; review.rs Command has no deadline, so host-to-worker delay may mint a fresh3s budget. Implement conservative native/browser monotonic anchor and versioned expiry propagation, with delayed-host-output rejection before claiming end-to-end deadline safety. This current fix closes only worker-to-document race. Continue app adapters/isolated licensed Office-Hancom qualification; whole goal active/INCOMPLETE. Previous/current turns made product/evidence progress, not repeated blocker waits.

## Native-host expiry and installed requalification — 2026-10-05

Code f15d45681531e6a90eb500600ca7bd9d36c12096 preserves original native operation expiry through DLL Server ABI v2, Rust review protocol v2 and MV3 worker. Browser sends a monotonic anchor before native receipt; CreateV2 crosses the next native tick boundary (bounded100ms) to avoid a quantization-forward anchor. PollV2 returns the server-clamped sender expiry. Host conversion subtracts startup/transit time; worker refuses missing/nonfinite/expired/>3s future deadlines and forwards the same stamp to the existing DOM final check. The30s review grant is separate. Old ABI/protocol fails closed; paired host/DLL/extension deployment required. No COM preemption or hard-real-time/suspend-clock guarantee is claimed.

Evidence D:/dev/grammar-evidence/grammar-host-expiry-20261005: Rust host21, node worker/core73, actual DLL ABI6, actual Rust host+DLL synthetic-peer relay6 and modified isolated Chromium79 PASS. Last includes native host/worker round-trip textarea/CE exact selected replacement and changed-source refusal. Source fixture uses synthetic desktop peer and synthetic Deep, not an installed app. Initial ABI test signature compile failure and client-capture guard compile failure are preserved (api-build/api-build-r2); corrected api-build-r3 compiles W4/WX. client-build receipt includes one changed but uncompiled browser_bridge_api.cpp file; commit-link.json records mismatch rather than claiming every input matches. Actual relay DLL was built after the repair. Duplicate transitive-file hash listing removed after qualification; no binary behavior change.

Clean release/build-receipt.json and package-f15d456.zip bind source to app/host/DLL. Fresh restricted-Medium guest-installed: unmodified extension + installed Tauri global shortcut/edited Accept10 PASS, actual textarea/CE rereads, stale source/range, sensitive field, Dismiss/rearm/replay. guest-native: same package multiline Edit15 PASS including direct replacement/Undo, stale/privacy refusal and actual DLL unload with editor alive. Both apps exited naturally, both uninstalls complete with install/registration/data/processes0, all five guests stopped. This updates installed qualification from84df647 to f15d456 only for the stated synthetic editor scope; no generic editor claim. App a87564cc71fa12ba36d6c5f352ed4e3e9782271a32c9d1821994547e6191d88e; host8df0f91274d627f79835da89121aa619e6fc30970ab3d5d228a1d456b1ef07cc; DLL02e4f0a10dba72d0fae2b3b596a356241dc2ed47712363a086468144956a201b; ZIP57e4eb661774c540cfbb0e095e2ed0cf94076e20d8aee2f37c0338f392846e9c; extension pnjdajaokpljfmhfmmmjmdghpchldlhb. REVIEW.md is a separate pass by the same integrator, not independent review.

Next concrete code boundary: dormant Word dispatcher still refuses finite wire expiry. Thread original ticks through its connection/request gate and native observation checks, using queued-callback counterexamples before qualification. Word bootstrap stays disabled until actual application safety/installation is qualified. Excel/PowerPoint/Hancom adapters remain incomplete; actual licensed isolated Office/Hancom access is still pending. Known no-autoscroll Edit Undo failures remain outside qualified scope. Live Provider, physical IME, general profiles and suspend/resume clocks NOT_RUN. Whole goal active/INCOMPLETE; no main adoption/push/release. Current turn made implementation and installed evidence progress, not a repeated blocker wait.

## Dormant Word sender expiry — 2026-10-05

Code11aa022 carries finite original operation ticks through common desktop routing, Word connection/actor/session gate and metadata observation guards. Admission, callback entry and final source/binding verification retain the same raw tick deadline alongside steady time. Metadata guards use min(review, operation) expiry. Capture gate is released after successful capture; a fresh explicit Accept supplies a new operation deadline within the existing30s review grant. The common route no longer unconditionally refuses finite wire expiry, but target_apply.cpp bootstrap still admits only qualified Edit/RICHEDIT50W and never constructs Word. No Office activation or new editor support is claimed.

D:/dev/grammar-evidence/grammar-word-expiry-20261005: real Windows TSF and pipe with inert Word object model on an owned restricted-Medium desktop. Route21 PASS, actor21 PASS. Six new cases: expired capture admission; deferred capture expiry with Word Text getters0; expired Accept admission; deferred Accept expiry with no additional Word Text getter; expiry during a getter with exact unchanged editor; and successful fresh Accept after capture-operation expiry. Existing cases retain stale/channel/event/reentrancy/privacy/source and callback/ref cleanup checks. Launcher natural exit/job_empty1/forced0/input+thread desktop restoration and handle closure confirmed; guest stopped. Both source-bound probe builds and production DLL W4/WX compile PASS. route-build-r2 adds explicit read counters before any guest execution; first build superseded, not a failed test. Original dirty receipts retained, exact source inputs matched to code in commit-link.json. REVIEW.md is a separate same-integrator review, not independent.

Installed f15d456 remains current qualified installed package; this dormant code is not a new installed acceptance. Blocking COM getters cannot be preempted: a call already in progress may finish after expiry, then authority fails; no cancellation-completed claim follows. Actual Word document model, deployment, revision events, privacy/Undo/IME remain NOT_RUN. Excel/PowerPoint/Hancom implementations remain incomplete and require editor-specific evidence; no generic read-then-write substitute is authorized. Next focus is the actual application-adapter deployment/correlation boundary and remaining named adapters, with licensed isolated Office/Hancom environment still pending. Whole goal active/INCOMPLETE; current turn made product and native-test progress. No main adoption/push/release.

## External application environment required — 2026-10-05

After completing independent expiry repairs, the next actual application integration is blocked by the same isolated Office/Hancom prerequisite recorded in d9095f4 and25f56b0 and rechecked this turn. Read-only Hyper-V inventory finds only ManeuvSim-Clean-Golden, whose Grammar ownership is unestablished; no guest access or change attempted. No active Grammar Sandbox; task payload has probes, no identified Office/Hancom installation media. This is not proof that the user's host/unrelated VM lacks Office, and does not authorize using them. No running test is being awaited.

D:/dev/grammar-evidence/grammar-application-environment-20261005/ENVIRONMENT.json and HANDOFF.md record the evidence, full-scope completion gaps and exact resume boundary. Need an accessible task-owned isolated Windows VM with Word/Excel/PowerPoint and Hancom installed, or installer locations and isolated installation arrangement; Owner handles any account/license activation. Latest installed qualified candidate remains f15d456; current production source11aa022 includes dormant Word expiry. Word actual-provider/deployment remains NOT_RUN; Excel/PPT/Hancom adapters incomplete. Repeating model-only tests would not resolve the next real application correlation/atomic-mutation requirements. Goal INCOMPLETE and externally blocked, not complete or Owner-paused. Preserve all code/evidence and resume actual app integration when the environment is supplied. Main/push/release untouched.

## LibreOffice alternative unblocks independent application work — 2026-10-05

Owner proposed LibreOffice and simple XML in Sandbox. This corrects the previous section's overly broad blocker: actual Microsoft Office/Hancom qualification still needs their isolated applications, but LibreOffice implementation and testing can proceed now. Do not treat missing licensed applications as a blocker for every named-format task. No Office/Hancom or universal-editor equivalence is claimed.

Added scripts/mission/guest-libreoffice-format-probe.ps1. Official TDF Portable26.2.4 archive SHA2564bde93374aef4409243505b20d16561a4628ac7591457dd01fc6e1ccf571ba65 matched publisher digest. Installed runtime identifies26.2.4.2/build0229ac93fcf0d7cbc6376066c6f35021cef002dc. License reviewed: engine MPL2 plus bundled third-party terms; wrapper separate license. Host only passively extracted/repacked files with existing7-Zip25.00; no installer or LibreOffice runtime executed there. Final guest directly extracts original PAF with mapped existing7-Zip, uses its own profile, restricted Medium token, network/clipboard/audio/video/printer disabled. Unused host engine.zip is preparation only.

Evidence D:/dev/grammar-evidence/grammar-libreoffice-20261005: guest-r3 real engine Flat ODF XML -> DOCX/XLSX/PPTX -> Flat ODF3 PASS with exact English/Korean/emoji paragraph/cell/slide text, artifact hashes and actual rereads. All operations exit naturally; remaining LibreOffice processes0, profile and extracted app removed; all three owned guests stopped. guest and guest-r2 FAIL preserved: automatic format detection imported minimal XML markup as plain text. Saved r2 roundtrip paragraph proves this; r3 explicitly specifies documented Flat ODF input filters, without changing expected text or timeout assertions. First run lacked retained documents; r2 repaired failure-artifact retention. No Grammar production code changed.

Reproduce using existing guarded MediumLaunch, script mapped as Guest-App-LibreOffice.ps1, host-machine.txt, PS7, and readonly C:\GrammarLOTools mapping containing publisher-verified engine.paf.exe plus existing7z.exe/7z.dll; C:\GrammarOutput writable and unique. Exact configs/scripts/hashes are in external evidence. Engine extraction bound300s; each conversion60s, output drain5s. These are test operation ceilings, not product latency claims. Synthetic documents only; no login or Provider.

This is headless real application-engine qualification, NOT UI selection/Accept acceptance. Next: LibreOffice-specific application bridge/capture and explicit Accept with exact document/range/source/revision, stale/user-edit refusal, Undo and cleanup in the same isolated environment. Current standard Edit/Word paths must not be enabled on LibreOffice just because formats open. GUI/IME, Grammar Accept in LibreOffice, complex formatting, HWP/HWPX and actual Microsoft Office/Hancom remain NOT_RUN. Installed Grammar qualification remains f15d456; mission INCOMPLETE. No main adoption, remote push or release.

## LibreOffice actual GUI and target-thread component — 2026-10-05

Added guarded guest-libreoffice-selection-probe.ps1, real UNO selection/Undo probe, and a test-only Python UNO component plus independent affinity client. No Grammar production activation or installed package changed. Same publisher-verified LibreOffice26.2.4.2 and bundled Python, offline restricted Medium Sandbox, fresh disposable profile, document macros NEVER_EXECUTE. OXT registration is profile-local only, no shared/system registration and no macro security change. Task synthetic documents only.

Evidence D:/dev/grammar-evidence/grammar-libreoffice-selection-20261005. First direct probe17 successful checks then FAIL: Impress direct shape.String replacement creates no Undo action. R2 Text/SelectAll/InsertText alternative lost its UNO bridge; terminate() exception obscured the original failure, fixed by preserving failure before cleanup. R3 verifies actual window visibility and adds Writer duplicate-string/distinct-range and user-edit/source/revision observations (19 successful checks), but remains FAIL on Impress Undo and later crashes soffice.bin during cleanup. Owned guest Application event1000/1001 confirms crash, not its cause. Historical failures retained.

Pinned upstream source inspection identifies per-call SolarMutex in setString and insertString delegation; swapping those methods cannot by itself prove transaction safety. With process-local SAL_SKIA=raster, R4 again has19 successful checks and unchanged Impress Undo FAIL, but closes all3 documents, terminates desktop, natural office exit0, remainingProcesses0 and cleanuptrue. One raster run does not prove graphics was the crash root cause. No deadline increased and no failed assertion removed. Writer/Calc observations are not full Grammar acceptance.

Separate guest-affinity4 PASS: own temporary org.grammar.fixture.Affinity component loads in LibreOffice rather than the external Python driver; AsyncCallback runs in actual Writer window PID8096/thread8100, exactly matches native GetWindowThreadProcessId, reads the synthetic selected string with document unchanged, rejects replay. Natural Python/Office exit0, owned profile/app removed, all five guests stopped. This component has no mutation method. Random local UNO pipe is fixture plumbing, not qualified authenticated product IPC. Source/package hashes and exact guest configs are in external evidence; no general user-profile deployment.

Next: reuse this proven target-thread execution boundary for a Writer review transaction with original operation deadline, exact document/controller/range/source/revision, opt-in, privacy refusal and authenticated transport. Test edits queued before Accept, equal text at another range, source changes, reentrant events, deadline expiry and Undo against actual Writer. Affinity alone does not prove continuous lock ownership/non-reentrancy/atomic mutation. Impress Undo remains unresolved; Microsoft Office/Hancom/HWP/HWPX, physical IME and complete Grammar shortcut/Instant/Accept in LibreOffice remain NOT_RUN. Qualified installed Grammar stays f15d456; mission INCOMPLETE, no main/push/release.

## Writer target-side transaction experiment — 2026-10-05

Entry a1bcae7, same mission worktree. Added synthetic-only in-process UNO Writer
review component/client and adversarial in-process hooks. New unsaved plain-body
Writer only: actual PID/UI-thread affinity, selected range/source/revisions,
one-shot token, original <=3s operation and separate30s review/cache expiry,
explicit enable/disable, exact rereads and native Undo. No installed Grammar UI
integration, authenticated product IPC, general privacy qualification, saved DOCX,
Calc/Impress/Hancom support or product activation is implied.

Evidence D:/dev/grammar-evidence/grammar-libreoffice-writer-20261005 preserves
all seven runs. Basic14 and external-listener18 checks passed; later external
listener run lost its URP bridge (FAIL, not acceptance). Target-internal red-local
reproduced two defects: pending disable returned BUSY and outer APPLIED with a
write; modification callback user edit was followed by an incorrect appended
replacement yet APPLIED. Immediate epoch revocation plus UI-thread detachment
and scoped modified-state notification batching fixed those specific cases.
Green27 PASS includes exact Undo/dirty flag restoration, permission revocation,
source/selection changes, expired reads, protected section, real30s cache expiry
and document disposal. Budgets/assertions unchanged. Launcher now waits its
existing15s natural-exit budget even after a failed Python test, rather than
prematurely killing the Office wrapper.

Final expanded guest-undo remains FAIL:27 checks PASS, undo-callback-user-edit-
preserved FAIL. Actual target undoActionAdded edits the document during setString;
final body is USER INSIDE UNDO CALLBACK plus Grammar replacement, reported
APPLIED. Thus batching modification notifications is insufficient for atomic
replacement. Do not ship this prototype. The failing assertion remains strict.
Both latest guests exited Office0 without force, cleanuptrue/processes0, and were
stopped. Earlier red-local forced cleanup failure is preserved, not relabeled.

Pinned LibreOffice26.2.4.2 source shows SwXTextCursor::DeleteAndInsert performs
DeleteAndJoin then insertion. insertString(absorb=true) calls the same operation,
so changing that API name is not a repair. XUndoManager.lock disables Undo
recording; it was not used to erase required Undo behavior. See external upstream
source snapshots and REVIEW.md. Next: investigate a native single replacement
operation or safely deferred Undo notifications with preserved native Undo;
re-run the same target-internal failure before any product IPC/UI connection.
Full-document rollback, generic paste and weakened assertions remain excluded.

Same-integrator separate review only. Syntax/OXT byte matching and governance
checks are distinct from native result; source preview is candidate package-only.
Latest qualified installed Grammar package remains f15d456; main4e833988
unchanged, no push/release, mission INCOMPLETE.


## Writer activation continuation — 2026-10-05 (core/native checkpoint)

Owner explicitly requests continued work through product Writer activation.
The bc58b4d prototype's callback corruption is addressed with bounded custom Undo
recording, post-callback validation, no whole-document rollback, exact range
Undo/Redo and disposal of volatile selected text. Core moved into
adapters/libreoffice/pythonpath; guarded test entrypoints remain in mission tools.
The optional Writer native client pins exact soffice.bin path/hash and trusted
broker, and the app routes SALFRAME capture through it with no unsafe fallback.
Default native builds keep Writer disabled. No installed product acceptance yet.

Evidence: D:/dev/grammar-evidence/grammar-writer-activation-20261005.
R8 soffice crash remains unresolved, not silently reclassified. R9-R11 pass after
removing an unnecessary retained cursor; causality is unproven. R14 documents
exact successful mutation with lost native response; early pipe disconnect was
reproduced by a failing bridge test. R15 preserves terminal reply lifetime and
passes the actual GUI/native tests,32 repeated Undo/Redo cycles, wrong-image
refusal, exact reread, natural exit and cleanup. Bridge6 tests, Rust target Apply6,
typecheck/cargo-check and MSVC W4/WX compile checks pass. These are not installed
widget acceptance, saved DOCX qualification, physical IME or live Provider tests.

Next: product-only visible opt-in/disable entrypoint, paired installation,
authenticated installed hotkey/Instant/reviewed Accept, negative cases and review.
No main adoption, remote push, normal-profile installation or live Provider use.


### Writer product-entry checkpoint (R21)
Product-only OXT registers only the visible review-panel job/menu, not the fixture
XJob or hooks. Paired library path/hash config, default-off, explicit per-document
Enable, Disable/close and one-review completion are implemented. Native builds
still require explicit Writer image/path input; defaults remain disabled.
R21 product extension/native client9 checks PASS: actual menu opens disabled,
accessible Enable, exact native capture/Accept reread/Undo, no silent rearm and
Disable refusal. Office/Python exit0, natural cleanup and processes0. R16-R18
failed test accessibility lookup (pinned26.2.4.2 uses peer XAccessible property;
button names include generated mnemonics). R19 foreground assertion and R20
setup focus timeout remain FAIL; R21 adds diagnostics and passes, not proof that
all focus variability is resolved. Installed Grammar widget is still NOT_RUN.
Package boundary2 and bridge6 tests PASS; syntax/governance PASS. Product widget
fixture/guest installer is prepared for the next clean source-bound build.
No normal-profile install, saved-document support, main adoption or publication.


### Installed Writer flow and post-review boundaries (R23/R24)
Clean10737e9 build receipt, package and paired product OXT are preserved in
D:/dev/grammar-evidence/grammar-writer-activation-20261005. R22 stopped at the
Medium launcher naming guard before install; renamed the guest script to satisfy
the existing guard. R23 installed widget6 checks PASS: actual hotkey/Instant,
edited Accept exact Korean/emoji surrounding-text reread, Undo, changed-source
preservation, Disable refusal and natural exit. Install/removal/Office cleanup
complete, processes0. Revoked channel honestly shows outcome unconfirmed; fixture
accepts that documented transport outcome and still requires zero text changes.

Post-review changes bind the greeting to the exact live Writer window, require
foreground document for capture, and reject non-default formatting through actual
text portions before source reads. R24 actual GUI/core/native54 checks +32 cycles
PASS; shared browser/Writer Windows transport15 cases PASS, processes0. Aggregate
cursor property states can remain DEFAULT_VALUE after direct formatting; the new
negative fixture proves actual bold text150 and refusal before source read. These
are safety improvements after the installed10737e9 baseline, not a claim that the
older artifact already contains them. Multiline and final installed recheck next.
No provider/accounts/user docs/profiles/main/push/release. Mission INCOMPLETE.

R25/R26 post-review core57 checks PASS, including multiline replacement/Undo/Redo,
all prior user-edit races, non-default character format refusal before source read,
exact native peer/window handshake and32 repeated cycles. Both natural exit0,
cleanuptrue/processes0. The shared transport's15 actual Windows scenarios also
PASS in R24. Next clean final build and installed-widget recheck; standalone
package generation or this core result does not close that gate.


### Final installed Writer activation (R27, 2026-10-05)
Runtime commit 0fc862498206f536adf5156fa7d238792c89efc5 is built from clean
source and qualified with its paired product OXT. R27 installed-widget6 checks
PASS: actual shortcut -> Instant -> edited Accept -> exact Korean/emoji Writer
reread -> Undo; concurrent source edits preserved, Disable prevents mutation,
and app exits naturally. Office/Python exit0, cleanuptrue, forcedfalse,
remainingProcesses0; uninstall complete with no install/registration/data residue.
All owned Sandboxes stopped. This activates the product candidate for the exact
pinned LibreOffice26.2.4.2 new unsaved default-style plain-body scope only.

R26 core57 checks plus32 repeated transactions and R24 shared transport15 cases
PASS. Rust all targets365 test executions PASS,39 ignored (includes repeated
modules across targets), typecheck/build/provider contracts/bridge6/package2 PASS.
The window-chrome contract initially FAILed because it still forbade any Apply
branch. Its test and App.tsx were identical at entry bc58b4d and runtime0fc8624.
The revised AST check preserves visible draft/status after Copy or Apply, explicit
Dismiss, captured Apply capability and stale-result checks, and rejects success
wording for uncertain outcomes. No product code changed for this test repair.
Original failure log remains node-window-0fc8624.log. The later evidence/test
commit is not the source of the runtime binary; no rebuild is implied.

Evidence root: D:/dev/grammar-evidence/grammar-writer-activation-20261005.
Runtime receipt final-build/receipt.json; product-package-0fc8624.zip SHA256
02202cc38d3e3ba600e94f9e009a94740ad6548da5a84552c6d8c579e9c7834e.
Paired GrammarWriter-installed-0fc8624.oxt and extension-0fc8624.json bind the
installed DLL path/hash. guest-r27 contains app/UNO/launcher/uninstall receipts.
App SHA256 45c3d0cf67b5ee005d2207f899c2f6bdf087ef2ad700fd2c365e7a693edeeb76;
DLL SHA256 61c50bb11ae3243efe29b2cadd74bfbc3c4e944d8dfdb2e4767167002e471376.

Lost/revoked transport still reports Apply outcome unconfirmed and never retries;
zero changes are established by the fixture reread, not that status alone.
R8 crash cause and R20 setup-focus variability remain unproven historical limits.
Review was a separate pass by the same integrator, not independent human review.
Saved DOCX/rich documents, physical IME, ordinary profiles, live Providers and
actual Microsoft Office/Hancom are NOT_RUN. Default builds remain Writer-disabled
unless supplied the explicit pinned Writer image/path; paired OXT activation is
explicit per document and one review. No normal-profile install/main/push/release.
Mission INCOMPLETE. Next broader Writer document/format qualification requires
preserving exact range, formatting, Undo and user edits before expanding admission.


## Candidate Source replacement checkpoint — 2026-10-05
Owner requested a replacement package from the current local candidate. Generator
adds an explicitly authorized candidate snapshot mode, preserving default main-only
and non-applicable preview modes. Source02 reports the measured candidate branch,
not the historical repository_branch=main field. Exact12 and integrity checks
remain. Installed runtime remains0fc8624/R27; main4e833988 unchanged, mission
INCOMPLETE. Upload/fresh-chat verification remain Owner work, never claimed here.
Next development: saved synthetic Writer DOCX admission and exact Apply/Undo
qualification; retain disabled saved/rich-document admission until verified.


### Post-package Writer saved-document prerequisite
The replacement Source snapshot is frozen at bf3d3f9; do not silently rewrite it
for this subsequent test-only continuation. Actual restricted-Medium Writer suite
now61 checks PASS plus32 repeated transactions. Added real Save As DOCX exact-text
preservation, prior-token refusal before source read, reopen exact text and saved
admission refusal; disk SHA256 unchanged after denied operations. Production core
is unchanged and saved/rich Writer admission stays disabled. Python/Office exit0,
cleanuptrue/forcedfalse/processes0, guest stopped. No installed-widget rerun or
saved-document Apply support is claimed. Evidence:
D:/dev/grammar-evidence/grammar-writer-saved-boundary-20261005/guest.
Next implement saved-document identity/lifecycle and format-preserving transaction
only after adversarial qualification; no user documents or normal profiles.


## Source semantic repair after Owner verification
The reported SEMANTIC_FAIL identified a stale F-06 broad OTHER_EDITORS_UNQUALIFIED
summary despite installed bounded Writer qualification. Current verdict now names
Edit/RichEdit, Chromium and bounded Writer, with explicit per-adapter scope and
remaining unqualified cases. Historical verdicts/receipts remain untouched.
Owner intentionally retains eight old Project Sources: exact12 now means current
authoritative00-11, not total Project inventory. Prompts retain missing/duplicate/
ambiguous-current-version refusal and never treat retained history as authority.
Regenerate Source02 and fresh-chat expected JSON together. No product changes,
new runtime tests, main adoption or claimed external SEMANTIC_PASS in this repair.


## Supplied legacy ZIP integration
Read all seven Markdown files in Owner Downloads.zip. Preserved exact originals
with SHA256 in LEGACY_SOURCE_INTEGRATION and mapped durable requirements into
Charter/Architecture; Source11 includes the complete mapping/history appendix.
Historical no-task/no-P3B and R6 main/artifact/account/acceptance statements are
not current authority. Multi-provider architecture plan is absent:7/8 integrated,
1/8 NOT_READ_NOT_INTEGRATED. Exact current12 export retained; no Project deletion,
product code change or new runtime acceptance. External fresh-chat verification
still required. Evidence grammar-legacy-source-integration-20261005.


## Saved Writer implementation candidate — 2026-10-05
Added bounded local plain DOCX admission with exact live object/view/URL and
save/lifecycle event revocation, preserving30s review/original3s request budgets.
The generated OXT switch is explicit and default-off. Product panel can consume
that packaged switch; no arbitrary UNO request or document can grant it.
R2 core69 checks+32 cycles PASS, R3/R4 all73 assertions pass but final terminate
response disconnect was marked FAIL; actual launcher observes Office0/cleanup.
R1 foreground setup FAIL preserved. R5 keeps only termination-call DisposedException
as unconfirmed transport, requiring launcher natural exit0/processes0/cleanup for
final success. Core and installed-widget qualification are still being completed.
Existing installed acceptance0fc8624/R27 stays new-unsaved only until new receipts.
Evidence: D:/dev/grammar-evidence/grammar-writer-saved-apply-20261005.


## Saved plain DOCX installed qualification — 2026-10-05
Product extension source5a339b8 with explicit --enable-saved-docx is now qualified
in the owned offline pinned26.2.4.2 Sandbox with installed app/native0fc8624.
These are two separately bound artifacts, not a claim that the old app commit
contains the new Python adapter. OXT SHA256:
997ed14943ad2e8d6cce0a1dc9e583dabcbd359af8c5433c5156725f69c3551b.
The paired DLL hash remains61c50bb11ae3243efe29b2cadd74bfbc3c4e944d8dfdb2e4767167002e471376.

Core R5:73 checks +32 Apply/Undo/Redo cycles PASS, actual saved/reopened DOCX,
Save and SaveAs A-B-A revocation, Save inside Undo entry, user edits, readonly
refusal and event detach. R1 foreground failure and R3/R4 termination reply
FAIL preserved. R5 does not equate a disconnected terminate response to cleanup:
launcher independently requires actual Office exit0/processes0/profile removal.

Installed product guest-product-r1:6 widget checks PASS plus actual DOCX reopen
and unchanged disk hash (no automatic file Save). Real hotkey/Instant/edited
Accept/exact reread/Undo, source edit preservation, Disable and natural app exit.
Office/Python0, cleanuptrue/forcedfalse/processes0; uninstall complete. Guest
stopped. Default OXT builds remain saved-disabled; qualified candidate uses the
explicit packaged option and visible per-document one-review opt-in. No automatic
Save or new rich formatting support. Current F06 scope includes this bounded saved
case while preserving the previous unsaved-only scope as historical export data.

Evidence: D:/dev/grammar-evidence/grammar-writer-saved-apply-20261005.
Package3/bridge6 checks and governance PASS; no Rust changes, no full Rust rebuild
claimed. Same-integrator review only. Installed original app/native hashes checked
by the widget fixture; OXT receipt binds every Python/config input. Next: preserve
rich formatting through reviewed mutation and Undo, and qualify additional DOCX
structures before expanding admission. Actual Office/Hancom, physical IME, normal
profiles and live Providers remain NOT_RUN. Mission INCOMPLETE; no main/push.


## Uniform Writer formatting candidate — 2026-10-05
The next bounded capability preserves homogeneous Western/Asian/complex weight,
opaque non-theme RGB and paragraph alignment in one Standard body paragraph.
Defaults remain off; the paired OXT exposes --enable-uniform-format explicitly.
Unknown interop metadata, theme/transparency, mixed runs, hidden text and multiple
paragraphs stay refused. Formatting plus effective style defaults participate in
capture validation and custom Undo fingerprints. RGB DOCX roundtrip metadata is
strictly matched, not a generic permission to copy arbitrary metadata. Product
entry/fixture additions are isolated; no change to app/native binary0fc8624.

Owned Sandbox R10:28 native checks and16 formatted Apply/Undo cycles PASS, including
saved/reopened DOCX and unchanged file hash. R1/R2/R7 postwrite failures, R3/R4/R5
saved-format limitations, R6 unused-complex-color assumption, R8 UNO array typing
and R9 incorrect newline fixture are preserved. Actual paragraph break fixture
now verifies two paragraphs before refusal. R10 natural Office/Python0, cleanuptrue,
forcedfalse/processes0. Existing plain regression and installed widget qualification
are next; no F06 or general Writer acceptance promotion yet.
Evidence:D:/dev/grammar-evidence/grammar-writer-uniform-format-20261005.


## Uniform Writer installed qualification — 2026-10-05
Product OXT2a16115 (aa1f4df00d357f5e0b2127cd3360c2e9d25c4157f55f4d65e1410a2d6ce509dc)
with --enable-saved-docx --enable-uniform-format and unchanged app/native0fc8624
passes guest-product-r1:8 installed widget checks plus5 enclosing checks. Actual
hotkey/Instant/edited Accept/reread/Undo preserves every portion's script weights,
RGB and alignment. Source edit and Disable preserve the document. The Disable
UI reports "Apply outcome unconfirmed"; exact reread separately proves unchanged
text, not a confirmed protocol rejection. No automatic retry or Save.

Qualification is only pinned LibreOffice26.2.4.2, owned fresh profile, one Standard
body paragraph with homogeneous supported weights, opaque non-theme RGB and
alignment, new or local DOCX. No mixed formats, arbitrary interop metadata, theme
color, hidden fields, multiple formatted paragraphs or complex structures.
Ordinary product builds still default both capabilities off; previously qualified
plain mode stays separate. This is not general rich DOCX or normal-profile support.

Plain regression:73 checks/32 cycles PASS. Formatted R10:28 checks/16 cycles PASS.
Product app/Office/Python natural0, forcedfalse, zero residual processes, complete
uninstall. Exact original app/DLL hashes and new OXT input hashes are separate
receipts in D:/dev/grammar-evidence/grammar-writer-uniform-format-20261005.
Final R11:32 checks/16 cycles PASS, including inherited-style-change refusal,
explicit Save after Redo and exact text/format after reopening that saved file.
Natural Office/Python0, cleanuptrue, forcedfalse/processes0; every guest stopped.
F06 and Source generator now express this bounded union while preserving old
unsaved-only and saved-plain snapshots. Same-integrator review, no independent
reviewer. Mission INCOMPLETE, main unchanged, no push or live Provider.

Next product work: mixed-format range preservation and paragraph-aware Undo.
Keep the uniform capability bounded until independent ranges/styles and structural
changes have actual installed qualification. Existing app/native0fc8624 remains
receipt-bound; no full Rust rebuild is claimed in this Python-only expansion.
Source behavior77, package4, bridge6 and governance/self-test PASS. Source scope
changes preserve prior saved-plain and unsaved-only snapshots as history.

## Mixed surroundings Writer checkpoint — 2026-10-06

Product commit a3b231a; synthetic preparation correction4f6bbb5. Same candidate
branch; main, credentials, profiles and remote unchanged. Explicit default-off
`--enable-mixed-surroundings` requires `--enable-uniform-format`; add
`--enable-saved-docx` for local DOCX. Paired app/native remains0fc8624.

Supports one Standard body paragraph with a homogeneous selected range surrounded
by other supported bold/RGB runs. Entire text/layout fingerprints include effective
style defaults, paragraph alignment, actual run lengths/values/states and strict
RGB metadata. Exact expected postwrite layout is checked; custom range Undo/Redo
checks whole document hashes and never restores the whole body. Cross-format
selection and selected/replacement line breaks are refused before writing.

Native final-r2:39 checks (including16 saved/reopened Apply/Undo cycles) PASS;
inside/full run/prefix/suffix, Undo/Redo, stale outside format, custom Undo refusal,
multiple paragraphs and selected line break refusal, explicit Save/reopen fidelity.
Installed product-r5:8 widget +5 enclosing checks PASS; real hotkey/Instant/edited
Accept/exact text and per-character formats/Undo, changed source and Disable. No
automatic Save. Disable may report outcome unconfirmed; reread independently
proves no mutation, and there is no replay. App/Office/Python natural exits0,
cleanuptrue/forcedfalse/processes0 and complete uninstall.

Product-r1..r4 FAIL preserved. Setup recreated a saved synthetic paragraph but
left inherited DOCX original-color tags inconsistent with its new RGB. Read-only
exact packaged core diagnosis identified unsupported_color_interop; resetting to
DEFAULT and even all properties did not remove inherited tags. Only the synthetic
setup now writes a typed explicit empty interop sequence. Product admission was
not relaxed; inconsistent imported color metadata stays unsupported. A final
fixture startup from the output directory was refused by MediumLaunch (NOT_RUN);
its guard stayed intact and final-r2 used the approved payload path.

OXT SHA256 971ebfddb77523c6ef021d7e2f98cca68430e77a8f2c516a1ccceb84308b6389.
Exact checkout input hashes and Git clean-filter blob binding are in
D:/dev/grammar-evidence/grammar-writer-mixed-20261006/source-input-proof.json.
Raw Git LF bytes and Windows checkout CRLF bytes differ; do not compare their
SHA256 as though they were the same representation. Existing binary hashes are
separately verified, not falsely attributed to the current Python-only commit.

Same-integrator review only. Layout oracle2/package5/bridge6 tests PASS; final
plain73/32 cycles and uniform32/16 cycles native regressions PASS with natural cleanup; Source78 and governance/self-test PASS. All guests stopped; receipts in RESULT.md.
Cross-format selections, multiple formatted paragraphs, arbitrary rich structures,
actual Office/Hancom, physical IME, normal profiles and live Provider remain
unqualified/NOT_RUN. Next authorized work is paragraph-aware preservation and
explicit cross-format replacement semantics. No main adoption/push/release;
mission INCOMPLETE. This supersedes the prior checkpoint's next-work summary.


## Writer paragraph-preserving checkpoint — 2026-10-06

Implementation9443ff29018a2fa96a7b71c0f0a7abec6de7eaf6; candidate branch remains
codex/grammar-local-trial-ba58b7d. Evidence directory:
D:/dev/grammar-evidence/grammar-writer-paragraphs-20261006.
This supersedes the mixed checkpoint only for the newly qualified paragraph mode.
Prior mode acceptance and historical failures remain intact.

New user capability: a new/local saved DOCX Standard body may have multiple
paragraphs and empty paragraphs. Select one homogeneous range within ONE paragraph,
explicitly Enable, shortcut, Instant, edit and Accept. Other paragraph text,
supported character weights/RGB, paragraph alignment and empty typing formats are
preserved. Cross-paragraph/cross-format selection, unsupported structures and
inconsistent imported color metadata still refuse. No automatic file Save.

Core reads actual separators (Windows CRLF has two string code points, unlike
native cursor steps); delimiters never merge. Layout transformations retain
zero-length empty typing runs and shift later paragraph offsets. Exact text and
layout digests bind Accept and custom Undo/Redo. No whole-body rollback or replay.
Options default off; product --enable-paragraphs requires mixed/uniform options.

Evidence: guest-final core41 PASS including8 saved DOCX cycles, mixed styles inside
middle paragraph and changed empty typing style blocking old Undo. Installed
product guest-product-r1 widget8 + enclosing5 PASS: actual shortcut, Instant,
edited Accept, full paragraph text/format rereads, Undo, source edit preservation,
Disable and no implicit Save. Natural app/Office/Python0, cleanuptrue, forcedfalse,
remainingProcesses0, uninstall complete. Disable still displays outcome unconfirmed;
independent reread proves no mutation, not a new claim about that UI status.

Preserved failures: R1/R2 refused mixed_range_bounds before writing because a
width-one separator assumption disagreed with actual CRLF. Deterministic tests
also exposed merging adjacent delimiters. R3/R4 failed the saved-reopen format
snapshot; R4 subsequently wrote then returned UNKNOWN_CHANGED/postwrite_layout_mismatch.
Detailed per-character comparison refuted actual saved text/format loss: imported
DOCX added redundant identical zero-length portions on nonempty paragraphs.
Canonicalization now ignores only those redundant portions; the independent
oracle checks real empty paragraph typing metadata. R5 PASS37, final PASS41.
No thresholds increased and no failure receipt replaced.

Product OXT sha256 eda1d4c13225d0436895a90304caca3e84826fd7c46d1231b5d17c00fda55a9d.
source-input-proof.json binds checkout bytes and Git clean-filter blobs to9443ff2.
Paired unchanged app/native commit0fc862498206f536adf5156fa7d238792c89efc5:
app45c3d0cf67b5ee005d2207f899c2f6bdf087ef2ad700fd2c365e7a693edeeb76;
DLL61c50bb11ae3243efe29b2cadd74bfbc3c4e944d8dfdb2e4767167002e471376.
Pinned LibreOffice26.2.4.2. Native fixture contains test services; product OXT does
not. RESULT.md and QUALIFIED_COMBINATION.json contain final checks and receipts.

Review: separate same-integrator pass (REVIEW.md), no independent reviewer claimed.
Deterministic layout4, package6, bridge6 and Source behavior79 PASS. Full Rust and
Chromium suites not rerun; their production sources/binaries were unchanged.
Source/state describe this limited union; no external Project UI application.

Next inside GRAMMAR-GLOBAL-ACCEPT-DESKTOP-BROWSER-AND-APPLICATION-ADAPTERS:
define and test cross-format/cross-paragraph replacement semantics before enabling
those ranges, then continue application-specific adapters. Complex Writer,
Calc/Impress product Apply, actual Office/Hancom, physical IME, normal profiles and
live Providers remain incomplete/NOT_RUN. Mission INCOMPLETE, candidate only,
main not adopted, no remote push/release.

Final same-core regressions: mixed39/16 saved cycles, uniform32/16 and plain73/32 PASS; each native launcher confirms natural Office/Python0, cleanuptrue, forcedfalse, processes0. All owned guests stopped (sandbox-final.json). Main remains4e833988ed27086dd6d4acb78d39fc3e9c98c707.


## Cross-format Writer selection — 2026-10-06

Implementation600aa72, base07ee35d; evidence D:/dev/grammar-evidence/grammar-writer-cross-format-20261006.
New behavior: explicitly enabled package option permits different supported
bold/RGB styles inside ONE selected Standard paragraph range. Replacement uses
first selected style; Undo restores the original run styles. The panel discloses
this before Enable, observed by the installed test. Existing modes default to
homogeneous selection; all options remain defaultoff. Cross-paragraph selections
and break-containing replacement still refuse. No automatic Save or whole-body
rollback. Same per-document opt-in, identity/revision/source and sensitive guards.

Native R1/R2: first cross-run replacement succeeded, Undo then failed after
mutation with unsupported_theme_or_transparency. Diagnostic reread showed the
restored second run became CharColor=-1/CharTransparence=100. A portion's
DEFAULT_VALUE inherited a run-local red, not the document default; resetting it
lost that value. Restore now sets measured effective values. No color guard was
relaxed. R1/R2 failures and natural cleanup are preserved. Initial host cp949 test
preparation failed before guest connect; explicit UTF8 corrected it.

Final R3:47 checks including8 saved/reopened DOCX Apply/Undo/Redo cycles PASS.
Two and three style runs, Korean/emoji, exact whole-body text/format snapshots,
stale outside-format and empty typing style Undo counterexamples retained.
Installed product-r1:8 widget +5 enclosing PASS, actual shortcut/Instant/edited
Accept, complete text/format rereads and Undo, changed-source/Disable preservation,
no automatic Save. Natural app/Office/Python0; complete uninstall/processes0.
Disable's UI still reports outcome unconfirmed; separate reread proves unchanged.

Product OXTsha256:e0898151d44182cc3e63c41510336fbfaf11595677dd2da53e3461bac1826434.
source-input-proof.json binds all packaged source bytes/Git blobs to600aa72.
Unchanged runtime0fc862498206f536adf5156fa7d238792c89efc5 is separately hash-bound
by installed receipts; native fixture services are absent from product OXT.
Deterministic layout6, packaging7, bridge6 and Source behavior80 PASS.
Separate same-integrator review only (REVIEW.md), not independent peer review.
Full Rust/Chromium and physical IME/live Provider not rerun; those binaries unchanged.
Final prior-mode regressions/cleanup are in RESULT.md; no claims based on mere launch.

Continue paragraph-spanning replacement with defined paragraph-style/Undo semantics
before activation. General Writer, Calc/Impress product Apply, actual Office/Hancom
remain incomplete. Candidate only, mission INCOMPLETE, no main adoption/push/release.

Final prior-mode native regressions: mixed39/16 saved cycles and uniform32/16 PASS on the same core. All final guests naturally exit Office/Python0 with cleanuptrue, forcedfalse/processes0. Sandbox inventory empty. Plain mode was not rerun this round: changed formatting helpers are not reached in plain mode; its prior73/32 remains historical evidence.


## Paragraph-topology preserving Writer transaction — 2026-10-06

Base9a7db79. Initial span implementation6b56f99; installed-focus compatibility
repair6ba7c9f. Evidence D:/dev/grammar-evidence/grammar-writer-span-20261006.
Final revision-guard artifact and exact verification are in RESULT.md.

New contract: an explicit defaultoff span option permits selections crossing
existing Standard paragraphs, limit8; the owned fixture has4 paragraphs including
one empty and distinct alignments/colors. Draft keeps paragraph count and empty
parts. Right-to-left nonempty-part replacements leave every paragraph delimiter
untouched, each using its first selected character style. One native Undo context
groups range actions; Undo restores original selected runs and Redo reapplies.
Exact full-body/layout verification remains at each part. No automatic Save.
Merge/split and insertion/deletion of empty parts remain refused before writing.

Each part checks exact enrolled document/controller/selection/source/layout,
permission, original deadline and intervening revisions. Native capture requires
Writer foreground; explicit Accept can occur while the review widget owns it,
matching the existing single-range contract. Partial failure is UNKNOWN_CHANGED,
stops later parts, keeps dirty state, preserves user edits, and never rolls back
the whole body or automatically replays. No user documents, live Provider or normal
profile used. UI policy is visible before Enable; dialog space increased for it.

Preserved failures: R1-R3 native saved-span writes returned UNKNOWN_CHANGED after
a postwrite layout mismatch. Actual effective colors/alignment/text matched for
completed parts, but a whole-paragraph replacement relocated DIRECT run color to
an equal DIRECT paragraph property, leaving the portion DEFAULT. Canonicalization
only recognizes that exact direct parent/equal value; real style inheritance or
unequal/ambiguous properties remain distinct. R4 and first final native run refused
capture with document_not_foreground; no product gate relaxed. The native fixture
now focuses Writer after Enable as the installed flow does; failed calls are not
retried or reclassified. Product-r1 captured/Instant passed but Accept timed out:
span-only foreground precondition conflicted with widget review.6ba7c9f aligns
that boundary with single-range Accept. Product-r2 passed8 widget+5 enclosing checks.

Native R5 and final-r2 passed81; those precede the additional same-content revision
counterexample and are kept as historical receipts. Final records distinguish the
revision-guard run and actual paired installation, rather than borrowing counts.
Disabling still has outcome-unconfirmed UI; exact reread separately proves no
mutation. Same-integrator separate review only (REVIEW.md), no independent reviewer.
Full Rust/Chromium suites not rerun; their source/binaries unchanged. Physical IME,
general profile, live Providers, actual Office/Hancom remain NOT_RUN/unqualified.

Continue paragraph merge/split semantics and application-specific adapters within
GRAMMAR-GLOBAL-ACCEPT-DESKTOP-BROWSER-AND-APPLICATION-ADAPTERS. Mission INCOMPLETE;
no main adoption, push/release or general-profile installation. Source preview is
not external Project UI application. Historical failed evidence remains unchanged.

Final revision-guard code cde1097: native guest-revision PASS82, including unchanged-content revision event stopping later writes; installed guest-product-final PASS8 widget/5 enclosing. OXT9dc837a2c308f7d79e045cc0869ca31d4d3cc4e1caa21fd6e00fd768b8bbc709 pairs unchanged runtime0fc8624. Both natural exits0/cleanuptrue/forcedfalse/processes0, installed removal complete. Exact source binding: source-input-proof-qualified.json; fixture/product core bytes equal. Prior-mode final regression and shutdown recorded in RESULT.md.

Final prior uniform-format native regression PASS32/16 saved cycles on the same core; natural cleanup complete. Layout9/package8/bridge6/Source81 and governance/self-test PASS. All owned Sandbox instances stopped. Final exact12 preview is separate from external Project application and product acceptance.


## Uniform paragraph topology — 2026-10-07

Base deb15d8; implementation62d4a89, soft-linebreak containment86dfae4,
installed fixture fixesd9a6b1e/a426701. Dedicated candidate unchanged; main remains
4e833988ed27086dd6d4acb78d39fc3e9c98c707, no adoption/push/release.
Evidence D:/dev/grammar-evidence/grammar-writer-topology-20261007.

Explicit defaultoff uniform-topology option requires prior span flags. At most8
input/output Standard paragraphs, identical whole-paragraph character/RGB/alignment
metadata, permits reviewed merge/split and empty-part changes. Others retain old
span policy. Soft linebreaks inside affected paragraphs refused. One range write,
exact whole-document text/layout hashes, custom range Undo/Redo, no automatic Save.
Failed writes remain dirty/UNKNOWN_CHANGED; no rollback/replay. Undo after external
edit refuses without changing it. Native text representation kept in source/hash;
Writer setString gets CR paragraph delimiters for this path only, including Undo.

Native guest-qualified PASS47 (8 saved cycles), installed guest-product-r3 PASS10
widget/5 enclosing; source change/Disable, exact rereads/styles/Undo, natural exits0,
cleanuptrue/forcedfalse/processes0 and uninstall complete. OXT SHA256:
047f69398018141db11f076620790f0b04da19fef4042c860ebfb5c15d97f095.
Paired app/native0fc8624 unchanged, hashes in QUALIFIED_COMBINATION.json. Product and
fixture core bytes equal; source-input-proof-qualified.json binds Git blobs/bytes.

Preserved failures: guest-r1 foreground setup (no mutation tests); guest-r2 merge
passed but literal CRLF Undo inserted extra linefeeds; guest-product-final omitted
initial saved synthetic text from fixture oracle; guest-product-qualified timed out
before widget entry on document activation (exact original stage not known). The
fixture now activates its owned document and labels timeout call sites, without
changing product foreground checks or deadlines. R3/guest-final native44/46 are
intermediate evidence, not final. Soft-break ambiguity discovered in separate
same-integrator review; no independent peer review. Layout12/package9/bridge6 PASS.
Prior span/native regression and Source validation final results in RESULT.md.

Continue differing-format topology semantics and application-specific adapters.
Complex Writer, Calc/Impress product Apply, actual Office/Hancom, physical IME,
normal profiles/live Provider remain incomplete/NOT_RUN. Do not claim general
Writer or whole-product completion. Exact next task unchanged. Local Source preview
is not Project UI application; no new external Source upload prerequisite.

Final prior-span regression guest-span-regression PASS82 on the same core; including
8 saved single-range and8 saved span cycles. Natural cleanup0/forcedfalse, all owned
guests stopped. Total native129, installed15. Source behavior82 and governance PASS.
Unchanged Rust/Chromium and plain/uniform native suites not rerun this round; prior
scope evidence remains historical. No new source UI application claimed.


## rHWP API investigation — 2026-10-07

rHWP0.8.7 pinned1a76570: local sandboxed Chromium API fixture16 PASS for synthetic HWP/HWPX exact apply/revert/export/reopen and stale/hash refusals. Both moved-selection probes accepted old targets: integration gap, not Grammar acceptance. No adapter activation. Readonly dev hooks only; autosave drafts0, metadata recent4, task profile removed. Render rollback/concurrency, selected plaintext capture, target-side authority and privacy lifecycle pending. Evidence grammar-rhwp-20261007/run-final/receipt.json; mission INCOMPLETE, no main/push.

See adapters/rhwp/README.md for pinned provenance, reproduction, preserved failures
and limits. First paragraph control refusal was a fixture assumption, not relaxed
policy. Selection movement is not guarded by upstream explicit-target API. Next:
implement target-side selection/revocation fencing and review render-failure
rollback against concurrent edits before any Grammar integration. Existing F-06
qualified scopes unchanged. Same-integrator review only; no independent review.

## rHWP target-side binding and render failure — 2026-10-07

Base a9b5997. rHWP target-side gate continuation: single-use30s reviewed binding checks current editable selection and exact document at capture/Accept and synchronous mutation callback; no product activation. Pinned upstream input/history overlay suppresses Grammar async-render rollback, removes only failed command snapshot, marks dirty and quarantines failure. gate-r2 reproduces lost intervening application edits in both formats; retained as safety FAIL despite API aggregate label. Final evidence grammar-rhwp-20261007/gate-qualified/receipt.json. Installed routing, selected-only capture, normal Undo and synchronous fault qualification remain pending. Mission INCOMPLETE; no main/push.

Component and overlay in adapters/rhwp. Raw upstream selection probe intentionally
continues to accept old targets; guarded cases must refuse with unchanged full text.
Render-failure injection uses real WasmBridge external mutation/event, not human
input. No whole-product/installed or Hancom qualification. Same-integrator review;
no independent reviewer. Final measured results and cleanup in external GATE_RESULT.md.

Final gate-qualified32 PASS; both render-failure application edits preserved, exact
prior history identities retained and failed entry absent. gate-final count oracle
FAIL preserved and corrected to actual identities. Profile/browser/server removed;
port17707 closed. Governance/Source behavior/JS syntax/diff PASS. No activation.

## rHWP history fence — 2026-10-07

Base d33a64c. rHWP Grammar history fence: each successful Grammar snapshot has before/after document SHA256; Undo/Redo refuse changed images before mutation/stack movement and retain the blocked entry. history-red reproduces external edit loss on Undo and Redo; history-green preserved expected-refusal fixture failure. history-final38 PASS incl normal4 roundtrips per HWP/HWPX and both changed-source history refusals, exact text and entry identity. API component only, no installed activation. Source pin unchanged; overlay and fixture hashes in grammar-rhwp-20261007/HISTORY_RESULT.md. Synchronous fault/reentry, rich formatting, selected-only capture and actual widget routing remain pending; mission INCOMPLETE, no main/push.

Postimage fingerprint failure now releases unrecorded snapshot without rollback;
that fault path was reviewed but not injected in this round. Same-integrator review
only. Continue selected-only capture/visible review integration and synchronous
fault qualification; existing product activation scopes remain unchanged.

## rHWP selected-text review component — 2026-10-07

Base7471b19. rHWP selected-text review component: explicit Enable/Capture, editable browser draft and Accept/Disable; trusted host eligibility required before source/export. Whole plain paragraph only; UI payload id/source only, internal hashes remain target-side. Edited Accept preserves original document/selection binding and30s expiry. review-final20 UI assertions PASS across HWP/HWPX including disabled/ineligible zero reads, partial-selection refusal, stale ID/selection/source and revoked admission. No Instant/Deep or installed desktop connection; production admission/deployment/lifecycle still unqualified. Prior gate/history regression results in grammar-rhwp-20261007/REVIEW_RESULT.md. Mission INCOMPLETE, no activation/main/push.

Review uses original source as editable draft; not yet Instant. Fixture alone supplies
real editor dependencies through dev hooks and synthetic admission. Before installed
activation, connect trusted in-editor bootstrap, real sensitive-document admission,
close/navigation cleanup and existing local Instant/widget transport. Keep payload
selected-only. Internal full-document export hashes are not sent to review UI.
Separate same-integrator review only; no independent peer. See external result for
exact final regression and cleanup; current F06 qualification unchanged.

Final UI20 and gate/history38 PASS, Source/governance/JS syntax/diff PASS.
Owned profiles/browser contexts removed, server stopped and port17707 closed.

## rHWP local Instant — 2026-10-07

Base2d9d972. rHWP local Instant: optional selected-text analyzer reuses Chromium UTF16 cache/review assembly and real Grammar native host analyze engine. instant-r1 HWP/HWPX10 PASS: draft correction then explicit Accept, delayed response preserves user draft/Disable/selection boundaries. One in-flight request, no silent retry/fallback; abort prevents publication, not a claim of physical transport cancellation.6 bounded orchestration unit tests PASS. Owned offline locked debug host proof and final regression in grammar-rhwp-20261007/INSTANT_RESULT.md. Test-only Playwright bridge, no installed activation/desktop hotkey/production transport; mission INCOMPLETE, no main/push/Provider.

Next: trusted in-editor bootstrap/admission/lifecycle and bounded production local
transport, then installed shortcut/widget/package qualification. Do not promote the
test-only exposeFunction bridge to production. No Deep or live account activity.
Same-integrator review only; native host library unused warnings retained.

Final Instant10/UI20/gate-history38/unit6 PASS.8 real local host exits0, final
profiles removed and server/port closed. Source/governance/syntax/diff PASS.


## rHWP trusted bootstrap — 2026-10-07

Base3ff6707. rHWP trusted bootstrap: pinned Studio main imports lexical review initialization, default-deny admission; synthetic build only permits normal HWP/HWPX. bootstrap-r2 8 and bootstrap-denied-r1 4 checks PASS: explicit edited Accept exact reread, document generation clears draft/opt-in, pagehide disposes panel, default denies before target text read. Dev input/event/render globals removed; __wasm retained solely as reread oracle. Initial bootstrap-r1 import-resolution FAIL plus EBUSY cleanup preserved; later cleanup request policy-rejected, residual owned profile remains. No installed activation, real sensitive-document policy or production Instant transport. Visibility path implemented but not separately qualified. Evidence grammar-rhwp-20261007/BOOTSTRAP_RESULT.md; mission INCOMPLETE, no main/push/Provider.

Next: trusted document-sensitive admission and bounded production selected-text
local transport, then installed package/widget routing qualification. The test
server synthetic admission option is not a deployable permission decision.
Bootstrap source-only review and earlier Instant bridge evidence remain separate.
Same-integrator review only; no independent peer. Initial failed-run residual
profile contains synthetic task data; cleanup was not completed or relabeled PASS.

Final bootstrap8/default-denial4/UI20/unit6 PASS; Source/governance/syntax/diff
PASS. Passing profiles removed, owned server stopped and port closed. Failed-r1
residual explicitly retained as above. Source/receipt hashes: bootstrap-proof.json.


## rHWP document admission guard — 2026-10-07

Base099543b. rHWP admission refusal guard: trusted loaded/generation/protection/format metadata and separate host policy checked before capture/Accept. Observed protection, unknown metadata or revoked host policy latch denial for that generation; polling clears draft/session, close terminal. admission-red uses exact099543b bootstrap and fails at22 target reads after synthetic protection flag plus immediate Accept. admission-r1 10 PASS; no password contents/encrypted-file qualification. Default host policy remains deny; mutable save-intent flag is not full sensitive/protected provenance. Production transport and installed routing remain pending; no main/push/Provider, mission INCOMPLETE. Evidence grammar-rhwp-20261007/ADMISSION_RESULT.md.

Necessary refusal checks only: a trusted default-deny host must still establish
real document provenance and sensitive-document eligibility before deployment.
Polls cannot certify unobserved transient flag changes. Same-integrator review
only. Prior bootstrap-r1 residual profile remains unresolved; this round does not
retry its policy-rejected cleanup. No installed HWP/HWPX acceptance expansion.

Final admission-final10/default-denial4/admission-unit9/Instant-unit6 PASS.
Source/governance/syntax/diff PASS. New red/green profiles removed; owned server
stopped, port17707 closed. Exact source/receipt hashes in admission-proof.json.


## Owner-requested gpt-6-luna live quality trial — 2026-10-07

Owner explicitly authorizes migrating5.6-luna to gpt-6-luna and actual synthetic
quality/speed testing across implemented features. This supersedes the no-live
boundary for this trial only; no credential copying/login by agent or user data.
Default/fast translation thread/turn model changed; Codex tests39 PASS/6 ignored.
Owned Sandbox20980ce1-ef75-4528-9fd9-4d014e68c50e preflight reaches product client,
reports LOGIN_REQUIRED_NOT_RUN and shutdownComplete, zero inference calls. Owner
must log in inside guest via C:\GrammarQualityPayload\Owner-Login.cmd. Agent can
then run guest-quality-trial.ps1 -Live via wsb ExistingLogin; no further approval
needed for authorized synthetic trial. Guest remains active. Initial10-case runner
covers shared client only; additional terminology/languages/cancel/UI/editor rounds
remain. No quality/latency verdict until real outputs reviewed. Initial missing-DLL
failure retained; app-local runtime resolved it. Evidence and payload hashes:
D:/dev/grammar-evidence/grammar-6luna-trial-20261007/STATUS.md. No main/push/release.


## gpt-6-luna actual Sandbox trial checkpoint — 2026-10-07

Owner completed guest login; agent stopped guest Magnify with explicit permission.
No credentials read/copied, account changes, host user data, main adoption or push.
Old pinned CLI0.144.6 rejected gpt-6-luna (codex_model_capability_unavailable);
its complete catalog omits the model. Bundled CLI0.162.0-alpha.2 catalog includes
it and exact-version test-only Rust transport completed actual synthetic calls.
Production CLI pin stays0.144.6 pending compatibility/privacy/installed qualification;
therefore the model-default edit does NOT yet make the normal installed product usable
with6luna. No automatic fallback to5.6. New CLI acceptance is not claimed.

36 corpus outputs across baseline10, extended10, context-low3, context-full3,
final10; plus one interrupted turn and one successful recovery (38 new-CLI turn
attempts). Two old-CLI rejected attempts are separately retained. Requested model
is6luna; server-reported model UNKNOWN. All raw source/output is synthetic.
Manual same-integrator review: baseline9/10 and final9/10 usable for these samples;
three-language/auto-direction/terminology7 cases preserve intended facts/terms,
with literal Chinese approval phrasing. Targeted reported-speech case retains
meaning/gender problems: prompt strengthening did not reliably fix it; low3 has
0/3 fully usable, full-medium3 has1/3. Final fast case preserves event scope but
again invents female gender. Do not claim quality PASS, general accuracy or repair.
Full comparison changes prompt/schema/effort together, not an isolated effort test.
Production effort remains unchanged. User must review before explicit Accept.

Client end-to-end includes prepare/start/complete/validation, excludes initial
connection and visible-widget latency. Final10 min3238/median4186/max8519ms;
full-medium counterexample max32058ms. Small targeted sample, no reliable p95.
Cancellation acknowledgement1ms, outcome INTERRUPTED, same-client recovery2907ms;
not proof of remote physical cancellation time. Final10 shutdown succeeds and
owned runtime directory is actually absent. Guest remains active/authenticated;
whole-Sandbox cleanup not claimed. Earlier intermediate binaries were overwritten;
only final10 has final source-byte/test-binary proof, not retroactive provenance.

Validation: library91 PASS/10 ignored; application239 PASS/30 ignored; governance/self-test/diff PASS. Original failures (including standalone PS protocol diagnostic failure
on both old/new CLI) retained, not counted as provider success. Ordinary test runs
cannot launch ignored live runner. Failure receipts now produce nonzero exit;
safety-start failure also preserves a receipt. Same-integrator review only.
Evidence: D:/dev/grammar-evidence/grammar-6luna-trial-20261007/QUALITY_REVIEW.json,
FINAL_TRIAL_PROVENANCE.json and live-new-*.json. Earlier prelogin STATUS/PREPARATION
are historical. Next: exact newer CLI hardening/protocol/privacy qualification,
then installed widget/Chromium/Writer live routes, stale/cancel/draft protection and
visible latency. HWP/rHWP unimplemented installed routing remains unqualified.
Mission INCOMPLETE; Source materiality changed, no new package/UI application yet.


## Gemma priority and actual local GPU trial — 2026-10-07

Official Google Gemma4 12B IT QAT Q4_0 revision29d097773436b69ff9feafd636ab4cf873786537, modelSHA93567e57a8fe10b23569b9d9ec38cd005deedf71e29477c421a4b83f418a538b. Pinned llama.cpp b11461 CUDA12.4 portable binaries; all download sizes/SHA256 verified against upstream metadata. No modelmmproj loaded, text-only. No installer/driver/security changes. Host RTX5070Ti16GB, synthetic corpus only; not Sandbox or installed product acceptance.

R1: load38484ms, first request26046ms, following9 requests313-515ms.10 complete outputs. Manual review9 usable, grammar-en04 incorrectly neutralized explicitly stated She. First-request latency remains a recorded miss; internal JIT cause is a hypothesis, not established.
R2: clarified preserve explicit gender/pronouns. Load6578ms;10 requests313-547ms, median359.5ms,10/10 complete. Explicit She preserved, but context translation now invents she for an unspecified Korean name;9 usable/1 needs edit under same-integrator review. No general quality parity or quality improvement claim. Neutralization and invented-gender counterexamples both retained. No silent fallback or retry.

Latency is full local HTTP replacement completion only, excluding document detection, debounce, source qualification, UI publication and Apply. These results support further integration, NOT recommendation<=1s or Apply<=0.5s acceptance. Exact inference setup differs from prior CLI: replacement-only output, deterministictemperature0, bounded prompt/no JSON explanations; no model-only speedup ratio claimed. Small10-case samples cannot establish p95 reliability. No hidden first-run exclusion: both complete receipts retained.

Server loopback only, ephemeral bearer401 test passed, offline flag, webUI disabled, no remote inference. Both owned server processes exited and ports closed; model files retained intentionally. No credential reads/copies, user documents, main/push/release. Existing authenticated Sandbox left untouched.

Changed repository direction: Contract1.5.2/pins, charter/decisions/roadmap/acceptance/ledger/state; explicit user-requested persistent memory note in memories/extensions/ad_hoc/notes. Uploaded ChatGPT Project Source NOT replaced. Source materiality changed. Governance+selftests PASS, Source package tests82 PASS, Python compilation and diff checks PASS. Source checks are not product tests. Actual GPU runner2 rounds completed; no independent reviewer.

Next: production bounded local-engine IPC/lifecycle and independent local/Deep scheduling; connect revision-bound recommendation cache and measure input-to-popup plus Accept-to-reread per adapter. Retain quality defect and evaluate wider held-out cases before activation; keep cloud providers optional. No installed Gemma activation yet. Mission INCOMPLETE.

Evidence: D:/dev/grammar-evidence/grammar-gemma-20261007/RESULT.md.


## Local/Deep independent lanes — 2026-10-07

# Local/Deep lane separation

Base397c3a7. Two production-worker regressions first failed: local request could not obtain a port while Deep was running or reserving its cleanup marker. Change gives each admitted document one separately bounded local port/pending request while retaining existing Deep port/ticket ownership. Maximum4 admitted documents unchanged; each can now own at most one local and one Deep data channel (control/desktop channels unchanged). No Gemma production invocation or new model activation yet.

Disable/navigation resolve and discard local pending requests, invalidate the port before disconnect, and retain Deep cancellation/cleanup acknowledgement. Local failure cannot erase a Deep marker. Local continues when a historical incomplete Deep marker blocks external admission. Duplicate local requests remain busy, with existing timeout. UI busy text corrected to local analysis. This does not claim queue wait or full recommendation within1s.

Unchanged red tests pass;77 combined core/worker tests pass, including4 new independent-lane tests. JS syntax/governance/diff checks pass. Same-integrator review only. Browser trial details follow in result.json; initial missing evidence-parent failure preserved as browser.log. No Provider calls or user profile/documents; no main/push.

Next: bounded Gemma local runtime transport/lifecycle and revision-bound cache integration, then input-to-popup<=1000ms and Accept-to-reread<=500ms checks. Current local analyze still uses the existing Instant rules; no inference-only timings promoted to UI acceptance.

Browser modified isolated extension with real existing host: 67 checks PASS, runner exit0 after context/server/registration cleanup. Synthetic profile/evidence retained; not unmodified installed package or live Gemma acceptance.

Evidence: D:/dev/grammar-evidence/grammar-local-lanes-20261007/RESULT.md.


## Gemma Rust runtime candidate — 2026-10-07

Base ad3271b. Added library-only local_gemma engine, not wired to installed app/native host. Existing suspended Windows Job ownership and runtime workspace cleanup reused. Absolute local artifacts, loopback bearer, no proxy/redirect, offline/no-webui, null output streams, bounded input/output, single request per engine and exact epoch/revision/source binding. Request deadline includes HTTP parsing; errors/cancellation stop the actual Job and reap/verify zero processes plus workspace removal within a separate fixed5s cleanup budget. This is not a1s cancellation guarantee. Dropped request futures block reuse until explicit stop; Drop is not cleanup acceptance.

Actual pinned Gemma12B QAT Q4_0 and llama b11461 on owned host GPU: first Rust run ready5875ms; three requests146/239/320ms, but all introduced wrapping quotes. Preserved as quality failure. Replaced free text with constrained JSON replacement and strict serde schema; no stripping of legitimate source punctuation. Official response-format reference: https://github.com/ggml-org/llama.cpp/blob/master/examples/json_schema_pydantic_example.py . Second run ready5616ms including warmup; requests338/425/453ms, reviewed exact corrections for synthetic English agreement, negation/uncertainty and Korean spelling. Cancellation10ms trigger -> actual cleanup955ms, Job0, workspace removed, readyfalse. Initial run cleanup965ms also retained. No broad quality parity claim; prior gender defects remain open. Model SHA independently rechecked matches93567e57...18a538b.

Validation: Rust library94 passed/11 ignored; explicitly invoked GPU lifecycle test separately PASS. Production JS cache with these three real outputs21 assertions PASS (exact replacement construction, source/epoch/revision refusal, disable); this is receipt replay, not live IPC or DOM mutation/installed UI. reqwest=0.13.4 already locked transitively; added direct dependency without new package versions, default features off; MIT/Apache2.0, Rust minimum1.85. Existing library warnings remain. Initial private-module compile failure and edit-script encoding failure preserved in turn/logs; repaired, no tests removed.

Same-integrator review only. Remaining blockers before activation: trusted artifact verification at production admission, shared resident ownership across multiple native-host processes, loopback port ownership race/peer qualification, cancellation/deadline/output-flood/drop fault matrix, cache routing and actual input-to-popup/Accept-to-reread. Multiple processes must not each spawn a full model. No production monitoring/activation enabled. Translation/paraphrase wider quality corpus and end-to-end1s/0.5s acceptance remain pending; no Apply changes. Existing authenticated Sandbox untouched. No main/push/release/Project UI replacement. Mission INCOMPLETE.

Evidence: D:/dev/grammar-evidence/grammar-gemma-20261007/rust-lifecycle.json (quality failure), rust-lifecycle-r2.json, rust-all-r2.log, cache-result.json. Next implement shared engine ownership and qualify fault paths before installed cache integration.


## Gemma shared owner candidate — 2026-10-07

Base1ca5e1d. Added in-process Service/Client: exactly one supplied resident engine per service, one executing request plus at most4 queued requests. Clients cannot spawn or silently restart models. Queue admission is nonwaiting/busy; the original absolute request deadline includes queue time. Cloned clients share the owner task. Dropping/timing out the waiting client sets cancellation; the owner retains and awaits the production request so actual Job/process/workspace cleanup completes. Explicit close returns cleanup acknowledgement; dropping Service signals shutdown but is NOT itself cleanup completion evidence. Shutdown and elapsed-deadline results are refused before delivery. No replay/fallback.

This is not cross-process discovery/IPC or installed activation. A trusted launcher must still verify artifacts and own the sole service; separate callers could currently construct separate services. Do not wire one model into each native host. Cross-process single ownership, peer identity and admission remain required. Fatal request cancellation currently terminates the engine; automatic restart is deliberately absent. Need evaluate nonfatal server cancellation with real completion/slot evidence before preserving a warm model across superseded requests.

Verification: focused channel test covers full-queue refusal and waiter-drop cancellation. Production HTTP reader loopback tests cover32769-byte rejection and a stalled body under an absolute deadline. Initial fake HTTP server replied before reading the request, causing UnexpectedMessage; preserved service-tests-r2.log, repaired fixture handshake without weakening assertions. Full Rust library96 PASS/12 ignored in service-all-final.log; explicit real GPU fault trial recorded separately. Three real engine rounds test dropped waiter, deadline, shutdown; close acknowledges Job0 and workspace removal, independent runtime-path and closed-port checks pass. R1 total abort+cleanup1102/1061/1082ms; R2 final receipts are authoritative for the revised delivery guards. These are cleanup times, not successful recommendation/Apply latency.

Review: same integrator only. Previous structured-output3-case grammar result unchanged; no new translation/paraphrase quality or installed popup/Apply acceptance. Direct future-drop outside Service remains caller-owned; use Service for product integration. Existing84 library warnings not changed. Existing authenticated Sandbox untouched; only owned host GPU/synthetic loopback processes. No main/push/release/Project Source upload. Mission INCOMPLETE.

Next: cross-process single-service admission and shared IPC, preserve bounded cancellation while avoiding repeated cold starts, then revision-bound adapter routing and full input-to-popup/Accept-to-reread measurements. Wider semantic quality and original gender counterexamples remain open.

Evidence: D:/dev/grammar-evidence/grammar-gemma-20261007/service-faults-r1.json, service-faults-r2.json, service-live-r1.log, service-live-r2.log, service-all-final.log.


## Gemma process ownership and deferred CUDA tuning — 2026-10-07

Owner asks to defer optional CUDA optimizations if they distract from the existing recommendation1s/Apply0.5s goal. Additional Flash Attention/backend-sampling/architecture-build A-B profiles are now follow-up work, not a prerequisite. Existing CUDA backend remains in use; no option/driver/model quantization change this turn.

Base cdc6e23. Gemma start now obtains a Windows exclusive-share file lease before creating a workspace or spawning the model. Concurrent owners using the same trusted temp-root namespace are rejected; no waiting/fallback. Lease is released only after explicit stop proves Job0, child reap and workspace cleanup. A single absolute5s cleanup deadline is retained across repeated stop and destructor paths instead of restarting that budget. Unexpected destructor tries bounded termination/cleanup; failure deliberately retains one lease handle until the owner process exits, preventing repeated admission in that namespace. Destructor itself is not an acknowledged shutdown API.

Scope: this is process exclusion, NOT a shared cross-process request broker, peer authentication or OS-wide hostile-process protection. Processes using different temp roots are separate namespaces; production launcher must pin the shared owner location. Forced OS termination release is tested for the lease holder only, not a complete model-child crash qualification. Fixed empty gemma-owner.lock remains under codex-pencil-runtime-v1 intentionally; it stores no text/token/endpoint/PID. Test-only UUID directories are removed nonrecursively after child exit. No documents or credentials used.

Verification: actual Windows test executable child rejects the held lease, acquires after release, exits without Rust destructors and allows subsequent acquisition; empty-marker size0 verified. Rust library97 PASS/13 ignored. Explicit GPU service fault test additionally refuses duplicate Gemma start before loading and runs3 successive engine lifetimes. Cleanup receipts: dropped waiter1199ms, deadline1129ms, shutdown1123ms; every case workspace removed/port closed/explicit cleanup acknowledged. Shutdown caller observed gemma_deadline while owner cleanup continued; deadline return is not cleanup completion. No success-latency claim from these fault durations. No thresholds widened. Original evidence preserved.

Review by same integrator only. Installed activation, shared IPC, artifact admission and endpoint peer binding remain pending. Keeping the model warm after cancellation remains unresolved; current cancellation deliberately terminates it. CUDA tuning deferred, wider grammar/translation/paraphrase quality and full UI latency still pending. Authenticated Sandbox untouched. No main/push/release; mission INCOMPLETE.

Evidence: D:/dev/grammar-evidence/grammar-gemma-20261007/lease-all.log, lease-live-r1.log, service-faults-lease-r1.json. Next implement trusted shared broker/adapter routing and warm cancellation qualification.


## Gemma warm supersession candidate — 2026-10-07

Base04290b6. Added explicit recommend_supersedable engine/client path for committed-source replacement while permission remains active. On supersession it awaits the already-running bounded request within its ORIGINAL deadline, discards the completed result and verifies the single owned slot is idle before reuse. This is completion-and-discard, NOT physical inference cancellation. No new request/replay/model reload. cache_prompt=false on requests prevents intentional reuse of prior prompt tokens. Request data can remain in bounded runtime memory until overwritten/engine shutdown; no memory-zeroization or cache-erasure claim. Ordinary recommend cancellation, deadline/error, permission shutdown and Service.close retain actual process termination and verified cleanup. Shutdown interrupts even a supersedable operation and explicitly stops the owner. Direct caller must choose correct API; adapters not wired yet.

Pinned upstream b11461 source review: /slots action=erase is gated by slot_save_path, including erase. First GPU warm-r1 failed with gemma_erase_status and stopped engine; preserved. Do NOT enable disk prompt storage to unlock this API. Replaced erase with completion plus bounded authenticated GET /slots id0/is_processing=false confirmation. Reference https://raw.githubusercontent.com/ggml-org/llama.cpp/b11461/tools/server/server-context.cpp (post_slots and SLOT_ERASE) and tools/server/README.md. No CUDA tuning changes.

Real GPU warm-r2:3 same-PID cycles, discarded completion443/454/463ms; next correct request438/442/449ms (p50=442,p95/p99/max=449 using nearest-rank,n=3). All under1s inference deadline, no output from superseded request. Actual10ms deadline still terminates: cleanup993ms,Job0,workspace removed. Shared service warm-service-r1 aborts old supersedable waiter then gets next revision correction in802ms INCLUDING QUEUE+INFERENCE (two-request interval814ms); exact She has two books checked, close cleanup acknowledged. Single synthetic service sample, not UI recommendation/p95 acceptance. No startup timing improvement claimed.

Final actual hard paths service-faults-warm-final: dropped ordinary waiter1107ms,deadline1163ms,shutdown1293ms including cleanup; duplicate model refused, runtime removed,ports closed. Shutdown client deadline may return before cleanup acknowledgement, kept distinct. Full Rust97 PASS/15 ignored; ignored GPU tests explicitly invoked separately. No tests disabled to pass. Same-integrator review; installed IPC/peer binding, disable per-document routing, coalescing under multiple clients and wider quality remain pending. CUDA optimization deferred; no main/push/release, no Sandbox/account operations. Mission INCOMPLETE.

Evidence D:/dev/grammar-evidence/grammar-gemma-20261007/warm-r1.json,warm-r2.json,warm-service-r1.json,service-faults-warm-final.json,warm-all-final.log. Next connect trusted cross-process broker and revision-bound adapter requests, then measure complete input-to-popup and Accept-to-reread.


## Gemma named-pipe transport candidate — 2026-10-07

Baseef122ae. Added library-only Windows named-pipe endpoint for one authorized client/request, feeding the existing shared Gemma service. Fresh random local name, first-instance guard, remote clients refused, OS server PID checked before sending document bytes, OS client PID checked before server reads the frame. Trusted launcher supplies expected live child PID/document epoch and revocable permission; no client-declared PID trusted. Strict bounded32KiB JSON frames, revision/source binding, one absolute deadline and reply acknowledgement before server closes. Disconnect while inference is pending drops the client waiter, invoking existing owner cancellation; Endpoint return alone is NOT cleanup acknowledgement. Close remains owner responsibility. No arbitrary operations/file paths/Provider calls in protocol.

Actual owned two-process Windows+GPU trial pipe-child-r1: correct She has two books returned in362ms including IPC/inference; child exit0, serveOk, owner closeOk and runtime removed. This is synthetic process integration, not the Chromium native-host binary or installed UI. Final guard added client deadline check before successful delivery. Final Rust library100 PASS/17 ignored; separate explicitly-invoked GPU test PASS before that final deadline guard. Tests cover wrong server PID with zero document bytes, oversized prefix rejection, wrong client PID, wrong epoch, already-revoked permit. Negative permit tests use a closed service to prove rejection before service admission. Existing84 library warnings retained.

Dependencies: enabled net on already locked Tokio1.50 and Pipes on existing windows-sys0.59; Cargo.lock unchanged/no package upgrades. Same-integrator review only. Ticket/Permit must be supplied by trusted launcher retaining actual process handles (PID reuse/registration policy still requires integration). This module is NOT discovery, installer registration, OS-wide authorization, persistent broker process, or endpoint-count admission policy. It does not yet connect native host/Chromium cache, and it sends hard-cancellation requests only; supersession routing remains explicit future work. No normal profile/user docs/credentials/Sandbox changes/main/push/release. CUDA tuning remains deferred; mission INCOMPLETE.

Next: trusted broker launch and ticket distribution to qualified native hosts, per-document revoke and endpoint cap, then actual browser opt-in->Gemma cache->popup->Accept/reread tests with full latency. Avoid enabling transport via arbitrary environment/file configuration. Evidence D:/dev/grammar-evidence/grammar-gemma-20261007/pipe-child-r1.json,pipe-owner-r1.json,pipe-live-r1.log,pipe-all-final.log.


## Native host Gemma dispatch candidate — 2026-10-07

Base4a855a5. Converted native analyze actor branch to an awaitable local-service seam; default remains existing Instant. An explicitly supplied trusted pipe Ticket selects Gemma and validates existing request restrictions before pipe use. Full changed-window replacement maps to suggestion0..UTF16length, gemma:grammar, exact id/epoch/revision; unchanged result emits no suggestion. Failed Gemma does not retry/fall back to rules. EOF or an additional message while one local request is pending drops its transport wait; the separate owner still owns actual cleanup, not the native actor. External Deep state/lifecycle unchanged. Production ProviderService supplies no Ticket, so general installation remains unchanged; no environment/origin-config ticket bypass added.

Actual GPU host test calls the production run_actor/default local dispatch/analyze_gemma/real named pipe/shared service path. Test adapter only supplies the owned Ticket. R1 three English/Korean corrections397/475/517ms; final dispatch R2 405/486/534ms (n3 nearest-rank p50=486,p95/p99/max=534); exact negation, text, native id/epoch/revision and UTF16ranges asserted, owner close acknowledged. Actor and pipe server are in the same test process this round; previous two-process pipe trial is separate evidence, not combined installed/native-stdio/browser qualification. Receipts contain synthetic text only.

24 native host tests PASS/1 GPU ignored and explicitly run separately. Added EOF pending-future drop, Deep-field refusal before pipe and no silent rules fallback tests. R2 actual response replay through production Chromium DocumentSession cache21 PASS including explicit replacement, changed-source/stale revision/disable refusals; replay is NOT live DOM/UI acceptance. No decoder/render/Accept latency claim. Same-integrator review only. Trusted broker launch, process-handle-bound ticket distribution, repeated endpoint renewal and permission lifecycle still pending; no installed Gemma activation. CUDA tuning deferred, mission INCOMPLETE; no main/push/release/Sandbox or credential operations.

Evidence D:/dev/grammar-evidence/grammar-gemma-20261007/host-gemma-r2.json,host-gemma-r2-cleanup.json,host-gemma-cache-r2.json,native-local-tests-final2.log,native-gemma-live-r2.log. Next implement trusted launcher/host admission then actual opt-in->popup->Accept/reread.


## Pinned broker admission candidate — 2026-10-07

Base2632ca6. Added explicit executable ImagePin: absolute canonical path, SHA256,128MiB bound and retained deny-write/delete file handle. ProcessPin verifies live OS process image against that pin and retains its process handle through the request to prevent PID reuse. Hash pins are supplied by trusted launcher, not browser input; a hash of a running test image is evidence binding, not signing or general publisher trust. No credentials read.

New single-use Admission bootstrap pipe verifies host process before parsing metadata; client verifies broker before sending version/epoch (no document text). Only then issues a fresh analysis Ticket bound to the same live broker process. Grant retains peer pin and consumes once; server retains host pin through endpoint completion. Revocable global permit and same absolute deadline cover handshake+analysis. Native host default local dispatch supports this explicit broker Connection; production ProviderService still provides none. No origin-config/environment bypass or ordinary install activation.

Actual GPU production native actor -> broker handshake -> ticket -> pipe -> shared Gemma -> suggestion: English/Korean3 correct outputs413/486/507ms (nearest-rank p50=486,p95/p99/max=507,n3). Image SHA verification/model startup occurred BEFORE this timer and must remain counted as startup, not hidden in eventual end-to-end latency. This round actor/broker share the test process; real two-process image-bound broker acceptance remains pending. Prior two-process plain pipe evidence is separate. Cleanup owner acknowledged. Final tests: Rust library103 PASS/17 ignored, native host24 PASS/1 ignored (GPU test invoked explicitly), native response cache replay21 PASS. Wrong hash/dead PID/wrong host image/wrong broker image tests pass; wrong broker receives zero bootstrap metadata. Own temporary fake image files removed. Same-integrator review only.

Remaining: long-lived broker executable/launcher, bounded concurrent admission, repeated endpoint renewal, installation-pinned config delivery and actual native stdio/browser UI. Current API is a single admission primitive; it is NOT a continuously running discovery service. No OS-wide or hostile same-user-process security claim. Existing installed settings unchanged. CUDA tuning deferred; no main/push/release/Sandbox/account changes. Mission INCOMPLETE.

Evidence D:/dev/grammar-evidence/grammar-gemma-20261007/host-broker-r1.json,host-broker-r1-cleanup.json,host-broker-live-r1.log,broker-all-final.log,broker-host-final.log,broker-cache-r1.json. Next owned executable+repeated admission trial, then installed browser flow.


## Resident Gemma broker executable candidate — 2026-10-07

Base e186d11. Added grammar-gemma-broker.exe with explicit launcher-provided runtime/model/host paths and SHA256 pins. Model hash is streamed in 1MiB blocks (32GiB ceiling), with a retained deny-write/delete handle; runtime and host keep existing ImagePins. Startup has one180s deadline including hash and model warmup. stdin EOF/any byte revokes admission. During model startup revocation is observed after bounded engine startup completes; prompt physical startup cancellation is not yet qualified. stdout reports readiness only after warmup; broken stdout follows acknowledged cleanup rather than panic. No installer/discovery/native-host configuration activation.

Admission::run retains a continuous bootstrap listener, creates one-use endpoints repeatedly and caps connected operations at4. Excess peers are disconnected, not queued. Connected handshake+analysis has one1s budget; idle listening is not a request. Revocation closes endpoints and the executable then awaits Service.close, which verifies process/Job/workspace cleanup. A failed engine is not silently restarted. Hash pins do not establish publisher trust, and this is not a hostile same-user security boundary or a complete runtime DLL manifest verifier.

Actual two-process host GPU synthetic trial: test client executable pinned as host -> standalone broker -> one resident model, three sequential grammar requests. Final binary SHA256 5b877b1fad796cf7f702ec747e51f2c1d8aaf69faaa241e5fac4d9996d6b86d2; resident-binary-r2.json reports439/438/438ms including bootstrap/IPC/inference, correct exact replacements (English only, n3); startup114656ms including full model hash in unoptimized debug build. R1 startup121871ms,429/424/432ms retained. These timings are NOT full popup latency or translation/paraphrase quality qualification. Both runs acknowledged actual cleanup and exited0. Final startup-rejection script4/4 covers runtime/host/model bad hashes and control EOF before model start; no readiness output. It creates/removes only its synthetic invalid model.

Final Rust library105 PASS/18 ignored; native host24 PASS/1 ignored. GPU test invoked explicitly in addition to ignored baseline. Repeated ticket8, stalled-peer cap4, global revoke and prior image checks pass. Existing84 library-test/156 library warnings remain; no new warnings. Same-integrator separate review found stdout panic path and corrected it before final binary rerun; no independent reviewer. Added direct sha2=0.10.9 already in lockfile, crates.io checksum unchanged, MIT OR Apache-2.0 (RustCrypto/hashes); no version upgrades. Dependency used to bound memory during model hashing.

Evidence: D:/dev/grammar-evidence/grammar-gemma-20261007/resident-binary-r2.json,resident-live-r2.log,resident-admission-r1.json,resident-all-final.log,resident-host-final.log,resident-build-final2.log. Source/receipt association is candidate-only, not a personal package. Next: trusted installation/launcher configuration for actual native stdio, permission lifecycle and browser opt-in->cached popup->Accept/reread, plus optimized startup and startup-cancel qualification. Recommendation<=1s and Apply<=0.5s whole-flow targets unchanged; optional CUDA tuning deferred. No normal profile/user docs/credentials/Sandbox change/main/push/release. Mission INCOMPLETE. Current Project Source UI is older; no upload or fresh-chat acceptance claimed.


## Gemma actual native stdio and Chromium candidate — 2026-10-07

Base a04ac4e. OriginConfig now admits optional strict gemma{pipe,broker_sha256}; the executable is fixed to sibling grammar-gemma-broker.exe and pinned before local dispatch. No browser-supplied executable/pipe configuration or environment activation. Absent option preserves rules; explicit configuration failure refuses local analysis without fallback. Local pin failure is carried by ProviderService instead of aborting all Deep consent/cleanup commands. Personal installer still does not provision this route. Trusted launcher and ordinary installed activation remain pending.

New scripts/mission/gemma-browser-trial.mjs uses copied extension/key/host-config and loopback host_permissions, unique reversible native registration and new headless Chromium sandboxed profile. Production main/run_stdio/actor, broker process and actual GPU inference are used, not the earlier injected actor seam or cache replay. The extension product code is unchanged. Fixture page contains only synthetic fields. Opt-in uses the real popup enable button on the owned tab; toolbar/activeTab acquisition is NOT covered. Recommendation measurement is committed synthetic input dispatch through250ms debounce/native startup+IPC+inference to visible full source->replacement annotation DOM and next requestAnimationFrame; not a physical-pixel/IME measurement. Accept uses trusted Playwright input, capture-event timestamp and next timer-turn exact reread, then a second independent reread. Metrics observers do not replace responses or application functions. Pause immediately after completed acceptance prevents unrelated subsequent reanalysis from contaminating the action check.

R5 live-browser-Uc7whC/receipt.json: textarea744.1ms recommendation/15.5ms Accept; simple contenteditable646.7ms/10.8ms. Correct She has two books / He does not agree, English n2 only. Nearest-rank p50/p95/p99/max: recommendation646.7/744.1/744.1/744.1; Accept10.8/15.5/15.5/15.5. Zero deadline misses for these2 rows; insufficient for general quality/latency claims. All8 checks pass: no pre-field analysis, cached card and Accept no additional analysis for both fields, exact reread, source modified at mousedown preserved on refused Accept, password refusal and Disable UI removal. Broker cleanup complete/exit0, unique registration removed, profile removed; subsequent path-scoped process inventory empty. Release broker hash f49be864a068a55acc02b6eb123dcb20560fa7afd313f2b51b6ea4bf6c1ff7cd; host e4a9ec85fd6485adc5f72cfb4719bfb43dd5ddf5efcfb3b4f5eab840d668ba49. Full model hash+warm startup9283.8ms. This used standard release optimization, not new CUDA tuning. Prior debug startup105579ms remains separate.

Failures retained: browser-gemma-r1/live-browser-iVQ921 popup test captured its own tab before page activation; r2/live-browser-z25Yr5 untrusted DOM click was correctly refused; r3/live-browser-wP8g8Q capture listener microtask observed source before the product click listener ran (recommend860.7ms, invalid premature Apply measurement). Fixed fixture tab ordering, trusted input and after-dispatch timer/independent reread, without weakening product trust checks. First complete run r4/live-browser-684b7w passed790.5/613ms and14.6/14ms before the final source-race test was added. Old test_host.py sent request+EOF together, conflicting with already-existing async EOF revocation; reproduced FAIL, updated positive requests to consume reply before EOF, preserving malformed/EOF tests and production revoke behavior. Added unavailable Gemma/no-fallback and independent Deep consent refusal checks.

Final host25 PASS/1 ignored; core/worker77 PASS; real native protocol PASS; optimized build and governance PASS. No product frontend edits. Review performed by same integrator: found and repaired local-config error unnecessarily blocking independent Deep control, verified strict config/no fallback and synthetic-only registration cleanup. No independent reviewer. This is not packaged installer/toolbar/normal profile, physical keyboard/IME, long-lived concurrency, translation/paraphrase/full quality, global editor Gemma integration or whole-product acceptance. README records opt-in schema and test scope. Next: expand multilingual/meaning/false-edit quality corpus and longer latency coverage, integrate trusted personal launcher/readiness+revocation and unmodified installed browser path; other editors remain separate. Startup cancellation still awaits qualification. Optional CUDA tuning deferred. No user docs/accounts/Sandbox changes/main/push/release. Mission INCOMPLETE; Project Source UI remains older, regeneration/upload not claimed.

Final cleanup-hardened harness recheck R7: live-browser-GfMCox/receipt.json, scriptSHA256 5c202bc362c65b7e996947676b80d68345c4b3ed5356707f8e81d6afd95c9f6f. Same release binaries. Recommendation741.5/628.4ms; Accept13.7/14.6ms; startup9283.5ms. All8 checks and browser/broker/registry/profile cleanup PASS. R6 (before cleanup hardening) also passed778.9/626.2ms,14.3/13.1ms; retained separately. Hardened harness continues broker/registration cleanup even if browser close fails, handles control-pipe errors, and includes browser-close outcome in cleanupPass. Final nearest-rank p50/p95/p99/max in receipt; n2 only, not statistical qualification. No further product changes after the tested release build.


## Gemma grammar corpus and compact exact-edit output — 2026-10-07

Base159ff66. Added versioned synthetic English/Korean grammar corpus and actual-browser corpus runner (same modified owned extension/profile/native stdio/broker/GPU path). Expectations and1s recommendation/0.5s Apply budgets are fixed before each cohort runs; no paid/remote Provider. Normal text, tense, negation/uncertainty, numbers/decimals, pronouns, API/JSON terms, quoted adversarial text, emoji, Korean spacing/spelling, longer paragraphs and multiple errors are included. No translation/paraphrase qualification.

Found real latency limitations: r1 original14 short cases quality14/14,max929.6ms; r2 added4 long cases,quality18/18 but all4 long requests missed1s (1096.1–1229.7ms). r3 object-shaped edits preserved18/18 and sped long cases, but short agreement1025.8ms and initial smoke1073.9ms missed. r4 selected full replacement for sources<=64 UTF8 bytes and exact edits for larger input:18/18,max943.1ms. Four initially held-out cases were then added; r5 had21 correct outputs and1 unavailable result, plus typo1043.8ms and two-error failure status1276.4ms. That1276.4ms is failure observation, NOT a usable recommendation. Typed cause of the unavailable result was not exposed by the transport; do not assert a proven parser or decoder root cause. Broker cleaned successfully; post-corpus smoke could not recommend from that stopped/unavailable engine, as recorded. No automatic restart/replay was added. These4 cases were used in the subsequent repair and are no longer independent held-out evidence.

Final implementation chooses its sole request format BEFORE inference: <=64 UTF8 bytes retain the established full replacement response; larger input requests compact edits as arrays of two strings [exact before, corrected after]. Unchanged longer input returns an empty edit list without echoing the source. Max16 edits, strict pair length/schema and existing256-token/32KiB response/8192-byte text/absolute deadlines remain. Reconstruct against the original bound source: nonempty, changed, unique exact before strings (including overlapping occurrence ambiguity), no overlapping ranges; apply in reverse byte-range order, preserve UTF8, bound output size. Unknown/invented/ambiguous/malformed edits are refused, not guessed. No provider/model fallback, replay, debounce reduction or threshold relaxation. External document Apply authority is unchanged. Short full-text response remains supported, not an error fallback.

Final r6 live-browser-RTxYTd/receipt.json:22/22 exact expected outcomes;13 already-correct texts unchanged,9 corrections accepted and reread exactly, false edits0. Recommendation n22 p50=764.9,p95=943.1,p99/max=944.1ms; deadline misses0. Apply n9 p50=14,p95/p99/max=15.5ms; misses0. Additional original smoke/source-change/password/disable checks passed. Startup9951.9ms (hash+warmup, separately measured). Corpus hash495b79889fc3756b3b36ca3f1a1230eff8cc476c353d32b7ac52c7c600da5646; broker1625c9cb207ad7c01ab87521b33cb7e50f24d791dcefd14d3a1cb9ffe87552f0; host5ae84c662e0900b125ef622144e9f983fa601c8ae7b0f12ddd0d090428594118. Top-level acceptance now combines corpus quality/coverage and deadlines with smoke and cleanup; earlier r2-r5 top-level smoke-only latencyPass must not override their corpus misses or unavailable rows. Failed logs retained, not reclassified. Module/script hashes are in the final receipt.

Verification: Rust library107 PASS/18 ignored; host25 PASS/1 ignored; core/worker77 PASS; actual native protocol and optimized build PASS. Unit regressions cover exact Unicode reconstruction, unchanged source, overlapping repeated occurrence, overlapping edits, invented/empty/no-op spans, malformed pair, excessive output and truncated completion. Real GPU/browser corpus separately invoked. All owned browser/broker processes, temporary native registration and profile cleaned; final owned-runtime process inventory empty. Existing warnings unchanged. Same-integrator review only, no independent reviewer.

Scope limits:22 synthetic regression examples on this GPU/headless modified extension, not general language quality, Grammarly equivalence, physical input/IME, all8192-byte inputs, arbitrary dense errors, concurrent documents, installed personal launcher or global editor acceptance. Translation/paraphrase and additional adapters remain unconnected to this Gemma path. Next expand independent quality/length/load coverage and explicit intents, then trusted launcher/installed path; do not enable unverified scope. Optional CUDA tuning remains deferred. No user documents/accounts/Sandbox changes/main/push/release. Mission INCOMPLETE. Project Source UI still older; no upload claimed.


## Gemma independent grammar cohort and literal preservation repair — 2026-10-07

Base: 2d2c5c8c2e4fc47d32768acae61bc55161f2cd8a. Owned host RTX5070Ti, pinned local Gemma model/runtime, real stdio/broker, modified extension and headless owned Chromium profile. No user documents, accounts, live cloud requests, normal profile, installer activation, main, push or release.

Frozen new cohort13: past tense/price, singular they, conditional, plural/article errors, Korean, code identifiers, intentional quoted typo, repeated/two distinct errors and a longer correct paragraph. r1 live-browser-DE9Hxt ran prior production binaries:12/13 exact, false edit1 (quoted literal `teh` -> `the`), all13 within1s. Wrong proposal was recorded and NOT accepted. Cleanup complete. This was first-run independent evidence; after repair these cases are regression, not independent qualification.

r2 live-browser-tlaHz2: generic preserve-literals prompt did NOT fix the quote. Combined35 had34/35 exact, false edit1; one unchanged mixed-term row1145.3ms. Cleanup complete. r3 live-browser-2M56F8: stronger literal instruction plus actual output guard fixed this reproduction;35/35 exact, false edits0, but correct conditional1143.5ms missed. Cleanup complete. No thresholds or expected answers changed; failed runs remain FAIL.

Implementation: grammar-only straight double-quoted/backtick spans are treated conservatively as verbatim, in order. Changed/added/deleted/reordered literals are refused before publication; this is NOT a general code/escaped/curly-quote parser. Unbalanced delimiters are rejected before inference and ready-state mutation. A generated literal violation follows the existing stop/cleanup error path; it is not successful correction and is not silently replayed. Corrections INSIDE these literals are outside this candidate policy, an explicit coverage limit, not general unchanged language quality. A source containing apostrophes alone remains eligible.

The short <=64-byte format now permits explicit replacement:null for already-correct input, avoiding full-source generation. Missing key/unknown fields/wrong types/empty replacement remain rejected. Larger exact-edit format, request/debounce/deadline/token budgets, binding, explicit Apply and no-fallback boundaries unchanged. Candidate text is still reviewed before Accept.

Harness qualityPass now includes post-generation errors such as failed Apply reread;3 regression checks exercise the real summary function. Combined mode retains hashes for both corpora. Unit/source review by same integrator only. Translation/paraphrase intents remain unconnected to this engine's Binding/IPC; no support claim is made. Full length/load, general-language quality, physical IME, installed launcher and global adapter coverage remain pending.

Final production-build r4 live-browser-RO0iIf:35/35 exact,20 correct unchanged,15 corrections accepted+reread, false edits0. Recommendation p50=667.5,p95=927.4,max1244.3ms; ko-spelling missed1s. Apply n15 max25.4ms. Startup11923.8ms; functional/cleanup PASS but aggregate acceptance FAIL. Earlier conditional and literal failures are preserved, not discarded. The nullable output does not establish tail-latency qualification.

Added test-only timing decomposition (wall-clock millisecond timestamps, separate from monotonic end-to-end measurement): committed input -> native send, native round trip (includes host/IPC/queue/inference), response -> paint. This is observation only, with no scheduling, debounce or model changes. No inference-only claim from native round-trip timing.

Timing run r5 live-browser-l0XqXL uses the SAME product binaries as r4. 35/35 exact,20 preserved/15 corrections,0 false edits,0 deadline misses. Recommendation p50=660.9,p95=860.1,p99/max877.7ms; Apply n15 p50=13.8,p95/p99/max14.5ms. Startup13245.4ms. Complete source-change/password/disable and cleanup pass. Phase measurements all35: input-to-native-send min252/median261/max507ms; native roundtrip255/356/508ms; response-to-paint2/10/18ms. The prior1244.3ms miss was NOT instrumented, so its internal cause cannot be inferred from this passing run. Same-binary variability remains unresolved, and a single passing run is not tail-latency qualification.

Verification: library109 PASS/18 ignored; optimized release build PASS; real native protocol PASS; test-summary3 PASS; governance and diff checks PASS. Existing build warnings unchanged. Final source/binary association is holdout-source-final.json. All5 run logs/receipts retained. No source-to-installer claim. No automatic restart, fallback, changed deadline/debounce, CUDA tuning, or expected-answer relaxation.

Next: instrument input scheduling/content-to-worker/storage authorization separately before performance changes; quantify repeat/load variation without dropping slow rows. Keep privacy checks and coverage. Then add explicit translation/paraphrase request binding and quality evaluation; those intents are currently absent from Gemma Binding/IPC. Mission INCOMPLETE. Project Source UI not regenerated/applied.


## Chromium debounce reconciliation repair — 2026-10-07

Base010df04. Candidate local worktree only. Actual production scheduling bug reproduced: input schedules analysis at250ms; heartbeat reconciliation at100ms sees text different from the last analyzed revision and used to postpone the same analysis to350ms. This can add up to nearly another debounce interval. It is a demonstrated cause of avoidable latency, NOT proof of the cause of every earlier GPU deadline miss.

Extracted the existing reconciliation body without behavior change for direct testing. VM tests execute the real production closure, with DOM/transport dependencies stubbed and fake time; no duplicate scheduler. Red:3 PASS/2 FAIL (postponement and uncleared expired timer ownership). Fixed reconciliation to preserve an existing timer while still invalidating stale UI immediately. New input still restarts250ms debounce; programmatic changes without a pending timer still schedule. Expired/cancelled timers release ownership; composition start cancels, composition end schedules. No debounce/target reduction, authorization cache, inference fallback or weakened source validation. Latest scheduler6 + result-summary3 PASS, core/worker77 PASS. Added scheduling test to existing Windows CI workflow; local YAML parse PASS, cloud CI NOT_RUN/no push.

Actual Chromium regression with production extension copy and real local-rule host: scheduling-browser/browser-HQIe4L/result.json,67 checks PASS. Includes changed-line annotations, exact Accept/reread, source/beforeinput races, keyboard actions, password/sensitive refusals, protocol composition, selection review and navigation revocation. Synthetic Deep=false; live Provider NOT_RUN. It is a modified isolated extension fixture, not unmodified installed package/physical IME or GPU inference. Context/server closed and registration remover completed (process exit0); fixture profile retained as task-owned synthetic test data. Initial harness launch failed before setup because its evidence parent directory did not exist; scheduling-browser.log retained, successful retry separate.

GPU tests were NOT successful. Initial instrumentation edit had a Python syntax error before any file write; subsequent stages-r1 used previous harness and failed before corpus execution at broker startup hash/deadline (live-browser-rVTf8B). After instrumentation/scheduler repair, stages-r2-fixed also failed before corpus execution: gemma_start_timeout (live-browser-kztnlR). Both broker exit1; registrations removed and profiles deleted; no broker cleanup-complete acknowledgement, cleanupPass remains false. Never count this as successful runtime acceptance. No1s performance claim from this turn. Runtime --version returned0/build11461 in2.672s; GPU metadata still detects RTX5070Ti. These do not diagnose or repair model startup. Model hash verification and180s deadline unchanged; no security changes or arbitrary retries. No runtime/model binaries changed.

Added fixture-only detailed timing: input->content send->worker arrival->native send->native response->paint; bounded observed deniedOrigins storage timings. Native request construction strips the fixture timestamp. Wall-clock milliseconds diagnose phases; existing monotonic total remains acceptance metric. Concurrent heartbeat storage reads can overlap; the recorded observed storage sum is not guaranteed causal authorization cost. Repeat count1..3 and unique row IDs preserve every case. Instrumentation source parses/tests pass, but its GPU phase receipts remain NOT_RUN because startup failed.

Next: diagnose the model startup readiness/hash/health stages under current host conditions; keep current synthetic scheduling result separate. Then run the fixed35-case corpus repeatedly with the new timing, preserve all misses and quality failures, and assess1000ms/500ms targets. Translation/paraphrase binding, installed launcher and other-editor Gemma integration remain pending. No independent reviewer; same-integrator boundary review and tests. Mission INCOMPLETE, no main/push/release, user documents/profiles/accounts or system changes. Project Source UI unchanged.


## Gemma startup phase diagnosis and repeated browser timing — 2026-10-07

Base9b6f6eb. Added fixed phase-name/elapsed-time diagnostics for artifact pins, model hash, lease/workspace, process spawn, health, warmup and ready. No paths/endpoints/tokens/documents/model responses are logged. Broker phases and engine phases use separate time origins. Optional GRAMMAR_GEMMA_TIMING=1 (trusted fixture launcher only, defaultoff) records HTTP completion/decoding time and transport success, before the literal-output guard; it is NOT GPU-kernel-only timing or a quality verdict. Existing180s startup deadline and1000ms recommendation/500ms Apply targets unchanged. No model/runtime/version, quantization, CUDA tuning, prompt, schema, debounce, provider or cleanup ownership change.

Harness preserves at most4096 bytes of broker phase diagnostics and a live diagnostic file. File-write errors are recorded, fail the trial and do not escape the callback/skip cleanup. stdout remains the original machine protocol. All inference is owned synthetic local GPU, modified isolated Chromium fixture; no user documents/accounts/normal profiles or live cloud Provider.

First instrumented run live-browser-9XgyBv/receipt.json (startup-phase-r1.log): model hash verified2957ms after broker start; engine spawn794ms, health wait entered922ms, health succeeded59100ms, ready77704ms. Overall startup81072.9ms. This locates this run's preparation delay after hashing, in loading/readiness and first warmup (about58.2s and18.6s), but does not prove a CUDA/driver/filesystem/cache cause for earlier failures. Actual corpus35/35 correct,20 unchanged/15 corrections; recommendation max1195.4ms (Korean greeting), Apply max15.2ms. Initial smoke also missed at1058.8ms. Aggregate FAIL retained; cleanup PASS. Slow greeting: input->native254ms, native roundtrip788ms, response->paint154ms; content->worker1ms, worker authorization0ms. Previous debounce fix held: corpus input->native252..262ms. Its earlier250..500ms reset defect is distinct from remaining native/render variability.

Final run live-browser-iGQOGC/receipt.json (startup-phase-r2.log): hash3165ms, engine health entry908ms, health ready6636ms, warmed ready7038ms; overall startup10582.1ms. Same pinned model/runtime, no budget relaxation. Corpus35 cases x2 with alternating textarea/contenteditable:70/70 exact,40 unchanged/30 corrections accepted+reread,0 false edits,0 missing results,0 deadline misses. Recommendation p50=612.8,p95=744.3,p99/max777.7ms. Apply n30 p50=14.6,p95=15.5,p99/max15.7ms. Initial2 smoke recommendations924/579.4ms (all72 inputs max924ms); Apply13.8/14.2ms. Source-race/password/disable checks and browser/broker/registration/profile cleanup all PASS. Model request diagnostics73 observations (including smoke and source-race): p50=346,p95=475,max550ms. Corpus phase ranges: input->content250..263, content->worker0..2, worker->native0..1, native roundtrip248..514, response->paint1..19ms. Observed deniedOrigins storage reads0..1ms. These ranges do not imply every latency maximum belongs to the same request.

Validation: Rust library109 PASS/18 ignored; release build PASS (existing warnings); scheduling+summary9 PASS; real native protocol PASS; governance/diff PASS. Same-integrator review only. No independent reviewer. Startup availability recovered in these two runs, but earlier startup failures and large readiness/warmup variability are NOT resolved or reclassified. No general1s guarantee, arbitrary input/load/physical IME, installed product, translation/paraphrase or global adapter acceptance. Existing literal correction exclusions remain. No main adoption/push/release or Project Source UI update.

Next: preserve phase diagnostics for a repeated loaded/cold readiness/cancellation test and further tail-latency checks; do not enlarge deadlines or remove artifact verification. In parallel scope selection, explicit translation/paraphrase request binding and quality evaluation remain independent implementation work; Gemma's current Binding/IPC is still grammar-only. Mission INCOMPLETE. Final commit/file/binary association: startup-source-final.json.

## Gemma startup cancellation repair — 2026-10-07

Base2bfd7c2. Broker stdin EOF previously revoked admission but did not interrupt engine health/warmup. Real owned GPU red startup-cancel-red.json: EOF during health still reached warmup/ready, exit after6938ms. This proves delayed cancellation, not a model loading cause. The first observer incorrectly included Windows console infrastructure among model children (cwd C:/WINDOWS), so its childrenGone/workspacesGone=false is NOT evidence of a runtime leak; raw receipt preserved. Exact model workspace was subsequently confirmed absent. Corrected observer filters the verified runtime executable and checks its exact owned workspace only.

Broker now passes its existing active flag into start_cancellable. Initial revocation refuses before opening artifacts; a25ms async check interrupts health/warmup I/O under the original180s startup deadline. Revocation wins over simultaneous completion. The engine remains owned and explicitly awaits stop(): Job termination, child reap, active Job count zero and workspace deletion, with the existing single5s cleanup deadline. No dropped start/spawn future, cleanup scheduling claim, retry, model/prompt change or budget increase. Synchronous artifact/workspace preparation and in-progress process creation are still awaited; this is not a universal25ms cancellation guarantee. Startup errors retain typed failure and no stdout cleanup-success protocol is invented.

Acceptance budget declared before execution: EOF-to-broker-exit<=6s (existing5s teardown plus1s observation/exit allowance), actual model exit, exact workspace absent, no admission/ready publication and typed gemma_start_revoked. This does not replace1000ms recommendation/500ms Apply budgets. Real GPU first fixed health125ms/warmup1110ms, both cleanup verified. Final tracked harness scripts/mission/gemma-startup-cancel-trial.py (development psutil7.2.1; no product dependency change) health94ms/warmup1016ms, both PASS. No forced termination. Receipts startup-cancel-{health,warmup}-final.json retain binary/script hashes. Run sequentially with health or warmup plus a fresh evidence label; old receipts are never overwritten.

Same release binaries normal modified Chromium/GPU regression live-browser-0ynFCt/receipt.json: startup9148.6ms; corpus35/35 exact,20 unchanged/15 correction Accept+reread,0 false edits/latency misses. Recommendation p50=596,p95=729,p99/max736.8ms; Apply p50=14.1,p95/p99/max14.7ms. Initial2 smoke max876.2ms recommendation/14.5ms Apply. Cached card/no-inference Accept, source-race/password/disable checks and browser/broker/registration/profile cleanup PASS. Not a new independent corpus, installed package, physical IME, translation/paraphrase or general quality guarantee. Earlier startup/latency failures remain unresolved historical evidence.

Validation: Rust library113 PASS/18 ignored (4 added startup tests); release build PASS with existing156 warnings; Python syntax and governance/diff PASS. Same-integrator ownership/privacy review, no independent reviewer this turn. A receipt summary command initially failed due to console-default CP949; explicit UTF8 read succeeded, no product effect. Evidence root D:/dev/grammar-evidence/grammar-gemma-20261007; STARTUP_CANCEL_RESULT.md and startup-cancel-source-final.json. Candidate only; mission INCOMPLETE, no main/push/release or Project Source upload.

Next: explicit Gemma translation/paraphrase mode and target-language binding across service/IPC/UI, with separate quality/latency corpus and no silent grammar fallback. Loaded/cold startup variability and broader editor/installed qualification remain open; current verified cancellation covers health/warmup only.

## Gemma explicit rewrite intent and first quality observation — 2026-10-07

Base2d73cc6. Added typed Grammar/Paraphrase/Translate(en|ko) intent to Binding. Result matching now includes exact mode/target language as well as epoch/revision/source. Service carries it unchanged; named-pipe request/reply require it and reject another target's response. Missing/unknown/extra intent fields fail closed. An initial strict-deserialization regression showed serde unit variants silently accepted extra fields despite the enum annotation; empty struct variants in the custom deserializer repaired it, same assertions now PASS. Initial host build missed two compact Binding constructors; fixed by explicitly setting Grammar on every existing host route. Existing browser analysis remains grammar-only; no silent translation or cloud fallback. Mixed older wire clients now fail closed; rebuild broker/host together.

Translation/paraphrase use complete structured replacement output; compact edits remain grammar-only. Existing quote/backtick, truncation, output bounds, cancellation/deadline/cleanup and source matching remain. Prompts preserve literals and treat input as data. First12-case owned GPU service+actual same-process named-pipe test completed; no UI/Apply integration for new modes. Observation timeout10s was declared separately to inspect quality and completion time, not substituted for production broker1s or the Owner end-to-end target. Every measured request remained below1s. Runtime/model hashes were verified before test startup. Synthetic outputs only are retained externally.

Quality experiments are preserved: intent-quality-r1, r2, r3 and finalr4.json. Initial English paraphrase overly formal; ordinary English word approved retained in the translation of a metalinguistic instruction. A naturalness prompt experiment r2 changed must not to can't and reduced politeness; this is a real quality regression, not PASS. Replaced that prompt with original-register/modality preservation and added a conservative output guard rejecting changed English modal/negation marker sequences during paraphrase. Regression test uses the observed r2 sentence. It is not a multilingual semantic proof, may reject valid rewordings, and does not claim to preserve every name/number/fact algorithmically. Final r4 restores may/must not. Korean formality and event/number/uncertainty examples were checked against the corpus criteria.

Final r4:12/12 responses and exact request bindings,0 request errors, cleanup acknowledged and workspace removed. Service+pipe p50=487.0ms,p95/p99/max643.7ms (UI/debounce excluded). Same-integrator manual review intent-review-r4.json:10 acceptable for these examples,2 LIMITED (more formal/wordier English paraphrase; ambiguous untranslated metalinguistic approved). Thus PARTIAL_QUALITY_NOT_UI_QUALIFIED, not12/12 full-quality acceptance. This short corpus was used in prompt tuning, not a fresh holdout. No general quality/1s recommendation guarantee or new-mode Apply acceptance.

Final paired release binaries normal grammar modified Chromium regression live-browser-jtrZ07/receipt.json: startup8705.1ms,35/35 exact,20 unchanged/15 corrections with Accept+reread,0 false edits/target misses. Recommendation p50=595.1,p95=712.8,p99/max729.1ms; Apply p50=14.7,p95/p99/max16.1ms. Initial2 smoke max760.9ms. Cached card/no-inference Accept, source-race/password/disable and all browser/broker/registration/profile cleanup PASS. Earlier pre-final grammar run also PASS, kept separately. New modes were NOT activated in this browser run.

Validation: library117 PASS/19 ignored; host25 PASS/1 ignored; real GPU ignored rewrite trial executed separately; final release build PASS (existing warnings); governance/diff PASS. Same-integrator boundary review only. Corpus scripts/mission/gemma-rewrite-corpus.json. Reproduce GPU observation with GRAMMAR_GEMMA_TRIAL=owned-synthetic and GRAMMAR_GEMMA_REWRITE_LABEL=<fresh alphanumeric-or-hyphen label>, then cargo test --release --manifest-path src-tauri/Cargo.toml --lib local_gemma::tests::owned_gpu_rewrite_intents -- --ignored --exact. Evidence D:/dev/grammar-evidence/grammar-gemma-20261007/INTENT_RESULT.md and intent-source-final.json. No main/push/release/Project Source UI update; mission INCOMPLETE.

Next: connect explicit local mode/target selection to an isolated review UI with stale-mode invalidation and safe Accept; qualify translation/paraphrase end-to-end before installed activation. Preserve the two quality limitations and evaluate an independent longer corpus rather than tuning these12 indefinitely. Startup/tail variability, physical IME and other-editor Gemma integration remain open.

## Explicit local rewrite UI and safe Accept — 2026-10-07

Base3db00c6. Added Local review (Gemma preview) selector for Paraphrase/English/Korean and explicit Review locally action. Automatic changed-text grammar analysis stays unchanged. No automatic translation, model startup, cloud fallback or inference on Accept. Full currently enabled field is reviewed only after an explicit trusted click; selected field must still be the enabled target. Editing a draft prevents mode change; accepted mode changes clear caches/reviews and invalidate in-flight results through a generation check. Results never replace a card edited/opened while inference was pending. Existing exact source/revision/range mutation+reread path handles Accept and Apply edit. Pause/disable/navigation ownership remains.

Host carries optional typed intent only on analyze; rules-only host refuses rewrite instead of silently performing grammar. Gemma response matches the whole Binding and returns intent; worker validates exact mode/target/revision and closes malformed-response ports. Unknown/malformed intent is rejected before native send. Existing absent-intent grammar frames remain compatible; no new Deep authority. Unit counterexamples include missing/extra/unsupported intent, wrong-language reply and rules-host refusal. Sensitive focus cannot redirect a local rewrite to an older enabled field.

First live UI run live-browser-xP4Rdj FAIL: Playwright selectOption dispatched an untrusted change, ignored by the product's trusted-gesture handler, so the intended mode was not selected. Preserved failure/cleanup PASS; corrected fixture to real Home/ArrowDown keyboard events without weakening product checks. Corrected run live-browser-FJqP0p exercised12 rewrites and all exact Accept rereads, max628.9ms/15.3ms. Further run live-browser-OMLeys included35-case grammar regression and sensitive-focus check: grammar35/35,0 false edits/misses, recommendation p50=594.7,p95=728.7,p99/max742.8ms, Apply max15.1ms; rewrite max643.1ms/24.6ms; all cleanup PASS. Its generic PASS describes functional/timing checks only, not rewrite language quality; final reporter now makes this distinction explicit.

Final tracked fixture live-browser-fmRrAo/receipt.json (rewrite-ui-final-edited.log) uses same product sources/binaries, records actual transmitted intent, preserves partial rows on failure and includes edited Apply. Warm12-case rewrite UI: request-click->complete suggestion publication p50=485.8,p95/p99/max727.2ms; explicit Accept/Apply edit->exact reread p50=13.4,p95/p99/max14.1ms. No deadline misses. One English translation was explicitly edited to append Reviewed. and applied exactly. Cache inspection and both mutation actions sent zero additional inference. Dirty-draft mode refusal, in-flight mode switch discard, sensitive-focus refusal and unchanged source before Accept PASS. Browser/broker/registration/profile cleanup PASS; engine startup9432.3ms is separate. These explicit-button timings are NOT committed-input proactive translation measurements; grammar keeps that separate measurement.

Final generated12 replacements match intent-quality-r4 exactly: prior same-integrator review10 acceptable/2 LIMITED remains (English register/wordiness and metalinguistic approved left English). No independent holdout/human quality review. Receipt verdict FUNCTIONAL_TIMING_PASS_REWRITE_QUALITY_PENDING, not full product acceptance. Modified isolated extension, task-owned synthetic textarea/simple contenteditable and real GPU/native stdio; not unmodified installed package, physical IME, arbitrary documents or other editor activation. New rewrite preview is not in an installed/released package.

Validation: node core/worker/scheduler84 PASS; host26 PASS/1 ignored; release build/syntax/governance/diff PASS. Same-integrator source/privacy/mutation review only. No main/push/release or Source UI update. Evidence root D:/dev/grammar-evidence/grammar-gemma-20261007; REWRITE_UI_RESULT.md and rewrite-ui-source-final.json. Reproduce with GRAMMAR_GEMMA_REWRITE_UI=1 and GRAMMAR_GEMMA_BIN_DIR=src-tauri/target/release using scripts/mission/gemma-browser-trial.mjs; optional GRAMMAR_GEMMA_CORPUS=all adds grammar regression. Mission INCOMPLETE.

Next: independent longer translation/paraphrase corpus, register/meaning/coverage failures and end-to-end latency tails; retain current12 as regression. Installed launcher/package and other-editor Gemma integration still need separate qualification. Cold-start variability remains unresolved.


## Longer Gemma rewrite holdout and explicit deadline reporting — 2026-10-08

Base8c7bd05. New independent eight-case synthetic longer translation/paraphrase corpus includes amounts, negation/modality, conditions, event order and exact literals. First owned GPU/pipe observation intent-quality-holdout-r1.json:8 exact bindings,4/8 exceed1000ms,max1385.005ms, cleanup complete/workspace removed. Observation allowance10000ms remains separate from production1000ms. Same-integrator manual review:7 useful outputs meeting these case criteria,1 unchanged Korean paraphrase with no rewording coverage; not whole-language quality acceptance. Corpus is no longer independent after subsequent experimentation.

Rejected compact-edit paraphrase experiment intent-quality-holdout-edits-r2.json: third case2307.457ms and gemma_modality_changed,5 NOT_RUN,cleanup complete. First translation also missed1000ms. Reverted that production prompt/format change; retained existing grammar-only compact edits. No claimed speed improvement or relaxed deadline.

Retained production repair: elapsed local deadline is returned with exact id/epoch/revision/intent, empty suggestions and no replay/Deep fallback. Explicit UI says time limit exceeded/no changes applied. Actor regression verifies one call, no fallback, exact error binding. Existing absent-intent failure behavior retained. This error classification states that the deadline elapsed, not a root-cause diagnosis for the underlying transport/engine failure.

Actual modified-extension GPU/native UI live-browser-siLetl: first3 longer cases exact Accept/reread (recommend963.6/808.4/810.6ms,Apply max14.3ms). Fourth Korean paraphrase deadline error visible1017.5ms, source unchanged; remaining4 NOT_RUN. Engine logged1013ms failure; cleanup browser/broker/process/profile/registration complete. Overall FAIL preserved. Initial short grammar smoke in same run took1376.9ms (target miss), so even short-case tail guarantee remains unproven. Startup100201.2ms including model hash51854ms and engine48254ms is separate, unresolved cold variability.

Same final product short regression live-browser-53ID2K:12/12 explicit rewrites and exact Accept/edited Apply rereads; recommendation max691.5ms,Apply max16.1ms; smoke2 max742.2/14.3ms; cleanup complete, startup8445.1ms. FUNCTIONAL_TIMING_PASS_REWRITE_QUALITY_PENDING only: previous10 acceptable/2 limited quality classification remains. These explicit-button timings are not proactive translation timing, installed qualification or Sandbox evidence.

Focused local_gemma library26 PASS/9 ignored (101 unrelated filtered); host28 PASS/1 ignored; node core/worker/scheduling84 PASS; release build PASS with existing warnings. First new test used invalid intent kind key and timed out; corrected test to actual mode wire schema, original log retained. Same-integrator code/boundary review only. Evidence D:/dev/grammar-evidence/grammar-gemma-20261007/HOLDOUT_RESULT.md and holdout-source-final.json. No main/push/release or Project UI update. Mission INCOMPLETE.

Next: diagnose long-output and cold-start tail costs, compare authorized Gemma E4B speed/quality against this unchanged regression corpus and a fresh holdout before choosing any smaller model. Do not assume faster equals acceptable quality; preserve1s/0.5s goals. CUDA-specific tuning remains deferred. Installed launcher/package, physical IME and other-editor Gemma integration remain separately unqualified.


## Gemma E4B comparator and shared-GPU failures — 2026-10-08

Base49bbf72. Official Google E4B IT QAT Q4_0 downloaded without credentials, revision4b4a2c1d584be7264f87aac328a1bc739ce81b6c,5154941280bytes,SHA676c35070db6dbe52f93e9c864ee0fba4eddea94b9c875d9cb10daff453fbaee. Upstream metadata and streamed bytes match; Apache2.0 metadata/license reference saved. Model remains external, no dependency/lockfile/system installation change. Official references https://huggingface.co/google/gemma-4-E4B-it-qat-q4_0-gguf and https://ai.google.dev/gemma/apache_2 .

Only owned benchmark/test selectors added: GRAMMAR_GEMMA_TRIAL_MODEL=e4b or12b (default12b), fixed file/hash whitelist; unknown models rejected before browser trial. Production model/prompt/guards/deadlines unchanged. Test receipt now identifies model/runtime and supports new six-case fresh corpus created before E4B output review. Final fixture additionally records its test-executable SHA256 and artifact verification/startup time and preserves startup errors/NOT_RUN instead of leaving only an empty partial receipt; that new error-report branch is compile-reviewed, not exercised by another GPU failure trial. Earlier quality receipts pin model/runtime/corpus but did not capture the test executable hash before the reporting-only rebuild; do not claim an exact test-binary receipt chain for those earlier trials. Browser fresh corpus selection is not added in this round; fresh is the Rust service/pipe fixture only.

E4B long regression intent-quality-e4b-holdout-r1.json:6 valid replies meeting case criteria, seventh1243.4224ms rejected gemma_modality_changed, last1 NOT_RUN. Rejected raw output unavailable; this guard failure is not proof of a particular semantic alteration. Seven attempts p50=730.3697,p95/p99/max1243.4224ms. No deadline relaxed; cleanup complete/workspace removed. Initial engine readiness126661ms separately observed. This corpus was independent only on the earlier12B first observation, not on subsequent experiments.

Fresh E4B six responses: p50=508.0949,p95/p99/max648.7863ms; bindings/cleanup PASS. Manual same-integrator review:4 case-acceptable,1 unchanged English paraphrase (no rewording coverage),1 entity ambiguity (지원 rendered Support; original lacks an explicit person marker, so not labelled a proven name-preservation error). No whole-language acceptance.
Same fresh corpus12B observation failed first request at10012.6878ms against the separate10000ms observation allowance;5 NOT_RUN, no output to judge. Cleanup returned runtime_cleanup_completed_late but workspaceRemoved=true; failure is preserved. Code path means remove_dir_all returned after the existing cleanup deadline, not proof of its internal delay cause. No bounded-cleanup PASS claimed.

Owned engine absent after that test, yet GPU readings showed54..86% utilization and9692..9838MiB used. Other/shared workload is present; no other process was stopped or reconfigured. This does not prove the cause of request/filesystem delay. Subsequent E4B browser attempt live-browser-KmyYCq recorded82 GPU samples (utilization2..98%,memory10261..14110MiB). Failed startup within original180s total budget; model verification76400ms, then health did not become ready. All browser functional/rewrite/Accept cases NOT_RUN, aggregate cleanupPass=false/acceptance FAIL retained. Broker exited1; task registry/profile removal recorded. Independent post-check found zero llama-server processes and zero session directories in the runtime parent. Do not substitute this for the receipt's failed qualification.

These are shared-host observations, not controlled proof of model speed superiority. Do not select E4B from fast rows while excluding guard failure/no-op/missing rows. Default remains12B candidate, which also still lacks the required tail-latency/availability qualification. No new installed product or Sandbox/other-editor acceptance.

Validation: focused local_gemma26 PASS/9 ignored,101 unrelated filtered; browser unknown-model negative PASS; syntax/governance/diff PASS. Same-integrator review only. Evidence D:/dev/grammar-evidence/grammar-gemma-20261007/E4B_RESULT.md, e4b-manual-review.json, e4b-comparison-protocol.json, e4b-source-final.json; original failures retained. Mission INCOMPLETE; no main/push/release/Project UI update.
Next: capture startup/hash/request/cleanup phase costs with resource context and repeat a controlled idle-GPU paired comparison before any default change. Improve no-op/ambiguity coverage using explicit-person counterexamples without rewriting the fresh trial history. CUDA-specific tuning remains deferred; general model/editor integration still pending.


## Gemma storage/cleanup phase diagnosis and SSD candidate — 2026-10-08

Base1a8e5ea. Added opt-in GRAMMAR_GEMMA_TIMING numeric diagnostics: model read/hash accumulated wall time, stop terminate/reap/descendant/workspace phases, and each owned remove_dir_all attempt. No document text, credentials, endpoints or filesystem paths in these diagnostics. File removal completion timestamp is captured before diagnostic output so the log cannot change its completion-vs-deadline classification. Existing budgets/ownership/error propagation remain. Stop logs show phases, not a claim of cleanup completion before actual checks.

Extracted broker's existing streaming/deny-write-delete model pin into pin_model, used unchanged by the real launcher and a no-GPU profiling test. Actual Windows small-file regression rejects wrong SHA, revoked/expired verification and file write/delete while the verified handle remains held. Broker retains the handle through engine lifetime; no cached-hash bypass. Default model remains12B, no CUDA/kernel change.

Observed D: maps to SATA HDD WDC WD80EAAZ; C: maps to NVMe SSD CT1000P3PSSD8 with169GB free at measurement. Actual production pin profiler on D:12B46623.98ms (read44680.329ms,hash1933.638ms),E4B42667ms (read41347.949ms,hash1312.599ms). Read timings include OS scheduling/cache/IO behavior, not a pure disk hardware benchmark. This identifies read calls as the dominant measured stage, not the internal cause of every historical slow boot.

Copied both exact pinned models without altering/removing originals to C:/Users/USER/.codex/grammar-owned-models/20261008-io-trial. Streamed copy SHA and destination reread match upstream pins; external ssd-model-copy.json. Added explicit owned-fixture GRAMMAR_GEMMA_MODEL_STORAGE=ssd (default evidence remains unchanged, unknown value rejected before browser startup). Trusted production broker already accepts explicit absolute model path plus SHA; no general installation/cache discovery added.
SSD pin profiler:12B2574ms(read1148.703/hash1418.877),E4B2192ms(read1039.939/hash1147.301). Copy/cache state and time/load differ, so this is NOT a cold-cache controlled hardware speedup guarantee. No cache eviction/system tuning performed.

Sequential same6-case service/pipe observation with SSD and lower observed pre-trial GPU footprint (about2.3GB; samples in ssd-paired-gpu.jsonl): both6 exact bound replies and complete workspace cleanup.
12B: artifact verification2568.19ms,engine15493.34ms; requestp50=715.7794,p95/p99/max1172.8766ms,one target miss(fresh-en-ko-exact). Six outputs acceptable for stated case criteria by same integrator, not independent/full-language acceptance.
E4B: artifact2058.94ms,engine5904.12ms; requestp50=502.173,p95/p99/max638.5748ms,no miss. Existing4 case-acceptable/1 unchanged English paraphrase/1 entity ambiguity retained; output matches prior E4B fresh trial. This corpus is now regression, not fresh again. No default-model promotion.
Both use unchanged10000ms observation allowance separate from production1000ms. Exact current test executable SHA is in each receipt.
12B stop788ms: reap about786ms, workspace1ms/remove<1ms. E4B stop456ms: reap430ms,workspace25ms/remove24ms. Historical10s request timeout and cleanup_completed_late remain FAIL; current fast cleanup does not prove the old deletion delay's cause or a repair.

Real modified Chromium/native/GPU SSD12B run live-browser-23rFuR: startup8657.72ms (pin2647ms,engine5899ms); grammar35/35,0false edits/misses,max811.6ms from committed input,Accept14.8ms. Explicit short rewrite12/12,max753.4ms from button click,Accept/edited Apply15.1ms; initial2 smoke max791.3/14.4ms. Exact rereads/stale/sensitive/disable/cache-no-inference checks remain. Stop1127ms(reap785,workspace341/remove340),browser/engine/registry/profile cleanup PASS. Verdict FUNCTIONAL_TIMING_PASS_REWRITE_QUALITY_PENDING, preserving prior short-language limitations. Not installed/Sandbox/physical IME/general editor qualification.

Validation: full library117 PASS/19 ignored, broker boundary test1 PASS/profile1 ignored in normal suite, actual HDD+SSD profile trials PASS; release broker/host build PASS with existing warnings; browser syntax/unknown-storage negative/governance/diff PASS. Same-integrator review only. No main/push/release, other-process termination, security/driver settings, user documents or profiles. Evidence D:/dev/grammar-evidence/grammar-gemma-20261007/STORAGE_PHASE_RESULT.md and storage-phase-source-final.json. Mission INCOMPLETE.
Next: use explicit SSD trial profile for further quality/tail work; diagnose E4B unchanged paraphrase/ambiguous entities and prior English guard rejection before any model switch. Long translation target remains unmet for12B. Preserve cold-start/shared-load and late-cleanup uncertainty, installed integration and global editor work; CUDA-specific tuning remains deferred.


# Gemma paraphrase diagnosis and speculative decoding trial — 2026-10-08

Base d6041a45954a2334c3ffd2ea24d427d505468559. Candidate only; mission INCOMPLETE. No model/prompt/decoding default change, no main adoption, push, release or installed acceptance.

## Quality diagnosis
E4B baseline English professional paraphrase returned Spanish. Existing English modality guard refused publication. Two other diagnostic baseline paraphrases were unchanged. Raw outputs belong exclusively to fixed synthetic fixtures, not user documents. This explains the observed regression case, not every E4B failure.
Three rejected test-only instructions: r1 explicitly demanded rewording but added Could / changed does not to cannot; r2 forbade adding modal verbs but still changed does not to cannot; r3 restricted rewriting to lexical substitutions and passed guards, but produced overformal unnatural wording and broadened discount to reduction and cause to reason. Guard success is not semantic quality success. No guard weakened and no prompt promoted. r3 English professional1140.6ms, Korean1392.8ms, English caveat884.6ms; quality failures remain even where speed is below1s.
Receipts paraphrase-prompt-diagnostic-r1/r2/r3.json and matching logs preserve outputs. r1 is exploratory and did not itself retain artifact pins. r2/r3 verify and retain runtime/model handles; each records its test executable SHA. Final ignored diagnostic retains r3 rejected instruction explicitly; earlier variants remain in this session's evidence, not the production prompt. Diagnostic completion means requests/cleanup completed, NOT quality acceptance.

## Speed experiment
Pinned llama b11461 CUDA12.4 help exposes --spec-type ngram-simple. Test-only cfg(test) selector GRAMMAR_GEMMA_TRIAL_SPEC allows none or owned-synthetic ngram-simple; invalid values fail before lease/workspace creation. Defaults for ngram remain untouched. Production builds contain no selector and preserve existing invocation. No custom CUDA kernel, assistant-model download, driver change or extra inference provider.
Measured same six-case service/pipe corpus, observation allowance10s separate from unchanged product1s. First pair uses identical test executable, second pair uses identical newer executable adding retained artifact pins. Executable/model/runtime/corpus hashes in receipts.
- ngram r1: first8411.4ms; second10006.3ms deadline;4 NOT_RUN.
- none r2:6 valid,3 target misses,max1383.7ms.
- ngram r2:2529.2ms,3783.9ms,then10011.2ms deadline;3 NOT_RUN.
- none r3:6 valid,6 target misses,max5557.7ms.
All completed outputs byte-match prior12B baseline on corresponding cases. Missing outputs are not parity evidence. All four cleanup results OK and actual workspace removal confirmed. spec-comparison-summary.json; spec-pair-r2-monitor.jsonl captures shared GPU context. High variable GPU utilization/memory occurred; sequential order and uncontrolled external load prevent causal speedup/slowdown attribution. Nevertheless no useful speedup was demonstrated and ngram is NOT activated. Previous deadline/cleanup failures remain failures.

## Retained changes and validation
Pure production prompt builder extracted without content/expression change; whitespace-normalized comparison against base confirms equivalence. Raw diagnostic calls actual generation and existing guards but bypasses publication: NOT product acceptance. Service/pipe fixture now retains verified deny-write/delete artifact handles for its lifetime. No UI, Apply or product privacy boundary changed.
Full library117 PASS/20 ignored. Production broker/host release build PASS, recorded in diagnostic-build-r2.log; first attempt used wrong host target name, preserved diagnostic-build.log, not a product build failure. Governance and diff checks PASS. Existing compiler warnings retained. Same-integrator review only, not independent review.

Next: keep12B baseline. Investigate output-generation work for long translations with a held-out quality corpus and resource-aware measurements, without shortening away source meaning or raising1s/0.5s targets. Shared-load availability/tail costs remain unresolved; installed Gemma/Sandbox/physicalIME/global-editor integration remains unqualified. Do not retry rejected E4B prompts or ngram blindly.


# Gemma inference phase diagnosis — 2026-10-08

Base 5be6475a5ec0070ccadc17c443ca3b3385178400. Candidate only, mission INCOMPLETE.

Added opt-in GRAMMAR_GEMMA_TIMING numeric metrics for runtime-reported prompt processing, prediction, token counts/cache count and local HTTP elapsed time. Missing/invalid counters are omitted, never reported as zero. Source text, completion, model paths, identifiers and arbitrary response metadata are excluded by a strict numeric allowlist. Default generation request, prompt, model, cache_prompt=false, output guards and deadlines are unchanged. The original complete response still reaches existing validation. Metrics describe completed HTTP responses only; timeout has no final per-phase breakdown and is NOT inferred from the warmup.
Pinned upstream field documentation: https://raw.githubusercontent.com/ggml-org/llama.cpp/b11461/tools/server/README.md (Timings and context usage). Counters are runtime-reported timings, not an independent GPU profiler.

## Actual synthetic GPU trial
Original eight-case long regression corpus, pinned12B SSD model/runtime, spec none, artifact handles retained. Startup24911.9232ms. Warmup completed:31 prompt tokens,3880.005ms prompt processing;20 generated tokens,1039.336ms prediction;HTTP4922.715ms;cache_n=0. This is a warmup prompt, NOT a user recommendation benchmark.
First long request exceeded unchanged10000ms observation allowance with gemma_pipe_deadline;7 NOT_RUN. No generated output, so no quality judgement or successful recommendation. The separate production1000ms target remains unchanged. Receipt retains exact attempt elapsed value; a single failed attempt cannot provide a representative latency distribution. No threshold increased or difficult row omitted to claim acceptance.
GPU samples remained high/variable; owned engine exit was followed by97..98% GPU utilization and about9.2GB used. Other workload was not inspected/stopped. This cannot isolate the model's idle performance or prove the cause of every delay. Do not repeat blind trials while saturated. Actual stop/descendant/workspace cleanup succeeded, with workspaceRemoved=true. Original failures stay FAIL.

## Validation and next work
Full release library118 PASS,20 ignored. New counter-sanitization regression covers valid data, hidden source/content/path fields, negative/boolean/string/object/null counter values, missing fields and nonfinite elapsed time. Production broker/Chromium host release build PASS with existing warnings. Governance self-test and diff checks PASS. Same-integrator code/boundary review only; no independent review.
Evidence:12b-inference-phase-r1.log, intent-quality-12b-inference-phase-r1.json, inference-phase-monitor.jsonl, inference-phase-library.log, inference-phase-build.log. Source/build identity in inference-phase-source-final.json.
Next: use these numeric phase counters in a lower-load owned12B long-corpus run to distinguish prefill vs output cost before changing decoding/model. Shared-load availability remains an unresolved product issue; lack of an idle benchmark does not excuse missed1s targets. No new installed/Sandbox/UI/Apply qualification this round. Existing Apply evidence is unchanged;0.5s target remains. No main/push/release/Project UI update.


# CUDA runtime comparison and complete bundle provenance — 2026-10-08

Base fb34614847dd72b0c757ff0d7d37f45b4e532e4f. No production model/runtime/prompt/guard/deadline change. Test-only runtime selector defaults cuda12; cuda13 is an explicit owned synthetic comparator, unknown value rejected. Candidate only; mission INCOMPLETE.

## Actual trials and limitations
CUDA12.4 long-corpus repeat12b-inference-phase-r2: startup47408ms; warmup prompt3754.730ms / prediction13356.498ms / HTTP17336.4763ms. First long request timed out against unchanged10s observation allowance;7NOT_RUN. Cleanup OK/workspace removed. Initial pre-trial GPU18%,11048MiB used, so this was NOT an idle/exclusive trial. Other GPU process names were read to establish competing applications; no contents, credentials or process controls used, none terminated. No cause assigned to a specific application.

Same llama b11461/commit4d756bc72, official Windows CUDA13.4 portable archives downloaded into task-owned evidence directory; no driver/system installation. SHA256 archive digests matched official release metadata. Driver596.49 accepted version query and actual inference. llama code MIT; NVIDIA CUDA component terms referenced separately in runtime-cuda13-license.json. No redistribution claim or executable upload.
CUDA13.4 long8: all bound replies, no transport errors, but8/8 miss1s. Service-pipe p50=4154.79925ms, nearest-rankp95/p99/max=6395.9683ms,min3608.6ms. Warmup excluded. Runtime-reported output generation sums to76.38% of measured HTTP duration; prompt processing alone712.9..1193.6ms. Thus this loaded run is not fixed by transport-only changes. Timings are runtime counters, not an independent profiler; no idle-hardware causal speedup claim. All actual cleanup checks OK;stop1501ms,workspace3ms. No Apply/UI trial in this round.

Four outputs differ from earlier CUDA12 baseline despite same EXE/model/prompts/seed; therefore do NOT assume bit-identical output across CUDA backends. Same-integrator review against fixed case criteria:7 acceptable for these cases,1 unchanged Korean paraphrase (no rewording coverage). Changed outputs retain release gating/all12 checks, conditional review/order, exact literals, and reviewed/unreviewed file sequence. Not broad language or register qualification. Full synthetic outputs remain in exact receipts; no user documents used.

## Provenance fix in owned fixture
Both distributions have identical llama-server.exe SHA a812655fed77b9f98d918e7aa0b1552403b76dacbeb9fa24c17dac9ed3d08ec1; backend/CUDA DLLs differ. EXE hash alone cannot identify CUDA implementation. The fixture now enumerates34 EXE/DLL files, hashes a filename-NUL-fileSHA-newline manifest in sorted filename order, checks a fixed bundle digest, retains deny-write/delete handles for all files through cleanup, and records runtimeId/runtimeBundleSha256. Model hash pin and existing request binding remain.
CUDA12 bundle785601b8f32ba6ab10374f8e9ead5d8deadd56e70940883ef65608dbfbb80860;CUDA13 bundled6ef94c506863a4621a7e8b1c5f37ac255756b28686a3325bbd52785620172ad. Per-file inventories saved separately. This closes comparator evidence ambiguity; it is NOT deployment of complete runtime-manifest validation into the product launcher. Existing older receipts are not rewritten/upgraded.

Validation: full library119PASS/20ignored, including actual Windows same-EXE/changed-DLL/additional-DLL fingerprint and denied write/delete while pins held. Final subsequent edit only fixed helper indentation. Production compilation was not repeated because all code changes are in cfg(test); existing production bytes/settings unchanged. Governance/diff PASS. Same-integrator review only. No source push/main/release/installed acceptance.

Evidence:runtime-b11461-release.json,runtime-cuda13-download.json,runtime-cuda13-license.json,runtime-bundle.json,runtime-cuda13-bundle.json,runtime-cuda12-bundle-upstream-check.json,12b-inference-phase-r2.log,intent-quality-12b-cuda13-phase-r1.json,cuda13-phase-summary.json,cuda13-phase-monitor.jsonl,cuda-bundle-library-r2.log. Exact candidate identity:cuda-bundle-source-final.json.
References: https://github.com/ggml-org/llama.cpp/releases/tag/b11461 ; https://developer.nvidia.com/blog/cuda-toolkit-12-8-delivers-nvidia-blackwell-support . Official Blackwell support motivates comparison, not a speed guarantee.

Next: await a GPU-available window for an unchanged paired12/13 trial; do not kill other apps or repeatedly run saturated trials. Recommendation1s/Accept0.5s remain unmet/unexpanded by these results. Keep defaultCUDA12 and12B until comparator quality and end-to-end timing justify promotion. Independently carry bundle provenance into trusted launcher/package integration before an installed CUDA13 option is activated. Project Source UI unchanged.


# Trusted broker runtime bundle boundary — 2026-10-08

Base f4cfe65347340dc0ff4279a4f2c13cfb639865bf. Production broker now requires seventh trusted argument runtime-bundle-sha, canonicalizes runtime path, verifies the adjacent EXE/DLL manifest and retains all verified file handles through service cleanup. Model/service starts only after this check. Existing 180s startup deadline and stdin revocation cover enumeration and each streamed read; max128 selected files/8GiB total. No new stage budget, fallback, default runtime/model change or kernel tuning. Updated every current repository launcher: browser trial, startup-cancel trial, ignored broker integration test. Six-argument old callers fail closed, not silently exempted.

Actual Windows production-helper unit regression: valid manifest, denied write/delete during pin lifetime, changed DLL, added DLL, missing DLL, deadline and revocation. Broker tests2PASS/1profile ignored; broker/host release buildPASS with existing warnings. Node syntax/Python AST/governance/diff PASS. Built broker external boundary receipt r2: legacy args refused, incorrect bundle refused, correct official CUDA12 bundle advances to deliberately missing-model refusal; readiness absent and no inference engine observed in all3. This is PRESTART verification, not successful model/UI/installed operation. Broker executable SHA is recorded.
Initial observer incorrectly required zero child processes; bundle mismatch correctly refused, but observer failed seeing one child. Kept original script/failure evidence. r2 records actual observed child images: Windows System32/conhost.exe only. It excludes only that exact console host from the no-inference check; no other subprocess is permitted. No claim that every descendant was absent instantaneously at broker exit. No process was forcibly terminated.

Same-integrator review: digest supplied by trusted launcher; no document/extension configuration route; numeric/error-only diagnostics; all pins remain live until cleanup; unknown/extra manifest files at verification alter digest; canonical resolved file parent must match runtime directory. This is adjacent-bundle provenance and existing-file pinning, not dynamic Windows loader attestation or protection against malicious same-account directory changes after verification. Installed launcher/package integration and a real ready-to-cleanup/UI run with this changed broker remain NOT_RUN. Do not promote CUDA13 or claim1s speed/0.5s Apply success from this work.

GPU preflight13% and10414MiB already occupied; did not repeat a saturated model benchmark. No GPU inference this round. Current user throughput target remains unmet. Next: owned startup-cancel/ready/service/cleanup integration when GPU resources permit, then installed launcher with reviewed bundle manifest. Continue independent packaging work without touching other apps. Candidate only,missionINCOMPLETE,no main/push/release or Project UI update.
Evidence:broker-bundle-tests-final.log,broker-bundle-build-final.log,broker-bundle-binary-receipt-r2.json,check-broker-bundle.py and corrected check-broker-bundle-r2.py. Source/build linkage:broker-bundle-source-final.json.

# Prototype completion priority and Gemma package prerequisite — 2026-10-08

Owner now prioritizes finishing installation/startup/API configuration and the first prototype without reducing current quality, even when latency goals are missed. Retain Gemma12B, existing inference and mutation guards. Short recommendation1s, paragraph3-4 sentences3s, Accept0.5s are targets, not current universal performance claims. This supersedes the previous next action of waiting for an idle GPU comparator run.

Build receipt schema3 builds and hashes grammar-gemma-broker with static CRT alongside the app/host/target DLL. Package generation requires schema3 and verifies the relocated broker again; installer includes the manifest-verified broker. Legacy receipt1/2 verification remains available for historical artifacts. No model/prompt/decoding/default-runtime changes. The package explicitly marks Gemma default startup pending.

Production receipt verification regression:14 checks PASS using task-owned synthetic source/binary bytes, including wrong broker hash/size, failed build, wrong CRT, valid relocation and relocated tampering. This is verifier coverage, not actual release build or installed acceptance. Governance+self-test PASS. Same-integrator diff review only.

Remaining: verified model/runtime setup; app-owned broker startup/revocation and desktop/default local analysis connection; API endpoint/model configuration and protected key storage with synthetic API tests; owned install/start/exit validation; scoped publication policy reconciliation and GitHub prototype delivery. Existing prototype publication request is recorded in Contract1.5.3 but has not been executed. Main adoption, user profile/data access and paid API calls are not inferred. Project Source requires later regeneration; mission INCOMPLETE.

Actual package follow-up: source a47e102576680402418b6118d2493c42689d0bd3. Initial build compiled binaries but correctly refused a receipt because Tauri converted pre-existing Cargo.toml checkout CRLF to committed LF (1115 to1079 bytes). Preserved first-build-rejected.json and logs. Confirmed working bytes equal HEAD and refreshed index metadata only; no content rollback or relaxed source check. Fresh r2 build passed unchanged physical source check, frontend typecheck/build, app/host/Gemma broker release builds and target DLL build. Existing unused/dead-code warnings remain. Package generation and installed four executable/DLL hashes PASS. Fresh owned install with SkipBrowserHost succeeded, then removal completed with zero remaining owned files/processes. Existing user data intentionally retained; no registry registration, model inference, UI or default startup acceptance in this check.

Evidence: D:/dev/grammar-evidence/grammar-prototype-20261008-a47e102-r2/{build-receipt.json,installed-files.json,install-receipt.json,uninstall-result.json,package.zip}. ZIP SHA256 73994602f432a82251e84409a4a49cf034ae731054c69c83dd48d9a83077243b. This is a local intermediate package; no GitHub upload/release. Next priority is model/runtime setup plus app-owned startup/default local route, followed by API connectivity and final prototype qualification, not more speed-only trials.

# Installed Gemma preparation and launcher — 2026-10-08

Continuation base9a06d81. Added pinned Gemma-Assets.json, Prepare-GrammarLocal.ps1 and Start-Grammar.ps1 to personal package/install. Preparation uses exact official model/runtime URLs and hashes, optional explicit offline AssetCache, bounded-size HTTPS downloads, staging and failure cleanup. Existing12B Q4_0 and CUDA12 bundle remain unchanged. First actual preparation refused runtime bundle: upstream archive includes20 additional CLI/benchmark executables absent from the previously qualified34-file server/DLL set. Preserve failure evidence; now exclude only these non-server EXEs, retaining DLLs/license files and the unchanged bundle digest. Real cache preparation subsequently PASS. No network download was exercised this round.

Launcher holds verified app/host/broker/catalog file handles, exclusive per-install session lock, validates installed receipt, waits for exact broker PID/readiness, publishes Chromium config only after ready, starts app, restores its own config and revokes stdin on exit. Actual cleanup marker+zero exit required after readiness. No silent restart/cloud fallback. Missing assets/changed binaries/existing endpoint/config conflict fail closed. Uninstaller refuses active launcher; under the same lock it marks installation uninstalling before removal to prevent a new launcher racing removal. Forced launcher termination may leave stale sidecar: refused rather than overwritten automatically. Desktop Gemma route and default app icon/startup remain pending.

Verification:12 Windows production-script tests with Rust stub executables PASS, covering valid launch/cleanup, bad hash, missing model, start failure, existing endpoint, external sidecar edits, missing cleanup marker, duplicate launcher, uninstall refusal while active, completed owned uninstall, uninstalling refusal, offline assets and corrupt-cache cleanup (some tests cover multiple checks). Initial test harness failures were UTF8 decoding and invalid escaped stub JSON; fixed harness without relaxing assertions. PowerShell syntax/governance/diff checks PASS. Same-integrator review; no independent reviewer this round.

Real12B model and actual a47e102 broker/host with a task-owned stub app: preparation PASS, startup->ready->stub-app exit->cleanup PASS,11.953s total cold lifecycle, original sidecar restored,0 remaining owned processes. This is REAL_MODEL_STARTUP_NOT_INFERENCE and NOT_INSTALLED_PRODUCT_UI. It does not qualify recommendation latency, quality or final installed Chromium/desktop flow. Evidence D:/dev/grammar-evidence/grammar-launcher-20261008/{setup-r1-failure.json,local/assets-receipt.json,startup-result.json,cleanup-check.json,MANIFEST.json}. Launcher SHA46fd71958529cbb3404925e38489cc0389a236f583aa0c0a42db8eb822a91e57. Model/runtime files retained only in task-owned evidence.

Next: desktop Gemma routing and app-visible model lifecycle/setup; reconcile operational timeout vs performance target without reducing quality; API endpoint/model configuration and protected key handling; final source-bound package plus installed UI/browser validation and authorized scoped GitHub prototype delivery. No push/release/main adoption, live API calls, user profiles/documents or source UI application occurred. Mission INCOMPLETE.


# Desktop Gemma grammar route candidate — 2026-10-08

Continuation base6e14ba3. Explicit Start-Grammar.ps1 -EnableDesktopGemma passes a pinned broker endpoint only to the app child; default remains off pending installed-widget qualification. Broker admits at most two independently pinned client images (host and desktop), retaining existing peer verification and one-shot request binding. Legacy seven-argument launcher compatibility retained. No model/prompt/decoding changes.

Desktop Grammar captures now use the shared local Gemma analyzer when explicitly configured. Candidate identity includes Gemma engine identity; source/session/generation/mode/terminology and active cache are checked before publication. Invalidation cannot reactivate a stale candidate. Deep completion preserves the matching active engine identity. Protected-term spans and line structure remain guarded; the conservative contiguous edit envelope may refuse separate edits around an unchanged protected term. Invalid span projection now fails closed instead of dropping protection. No automatic fallback/replay or editor mutation added. Analysis failure preserves the draft and reports unavailable.

Operational request ceiling15s is separate from recommendation1s/paragraph3s goals, with one deadline covering broker pin/admission/inference. Existing browser client deadline1s and regression admission deadlines remain unchanged. Blocking image pin runs off UI; one retained semaphore permit limits unfinished work after cancellation to one. Timeout stops waiting, not the blocking file read; this distinction remains explicit. Existing inference cancellation can terminate the resident model, and no automatic restart is implemented. This must be addressed before default activation; current owned trial requires app exit/launcher restart after such failure.

Verification already completed: full core120PASS/20ignored and app241PASS/31ignored; frontend typecheck/build PASS; JS runtime state27/contract24 PASS; production launcher script13 stub tests PASS including default-off/explicit-on child configuration. These are not installed widget/Accept acceptance. Existing unused/dead-code warnings remain. Same-integrator code/boundary review only, no independent reviewer this round. Additional final verification and actual-model result are recorded below.

Real-model R1 debug-broker trial failed before ready at182.85s; stderr unavailable, exact cause unproven, no inference, exact owned broker/runtime process count0 afterward. R2 diagnostic trial also failed before ready: artifact pins26594ms, model hash phase entered, then unchanged180s startup deadline. This locates R2 before model load/inference; do not attribute it to GPU saturation. R1/R2 failures are preserved. Release-broker comparison is separately identified, not retroactively upgrading those failures.

Evidence directory: D:/dev/grammar-evidence/grammar-launcher-20261008. desktop-runtime-tests.log, desktop-broker-tests.log, desktop-final-tests.log, desktop-broker-release-build.log, desktop-real-analysis-r1-failure.json, desktop-real-analysis-r2.json and its broker.log/log. No new source-bound full installer build or UI/Apply qualification this round. Existing package receipts refer to their original source only.

Remaining prototype work: installed desktop model lifecycle and real widget path; translation/paraphrase routing; explicit recovery after model cancellation; endpoint/model API configuration with protected key handling and synthetic API tests; final default startup/package/browser regression; scoped GitHub publication reconciliation and delivery. Mission INCOMPLETE, no main/push/release/user-profile or live API activity.

Actual R3 optimized release broker: artifact pins8974ms; full model hash verified at53918ms, model-pin44.943s = read42.963s/hash1.968s. Engine reached health then warmup at30.823s after engine start; overall unchanged180s startup deadline expired during warmup. Result FAIL gemma_start_timeout, no desktop user request executed. Startup warmup WAS attempted: the original receipt field inference=NOT_RUN refers only to the desktop user request and must not be read as no inference attempted anywhere. Future harness receipts now name userRequest explicitly. This is not a quality/latency success. Debug R2 and release R3 locate different failure phases; no unsupported GPU or IO root-cause generalization.

Actual R3 cleanup: engine stop1287ms including descendants1285ms and workspace1ms, workspace removalOK, no forced broker termination. Independent exact owned executable-path process check0. All failure receipts/logs retained; no threshold raised or slow case discarded. Final actual installed desktop capture/Gemma/Accept remains NOT_RUN. Evidence desktop-real-analysis-r3.json, desktop-real-analysis-r3.broker.log, desktop-real-cleanup-check.json. Warmup recovery/startup availability remains a prototype blocker; continue API/setup work independently rather than repeating saturated trials.

Final desktop-final-tests-r2.log: core120PASS/20ignored, app241PASS/31ignored after deadline-wait repair and restoration of the original1s admission test. Subsequent test-only edit renames failed-start receipt inference to userRequest for accurate warmup scope; no production code change. Governance+self-test and diff checks PASS. Source/binary/evidence linkage is retained in desktop-source-final.json outside repository.


# Gemma terminal failure and explicit relaunch ownership — 2026-10-08

Base868e3da. Found two linked lifecycle faults: a stopped engine's service continued accepting requests, and a dead broker made Start-Grammar exit/release pins and its install lock while the desktop app remained open. New real PowerShell launcher regression on an owned stub app/broker reproduced the latter before changes (recovery-red.log: launcher exited1 while app still open). No user desktop/profile/model invoked.

Service now closes its queue and drains pending callers after the engine enters not-ready state following a request. Broker watches actual queue closure and revokes listener/connected peers without needing another request. It waits for service cleanup before emitting cleanup acknowledgement, even when the operation fails; exit remains nonzero for failure. Launcher distinguishes acknowledged cleanup from operation success, restores only its own published sidecar, warns the user, and retains app image pins/install lock until app exit. No forced app termination, automatic restart, request replay, fallback Provider or weakened inference/Accept guards. Explicit recovery is close app from tray then relaunch; in-app model restart is not implemented.

Validation: full core122PASS/20ignored, app241PASS/31ignored; broker buildPASS; production PowerShell launcher15 testsPASS. New tests cover failure while app stays open, immediate owned-sidecar restoration, duplicate launch/uninstall refusal, app natural exit, successful explicit relaunch, and nonzero operation failure despite exact cleanup marker. Each fixture checks its exact owned broker is gone before removing task-owned files. New actual Windows named-pipe regressions close admission on service closure and disconnect an already connected idle peer. Existing renewal/capacity tests now use a live rejecting synthetic service (formerly closed service), retaining original assertions/deadlines; closed service now intentionally terminates admission. No test threshold relaxed.

Evidence D:/dev/grammar-evidence/grammar-launcher-20261008/{recovery-red.log,recovery-launcher-green.log,recovery-launcher-final.log,recovery-rust-final.log,recovery-broker-build.log,recovery-source-final.json}. Same-integrator review only; no independent reviewer. Model/startup timeout R1-R3 remains FAIL; no GPU inference repeated and no new real-model recovery, installed-widget, browser or Apply acceptance claimed. No frontend change requires frontend rebuild. Default desktop Gemma remains qualification-only.

Next priority: API endpoint/model configuration and protected key handling with synthetic endpoint tests, then default startup/setup and installed UI qualification. Do not repeat slow model benchmarks as a prerequisite for independent prototype work. Startup warmup availability, translation/paraphrase desktop routing, in-app explicit model restart and final source-bound package/release remain pending. Quality/model/deadlines unchanged; missionINCOMPLETE/no push/main/release or Project UI changes. Project Source snapshot is stale pending a later requested replacement.


# Warmup isolation and measured desktop first request — 2026-10-08

Base2778fd2. Earlier debug model-hash timeout and release warmup180s timeout remain FAIL. Before changing warmup, repeated the exact previously retained release broker (SHA846415dc...a949) with the same D-drive12B model: R4 PASS, old warmup524ms, desktop production analyzer575ms with exact expected grammar output and complete cleanup. Model-pin36.384s was dominated by read34.747s. Preflight GPU2116/16303MiB,21% utilization; earlier failing session had approximately9-10GB occupied. These are observations, not proof of GPU spilling, CPU fallback, or an identified root cause. No other process changed/stopped. Old warmup is not intrinsically broken, and the original startup failure remains availability-unqualified.

Changed startup from a discarded full grammar JSON response to a fixed no-document one-token chat decode. Startup alone accepts finish_reason length, exactly one completion token and valid content/no tool calls, then requires the sole slot idle. User grammar/translation/paraphrase prompts,256-token ceiling, complete-output rejection,model/quantization/runtime,source/term/literal guards and startup180s/request15s budgets are unchanged. The new startup response never enters the suggestion cache. This bounds unnecessary startup output, not a demonstrated solution for all resource-pressure stalls.

Added opt-in GRAMMAR_GEMMA_TIMING numeric-only HTTP header timing and a2s startup slots probe, with500ms read ceiling and existing32KiB response cap. Only processing boolean/decoded unsigned count are emitted; raw slots contents,text,path,token and generation parameters are discarded. Probe is a scoped future cancelled with startup, not a detached reader/task; original startup deadline includes it. Current runtime did not expose decoded count in R5, recorded null rather than invented0. Completion counters locate R5 warmup delay in prompt processing. CPU/GPU layer placement is still NOT_MEASURED; collecting a whitelisted backend-placement diagnostic remains needed for loaded failures. Fixed-runtime API reference: https://raw.githubusercontent.com/ggml-org/llama.cpp/4d756bc72/tools/server/README.md (slots and completions).

R5 same D model: startup14.636s, warmup4.707s (prompt4.705s,one output token), first/second/third desktop analyzer492/445/472ms, exact expected corrections3/3. p50=472,p95/p99/max=492ms,0/3 misses of1s component target. R6 final stricter warmup-response validation plus explicit owned C-drive SSD model path, identical pinned modelSHA: startup9.940s,model-pin3.282s(read1.771/hash1.504),warmup135ms;478/420/472ms,3/3 exact corrections,p50=472,p95/p99/max=478ms,0/3 component misses. Baseline old warmup524ms was faster than R5 new4.707s; do not claim universal speedup. OS cache/GPU load not controlled, so no causal SSD-only or warmup-only speedup claim. Samples are tiny short-English synthetic grammar smoke, not translation/paraphrase or general linguistic quality acceptance. Component timings include app image pin/IPC/result guards, exclude capture/UI publication/Apply. Warmup/startup not hidden in recommendation metrics.

Final Rust core123PASS/20ignored,app241PASS/31ignored; release broker buildPASS with prior warnings. Tests verify truncated warmup is rejected as a user completion, malformed/missing/tool response refusal and numeric-only diagnostics. Actual R6 stop843ms,workspace removalOK; exact task-owned broker/runtime process count0 independently confirmed. No user documents/accounts/API/system changes, no frontend/model/quality-degrading setting changes. Same-integrator review only.

Evidence D:/dev/grammar-evidence/grammar-launcher-20261008/{warmup-baseline-r4.*,warmup-one-token-r5.*,warmup-ssd-r6.*,warmup-unit-final.log,warmup-build-r6.log,warmup-cleanup.json,warmup-summary.json,warmup-source-final.json}. Actual test uses production analyzer in a test executable with release broker, not installed widget/Sandbox/browser Apply. Final desktop default remains off. Next: loaded-startup placement/availability evidence and app-visible preparation/retry without blocking app startup; then installed first/repeated/cancel/restart qualification. API configuration may proceed independently, but these successes do not close the historical timeout or whole-product readiness. No main/push/release/Project UI replacement; missionINCOMPLETE.


# App-first managed Gemma preparation status — 2026-10-08

Baseb5b9230. Explicit qualification switch EnableDesktopGemma now starts the pinned desktop app before broker/model readiness. Managed startup shows the window even when normal start-hidden preference is set; unmodified ordinary launcher mode retains its prior behavior. UI adds preparing/ready/failed status above all content including login/onboarding. Sequential1s status reads perform no inference or network. Ready transition never replays a capture. Existing drafts are untouched by status refresh/failure.

Launcher publishes a bounded ephemeral installation-local JSON session (random32hex nonce,version,phase,pipe only). App inherits nonce,launcherPID and manifest-derived brokerSHA; file cannot supply binary path/hash. Reader derives the path from its own executable directory, caps read4097 bytes/rejects over4096, rejects unknown fields/wrong nonce/phase/invalid pipe UUID. App retains one launcher process handle until process exit; dead owner fails closed without following PID reuse. Endpoint is read only for a new explicit capture; actual broker image/peer verification remains mandatory. Preparing/failed capture is refused, not deferred or sent to another engine. No document/history persisted.

Startup errors promptly revoke/stop owned broker, preserve open app and install lock until explicit app exit, and expose failed state. Early app exit during preparation prevents endpoint publication. Normal exit deletes only the exact status bytes written by this launcher and restores its own original host config. Failure within status publication or broker cleanup still retains app ownership via finally. Explicit retry remains copy/retain desired work,exit app,rerun launcher; in-app restart preserving the live capture is NOT implemented. Default desktop activation still off pending installed qualification.

Validation: production PowerShell with owned stub binaries18PASS, covering app-before-ready,ready transition without app restart,start failure app/lock retention,early app exit/no publication,session file cleanup and previous lifecycle/tamper boundaries. Initial15-test run had one expected contract mismatch: stub recognized old static PIPE environment only; updated it to require nonce+launcherPID+brokerSHA and added3 lifecycle tests. Original failure log retained; safety assertions remain. Rust app242PASS/31ignored; final tighter pipe UUID parser focused1PASS. Production app cargo checkPASS,frontend typecheck/buildPASS,JS state27/runtime24PASS. Library/GPU not rerun because engine unchanged. No new real model/Sandbox/installed widget/clipboard/Apply acceptance or speed measurement. Window visibility/layout and dead-owner behavior need installed E2E verification; current UI is build-tested only. Same-integrator review, no independent reviewer.

Evidence D:/dev/grammar-evidence/grammar-launcher-20261008/session-{launcher-initial,launcher-final,launcher-reviewed,rust-final,parser-final,production-check,frontend-final,state-contract,runtime-contract}.log and session-source-final.json. Next: installed managed startup/status/capture test with current source-bound package, then explicit in-app model retry without draft loss; high-load startup qualification still needed. Prior180s timeout remains unresolved under its original conditions. API settings can continue independently. No main/push/release/accounts/user profiles/Project UI changes;missionINCOMPLETE.


# Explicit model retry with retained desktop app — 2026-10-08

Base af75ee5. The managed desktop trial now offers Retry local model after an already-ready broker exits and its cleanup acknowledgement, process exit, stderr drain and host-config restoration are confirmed. The launcher retains the same app process, image pins and installation lock while preparing a fresh broker. It revalidates the original sidecar before publication; no capture/draft or request is supplied by the retry command and no prior analysis is replayed. After readiness the user captures again. Model, prompts, response limits, quality guards and deadlines are unchanged. Desktop option remains explicit/default-off.

Each recoverable failure receives a fresh random32hex ticket in the bounded ephemeral session; ready/starting states reject tickets. A create-new request file prevents duplicate clicks overwriting pending requests. The launcher consumes only its current ticket before spawning; stale requests cannot restart a later generation. The UI associates pending state with the generation so a rapid second failure does not leave retry permanently disabled. Requests contain no text, endpoint or executable selection. External config changes and unconfirmed cleanup fail closed. Startup/initial warmup failures do NOT yet offer in-app retry because the broker does not issue equivalent cleanup acknowledgement on every pre-ready exit. This change does not resolve the historical loaded warmup timeout.

Validation: production PowerShell launcher with owned stub executables20PASS (retry-launcher-reviewed.log), including two successive failures, stale-ticket rejection, same app PID across new broker PIDs, explicit recovery, no automatic restart, unconfirmed-cleanup denial, install-lock/uninstall exclusion, sidecar restoration and session removal. Rust app243PASS/31ignored before final status-generation serialization addition; final focused session tests2PASS after that addition. TypeScript/Vite buildPASS; existing JS state27/runtime24PASS. No installed UI click, real GPU, Sandbox, clipboard or Apply test in this checkpoint. App-process survival in the stub is not installed draft-retention acceptance. Same-integrator separate code review only; no independent agent review.

Initial logs preserved: one launcher assertion failed because the warning omitted existing tray-close guidance (guidance restored); first Rust compile failed from duplicate command registration during an interrupted edit (duplicate removed). Both are repaired, not prior product failures. Evidence D:/dev/grammar-evidence/grammar-launcher-20261008/retry-*.log and retry-source-final.json. Next: source-bound installed managed startup/retry qualification, pre-ready cleanup receipt design for warmup retry, then default setup/API integration. No main/push/release or user-account/profile actions. Mission remains INCOMPLETE.


# Pre-ready warmup failure cleanup and explicit retry — 2026-10-08

Base15be043. Gemma startup now has a typed StartupFailure separating operation error from cleanup_confirmed. Existing start/start_cancellable callers keep their String-error API. Only awaited engine.stop success after a load/health/warmup failure can set confirmation; early artifact/lease/spawn errors and failed cleanup remain unconfirmed. Drop is never treated as acknowledgement. Broker emits the existing cleanup marker on confirmed pre-ready failure, still exits nonzero, and never publishes a ready endpoint. Startup180s/cleanup5s budgets, model, prompts and user-generation quality guards are unchanged.

Launcher consumes a pre-ready cleanup marker separately from readiness, requires actual broker exit and diagnostic drain, then offers a fresh explicit retry ticket while retaining app/pins/installation lock. Initial and subsequent attempts use the same path. Unconfirmed failures remain blocked; app exit during preparation still revokes startup. A failed attempt left unrecovered remains a nonzero launcher result even without a published browser endpoint. No implicit retry, capture replay, alternate model or provider fallback.

Verification: production launcher with owned stub executables21PASS. Native production engine test invokes a shared synthetic Rust HTTP runtime on Windows: both invalid warmup and a3s test deadline reach POST, stop the actual process, remove its actual runtime workspace, and return cleanup confirmation; missing artifact returns no confirmation. Full Rust core124PASS/20ignored and app243PASS/31ignored. Subsequent stricter native assertions exposed a fixture bug: unread POST bytes caused gemma_transport instead of the intended invalid-output error. Failure log pre-ready-native-reviewed.log is preserved. Fixture now drains the complete bounded request body; final focused native test passes exact gemma_warmup_output and gemma_start_timeout assertions. Earlier full-suite counts predate this fixture-only correction; final focused and actual-broker tests cover it. Built actual debug broker plus production launcher/owned app and same synthetic runtime: two successive warmup failures, explicit retry, fresh failure ticket, new runtime PID, same app PID, no published endpoint, both runtime workspaces removed, final owned runtime PID exited and nonzero failure result all PASS. Command: cargo build --manifest-path src-tauri/Cargo.toml --bin grammar-gemma-broker; python scripts/mission/test-gemma-pre-ready.py. Fixture runtime is not a model and no external inference occurs.

Evidence D:/dev/grammar-evidence/grammar-launcher-20261008/pre-ready-{compile,native,native-reviewed,native-final,launcher,broker-build,broker-native,broker-native-reviewed,broker-native-final,core,app}.log and pre-ready-source-final.json. Same-integrator code review; no independent review. GPU load/latency, installed widget button/draft contents, capture/clipboard/Apply and default entry remain NOT_RUN this checkpoint. This repairs warmup-failure recovery, not the root cause of the historical loaded180s warmup timeout. Desktop default remains off; next is current-source installed managed startup/retry qualification, then default setup/API completion. No main/push/release/account/user-profile changes; missionINCOMPLETE.

# Managed installed UI preparation — 2026-10-08

Product source0214b3e fresh release build and schema3 package verification PASS. Evidence D:/dev/grammar-evidence/grammar-managed-ui-20261008-0214b3e contains build-receipt.json, package-result.json and package.zip. ZIP SHA2561e9c29f65883b8f087e8d757746f26a4b8e74140f5ec2dc75195b65b79fb206b. Build/package success is not installed acceptance. Package metadata still conservatively labels desktop routing pending; implementation exists behind the trial option, installed managed qualification remains pending.

Prepared managed-app-e2e.mjs, Guest-App-Managed.ps1, fixtures/managed-runtime.rs and prepare-managed-fixture.py. The separate package-fixture modifies only Gemma-Assets.json and MANIFEST.json, with explicit fixture classification. FIXTURE_OVERRIDES.json records unchanged app/broker/launcher/installer hashes; original package is preserved. Intended checks: actual preparing/failure/retry UI, edited draft/source preservation, zero replayed user requests and cleanup. This is synthetic asset-catalog/runtime qualification, not full unmodified-package/model acceptance. JS/PowerShell syntax and fixture compilation PASS; guest assertions NOT_RUN. Initial MediumLaunch compiler path formatting failed; normalized paths compiled successfully.

New Sandbox start failed with CO_E_APPSINGLEUSE 0x800401F6. Existing instance20980ce1-ef75-4528-9fd9-4d014e68c50e predates this task and is preserved. Owner was asked whether to replace it because closing loses unsaved guest work/login; no answer yet. No guest install, UI test, clipboard, inference or session termination occurred. Prepared payload has task-owned readonly inputs and fresh output, with network and clipboard redirection disabled in managed.wsb. Continue with the saved payload only after existing-session disposition. Product package remains bound to0214b3e independently of this later harness checkpoint. MissionINCOMPLETE; no main/push/release.


# Fresh Sandbox managed startup qualification — 2026-10-08

Owner explicitly authorized previous Sandbox closure and replacement. Previous20980ce1 was stopped; new2382b23d ran the owned Medium-integrity fixture. Product0214b3e app, target DLL, broker and launcher hashes matched the release manifest. Only synthetic asset catalog/manifest differ from original package; original package/ZIP preserved. No real Gemma, GPU, live Provider, quality or latency acceptance.

run11 PASS6 installed checks: hash identity, visible preparing widget, invalid warmup offers Retry after cleanup, actual Retry button reaches ready, exactly2 warmups/0 user inference, natural app/launcher exit with runtime workspace and session file removal. Editor also exited naturally. Uninstaller reports complete, install root/registration/processes/user data absent. New Sandbox stopped; inventory empty.

Full capture/draft test remains FAIL at foreground acquisition in run9, before capture. Do not count the separate --status-only scope as draft preservation or no-capture-replay proof. Full test remains the default and its assertions remain. UIA native field ownership now checks Win32 PID rather than unavailable accessibility provider PID, but foreground transition remains unresolved. Do not keep repeating the same focus hypothesis; next inspect owned foreground window/thread and desktop identity.

Failures run1-10 preserved: missing host origin from SkipBrowserHost (now task-owned registration), absent probe payload (preflight added), foreground/field discovery, PS5 policy refusal (compiled helper used without changing policy), synthetic runtime early exit (abandoned I/O robustness added; exact cause unproven), and missing slots id=0 (corrected to actual production idle contract). run10 reached Retry but synthetic ready failed; run11 supplies correct slot response. Product binaries and timing/quality gates unchanged.

Evidence D:/dev/grammar-evidence/grammar-managed-ui-20261008-0214b3e/{run1..run11,FIXTURE_OVERRIDES.json,build-receipt.json,CHECKPOINT.json}. Review by same integrator only. JS syntax, PowerShell parse, Rust fixture and C# helper compile, governance validator checked. Desktop default remains off. Next capture/draft qualification then default/API setup; historical loaded180s timeout cause unresolved. No main adoption/push/release or Project UI replacement; missionINCOMPLETE.


# Managed capture/draft retention observed; repeat focus unresolved — 2026-10-08

Base37fb752. New owned Sandbox60ab93ac-8069-4f38-a787-79901991a9db. Added numeric-only foreground diagnostics and10s helper invocation deadline; no product changes. run12 full installed test PASS7: real CtrlShiftG capture, editable synthetic draft during preparation, warmup failure preserves draft, actual Retry reaches ready with exact draft/source reread,2 warmups/0 user inference, natural app/launcher/editor exit and runtime/session cleanup. Product0214b3e binaries unchanged; fixture catalog/runtime synthetic. This is observed draft retention for this run, not repeat reliability, real Gemma quality/latency, Apply or default activation qualification.

Repeat run13 FAIL before capture: target visible but foreground remains PID5144. Read-only guest process query identifies Explorer/session1. On passing run12 target was already foreground before helper, so UIA focus is not proven to recover Explorer foreground. Two bounded pointer alternatives run14/15 refused target/cursor verification before button events; retained externally, not adopted. Final helper has no mouse injection, only previous UIA SetFocus plus diagnostics. No security/desktop permissions changed; exact restricted-desktop cursor failure cause remains unproven. Do not repeat generic focus guesses or treat fresh-VM success as repair. Next establish owned interactive foreground setup independently from product acceptance; API settings work can proceed independently.

All receipts/logs preserved under D:/dev/grammar-evidence/grammar-managed-ui-20261008-0214b3e/run12..15. Final uninstall complete: install root/registration/processes/user data absent. Sandbox explicitly stopped, inventory empty. C# compile, JS syntax and governance validation PASS; same-integrator review only. MissionINCOMPLETE,desktop defaultoff,no main/push/release.


# Optional API transport foundation — 2026-10-08

Base03b874b. Added provider/http_api.rs as an unregistered transport component, not a fourth active Provider or user-ready API feature. Exact HTTPS chat/completions endpoint and immutable model; reject URL userinfo/query/fragment, empty/control-bearing model and invalid keys. Caller lends an in-memory key for one explicitly consented request; no credential lookup/persistence/logging. Sensitive Authorization header. JSON stream=false/store=false; store=false is a request preference, not proof of external provider retention policy. No proxies, redirects, transport retry or fallback. Windows native TLS uses system certificate validation unchanged. Operational deadline120s covers HTTP wait/read, separate from1s/3s UX targets. Input128KiB serialized-byte limit, response256KiB with both declared-length and incremental chunk checks. Never truncate into success; require one stop-completed text choice with no tools/refusal. Errors carry fixed classifications/status only. Cancellation ends local waiting, not proof that a remote server stopped computation/billing.

Production adapter integration remains pending: UI endpoint/model configuration, protected key lifecycle, destination-bound consent/revocation, single-operation admission, canonical prompt with approved matched terms, existing result validation and document/revision/source binding. Do not wire raw complete() output directly to Apply or call it from local suggestion/cache paths. Existing Codex/Claude/Antigravity dispatch unchanged; no user setting or default changed. Local Gemma remains independent.

Validation:9 focused tests call actual HTTP code against owned127.0.0.1 servers with dummy key and synthetic text. Includes no consent/pre-cancel/invalid key/escaped input overflow without request, exact destination/model/body,302/429 refusal, length and chunked output caps, timeout, cancellation after receipt, malformed/truncated/tool result refusal. HTTP fixture override is private and cfg(test)-only; public config rejects plaintext including localhost. Full library133PASS/20ignored, desktop252PASS/31ignored. Initial7 tests also passed; cancellation fixture was then synchronized on actual request receipt and accept wait bounded to avoid scheduler-dependent hangs. No live API, account, real key, certificate handshake, UI or installed API acceptance.

Dependency review: retained reqwest=0.13.4; enabled native-tls for HTTPS. Added15 transitive lock entries (including non-Windows platform metadata); existing package versions/checksums unchanged. Windows additions native-tls0.2.18,hyper-tls0.6.0,tokio-native-tls0.3.1,schannel0.1.29 plus rustls-pki-types/zeroize; registry source/checksum/license/MSRV declarations in evidence dependencies.json. Windows compiler1.97.1 builds/tests them successfully; cross-platform builds not qualified. No broad upgrades.

Evidence D:/dev/grammar-evidence/grammar-api-transport-20261008/{focused-final.log,regression.log,provider-contract.log,dependencies.json}. Same-integrator boundary review only. Previous Sandbox foreground repeat failure and loaded Gemma startup root cause remain unresolved. Next session API settings/consent integration with existing capture review; keep API disabled until binding and credential lifecycle tests pass. No main/push/release,missionINCOMPLETE.


# Session-only API setup and revocation boundary — 2026-10-08

Base57cc3b4. Settings now includes Optional API setup; Tauri status/configure/clear commands keep endpoint/model, key and permission only in AppState memory. No file/environment/credential-store lookup and no network on save. Password input is cleared when submitted/removed/unmounted. Setup/key Rust-owned buffers use zeroize1.9.0 (already locked, direct dependency only); this is not a guarantee that all JS/IPC/HTTP temporary copies are erased. Status cannot serialize a key; errors are fixed codes. Restart starts unconfigured. UI explicitly says API rewriting is not enabled; backend requestEnabled=false. Existing three CLI providers and local Gemma selection unchanged.

Configuration uses a random epoch: clear/replacement rotates it, and stale UI writes are rejected before changing newer state. Invalid current replacement clears old setup rather than falling back. Permission is destination/model-specific within the prepared setup; a request also needs explicit consent. Single active ticket, cancellation on replace/clear/drop, old finish cannot clear new slot, ticket completion is single use. Final ticket guard rejects repeat complete calls including a previously cancelled ticket. In-flight tickets may hold their old memory key until their HTTP future settles; no immediate-all-copies-erased claim. Global Deep revocation clears API state as well and reports incomplete revocation if either branch fails. App shutdown clears the session before CLI cleanup.

Important remaining work: there is still no API rewrite Tauri command or active ProviderKind. Capture/source/revision/attempt, canonical matched-term prompt, result parsing/terminology validation and existing review/Accept integration are NOT wired. Ticket finish only proves API setup ownership; it is not document safety. Do not enable requests merely because the settings form works. Next integrate that explicit request path then installed synthetic qualification.

Validation:7 session unit tests including stale save after revoke, invalid replacement, old/new slot ownership, no permission, no secret status, clear/drop, cancelled ticket no-send. Original React component with mocked Tauri boundary4 headless Chromium checks: save clears password input, explicit removal, delayed save response after global revoke ignored, external requests0. This is not installed IPC acceptance. Initial UI harness launch lacked Playwright1234 browser; no test executed. Explicit existing Chromium1228 binary then passed in an isolated headless temporary profile; no downloads/user profile. Initial helper edit failed cp949 decoding App.tsx, corrected to explicit UTF8 before build.

Full library140PASS/20ignored and desktop259PASS/31ignored before final single-use guard; final provider-focused run42PASS/3ignored separately recorded. Shutdown also clears a poisoned API lock without skipping other owners cleanup; final production check recorded. Frontend type/build and all npm provider contract suites PASS. Rust/HTTP fixture keys/text synthetic; no actual API credentials, paid/live provider, TLS endpoint or installed UI test. Dependency change only promotes previously locked zeroize1.9.0 (MIT OR Apache-2.0) to direct use. Same-integrator review found/fixed stale-save and late-UI-response boundaries; no independent reviewer.

Evidence D:/dev/grammar-evidence/grammar-api-session-20261008/{session-final.log,regression.log,provider-final.log,frontend-final.log,provider-contract.log,ui-result.json}. Previous Sandbox foreground and loaded Gemma startup limitations remain; desktopdefaultoff,missionINCOMPLETE,no main/push/release.


# Explicit desktop API writing integration — 2026-10-08

Base d9cfdb4. Run API Deep now requests one explicit confirmation naming the configured endpoint/model before sending the captured selection and only matched approved terminology. The command calls the production rewrite state machine shared with CLI Deep: exact capture token/attempt, mode/language/terminology revision, strict structured writing output, line structure and current-result publication checks. API provenance is separate from the three CLI provider choices; auto rewrite still uses its existing route. Endpoint compatibility is specifically HTTPS chat/completions with the strict writing response contract, not arbitrary APIs. No quality relaxation, replay, fallback or automatic mutation.

Session epoch and per-call permission are both required. Configuration replacement/revocation cancels pending tickets, and publication holds the capture/session boundary so stale setup cannot publish. Cancellation retains the capture and allows the existing Instant draft path; already published Ready results remain reviewable. A cancelled API slot is retained until the request settles, preventing reconfiguration from accumulating concurrent local HTTP operations. CLI rewrite and API rewrite admission exclude each other. Cancellation does not prove remote compute or charges stopped. Memory-only key behavior and transport limits remain unchanged.

Verification: synthetic loopback HTTP, dummy key, production rewrite_selected_text_impl. Full Rust library142PASS/20ignored and desktop269PASS/31ignored. Subsequent explicit admission intent validation plus delayed-cancel regression: final focused11PASS (9 production-state tests +2 adapter tests). Cases include valid Ready/API result, no consent, stale epoch/token, revoked setup, replaced capture, malformed model reply without diagnostic echo, cancellation, changed intent, approved matched terms only, and delayed cancel preserving the current binding. Full suite precedes that last focused regression; do not report270 as a full run. Frontend build and npm test:provider PASS. Original React settings component with mock Tauri4PASS, not whole-App or native UI verification. Same-integrator source review, not independent review. Initial compile/typecheck failures preserved before fixes.

Evidence D:/dev/grammar-evidence/grammar-api-writing-20261008: initial.log, regression.log, final-focused.log, frontend.log, provider-contract.log and API settings component receipt. Historical installed package remains0214b3e; this source has not been packaged/installed. Real keys, paid/live API, external HTTPS compatibility, actual Run API Deep confirmation UI and downstream installed Copy/Accept are NOT_RUN. No user documents/profiles or external inference. Gemma desktop default remains off; missionINCOMPLETE. Next build and qualify the source-bound installed synthetic API workflow, then finish default prototype setup. Existing Sandbox foreground repeat remains unresolved. No main adoption, push or release.


# Installed API boundary qualification and native shutdown repair — 2026-10-08

Source63ddc8d clean release/schema3 package built; original app f221647fb8399e94da81fd04f4f9bf6fa028afc3fcdd09271f39cd75051c772c. Fresh owned Sandbox, network/host clipboard sharing off, Medium integrity. Real installed settings IPC and UI: memory setup/no connection, permission removal, owned shortcut capture, explicit API confirmation rejection/no connection, accepted request to loopback stalled TLS server, Cancel/socket closure with exact edited draft/source preservation, Copy with exact guest clipboard reread. No TLS bypass, certificate trust changes, valid HTTPS completion, real key or live Provider. Successful API result remains prior production Rust synthetic-loopback evidence, not installed successful-response acceptance.

Original-package runs1/3 reached these7checks then failed app exit with native_apply_cleanup_pending after the captured editor exited first. run2 stopped earlier at the existing guest Explorer foreground problem; no new API request. All failures retained. A first ERROR_INVALID_HOOK_HANDLE-only fix passed real Win32 tests but installed run4 still failed. Diagnostic-only DLL run5 confirmed worker/process WAIT_OBJECT_0, while Unhook returned false with last-error183; this error value does not establish the Windows internal cause. Diagnostic binary logs contain only PID/stage/code, never text. Instrumentation was removed from product source.

Final TargetApplyClient close still requires worker termination. Hook release accepts success, invalid-hook proof, or termination of the retained exact target process handle. Unknown unhook failure while that process is alive/unavailable remains pending. No PID lookup, deadline relaxation, mutation change or forced target termination. Rationale: Microsoft process/thread termination contracts free thread resources and signal retained process/thread objects (https://learn.microsoft.com/en-us/windows/win32/procthread/terminating-a-process and https://learn.microsoft.com/en-us/windows/win32/procthread/terminating-a-thread). Native old helper reproduces failure exit20 on expired hook; final native fixture tests real hook release/expired hook/owner-thread termination. Separate injected-Unhook-failure policy tests use real handles: no/live/invalid process refused, naturally exited owned child accepted. Runtime6 tests PASS, retaining cleanup-pending rejection.

run6 modified-DLL fixture8checks PASS, including same editor-first shutdown and actual app restart/API setup absent. App unchanged from63ddc8d; target DLL replaced and manifest explicitly records fixtureOverrides/original hash. Original package and BUILD_RECEIPT preserved. This is not unmodified-package acceptance. Installed root, test registration, processes and user data removed by uninstaller; pending session shutdown recorded separately. Evidence D:/dev/grammar-evidence/grammar-api-installed-20261008-63ddc8d/{run1..run6,build-receipt.json,package-result.json,hook-red.log,hook-policy-final,target-runtime.log}. Same-integrator review only. Next fresh clean-source package with this fix and unmodified installed repeat, then remaining default Gemma/prototype work. Sandbox foreground repeat remains unresolved; fresh-session success is not its repair. MissionINCOMPLETE,no main/push/release.


# Unmodified rebuilt API trial package — 2026-10-08

Product177bb863759dfefcd14a4f410e2b263b4a9bf7c6 clean release/build-receipt and schema3 package PASS. Fresh owned Sandboxe27d5385, Medium integrity, external network and host clipboard redirection disabled. Unmodified installed app/DLL8checks PASS, fixtureOverrides=null: real settings save/no connection/key absent from status, removal, real shortcut capture, per-request decline/no connection, explicit loopback TLS attempt/Cancel/socket closure and exact draft/source retention, Copy with exact guest clipboard reread, real restart with no API setup. Both app exit codes0, editor exit0; editor is deliberately closed before app exit to cover the new regression. Harness additionally records/asserts exit codes; this harness-only strengthening postdates product177bb86.

Uninstaller complete, all four remaining fields zero/false. Independent guest query confirms no install root, owned app/host/broker/editor processes or task registry children. Owned Sandbox stopped; final inventory empty. Original63ddc8d failures and modified-DLL diagnostic/repair trials remain preserved separately. Final binary is not the substituted-DLL fixture. No claim that foreground repeat unreliability is fixed.

Evidence D:/dev/grammar-evidence/grammar-api-installed-20261008-177bb86/CHECKPOINT.json and output/{app-result,guest-result,uninstall,independent-cleanup}.json. App SHA25634047460def3a01c56fcb508e7facaa12190afd551192b05bb696fd8a60fbf50; target DLL80034a489bfa535ed3f2a940c478da663a8ee0286880efd8d3b34a9da33c8c8f; ZIPa267152f3bfb0bcf5727858adc2e02091cca632656d2642607a610e9d18f347d. Verified ZIP copy is package.zip in that evidence directory. Original package directory remains in grammar-api-installed-20261008-63ddc8d/payload/package-final as package-result.json records.

Scope: installed setup/consent/cancellation/Copy/restart/cleanup acceptance only. TLS is intentionally never completed by the loopback fixture; no application request body or key is sent over that incomplete connection. Successful API result/review remains production Rust synthetic HTTP evidence, not successful installed HTTPS/provider acceptance. No certificate/security relaxation, real keys, paid API, model/quality/latency or new Apply qualification. Default Gemma remains off; next finish default launcher/prototype qualification while preserving prior model quality, and separately qualify valid HTTPS API responses. This checkpoint does not authorize main adoption/push/release and mission remainsINCOMPLETE.


# Managed launcher missing-assets recovery — 2026-10-11

Base9e09c1c. Managed qualification option now starts the integrity-checked app even when model/runtime files are absent. Each explicit attempt rechecks existence before allocating a broker. No allocation permits a fresh retry ticket without pretending cleanup occurred; actual broker failures still require confirmed exit/drain/cleanup before retry. Missing files after a click issue a new ticket. Preparing assets alone does not start the broker or replay any capture. Closing unresolved missing-assets state returns failure, removes the owned session and releases installation ownership. Unmanaged launch and package-integrity refusal remain unchanged. No quality/model/deadline change or default activation.

Verification uses production PowerShell launcher and Prepare-GrammarLocal with owned Windows stub executables and offline synthetic asset catalog/cache. It covers absent entire local directory, premature retry, atomic preparation while the same app stays open, explicit ready transition, sidecar restoration and process/session cleanup; separate cases cover missing-model exit/relaunch and tampered package rejection. Full launcher suite24PASS/0FAIL (48.771s), governancePASS and git diff --checkPASS. This is not real installed-widget draft acceptance or actual Gemma inference. Initial baseline red and intermediate implementation failures preserved in D:/dev/grammar-evidence/grammar-missing-assets-20261011. Same-integrator review only. No Sandbox session changed, live Provider, remote push, main adoption or release.

Next: qualify the missing-assets preparation/Retry flow in the source-bound installed widget, then finish default Gemma activation and translation/rewrite wiring. Existing installedAPI successful HTTPS-response acceptance, foreground repeat reliability, real model startup/quality/latency limits remain separate. MissionINCOMPLETE. Project Source not regenerated/applied for this checkpoint; prior exported snapshot is historical relative to this change.


# Missing-assets installed UI harness prepared, guest NOT_RUN — 2026-10-11

Base6b32f3d. Added --missing-assets to the existing managed installed-widget harness: expect initial failed banner, capture/edit draft, explicit offline Prepare-GrammarLocal cache installation, assert preparation itself creates no runtime, click actual Retry, verify ready/draft/source and exactly one warmup/zero replay, then natural runtime/session cleanup. Receipt now records missing-assets mode and declared manifest overrides; a launcher override cannot claim unmodified-package acceptance. Existing normal startup mode retains two warmups. Node syntax checkPASS; guest executionNOT_RUN. These are assertions prepared for execution, not observed UI behavior.

Attempted command to prepare external fixture payload and start configured Sandbox was rejected by tool policy (blocked by policy, no further reason). No payload directory was created. Do not reroute that rejected operation to bypass policy. A preceding WindowsSandbox.exe --help unexpectedly opened a default guest (owned process creation observed at13:20); that newly created guest eac13464-d40a-4728-aba5-8eea23109f57 was stopped without test/data/account operations. Final wsb inventory empty. No user session was terminated. No installed widget or model acceptance added, no default activation, no main/push/release. Next execute the prepared guest test only when this tool-policy boundary is resolved; retain offline synthetic fixture and explicit binary/launcher source provenance. Project Source unchanged; missionINCOMPLETE.


# Missing-assets installed widget recovery verified — 2026-10-11

Owner requested diagnosis/resolution of the prior blocked command and continued Sandbox testing. Prior rejection was pre-execution tool policy, not a Windows Sandbox failure; detailed reason unavailable, so no internal cause is claimed. Under that renewed request, separate scoped file preparation and isolated wsb start were allowed. No policy/security/permission change or elevation bypass. Network/vGPU/clipboard/audio/video/printer disabled, ProtectedClient enabled, payload read-only and separate output writable. Initial ExistingLogin failed0x80070520 before guest logon; connecting the owned session allowed the existing MediumLaunch restricted-user runner. No System execution.

Installed fixture uses unchanged product177bb86 app/broker/target DLL with launcher6b32f3d and an explicitly synthetic offline asset catalog. Byte comparison confirms only Start-Grammar.ps1, Gemma-Assets.json and MANIFEST.json differ from the preserved package. Harness32f23b2. This is declared fixture qualification, not an unmodified clean build of latest HEAD.

Eight installed checks PASS: package hashes, visible widget despite missing assets, actual owned shortcut capture/edit draft, missing-assets Retry, production offline Prepare-GrammarLocal with no runtime auto-start, real Retry to ready with exact edited draft and source retained, exactly1warmup/0user inference or replay, natural app/runtime/session cleanup. Source re-read unchanged. No linguistic inference/quality/speed claim. Guest wrapperPASS, uninstallercomplete/allremaining0, independent root/process/registry0. Owned Sandboxde16338b stopped and inventoryempty. stderr tray-icon removal/WebView unregister1412 diagnostics retained; they did not prevent natural exit or verified cleanup, and are not claimed fixed.

Evidence D:/dev/grammar-evidence/grammar-missing-assets-ui-20261011/CHECKPOINT.json, FIXTURE_OVERRIDES.json, output/{app-result,guest-result,uninstall,independent-cleanup}.json. Earlier policy-blocked NOT_RUN remains historical, now superseded only for this scoped test. Same-integrator review; no independent reviewer. Next clean-source package qualification and default Gemma/prototype integration, preserving pending desktop translation/rewrite and successful HTTPS API-response acceptance. MissionINCOMPLETE; no main/push/release, Project Source unchanged.


# Default managed desktop grammar candidate — 2026-10-11

Owner requested current-source package verification and default Gemma connection. Start-Grammar now defaults EnableDesktopGemma to true, with explicit false preserving browser-only startup. Exe-direct startup does not create a managed Gemma connection; user guide now directs users through the launcher. Desktop grammar only: translation/rewrite wiring is not promoted. Model/runtime hashes, quality guards, consent, source binding, deadlines and explicit Retry remain unchanged. Installed harness now launches without the old opt-in flag. Clean-source package/installed qualification is pending at this implementation checkpoint.

Old tests expecting browser-only behavior now explicitly select false. Crash cleanup test waits for the observable failed state rather than assuming app-start plus800ms implies broker exit (app now starts before broker). Initial two failed assumptions preserved in gemma-default-launcher-test.log; final run recorded separately. No package/release/main acceptance from default parameter alone.


# Current-source default grammar package qualification — 2026-10-11

Clean product540d6669f6d14edaf809262b396108a0742d015d release build (frontend/typecheck, app, static-CRT host/broker, target DLL) and schema3 source-to-artifact verification/package PASS. Full launcher24PASS. Default launcher enables managed desktop grammar without extra flags; explicit false keeps browser-only connection. Exe-direct launch remains unmanaged. Translation/paraphrase local desktop integration remains incomplete, not silently mapped to grammar. No model/quality/deadline reduction.

Unmodified package installed in owned Medium Sandbox: four startup checksPASS (hashes, widget with missingassets, default failed/retry state, close/error1/session removal); installer/uninstallercomplete. Raw harness generic scope label includes capture but this test made NO capture/draft claim; actual check array and CHECKPOINT narrow it. Follow-on synthetic catalog test in same guest failed before capture on owned foreground acquisition, preserved run-fixture-focus-fail. Fresh guest same test8PASS: actual capture/edit, production offlineprepare no autostart, Retry ready/editeddraft+sourcepreserved,1warmup/0replay, natural cleanup. Only catalog+manifest differ from source package; app/host/broker/launcher/DLL unchanged. New default switch itself is tested by launching without arguments. This is installed wiring/cleanup evidence, not real-model inference/quality/latency acceptance.

Nested mappings initially returned E_ACCESSDENIED; flat read-only payload + writable output worked without security relaxation. Both owned guests stopped; final inventory0. Uninstaller allremaining0 and independent root/process/registration0. Build warnings and tray/WebView shutdown diagnostics preserved; no claim of repair. Foreground repeat variability remains unresolved. Same-integrator review only.

Artifact D:/dev/grammar-evidence/grammar-default-package-20261011/payload/package-pristine.zip; ZIP24097b8a72d0f8479dcecec4530b1f78a1d6f8713b0ab1d58185ed3b5ff391b7. Evidence CHECKPOINT.json/build-receipt.json/package-result.json/FIXTURE_OVERRIDES.json and run-pristine,run-fixture-focus-fail,run-fixture-pass. No current source or launcher overrides in the product package. Docs evidence commit may follow this exact product source; do not mislabel that docs HEAD as binary source.

Default grammar wiring/package checkpoint complete. Remaining prototype work includes desktop Gemma translation/paraphrase integration, successful installed HTTPS API-response qualification, and actual-model installed use/quality/latency limits; broad editor acceptance remains incomplete. MissionINCOMPLETE; no main/push/release. Project Source not regenerated; prior defaultoff claims are historical after540d666.


# Desktop local writing intent integration candidate — 2026-10-11

Default Gemma capture now dispatches exact Grammar/Natural/Concise/Polite/Translate intent. English/Korean translation supported; Auto/Japanese/Chinese explicitly unavailable, with draft-preserving UI guidance. Concise/Polite have strict broker protocol variants and purpose-specific prompts; literal/modality protection retained. Mode/target language are captured and checked at admission/publication, encoded in cache identity; existing settings invalidation and source/term binding retained. No fallback, replay or automatic mutation. Conservative protected-span envelope may refuse transformation spanning protected terms; not a claim of unrestricted translation coverage.

Desktop full271PASS/31ignored; local Gemma34PASS/10ignored; focused desktop11PASS/1ignored. Installed five-intent fixture and fresh source package pending. New runtime fixture uses fixed synthetic replies, not linguistic quality evidence. Same-integrator review only; no new real-model quality/latency or external Provider acceptance, no main/push/release. Project Source not regenerated.


# Installed desktop Gemma writing-mode integration — 2026-10-11

Product55fae4997a7bf9539ffe95f141c1d37e9fc031f2 rebuilt clean with schema3 source receipt in grammar-writing-package-r2-20261011. Initial93f8f5d build failed on new enum variants missing from Chromium label match; corrected and host28PASS/1ignored. Desktop271PASS/31ignored, Gemma core34PASS/10ignored, frontend/typecheck/release/package verificationPASS. Product package is pristine; installed test modifies only asset catalog and manifest for fixed synthetic runtime. No app, host, broker, launcher or DLL substitution.

Owned Medium Sandbox run3 thirteen checksPASS. Existing startup/capture/draft/offlineprepare/Retry7 checks, then Natural/Concise/Polite/Translate-Korean/Translate-English: real shortcut capture -> mode-specific broker reply -> visible candidate -> user-edited draft -> actual Accept -> exact native source reread. Each Accept adds0inference. Natural app/editor/runtime/session cleanup adds final check. Raw report generic scope label is inherited; actual check array and CHECKPOINT identify the five Accept cases. This proves production wiring with synthetic replies, NOT actual model quality/latency or live Provider acceptance.

Failed run1 retained: test runtime did not reach entry, imports included VCRUNTIME140.dll; static CRT fixture rebuild (no guest DLL/security install) starts correctly. Failed run2 retained: synthetic polite response added would, violating unchanged modal-preservation policy; broker stopped, no candidate or mutation. Positive fixture now uses compatible polite wording. No policy relaxation. Polite/Concise may refuse valid transformations that alter modal words; literal, protected-term envelope and line guards remain conservative. Only explicit English/Korean translation supported; other targets fail with preserved draft and UI guidance.

Combined fixture-preparation/start command was rejected by tool policy with no detailed reason. Staged preparation, reviewed two-file diff/hashes and unchanged network/clipboard-off configuration, then independent start were allowed. Do not infer internal rejection reason. All three owned Sandbox sessions stopped. Final uninstallercomplete/allremaining0, independent process/registration/installroot0 and final inventory0. Existing tray/WebView shutdown diagnostics retained, same-integrator review only.

Evidence D:/dev/grammar-evidence/grammar-writing-package-20261011/{CHECKPOINT.json,FIXTURE_OVERRIDES.json,run1-runtime-start-fail,run2-modality-refusal,run3-writing-pass}. Package payload/package-pristine.zip SHA256bb2f727d7f8eacc072da547e40f46e10edd4afdb9483c3c4769b5b832e76d115. Final harness changes only correct synthetic response/build instructions, not product binaries. Current supported desktop writing connection checkpoint complete; actual model installed quality/latency, remaining translation languages, successful HTTPS API-provider responses and broad editor acceptance remain. MissionINCOMPLETE,no main/push/release; Project Source unchanged/stale for this feature.


# Actual Gemma CPU Sandbox writing trial — 2026-10-11

Basef35d32e, unchanged product55fae49/pristine package. Official12B Q4_0 model and pinned CUDA12 runtime. Fresh24GB offline/vGPU-disabled Sandbox is CPU evidence, not hostCUDA. Initial mapped startup failed; local-asset preparation exposed missing VC runtime (version probe exit0xC0000135). Microsoft-signed Visual Studio14.50.35719 CRT from redist directory supplied only on task process PATH; same runtime version then exit0. No system install/settings change; package dependency provisioning remains incomplete, now documented in PERSONAL_USE.

Realmodel ready observed26-38s. Runs3-6 failed owned foreground/cursor fixture (SetCursorPos error2); old generic occluded label was too broad, split in new owned-click helper. Verified target image/field/window-at-point before clicks; no product target checks changed. Explicit viewer reconnect with5s startup wait allowed real captures in run7/8. Concise/polite local analysis failed; final Hello translation fails15.138s after shortcut, exact source reread unchanged. Generic event does not expose internal reason: timing is consistent with15s deadline, not proof of every root cause. NO generated candidate Accept or Accept latency qualified. Historical failures retained.

Separate raw diagnostic, same production prompts/JSON schema/seed/temp/max256, declared120s wait only for observing complete outputs: Natural78.43s, Concise44.83s, Polite73.41s, EN-KO81.90s, KO-EN69.07s, four-sentence translation81.23s, four-sentence Natural76.85s. All7 outputs finish stop/validJSON. n7 nearest-rank p50=76.85s,p95=p99=max81.90s; all1s/3stargetsmissed. HTTP timing excludes app capture/publication, not end-to-end product timing. No productdeadline/guard/model change. Same-integrator manual review finds facts/negation/numbers/deadlines preserved on these cases, mild register drift in Natural; Polite adds Could and conflicts with existing modal guard. Raw outputs bypass product postvalidation and are not product acceptance or broad quality proof.

Diagnostic artifact reports runtimeExited=false because old helper checked exitCode after signal termination; preserve it. Source helper now observes exit event. Independent final check provesprocesses0/installrootabsent/registrationchildren0/taskassetsabsent; guests stopped and finalwsb inventory0. Existing tray/WebView stderr warnings not repaired. No live externalProvider, main, push, release or Project Source regeneration. Evidence D:/dev/grammar-evidence/grammar-real-writing-20261011/{RESULT.md,CHECKPOINT.json,CRT-PROVENANCE.json,output2/raw-quality.json,output2/run8/app-result.json,output2/independent-cleanup.json}; exact executable/harness hashes included. Next: CRT provisioning/admission, GPU-capable isolated actual-inference qualification, polite prompt/guard alignment without weakening protection, then installed realcandidate Accept. MissionINCOMPLETE.


## GPU writing / CRT follow-up, 2026-10-11 — candidate, execution blocked

Entry e1b64ae; current changes are not an adopted or newly installed package.
Private Microsoft CRT packaging verifies release-toolchain signatures and adds
manifest-bound DLLs only to the owned broker PATH. Missing/changed/extra-file
regressions were added. The polite prompt now requests an imperative Please form
without new modal verbs; the old Could response and lost obligation/negation remain
rejected. No safety guard or performance target was relaxed.

PASS: cargo check --locked --offline --lib and --tests, Python test-file syntax, governance
self-test, diff whitespace. BLOCKED: Rust unit execution at native link, launcher
regression setup, PowerShell trivial output, Sandbox inventory. These commands
stalled without a test assertion outcome. The common cause is unknown. No security,
driver, account, user-profile or machine-wide runtime change was made.

GPU hardware is RTX5070Ti16GB; the entire 12B file was hashed and matched the pinned
93567e57...538b value. The raw task-owned host probe is not an isolated guest or
installed Accept test. Read its terminal evidence before claiming any inference
or cleanup result: D:/dev/grammar-evidence/grammar-gpu-writing-20261011/RESULT.md.
No new source-to-package chain or Gemma-generated Accept is qualified.

Resume after the Windows execution environment responds: run the four new private
CRT launcher tests and full relevant launcher suite; run local_gemma Rust tests;
build from a clean committed source and package with release CRT; verify CRT-free
guest launch, actual GPU offload, all writing modes and exact source reread after
explicit Accept. Keep host-only inference and guest integration evidence separate.
Do not repeat a blocked process start indefinitely or kill unrelated workloads.
Main/push/release and Project Source are unchanged; mission remains INCOMPLETE.


## Recovered GPU-capable Sandbox actual writing / Accept checkpoint, 2026-10-11


Candidate source ceb1f7c970b1a6a4961f7ec9d2d668e41c0fc81b; clean build receipt build-receipt-r2.json.
Package payload/package-final.zip SHA256 5baac902b0f7b15ab8647d15b67c60af975b451e055242eec764fd78905644bd. Unmodified installed package; no fixtureOverrides. Main/push/release not performed; mission INCOMPLETE.

## Final installed trial
Windows Sandbox vGPU Enable, networking/clipboard disabled, task-owned RichEdit synthetic editor. CUDA0 RTX5070Ti was enumerated by the guest runtime (guest/devices2.txt). Actual pinned Gemma4 12B IT QAT Q4_0; no synthetic model response or cloud call. Runtime and model catalog pins unchanged.
All seven native shortcut -> actual candidate -> explicit Accept -> exact editor reread cases passed. Output was accepted unedited. App and editor exited naturally. No extra analysis event during Accept; this is not a new server-request-count measurement.

| Case | Shortcut-to-candidate ms | Accept-to-reread ms | Result |
|---|---:|---:|---|
| 1 natural -> en | 1115.1 | 149.0 | PASS |
| 2 concise -> en | 898.0 | 149.6 | PASS |
| 3 polite -> en | 802.7 | 132.8 | PASS |
| 4 translate -> ko | 986.7 | 152.1 | PASS |
| 5 translate -> en | 803.4 | 170.5 | PASS |
| 6 translate -> ko | 1235.8 | 122.9 | PASS |
| 7 natural -> en | 1113.7 | 167.1 | PASS |

Recommendation target1s: short Natural1115.1ms misses; other four short cases pass. Four-sentence translation1235.8ms and Natural1113.7ms meet3s. All Accept times meet500ms. n7, one sample per case, not robust percentile or broad performance acceptance. Sources/outputs/events in guest/app3/app-result.json.
Fresh local asset preparation 381.93s; app-start-to-ready 23.86s, measured separately. Not cold-boot measurements. Startup/preparation remain material usability issues. This repeat ran after the host build; the earlier loaded trial is preserved, not discarded.

## GPU and quality evidence boundaries
Host run3 proves49/49 layers offloaded to RTX5070Ti,7 actual responses438-813ms (HTTP only; p50=578ms, nearest-rank p95/p99=813ms), runtime exit and port closure. This is separate from guest/UI evidence. Guest GPU-memory counter returned no matching instances (GPU_ALLOCATION_NOT_OBSERVED); do not claim this measured process GPU utilization or guest offload count. GPU-capable guest device enumeration and actual installed timings are confirmed; host offload diagnostics must not be relabeled guest diagnostics.
All seven installed outputs equal the host outputs. Manual review in QUALITY-REVIEW.md preserves facts, numbers, prohibitions, obligations and deadlines on this small fixed set; Natural has slight formal-register drift. Same-integrator review, no independent human quality review or broad language guarantee.

## Repairs and checks
9fa23cf: signed Microsoft release CRT privately packaged, manifest hashed and pinned, broker-only child PATH. No system install or global PATH change. Polite prompt requests Please for imperatives; modal/negation/obligation guards unchanged and old Could result still rejected.
ceb1f7c: installer copies and verifies private CRT files. Launcher fixture retries transient atomic-read access errors within its existing deadline and keeps positive-test app alive until readiness. Assertions/product deadlines unchanged.
PASS: Rust local_gemma35 (10ignored); cargo check lib/tests; launcher28; installer prerequisite4 (including private CRT copy); PowerShell syntax; governance/self-test; clean release build/package. Existing library warnings remain. Final guest system vcruntime140.dll absent before/after yet model and app work using private files. Uninstall and independent check: owned processes0, install root absent, registry children0, user data0. Owned Sandbox stopped; final inventory empty.

## Preserved failures / limits
Initial host load timeout and Windows process-start stalls; run2 diagnostics insufficient to prove offload; old package app1 omitted private CRT during installation (repaired). App2 declared installer-only fixture:3 Accept passes,3 pre-capture focus-helper timeouts,1 reconnection timeout; stale failureMs in that harness is NOT inference latency. First recommendation14.212s and ready141.991s remain recorded. App3 resets capture timing each case and records pre-capture phase separately; all7 pass. Environment-stall root cause remains unknown; no reboot/security/driver relaxation used.
NOT_RUN: general profiles/documents, other editors/adapters, physical keyboard/IME, live external Providers, broad quality/endurance and reproducible binary builds. The requested seven-case isolated actual-model flow is verified; whole product and1s-all-cases acceptance are not.
Next: diagnose preparation/startup/first-request variability without weakening quality or safety; broaden bounded actual-model corpus and latency repetitions. Existing wider mission tasks remain.

Evidence: D:/dev/grammar-evidence/grammar-gpu-writing-20261011/{RESULT.md,CHECKPOINT.json,QUALITY-REVIEW.md}. Initial blocked reports retained as *-initial; this checkpoint supersedes their current-status summary only.


## 2026-10-11 — startup and first-response checkpoint

Startup/first-response checkpoint c0bcbda/2fd535a: same Gemma12B Q4_0 and quality/safety guards; optional SHA-pinned CUDA13.4 Blackwell package profile (default CUDA12.4 unchanged). Fresh Sandbox ready107.924s vs14.160s, first actual desktop response0.907s;6 Accept+reread PASS, seventh exact-byte difference FAIL retained then bounded equivalent polite-ending review. Separate raw GPU3 restarts x7=21PASS (18 identical/3 reviewed equivalent),425.7-924.6ms,49/49 offload and cleanup each; NOT repeated product Accept. Asset9 regressions PASS; selective extraction no measured total speed win, slower streaming prototype rejected. Clean2fd535a build/both profiles PASS; unmodified CUDA13 install/real assets/hash/remove PASS. Product repeat Accept blocked before capture by owned foreground; owner activation pending, no guards weakened. Main/push/release unchanged, mission INCOMPLETE. Evidence D:/dev/grammar-evidence/grammar-startup-repeat-20261011/RESULT.md and CHECKPOINT.json.

Model size/quantization, token budget, prompt, seed, warmup, deadlines and meaning/source/Accept guards are unchanged. CUDA13 evidence is RTX5070Ti driver596.49 offline vGPU Sandbox only; first-start comparison is one fresh guest per runtime, not a broad compatibility or cold-host benchmark. First-run CUDA12 response9.139s was from a later trial, not the107.924s ready trial. Raw21 is runtime evidence, not production routing/Apply acceptance. The equivalent Korean polite wording is two exact full outputs reviewed by the primary agent, not a loosened product quality filter or independent human assessment. All original red/focus tests remain external.

Asset placement is still about24-32s from an existing SSD cache; download time NOT_RUN. Preparation diagnostics distinguish copy/hash/extract. Do not claim a total extraction speedup: paired medians30.314s baseline vs31.139s selective. Streaming copy/hash was slower and removed. Host-only warmup alternatives showed no compelling benefit and were not adopted.

New package source2fd535a has clean release build, two catalog/profile checks, and unmodified CUDA13 install/private-CRT+binary hashes/actual asset preparation/uninstall evidence. Installed UI measurements used unchanged ceb1f7c app binaries with explicit preparer/catalog fixture changes; latest package UI remains pending. Sandbox is idle after removal, no owned model/app/editor remains, and no background continuation is promised. User was asked once to activate its foreground. Next: after that input, repeated latest-package medium app capture -> actual candidate -> explicit Accept -> exact reread; retain all source/foreground guards. Full Rust suite and browser regression not rerun this turn; no Rust/browser implementation changed. Current Source package is materially stale and has not been regenerated/uploaded.


## 2026-10-11 — serial fresh Sandbox installed Accept qualification

Owner-activated continuation: old Sandbox still refused foreground in2 before-capture tests (preserved). Changed operation to one VM/one viewer with verified complete stop before replacement. Unmodified2fd535a CUDA13 package3 fresh serial GPU guests x7=21 actual Gemma candidate/explicit Accept/exact reread PASS; unchanged12B Q4_0/settings/guards,18 byte-identical+3 previously reviewed polite alternatives. Natural app/editor exits, launcher cleanup, complete uninstall and VM/viewer shutdown each. Same-session focus issue and crash root cause unresolved; no all-editor claim. Evidence D:/dev/grammar-evidence/grammar-startup-repeat-20261011/ACCEPT-REPEAT.md. No main/push/release; mission INCOMPLETE.

Operating rule from Owner: do not accumulate Sandbox sessions or connection windows. Keep one healthy session, or completely stop its VM and owned viewer before creating the next. Verify no prior app/model/editor before each run, use an exclusive trial lock, and stop on failure. The three fresh guests passed; two attempts in the old activated guest did not. Default input desktop was available, so unavailable desktop was not established as the cause. No crash attribution is asserted. Final VM/viewer count0; no pending owner activation or background trial for this completed checkpoint.

Measured source2fd535a package values and exact scope are in ACCEPT-REPEAT-CHECKPOINT.json; the previous CHECKPOINT.json is historical. Model preparation and first-response latency are separately measured. This updates installed actual-model synthetic qualification, not whole-product acceptance or general Editor coverage. Product source unchanged in this continuation; governance/diff checks accompany this documentation commit. Project Source regeneration/upload remains pending.


## 2026-10-11 — Chromium actual Gemma personal package

Chromium actual Gemma personal package 4e28924: CUDA13.4 default (explicitCUDA12.4 retained), unchanged12B Q4_0/settings/guards. Clean build/default-profile/catalog/ZIP provenance PASS. Unmodified installed extension/host in single offline GPU Sandbox, actual keyboard activeTab:14 translation/paraphrase cases across textarea/simple contenteditable (2 paragraphs), explicit Accept/edited Apply and exact reread PASS; bounded semantic review PASS, fixed latency targets no misses. Dirty/stale/mode/sensitive/disable/navigation/cache-no-inference checks PASS, privateCRT and complete app/browser/host/profile/uninstall/VM/viewer cleanup. First fixture PowerShell5.1 policy FAIL preserved; used existingPS7 without policy weakening. Scoped RTX5070Ti/Chromium147 synthetic evidence only; no live Provider/main/push/release, mission INCOMPLETE. Evidence D:/dev/grammar-evidence/grammar-chromium-gemma-20261011/RESULT.md.

 textarea7, simple contenteditable7; English/Korean translation and paraphrase, including2 four-sentence cases. Explicit Accept13 and edited Apply1, exact content reread. Short response {'n': 12, 'min': 397.8999999999942, 'p50': 516.5499999999884, 'p95': 794.7000000000116, 'p99': 794.7000000000116, 'max': 794.7000000000116}ms; paragraph {'n': 2, 'min': 883.2000000000116, 'p50': 885.0500000000029, 'p95': 886.8999999999942, 'p99': 886.8999999999942, 'max': 886.8999999999942}ms; Apply/Accept {'n': 14, 'min': 23.39999999999418, 'p50': 28.10000000000582, 'p95': 29.20000000001164, 'p99': 29.20000000001164, 'max': 29.20000000001164}ms. Latency measures Review locally click through completed suggestion publication, not an arbitrary typing/debounce benchmark. Fixed1s short/3s paragraph/0.5s Apply targets, no misses. Model readiness 12320.1ms separately. Semantic quality reviewed by primary agent; finite synthetic corpus, not independent human/general language acceptance.

Dirty draft blocks mode change; in-flight mode switch discards late result; sensitive focus does not reuse old field; source changed at Accept mousedown remains exactly unchanged; cache/Accept emits no new analyze request. Disable clears UI/cache; cross-origin navigation revokes access; same-origin reload drops document opt-in (Chromium retains activeTab). No document text in extension storage. Browser/host/launcher exit and profile/uninstall cleanup complete; VM/viewer both0.

Preserved first-run FAIL: Windows PowerShell5.1 blocked the fixture .ps1 by its default execution policy before toolbar/model review. Fixture changed to existing PowerShell7; no execution-policy/security setting was weakened. R2 on prior2fd535a package passed12 cases, separately reviewed. Final new package passed14 cases. Node wrapper bounds420s account for two toolbar helper bounds and14 cases; product deadlines unchanged.



Personal ZIP source4e28924 has default CUDA13 profile and corrected guide. The record commit follows the tested package; do not label the ZIP with that later documentation commit. VALIDATION.md and ZIP.sha256 sit beside the archive. Preserve one-VM/one-viewer policy with stop/close before replacement. No Sandbox or test remains running. Full Rust suite not rerun (no Rust/browser product implementation changed); clean release build, real installed UI and governance checks are separate evidence. Current Project Source is stale, not regenerated/uploaded this turn.


## 2026-10-11 — Personal host installation and explicit low-GPU option

2026-10-11 Owner-authorized personal host installation: source dffcf8a at C:/Users/USER/GrammarPersonal; unchanged12B Gemma default plus explicit model-free startup and isolated profiles. Rust146+273 and launcher28+2 PASS; clean build/package/installed hashes verified. Actual host Chromium14 cases PASS with real model, pinned toolbar mouse and trusted actual-popup input, exact Accept/edited Apply reread and privacy boundaries. Desktop PARTIAL:5/7 PASS; first capture timeout and paragraph Accept refusal/source unchanged remain unproved; later foreground refusal and bounded Owner-focus wait expired, normal cleanup. MSIX-redirected initial install removed, unpackaged same-user install verified; personal settings/dictionary unchanged. Low-GPU no-model startup PASS, live Deep/CPU/smaller-model quality NOT_RUN. No main/push/release; mission INCOMPLETE; Project Source stale. Evidence D:/dev/grammar-evidence/grammar-host-personal-20261011/RESULT.md.



Unmodified installed extension/host; fresh dedicated profile, browser sandbox on.
Real toolbar mouse click grants activeTab; real action popup's Enable button receives
CDP trusted mouse input. The popup is not exposed as an ordinary Playwright page.
No programmatic permission grant, injected host permission or synthetic model response.
Raw receipt's legacy `extensions-menu` path label is inaccurate for this run: the helper
clicked the pinned action button directly. Keyboard and menu paths remain unqualified.

14 cases:textarea7/simple contenteditable7,13 Accept +1 edited Apply, exact DOM reread.
All14 replacement strings equal the previously individually reviewed finite corpus;
this is primary-agent bounded semantic review, not independent human/general language acceptance.
Short12:355.6–644.6ms (median454.4,p95/p99644.6); paragraph2:739.4–786.4ms;
Accept14:14.4–15.3ms. No fixed1s/3s/0.5s misses. Review-click to complete suggestion is
measured, not arbitrary whole-document scanning or typing/debounce performance.
Dirty draft, stale source, in-flight mode, sensitive focus, cached-no-inference, Disable,
navigation/reload and storage checks PASS. Browser/host/profile cleanup complete,
extension unchanged, managed launcher exit0.

## Desktop PARTIAL — do not call this all-green

Original unpackaged-host run7 cases:5 PASS with real Gemma and exact native RichEdit reread.
Successful recommendation573.5–977.7ms; Accept119.7–145.4ms. First shortcut capture timed
out. Paragraph translation was semantically approved (1092.3ms), but Accept left original
source unchanged and the exact reread wait timed out. No wrong replacement was observed.
The reason for those two failures was not proved; do not attribute them retrospectively
to focus alone. Later trials actually observed Windows foreground refusal before input.
An owned-review activation check and capture diagnostics were added to the external fixture.
The final180s user-activation wait expired; no input was sent, app/editor/model exited
naturally and their remaining process count is0. UI reactivation is needed to finish this
qualification; no automated continuation remains running.
Shutdown logs include WebView class-unregistration/tray warnings; exits and ownership
cleanup were separately checked. No claim that the warning root cause was repaired.

## Preserved environment/fixture failures

First light test read the WebView page before creation; bounded page discovery fixed it.
Sandbox portable PowerShell timed out before desktop app creation; host PS7 was used.
Exact image guard exposed MSIX AppData redirection to Codex LocalCache. The earlier
installation was removed without RemoveUserData. New installation used an unpackaged
same-user process (GetCurrentPackageFullName=15700), at C:/Users/USER/GrammarPersonal.
Windows documented process-tree policy was used; no elevation, token/credential copying
or global policy change. Initial keyboard/UIA toolbar failures, then missing normal-page
popup failures, are retained. The successful popup-target method is classified separately.

Existing personal settings/dictionary hashes unchanged; no user documents, ordinary browser
profile, clipboard contents, credentials or real Provider texts were used as test data.
No Sandbox was started. Fresh personal profile and two launch shortcuts contain no test corpus.
Low-GPU startup is verified; equal-quality offline CPU/smaller-model execution remains NOT_RUN.
Mission INCOMPLETE; no push/release/main adoption. Current Project Source is stale.

## 2026-10-13 host desktop diagnostic repair (acceptance pending)

Owner requested one trial session at a time. Diagnostic copy reproduced an app UI hang:
owned rustc.exe --version remained under the app for minutes; terminating only that
verified version-only child changed Responding from false to true. The synchronous
check_prerequisites handler called unbounded Command::output on the window thread.
Managed CLI processes also lacked CREATE_NO_WINDOW; console descendants were observed,
but their causal link to every earlier capture/Accept refusal is not yet proved.

Move prerequisite discovery off the UI thread, reuse bounded version probe/Job/drain
lifecycle, resolve Windows commands without spawning where.exe, and prevent managed
CLI console creation. New native child-console test and hung-version test pass; the
existing native timeout/cancel cleanup test passes in task-owned TEMP. First console
test failed on an immediate Job accounting assertion; bounded exit observation fixed
the test without changing production cleanup budgets. No new dependencies.

Initial capture and intermittent Apply acceptance remain OPEN. Diagnostic run began
with two successful real-model native Accepts, then foreground refusal and UI hang;
it is not all-green. Runner timed out; owned hung probe was terminated, app was quit
through its command and remaining synthetic target closed by WM_CLOSE. Subsequent
inventory found no trial app/model/editor processes. Personal installation unchanged.
Release candidate build/retest pending; no main adoption, push or release.
Evidence: D:/dev/grammar-evidence/grammar-host-personal-20261011/resume-20261013.

## 2026-10-14 — owned desktop trial cleanup, capture acceptance still blocked

Added scripts/mission/owned_trial.py: one named mutex, suspended child assigned to
non-breakaway kill-on-close Job before resume, hidden console startup, bounded
execution and 10-second cleanup, process-handle completion plus Job accounting0.
Forced cleanup is reported separately and never upgrades a failing trial. Five
native supervisor tests PASS (normal/no-console, timeout descendant, abandoned
child, competing session, supervisor termination). Developer-only installed
pywin32/psutil are used; no product dependency or provider/model change.

The previous direct Cargo diagnostic build exited0 but left a child and used the
development URL. It was not accepted as an installed UI test. Corrected to the
repository Tauri build path: exit0, natural cleanup/Job0, 88.343s. Diagnostic app
source5796a8e, SHA25646317c56cea9eabca7dc4617992a35b8280e0bcdbfe44924dad9bb371ddb77f4.
This is an incremental mixed diagnostic copy with explicit overrides, not a clean
package receipt. Personal dffcf8a install remains unchanged. Metadata mismatch,
stale copied diagnostic endpoint and wrong build-mode failures are preserved.
No concurrent desktop trials: prior process trees stopped before each attempt.

Final trial: actual Gemma ready13.317s, app responsive, exactly one synthetic editor.
Owned visible-window metadata showed app/editor only, no visible Command window.
Windows denied automatic foreground; 180s Owner activation wait expired before
hotkey/capture. No new capture or Accept acceptance. App/editor exited normally;
launcher reported model cleanup. Supervisor caught/reaped remaining conhost and
two WebView processes, confirmed Job0. Independent app/model/editor inventory0.
Supervisor result FAIL (test exit1), cleanup COMPLETE, total207.906s. This verifies
aborted-session cleanup, not the first-capture or intermittent Accept repairs.

Outside launcher entry restored; no session or background continuation left active.
Next: one Owner-activated synthetic editor session on this corrected diagnostic
build, with redirected target numeric diagnostics, to resolve the still-open first
capture and intermittent Accept failures. Do not weaken target/revision guards.
No main/push/release. Evidence:
D:/dev/grammar-evidence/grammar-host-personal-20261011/resume-20261014/RESULT.md

### Follow-up: actual desktop success after Owner activation (2026-10-14)

Supersedes the preceding activation-blocked outcome only for the following trials.
The corrected diagnostic app5796a8e ran7 cases, then a separate serial session ran
7 cases twice (14). Both sessions had first-capture success; all21 actual Gemma
outputs matched the previously reviewed finite corpus and all21 explicit Accepts
passed exact RichEdit reread. Natural/concise/polite, EN/KO translation, and two
four-sentence samples are covered. No target/source/revision guard was relaxed.
Numeric target diagnostics show successful capture and applied status. The external
activation helper now checks observed foreground identity even if SetForegroundWindow
returns false; this is fixture behavior, not a product security bypass.

Known window-thread blocking prerequisite probe is repaired and did not recur.
The original intermittent native capture/Accept failure mechanisms are not separately
proven repaired:21 successes establish bounded diagnostic regression evidence, not
all-editor/long-run qualification. Installed personal dffcf8a remains unchanged.
No claim of unmodified personal-package acceptance from this mixed diagnostic copy.

Short recommendations15:697.49-1873.43ms (p50 1086.15),9 misses of1s. Paragraphs6:
1361.56-1617.35ms,0 misses of3s. Accept21:189.22-2143.55ms (p50 382.90,p95 949.40),
6 misses of0.5s. Startup ready13.515/16.928s separate. No quality/model downgrade.
Primary-agent comparison to reviewed corpus only; no new independent human review.

Both successful app flows exited0 with natural app/editor shutdown. Supervisor
correctly reports FAIL_LEFTOVER_CHILDREN, not PASS:2 descendants per run needed
forced Job cleanup. Final Job0, independent app/model/editor inventory0, diagnostic
Gemma endpoint removed, session JSON count0. No trial or automatic continuation is
running; outside entry restored. WebView unregister1412 warning remains unexplained.
Final supervisor5 native tests PASS after process-handle completion reporting fix;
basic governance/diff checks PASS. Full product suites not rerun for scripts/docs.

Next: clean package of the repaired app and unmodified installed regression; retain
initial native-intermittency causality and latency misses as open qualification
limits. Do not restart trials without the one-session supervisor and complete cleanup.
Evidence: resume-20261014/FINAL-SUMMARY.json, diagnostic-results/app-result.json,
repeat-results/app-result.json, run/supervisor-result.json, repeat-run/supervisor-result.json.


## 2026-10-14 — clean personal package installed; foreground-blocked requalification

2026-10-14 clean a0ae323 schema3 release/package installed at C:/Users/USER/GrammarPersonal, previous installation backed up, model preserved, settings/dictionary hashes unchanged. One supervised installed actual-Gemma session reached ready in26.851s, but180s foreground activation wait expired BEFORE_CAPTURE: translation/rewrite/Accept NOT_RUN, no source mutation; diagnostic21/21 not promoted. App/editor natural exit;3 owned conhost/WebView descendants forced-cleaned, final Job0/process0/sessionJSON0/endpoint absent. Build exit0 with1 leftover cleaned; package/install supervisors PASS. No further session running, no main/push/release;mission INCOMPLETE;Source stale. Evidence D:/dev/grammar-evidence/grammar-personal-candidate-20261014/RESULT.md.

Package product source stays a0ae323; subsequent evidence-only commit is not a rebuilt binary. No diagnostic-app files were copied into this package. ZIP c6d2cebfcbe847af080cb056ffc8fddd86e9bdba5e46ec98c9fa2a3ea790ffb0; extension ID lgbocafnjihoapddngogkmjfenhhppmd. Normal browser profiles untouched; reload/requalification is separate. New installed RichEdit capture/quality/Accept remain NOT_RUN, not PASS. The same retained seven-case fixture can resume only through owned_trial.py after confirming no active session; do not reuse an expired PID or create overlapping retries.


## 2026-10-14 — installed actual Gemma desktop activation retest

2026-10-14 Owner-activated installed a0ae323 single-session actual Gemma/RichEdit retest:7/7 capture+reviewed-output matches,6 exact Accept rereads PASS. First natural rewrite review-window activation refused BEFORE_ACCEPT; unchanged source, Accept NOT_RUN, whole fixture/supervisor FAIL preserved. No capture timeout or target Apply refusal observed in executed paths; original intermittent causality not proven. Startup14.805s; short624-1330ms(2/5 misses1s), paragraphs1045-2129ms(0/2 misses3s), Accept310-2277ms(5/6 misses0.5s). App/editor natural exit,3 owned descendants forced-cleaned; final Job0/process0/sessionJSON0/endpoint absent, personal settings and installed pins unchanged. Product/package unchanged; no overlapping/restarted trial, no main/push/release;mission INCOMPLETE. Evidence D:/dev/grammar-evidence/grammar-personal-candidate-20261014/retry-activation/SUMMARY.json.

Next: diagnose the first review-window activation failure and remaining Accept latency, without treating a helper foreground refusal as a rejected product mutation. Timing includes UI click and exact reread observation; no claim of isolated native mutation time. Do not weaken binding checks or hide the blocked case. Current candidate package remains a0ae323, no further process is running.


## 2026-10-14 — installed review placement repair and measured Accept

2026-10-14 installed792032b review placement repair: cursor-relative window is clamped to physical monitor work area; old placement5/5red,new5/5PASS. Clean schema3 release/package/install PASS with unchanged Gemma12B Q4_0 and settings. One supervised actual-Gemma RichEdit session14/14 captures,reviewed outputs,explicitAccept/exact rereads PASS; all14 edge-position windows contained. Native mouse7 Accept113-332ms,CDP7 66-267ms,DOMclick-to-reread; isolatedbackend timing NOT_OBSERVED. Startup16.164s,short687-1371ms(2/10 miss1s),paragraph987-1090ms(0/4 miss3s). App/editor natural exit but2 WebView children required ownedJobcleanup,supervisor FAIL_LEFTOVER_CHILDREN retained; finalobservedownedPID0/product0/sessionJSON0/endpointabsent/settingsunchanged. Earlier observer/foreground failures preserved, original intermittent causality not fullyproven. No main/push/release;missionINCOMPLETE,Source stale. Evidence D:/dev/grammar-evidence/grammar-accept-phases-20261014/RESULT.md.

The old cursor+18 placement did not constrain the review window to the physical monitor work area. observer-r2/window-geometry.json recorded the review rectangle [2192,1457,2628,1926] outside a [0,0,2560,1392] work area. The actual mouse helper correctly refused an OS-clamped offscreen coordinate. The product now chooses the cursor monitor and clamps the complete outer window to its work area, including negative origins. Pure regression cases: old formula5FAIL, new5PASS. Oversized-window placement alone does not qualify tiny displays; mixed-DPI physical use remains untested.

Windows can still reject a helper's SetForegroundWindow. That refusal is not a product Apply rejection. The fixture now uses a hit-tested actual mouse click to the owned target/review; exact executable/PID and coordinate checks guard the input. No screenshot, OCR, ordinary document or browser profile was used. All14 final edge-position windows were inside the work area.

## Installed actual model test

One final session:14/14 capture -> real Gemma candidate -> explicit Accept -> exact RichEdit reread PASS. Seven native mouse inputs plus seven CDP comparisons; fourteen outputs match the previously reviewed golden or previously reviewed equivalent output. No newly relaxed semantic guard/model/prompt/quality setting. Local analysis event count unchanged during Accept; this is not a network traffic audit. Initial owned click activation succeeded without another manual user request.

Native mouse DOM click to exact reread113.391-331.537ms,p50=223.120,p95/p99=331.537;0/7 exceed500ms. CDP66.417-266.646ms,p50=101.941,p95/p99=266.646;0/7 exceed500ms. UI Applied confirmation104.8-381.6ms. Small samples and100ms reread polling are observed upper bounds; OSinput-to-DOM and backend-only time are not independently measured. Helper startup/input delivery is recorded separately. Do not relabel past0.310-2.277s observations as fixed backend times: the prior sequential timing included click automation waits, and causality for each old slow case is not proven.

Startup16.164s. Short results686.652-1370.891ms;2/10 miss1000ms. Four-sentence paragraphs987.231-1089.952ms;0/4 miss3000ms. Capture306.116-1216.944ms; no capture timeout observed. Full distributions/methods in fixed/SUMMARY.json and raw fixed/desktop-results/app-result.json. This is bounded installed RTX5070Ti synthetic RichEdit evidence, not general editor, physical IME or Chromium requalification.

## Failures retained and cleanup

Initial unmodified-a0 diagnostic mistakenly tried to replace readonly Tauri IPC invoke: three exact target changes occurred but missing trace made the fixture FAIL. Never promote this run to PASS. The later observer-r2 native clicks refused offscreen coordinates; session was explicitly stopped and cleaned before building. These raw files remain. Pure old-placement regression remains red. The final14-case app fixture PASS is separate from supervisor FAIL_LEFTOVER_CHILDREN: app, model launcher and editor exited naturally, but WebView PIDs63292/12440 remained and the owning Job terminated them. Process handles signalled and JobActive0. No unrelated process was killed. The first independent summary assertion failed without individual predicate diagnostics; a second fresh independent check passed all predicates, not evidence of natural descendant exit. Final owned PID inventory0, product/model/editor0, session JSON0, Gemma endpoint absent, CDP closed. No background trial remains; expired focus request closed and outside launcher entry restored.

## Package and verification

Clean source/build receipt, package and upgrade supervisors PASS. Five installed app/host/broker/DLL/launcher SHA pins match the manifest; settings/dictionary hashes unchanged. ZIP SHA256 `74a1f84c61b22bce3f08c1d7c79bc6adb1a5efc5350d2494acf2cad5c5da1103`. Extension ID `hgpoihgaommeinejocjecbpclednapoc`; normal browser profiles untouched, any personal loaded extension needs separate reload/requalification. Previous package backup `C:/Users/USER/GrammarPersonal-backup-a0ae323-20261014` preserved, along with the earlier backup. Model/runtime restored unchanged. Evidence-only later commit is not a different rebuilt binary.

Placement unit5PASS, owned supervisor lifecycle5PASS, C#compilePASS, fresh production build/package/installPASS, basic governancePASS. Scoped self-review of monitor units, negative coordinates, target ownership and timing semantics; no independent reviewer this round. Next: natural WebView descendant shutdown and broader repeat/mixed-DPI qualification; maintain existing speed misses and historical native-intermittency uncertainty. Mission remains INCOMPLETE.


## 2026-10-14 — public prototype source preparation

Owner explicitly authorizes source commit/push to the existing public CAPTW/pencil repository on codex/grammar-local-trial-ba58b7d. Contract1.5.4, wrapper pins, validator and schema now admit only that additional exact branch; main/release/live-provider restrictions remain. docs/PROTOTYPE.md is the current public build/install/limitations entry point. The new hosted workflow checks governance and frontend build only; it must not be represented as GPU/native product acceptance. Installed product remains792032b, with14/14 scoped actual-Gemma desktop evidence and WebView natural-exit limitation retained. No executable/model/evidence payload is intended for publication. Full outgoing-history scan and remote identity receipt: D:/dev/grammar-evidence/grammar-public-source-20261014. Source UI package remains stale; no main adoption or release.


## 2026-10-14 — Prototype 1 release preparation

Owner authorizes safe main fast-forward and executable GitHub prerelease v0.1.0-prototype.1. Remote main fb988fe is an ancestor of the verified public candidate; no divergent history. Canonical main has three unrelated untracked audit/export files which must be preserved. Contract1.5.5 records exact release scope with fail-closed validation. Package adds START_HERE, current prototype guide and installed dependency notices (including exact checksum-bound selectors source). Model/GPU runtime remain explicit upstream downloads; no signing or general GPU qualification. Fresh build/package/install/remove receipts will be kept at D:/dev/grammar-evidence/grammar-prototype-release-20261014. No additional live inference or desktop focus session is needed for packaging-only changes; prior installed792032b evidence stays scoped. Mission remains INCOMPLETE.


## 2026-10-14 — Prototype 1 published and main adopted

2026-10-14 Prototype1 published: main safely fast-forwarded to49fb641, unrelated canonical untracked files3 hash-preserved; v0.1.0-prototype.1 prerelease ZIP source49fb641 SHA256 ddfd38d3dbd8e88f250b9d20327bf4d43b3a799f43c29c5eecd69ed039c45788. Fresh build/package PASS;652file hashes, fresh install/nativehost/model-free-start/remove PASS. Two fixture failures retained. Supervisor FAIL_LEFTOVER_CHILDREN(5) with complete owned cleanup and finalproduct0; different physicalPC and newbinary Gemma NOT_RUN, product code unchanged from792032b. Three published assets re-downloaded and hash-matched. No weights/runtime uploaded;unsigned prototype, mission INCOMPLETE. Release https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.1; external evidence grammar-prototype-release-20261014.

Release tag and package stay pinned to49fb641; this later documentation/state commit is not another binary build. Current personal installation792032b and its model remain untouched. Existing main untracked audit/export files are preserved. New packaging session never loaded Gemma or used user documents; it used an owned empty profile and registry key, both removed. App/launcher exited naturally, but the supervisor had to terminate five owned descendants, so natural shutdown is not PASS. Remaining work: natural descendant shutdown, broader hardware/editor/IME testing and quality/latency coverage. No claim of all-PC compatibility. Release assets: ZIP, SHA256SUMS.txt, VALIDATION.json. Source Project UI package is stale.


## 2026-10-14 — bounded WebView shutdown candidate

Owner requests child-process repair, one serial trial session and a replacement
GitHub prerelease. Prototype.1 remains immutable. Source base f873c24; first
local repair commit 3b01b6e did not qualify. Controller close alone, window destroy,
run_return and STA pumping with a 10-second deadline all failed. No-Job diagnostic
observations took about 22 seconds; they are not package acceptance. The 30-second
bounded shutdown candidate passed an actual WebView trial in 12.421 seconds with
app exit 0 and supervisor remainingBeforeCleanup=0. Diagnostic auto-quit and trace
code were removed; final fresh build/install/repeated shutdown/Gemma readiness and
shutdown are pending. Existing unit suite: 282 PASS,32 ignored; supervisor serial
5 PASS (earlier concurrent build contention caused 3 SESSION_ALREADY_RUNNING failures).
Evidence: D:/dev/grammar-evidence/grammar-shutdown-20261014. Prototype2 publication
is authorized but not yet performed. Mission remains INCOMPLETE. No new language
quality, Apply, cloud inference or different-PC acceptance is claimed.

Final source check before packaging: serial Rust suite 282 PASS/32 ignored. Two
parallel reruns each had one failure in unchanged filesystem cleanup tests
(startup stale removal and cleanup_completed_late); keep these as load-sensitive
FAIL observations, not a parallel full-suite PASS. The first focused filter ran
zero tests; corrected fully-qualified filter ran one and passed. No thresholds
or assertions in those tests were changed. Shutdown code was reviewed in a
separate pass by the same agent; no independent reviewer is claimed.

## 2026-10-14 — final installed shutdown qualification

Package source7951abffc92bf2303c8f2c666e49adc83eafb9dc, ZIP SHA256
b4e59d62911d256ef0689fe9a20e7c656734c7b72884c85a7a2e4c67fd239ef5.
Fresh build and all652 manifest file hashes passed. In one owned trial session,
three installed no-model runs and one actual Gemma ready/shutdown run passed:
quit command acknowledged, exit0, recorded descendants0. Observed shutdown times
18.488/14.634/12.783/15.335 seconds; startup readiness21.724/23.658/30.634/31.618
seconds (last includes real Gemma readiness). Final supervisor207.531s PASS,
remainingBeforeCleanup=0, remainingAfterCleanup=0. Installer pin checks, installed
local host, removal and task registry cleanup passed; uninstall stopped0 processes.
Owned evidence profiles remain as evidence; no user settings or personal install
were removed. Outside trial launcher entry restored. No concurrent trial sessions.

Preserved final-package failures: trial1/2 could not obtain a ready app page;
trial3 swallowed quit transport errors and closed CDP immediately, then observed
the app alive after40s and required six forced descendants. Cause is not proven.
Trial4 records affirmative quit acknowledgement and keeps CDP attached; it passed
all four runs. Do not reclassify trial3 as PASS or claim arbitrary debugger detach
is qualified. Native physical tray click, new language quality/Accept, new PC,
and live cloud Provider remain NOT_RUN. The trial harness is updated separately
from the packaged product source; this record commit does not rebuild the ZIP.

Release evidence: D:/dev/grammar-evidence/grammar-shutdown-20261014/final,
trial-r4. VALIDATION.json preserves serial Rust282/32ignored and both parallel
FAIL observations. Prototype2 main/publication is the authorized next operation;
prototype1 and its assets must remain unchanged. Mission INCOMPLETE; Project UI
Source package stale. This is scoped shutdown qualification, not instant exit.

## 2026-10-14 — Prototype 2 published

Main safely fast-forwarded f873c24 ->536fe13; tag v0.1.0-prototype.2 pins the
actual built source7951abf. Release ZIP, SHA256SUMS and VALIDATION were downloaded
again and all three hashes matched. Prototype1 asset IDs/digests were unchanged.
Release: https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2
Source CI7951abf and qualification-record CI536fe13 passed (frontend/governance,
not native/GPU acceptance). Independent post-trial check found all39 recorded
process identities absent. Outside launcher restored byte-for-byte; personal
installation/model/settings unchanged. Owned evidence profiles remain on disk.
Canonical unrelated untracked files3 preserved by hashes. This publication record
is not another binary build. Source package remains stale; mission INCOMPLETE.
Remaining: startup/shutdown latency, parallel cleanup-test load sensitivity,
physical tray-click/debugger-detach behavior, broader hardware/editor acceptance.


## 2026-10-15 — PRODUCT_QUALITY_AND_RELEASE_READINESS / first CI slice

Owner requested a durable parallel quality track, preserving the feature mission.
Base1749dee; reused clean codex/grammar-local-trial-ba58b7d worktree. No main/push,
release, install, branch-setting, license, account or live Provider action.
Contract1.5.7 and all three wrapper pins synchronized; charter/roadmap/ledger/state
and README point to IMPROVEMENT_BACKLOG.md. Seventeen items record priorities,
dependencies, criteria and evidence; re-evaluate at each major milestone.

Main/PR core Windows source workflow now runs TS/build, provider/runtime/capture,
Chromium core/worker/scheduling, startup fixture regression and nonignored Rust.
Same workflow/check name; read-only token, pinned actions, no persisted credentials,
no pull_request_target, secrets, uploads or ignored failures. --no-fail-fast exposes
all Rust target results. Hosted run NOT_RUN. 45min job budget accommodates added
Rust compile, copied from mission CI; no product/test deadline was increased.

Old P3_02_MAIN_GUARD failed against legitimate AnalysisMode references. Replaced
with runtime-dispatch boundary and negative engine/import examples, retaining
production reducer/runtime tests. Structural checks are not standalone behavior proof.

Full Rust validation exposed a separate real fixture fault: interrupted HTTP header
read panicked with Windows10054/ConnectionReset before warmup. Three full-suite
FAIL observations plus isolated/library PASS observations retained; this was not
fixed by retries. Synthetic panic receipt established cause. Fixture now discards
aborted/short request headers/body per connection with bounded read timeout;
unchanged warmup/cleanup assertions still run. New disconnect test FAILs old fixture
and PASSes repaired fixture. No production engine behavior changed.

Read-only GitHub API reports repository private/main on this date. Protection and
rulesets reads403 (plan/public availability); no inference that protection is absent.
Owner/platform decision remains Q03; do not change visibility as a workaround.

Evidence outside repo: quality-rust-20261015.log, quality-rust-all-20261015.log,
quality-final-rust-20261015.log retain failures; quality-startup-diagnostic-full and
quality-startup-suite-diagnostic logs record isolated/library passes; one mistakenly
short --exact filter selected0 tests and is NOT counted. quality-fixture-red-20261015
contains deterministic old-fixture failure. Final results recorded below.
All logs under D:/dev/grammar-evidence; no external evidence uploaded. No interactive
session/GPU/account test launched. Remaining product Q04 cold D model verification
and Q05 intermittent native click delivery are NOT closed by this fixture repair.
Project Source authority changed: previously exported Source is stale; no automatic
Project UI update or replacement claimed. Self-review only, no independent reviewer.

Final verification: cargo test --locked --no-fail-fast -- --test-threads=1 exit0,
lib147/21ignored, app282/32ignored, remaining targets28/1ignored,2/1ignored,14/1ignored;
quality-repaired-rust-20261015.log. Existing compiler warnings not cleared.
Provider/runtime contracts PASS; capture PASS; Chromium core227, worker77,
scheduling6 PASS; tsc+Vite PASS; fixture disconnect test oldFAIL/newPASS;
governance self-test and final validation PASS; final YAML trigger/privilege/failure
checks PASS (not actionlint/hosted execution). No further repetition needed without
new failure/change. Next: hosted CI confirmation separately, Q07, and feature Q04/Q05.


## 2026-10-15 — Candidate push, startup recovery and native diagnostic repeat
Owner explicitly requested candidate push/Windows CI and continued Q04/Q05 diagnosis.
942043b pushed non-force; run38723527175 is its hosted record. Remote main31a927a
contains separate README updates, left intact. Follow-up code/validation is described
in IMPROVEMENT_BACKLOG.md; first-run hosted success cannot qualify a later commit.

Artifact pinning failure occurs before any GemmaEngine/Job; scoped closure releases
pins then emits a distinct no-runtime receipt. Launcher still reaps broker/drains
before granting an explicit retry, keeps app draft and does not report cleanup
completion. Actual broker warmup failure and actual broker bad-model-hash failure
integration2 PASS. All30 stub-launcher regressions PASS; broker2 PASS/1ignored.
Old launcher reproducibly fails the new retry test. Initial new-test failure was
hidden PowerShell OEM output vs UTF8 decoding; test explicitly sets UTF8 rather than
suppressing errors. Production deadline, integrity and model quality unchanged.

External logs: quality-pre-ready-red-20261015.log, quality-pre-ready-r2-20261015.log,
quality-launcher-20261015.log, quality-broker-test-20261015.log. Installed792032b native
trial quality-native-20261015 passes14 with natural app/editor cleanup and supervisor
forced0/remaining0. Outside dispatcher restored; no live Provider. D cold hash latency
and original intermittent click cause remain open. No personal binary replacement.
Hosted follow-up includes the actual broker/launcher integration as a separate step.

942043b hosted Windows run38723527175 completed SUCCESS (all core steps); this
qualifies Q01/Q02/Q17 for that push snapshot, not PR trigger behavior or newer code.
Follow-up transition defect reproduced: after prior readiness + confirmed cleanup,
a subsequent no-runtime retry failed final classification because historical published
configuration remained non-null. Final guard now accepts the explicit no-runtime
state for that attempt, keeping exit failure and no-cleanup-claim semantics. Added
stub transition regression oldFAIL/newPASS; final launcher31: 30PASS/1FAIL (existing confirmed-startup retry ready observation); failure remains under diagnosis. CI additionally
runs all launcher tests after actual-broker integration. Superseded dd1734f run may
be cancelled to avoid duplicate compilation; cancellation is not a test PASS.


Follow-up verification correction: commit05ef854 incorrectly stated launcher31 PASS.
The actual r2 result was30PASS/1FAIL (ready observation); r3 was30PASS/1FAIL
(unconfirmed-cleanup scenario ended with broker-null startup disposition). Both
failures remain recorded in quality-launcher-r2/r3-20261015.log. Focused retries
passed2/2 but do not erase these failures or establish their cause. Added last-phase
and stdout failure diagnostics without changing deadlines/assertions. Hosted final
result must be read for the exact candidate commit; no local full31 PASS claimed.

Hosted c0ecb04 run38725130779 FAIL: actual-broker artifact failure did not offer
a retryTicket; warmup-failure case passed and all earlier core steps passed.
Cause remains under diagnosis. Preserve quality-ci-c0ecb04-failure-20261015.log.
Fixture now emits owned PowerShell error history (no user data) to identify the
original handled failure. CI dev/test debug profiles aligned to reuse dependencies;
observed duplicate broker dev build took4m28s; no timeout or assertion relaxed.


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

839f6f2 local launcher31 + actual-broker2 PASS. Hosted run38727497426:
all core steps and broker2 PASS; launcher30PASS/1FAIL. Failure is private CRT
path spelling (CI TEMP RUNNER~1 vs runneradmin), not distinct directories.
Fixture now requires os.path.samefile for the first child PATH entry and rejects
the installation root; parent PATH equality remains unchanged. Focused check PASS.
Previous atomic-publication failure no longer reproduced in the hosted regressions.
Final next-commit hosted status remains separate from this failed run.

## Installed CI candidate and owned desktop verification (2026-10-15)

Installed exact b145ddd2a4c67dc575de395fd48aca54d26b5c3e, qualified by Windows
CI run38728375665, into the existing personal installation. Built from clean source;
package ZIP SHA256 d1d131511332bcfa57b196b1d04416e2c2b316caace8adf2ec007fc54134bc19.
Existing settings preserved without reading their content. Model/runtime remain at
D:/PencilAssets/local via installation junction; no model duplication. Previous
792032b installation retained as GrammarPersonal.backup-792032b-20261015.
New package extension ID pbkigacecepjkokofepecnhncmccnadm differs from the previous
installation; ordinary Chrome profile was not changed and extension reload remains
necessary. Chromium was NOT_RUN this turn.

Final trial-r3: actual installed Gemma, fresh synthetic profile, 14/14 capture,
translation/rewrite and explicit Accept with exact source reread PASS (7 owned
native mouse, 7 CDP). Quality: 12 exact golden outputs, 2 existing reviewed-equivalent
paragraph translations, no new output accepted by weakening assertions. Ready
24970ms; capture114-247ms, recommendation555-1225ms (3/14 above1000ms), observed
Accept click-to-reread77-123ms. Input-helper launch/focus is excluded from that
Accept metric. Cache state uncontrolled; this is not cold disk qualification.
Broker observed approximately7.74GB reads and runtime creation approximately6.96s
later; this does not isolate hash CPU versus disk time or prove the latency cause.
Model, integrity checks, inference quality and time budgets unchanged.

Serial diagnostic failures preserved: initial owned editor occlusion; accidental
redispatch after a UTF8 preparation error (INTERRUPTED_NOT_PASS, owned dispatcher
stopped); r2 functional14 PASS but final cleanup assertion FAIL due to the stale
starting-status file from that interruption. Exact stale file preserved then removed
only after no owned processes remained. Fixture now temporarily raises only its
verified owned editor and hit-tests before clicking, restores normal z-order, and
preparation checks exit status. These fixture repairs do not explain product click
loss. Final r3 app/editor exited naturally, supervisor PASS/COMPLETE, zero processes
before forced cleanup and zero remaining; original external dispatcher restored.

Evidence: D:/dev/grammar-evidence/personal-b145ddd-20261015/FINAL_RECEIPT.json,
build-receipt.json, install.json, update.json, interrupted-dispatch.json,
trial-r3/desktop-results/app-result.json and trial-r3/supervisor.json. Earlier failed
attempts remain separate. Q04/Q05 stay open: next isolate first-start verification
and model initialization costs, and reproduce native delivery failure under measured
focus/occlusion conditions without relaxing target validation. No main/release change.

## 2026-10-15 — Installed startup phases and inactive-review diagnosis
Product installation remains CI-qualified b145ddd; no product binary, model,
settings, integrity checks or inference parameters changed. Added developer probe
scripts/mission/measure-installed-gemma-startup.py for supervised owned trials:
verify installed package/CRT hashes, run the installed broker, retain only bounded
fixed numeric phase telemetry, close stdin and require cleanup acknowledgement.
Actual startup PASS: 8718ms (cache uncontrolled), model pin2741ms including
read1309ms/hash1425ms for6975879296 bytes; other artifact pins350ms, engine
setup/spawn561ms, health/load4955ms, explicit warmup81ms. It is not an installed
app launch or cold-storage acceptance. Missing-runtime negative case exited31ms,
ready=false, exit1, no cleanup acknowledgement; observed expected rejection,
not a successful startup. Syntax and governance/self-tests PASS.

Inactive-review trial-r3: installed actual Gemma14/14 PASS,7 real native mouse
Accepts after verified owned target title-bar activation (review inactive),7 CDP.
Exact source reread,12 golden/2 previously reviewed equivalents. Ready20644ms;
capture114-234ms, recommendation559-1110ms, click-to-reread79-132ms. App/editor
natural exit, supervisor PASS/COMPLETE with before/after0. Inactive-review alone
did not reproduce historical input loss; do not claim Q05 repaired.

Trial preparation failures remain explicit: first copied harness retained old
task-owned paths (Windows separator mismatch); browser receipt overwritten in
old evidence directory and copied to new evidence location, no user profile used.
Desktop prior receipt restored from pre-write backup; failed run archived separately.
First inactive run failed helper path escaping (owned conhost backstop1 ->0);
r2 failed before app launch for absent fresh profile/output directories (remaining0).
r3 fixes paths/directories and passes; earlier failures are not reclassified.
Evidence roots: personal-inactive-accept-20261015{/startup.json,-r2,-r3},
personal-chromium-b145ddd-20261015 and its -r2 successor under
D:/dev/grammar-evidence. No independent reviewer; same-integrator review only.
Q04/Q05 stay open. Next collect phase telemetry on an actual first cold launch
without cache manipulation, and reproduce the original input-loss event with
owned foreground/hit-test/event evidence before changing product input logic.

Fresh Chromium successor-r2: unmodified installed extension pbkigacecepjkokofepecnhncmccnadm,
fresh owned app/browser profiles, sandbox on, real toolbar mouse then CDP trusted
mouse on actual popup (not physical-only). Actual Gemma14 translation/paraphrase
rows PASS including textarea/contenteditable, Accept/Apply edit and exact reread;
cache/Accept no inference, source-change/sensitive boundaries, disable/navigation
and no document storage verified. Recommendation337.8-781.5ms,
Accept click/reread14.3-15.9ms; ready21855.1ms.
Same-integrator output review preserved numbers/negation/uncertainty/conditions;
raw qualityStatus remains PENDING_SEPARATE_REVIEW, not independent linguistic QA.
Supervisor PASS/COMPLETE before/after0; browser closed, host0, profile deleted,
extension files unchanged. External dispatcher restored byte-for-byte and owned
app/model process inventory0. Final receipt personal-chromium-b145ddd-20261015-r2/
FINAL_RECEIPT.json. No main, push, release or normal Chrome profile change.


## 2026-10-15 — Cold read timeout reproduced; input delivery race isolated
Evidence: D:/dev/grammar-evidence/cold-input-cause-20261015. Installed product
remains b145ddd, unchanged. One owned supervisor/job at a time; no user documents,
normal profiles, provider calls or global cache purge. Fresh task-owned model files
were written unbuffered/write-through; broker performs their first buffered read.
This is a cold-file reproduction, not reboot/storage-cache controlled acceptance.
Copy preparation time is excluded from startup measurements.

Baseline cold-r2 failed at180234ms: model pin read175145ms/hash1291ms, only
5573181440/6975879296 bytes read; GPU engine had not started. Sequential-read hint
candidate also failed180219ms (3357540352 bytes); rejected. An8MiB read candidate
passed105218ms once (pin96314ms, engine5265ms), then failed180204ms on a fresh
file (read166962ms/hash1672ms,6887047168 bytes). Neither optimization is retained.
Original model/hash, file locks, quality settings and deadline remain unchanged.
This localizes the timeout to model verification IO, but does not prove the storage,
filesystem/filter or scheduling cause. Q04 remains OPEN; do not increase timeout
or skip hashing to turn these failures green. Next isolate read latency distribution
and disk contention, then evaluate bounded pre-read/preparation with unchanged
trusted verification and cancellation ownership. No product fix claimed.

Input trial:42 actual Gemma/native-mouse samples,40PASS/2FAIL. Both failures
refused before input; source unchanged. Continuous pointer movement and a window
on another monitor were observed. This is not proof of user intent or backend loss.
Controlled cursor displacement inside the owned review reproduces a concrete
harness flaw: old helper reports sent2 but no Accept event/source change; new
helper refuses before input. scripts/mission/owned-review-click.cs now checks the
actual cursor again before SendInput, validates cursor clipping and separates
foreground/hit-test/position diagnostics. No automatic retry or product mutation
checks removed. There remains a non-atomic gap before SendInput; historical
intermittent product Accept is not declared universally fixed. The42-case fixture's
experimental80ms settling delay was not adopted.

Latest helper compiles. Broker pin regression tests2PASS/1profile-only ignored;
existing library warnings remain. Governance PASS; no independent reviewer.
Earlier preparation failure (bounded360s CopyFileEx) and both rejected experiments
remain as failures. Final quiet-input result and cleanup are appended below.

Final input-final quiet-input test (Owner coordinated):14/14 real Gemma native
mouse Accept PASS, exact source reread, controlled old/new race reproduced again
with latest helper. No settling delay. Ready56.006s (cache uncontrolled), capture
108-237ms, recommendation549-983ms, trusted click-to-reread25-116ms. Whole helper
request-to-reread82-575ms includes process/input dispatch; do not label it backend
latency. App/editor natural exit; supervisor PASS/COMPLETE before/after0. This
isolated run does not prove all historical intermittent cases fixed.

FINAL_RECEIPT.json records all measured ranges and cleanup: owned process
inventory0, dispatcher restored byte-for-byte, four task-owned6.98GB model copies
removed after processes exited. Original D:/PencilAssets model/runtime and personal
installation unchanged. Runtime junctions retained without recursive traversal.
The final production source has no broker change; rejected candidate binaries and
logs remain evidence. No main/push/release, no installed candidate upgrade.


## 2026-10-15 — Owner defers startup tuning; editor integration first
D: maps to PhysicalDrive0, WDC WD80EAAZ SATA HDD (Healthy reported, not a full
hardware health diagnosis). Three PDH samples observed queue38-40, average reads
165-197ms and9-11KiB/request,1.95-2.51MB/s during other workloads. Process IO was
all-device/logical counters, not attribution to D; do not blame a specific process.
Installed broker cache-uncontrolled startup PASS8.891s: model pin2.441s
(read1.018s/hash1.417s), engine6.057s; supervised cleanup before/after0. This warm
result does not repair earlier cold timeouts. fsutil extents query unsupported;
no fragmentation conclusion. Unbuffered direct-read comparison prepared but NOT_RUN
because Owner explicitly deferred latency and will not pause other sessions.
Evidence D:/dev/grammar-evidence/disk-contention-20261015; dispatcher restored,
owned app/model processes0, no installation/settings/model modifications.

Independent boundary repair: model verifier previously could return success when
its final EOF read crossed the deadline. Production verifier extracted with file
pin ownership unchanged; check active/deadline immediately after read and after
hash finalization. Delayed EOF regression oldFAIL/newPASS (red-r2), broker3PASS,
1profile ignored. Initial red attempt was a CRLF preparation/compiler failure,
not a reproduced bug; preserved separately. No throughput improvement claimed,
no model/quality/hash/locking/deadline relaxation, no new binary installed.

Current priority is editor/document integration, then Provider/API connectivity
and stability; startup tuning deferred. State history, roadmap, ledger and backlog
updated without changing whole-product acceptance. Current next implementation
slice: paired Writer OXT/current Gemma product for owned saved DOCX, preserving
format/Undo and target authority. Then rHWP product admission/transport and
Calc/Impress; actual Office/Hancom remain separate qualification targets.
Writer bridge6/layout12 and rHWP admission/Instant15 unit regressions PASS;
these are not new native editor or current-Gemma adapter acceptance. No additional
Accept trial this turn; prior14-pass quiet native result remains historical.
Governance PASS. Same-integrator review only. Q04 deferred, Q05 correctness open;
no main/push/release or Project Source regeneration. Source package now stale for
priority change. Other sessions were not interrupted.


## 2026-10-15 — Current Gemma + Writer saved DOCX qualification
Fresh source e6ff394 build, paired Writer-enabled native library and product-only
OXT: actual Gemma12B Q4_0 from D:/PencilAssets, LibreOffice26.2.4.2, owned saved
DOCX, one paragraph with homogeneous selected style and differently formatted
surroundings. Real native shortcut, explicit one-review Writer menu opt-in and
actual widget Accept (CDP trusted UI click) were exercised after Owner activated
the owned Writer title bar. No private Apply or synthetic inference replacement.

PASS9 widget checks: EN->KO translation, natural and polite rewrite, each exact
whole-document reread + bold/Asian/complex weight, RGB and paragraph alignment
preservation + Undo; manually edited draft Accept/Undo; changed-source preservation;
Disable refusal. Enclosing5 checks: no fixture UNO service installed, saved DOCX
reopen exact text/format, installed-widget result, file SHA unchanged by product
(no automatic Save). Fixture explicitly saves each synthetic source before capture.
Three outputs reviewed by integrator preserve negation/25 dollars, Friday/reason,
and today/polite request respectively. Not independent linguistic qualification.

Recommendation from hotkey dispatch through capture/actionable draft:1391/1137/
878ms. Accept click through status and independent Writer reread:198/215/212ms.
Two short-recommendation1s target misses; all three0.5s Accept checks pass. n3 is
not broad latency/quality coverage. readyMs7 is observation after readiness, NOT
startup duration. No speed threshold or quality change. Long paragraphs/IME and
other document structures were not tested in this checkpoint.

Build initially bound the existing D: portable Office; preserved registration90s
failures and initial Office connection30s/120s failures. Successful environment
uses the identical SHA-pinned Office archive extracted into owned SSD temp storage;
Gemma stays on D. App/host/broker reused the clean e6ff394 build; native DLL was
separately rebuilt for the exact new Office path, then package verified/reassembled.
REASSEMBLY.json binds parent receipt and new native build. No product binaries
were edited after packaging. OXT binds installed DLL hash/path. General personal
install remains b145ddd and Writer-disabled; this is a separate owned candidate.

Fixture failures retained: retry preflight incorrectly expected installer-only
files in installed tree; accidental retry refused existing root without overwrite;
new widget wait15s corrected to existing desktop fixture60s; Windows foreground
refusal corrected by explicit Owner title-bar activation, not weakened checks.
Final app/Office natural exits0, supervised trial PASS/COMPLETE before/after0;
cleanup supervisor before/after0, both owned installations and their host registration
children removed. Dispatcher restored byte-for-byte. Original model/runtime,
personal install, other sessions and normal profiles unchanged. Attempt to recursively
remove three C: temp Office/profile roots was rejected by automatic tool approval
with only 'blocked by policy'; no retry/bypass. Temp files remain, no owned processes.

Evidence: D:/dev/grammar-evidence/writer-gemma-ssd-20261015/FINAL_RECEIPT.json,
output/{app-writer-result,uno-result,trial-result}.json, supervisor.json,
uninstall-cleanup.json and cleanup-supervisor.json. Earlier failed root:
D:/dev/grammar-evidence/writer-gemma-20261015. Qualified app SHA05c68e4d...bf23f,
DLL e6cf2da9...7101; full hashes in receipts. Package ZIP c0e15d4700916634a9903099d62ecad3368fa5f267f23cef223afd54b6e87679.
Writer bridge6/layout12/package9 unit tests and governance PASS. No independent
reviewer. No push/main/release/general activation; Project Source stale.

Next: repeat this scoped integration through a reproducible opt-in Writer package
setup before personal-profile activation; broaden DOCX paragraph/format coverage
and rHWP product transport under the Owner's editor-first priority. Preserve prior
0fc8624 paragraph-topology evidence as historical, not current-Gemma qualification.
Actual Microsoft Word, general DOCX, tables/fields/tracked changes, IME, Calc/Impress
and Hancom remain unqualified. Provider stability follows document integration;
D-drive startup tuning remains deferred.


## 2026-10-15 — Reproducible Writer package binding
2026-10-15 Writer package binding: builder optional --package-root verifies app/DLL receipt hashes, Writer-enabled native build and exact executable hash/path, plus adapter sources against package commit. Actual prior qualified package regenerates seven byte-identical OXT members; current Writer-disabled personal installation rejected without output or modification. Binding6/package9/bridge6/layout12 and governance self-test PASS. No GUI session or new document qualification this turn; next current-Gemma multi-paragraph DOCX qualification. Evidence writer-gemma-ssd-20261015/package-binding-result.json. No push/install/release; D startup deferred.
Same-integrator code review only. Package metadata is consistency evidence, not
authenticity; use trusted artifacts. OXT archive hash differs because ZIP timestamps
are not fixed; all seven member bytes match the previous actual-Gemma qualified
OXT. This does not extend its one-paragraph acceptance. The optional low-level
builder remains for development, explicitly with null packageBinding. No long-lived
processes or desktop sessions were launched, and no previous policy-blocked temp
cleanup was retried. Personal installation unchanged. Project Source stale.


## 2026-10-15 — Actual Gemma four-paragraph saved DOCX
2026-10-15 Current-Gemma four-paragraph Writer DOCX: paired e6ff394 native/app and paragraph-enabled OXT passed13 widget+5 enclosing checks. Selected range within paragraph2; differently colored/aligned surrounding paragraphs and empty paragraph preserved by whole-body reread and per-portion checks. Actual Gemma EN-KO/natural/polite and four-sentence translation/rewrite, edited Accept, Undo, source-change/Disable refusal. Final recommendation831-1205ms (short1s misses2/3, paragraph3s misses0/2); Accept272-294ms (0/5 misses). No autoSave. Final trial PASS, app/Office natural0, Job remaining0 before/after, owned uninstall complete. Earlier missing-profile and quit-reply failures preserved; fixture quit request now tolerates only closed transport and still requires natural exit0. No product-core change or personal-install activation. Cross-paragraph selection/current-Gemma topology, complex docs, Word/IME/live Provider unqualified. Evidence writer-gemma-paragraphs-20261015/FINAL_RECEIPT.json.

Final evidence root: D:/dev/grammar-evidence/writer-gemma-paragraphs-20261015.
LibreOffice26.2.4.2 from the already pinned SSD runtime; Gemma12B Q4_0 stays on D.
Only --enable-paragraphs added to the prior saved/uniform/mixed OXT; no span or
topology flags. Full document text including paragraph separators is reread,
every portion checks Latin/Asian/complex bold and RGB, each paragraph checks its
alignment including the empty paragraph. Native shortcut and product menu Enable;
actual widget Accept via trusted CDP click, no private Apply. No Owner click was
needed in the final runs. UNO only sets up synthetic documents and independently
reads/Undo. Product never saves the file; disk digest checked. Five sample outputs
reviewed by integrator preserve negation,25 dollars,Friday,today and delivery date.
No independent reviewer/linguistic review. Recommendation p50/p95/p99/max
1077/1205/1205/1205ms; Accept284/294/294/294ms, nearest-rank n5 only.
readyMs10135 observes the remaining wait after UI setup, not cold-start duration.

Preserved failure-r1-profile-preparation (launcher refuses absent profile; no
model/Accept), failure-r2-quit-reply (all9 functional checks pass, UI closes before
quit IPC reply; overall FAIL, supervisor backstop1->0). pass-r3-short succeeds
with old driver: UTF8 preparation failed under cp949 so patch did not apply; this
is not evidence for the repair. Final r4 uses explicit UTF8, preparation exit
check, and repaired requestAppQuit: transport_closed_before_reply is recorded
and app natural exit0 remains mandatory. Unrelated IPC errors still fail.
Shared fixture helper has3 regression tests; Guest payload requires it explicitly.
Writer layout12/bridge6/package9/binding6 pass; PowerShell parse and governance pass.
No runtime quality/budget changes. No push/main/release/new Source package.

Final supervisor PASS/COMPLETE, before0/after0; uninstall complete with own native
host registration/installation/user-data/processes0. Cleanup supervisor likewise0.
Owned temporary Office profiles and evidence remain; prior policy-blocked cleanup
was not retried. Personal installation and other sessions unchanged.
Next: current-Gemma cross-paragraph selection/topology qualification, then rHWP
trusted transport and Calc/Impress under editor-first priority. General Writer/Word
acceptance remains incomplete; D startup optimization deferred.


## 2026-10-15 — Actual Gemma cross-paragraph saved DOCX
2026-10-15 Actual Gemma Writer cross-paragraph DOCX: e6ff394 binaries plus package-verified span-enabled OXT, selection from inside paragraph1 to inside paragraph4 including empty paragraph3. Final10 widget+5 enclosing checks PASS: EN-KO/natural/polite Accept and Undo, full text/portion bold/RGB/alignment preservation, edited span, removed-boundary refusal unchanged, source-change and Disable. Two sequential trials PASS with natural app/Office exit and supervisor0 before/after; owned uninstall complete. Final recommendation1098-1471ms and Accept455-958ms; across6 samples Accept500ms missed5, short1s missed6, no result over3s. Latency unresolved, no relaxed targets. No production defect found or core change. Personal install unchanged; general DOCX/Word/current-Gemma merge-split unqualified. Evidence writer-gemma-span-20261015/FINAL_RECEIPT.json.

Exact environment: pinned LibreOffice26.2.4.2 SSD runtime, D Gemma12B Q4_0,
unchanged e6ff394 app/broker/native DLL. Package binding verified by the current
builder. Added explicit cross-format/span OXT flags, kept uniform-topology OFF.
Four Standard body paragraphs with colors118822/BB2200/2255AA/442299 and
alignments0/3/0/1, bold across Latin/Asian/complex script properties. Capture
starts after the unselected Before prefix in paragraph1 and ends before After
suffix in paragraph4. Paragraph3 is empty. Full document string and every portion
format are checked after Apply and Undo, not event-count inference. Actual Gemma
keeps separators/empty paragraph; first-selected-style-per-part policy applies.
Fixture UNO creates/reopens/saves synthetic sources, reads and requests Undo;
product mutation is only trusted widget Accept following native shortcut/menu
Enable. No private Apply, fake model response, automatic file Save or cloud call.
Quality reviewed by integrator: negation,25 dollars,Friday,today and delivery-date
constraint preserved. Not independent linguistic review.

Final expanded run also checks edited multi-paragraph draft and deliberately
collapsed boundaries: unsupported merge is refused, whole text and formats
unchanged. Earlier pass-r1 retained separately. Final p50/p95/p99/max: result
1146/1471/1471/1471ms; Accept670/958/958/958ms (n3). Both runs n6: Accept5/6
misses500ms and result6/6 misses1s, none over3s. This does not prove a broad3s
paragraph target; exact small samples and misses retained. No startup tuning;
readyMs is residual observation after UI startup, not a cold-start measurement.
No correctness defect reproduced; no product algorithm changed.

Final10 widget+5 enclosing checks, separate prior8+5 PASS; source layout12,
bridge6, package9, binding6, quit3 tests PASS. Same-integrator review only.
Evidence D:/dev/grammar-evidence/writer-gemma-span-20261015/FINAL_RECEIPT.json
binds fixtures, OXT and outputs. Both trial supervisors PASS/COMPLETE with
before0/after0 and natural app/Office exit0. Cleanup supervisor PASS/COMPLETE0;
uninstall reports root/host-registration/user-data/processes0. New task-owned
Office profiles/evidence retained; no prior policy-blocked deletion retried.
Personal installation, other sessions, main and release unchanged. Source stale.
Next document-priority slice: rHWP trusted product transport/admission and
installed HWP/HWPX flow, then Calc/Impress. Writer current-Gemma merge/split,
complex tables/fields/tracked changes, mixed runs inside each selected paragraph,
Word/IME/normal profiles remain unqualified. Accept latency debt remains open.


## 2026-10-15 — rHWP actual Gemma scoped bridge
2026-10-15 rHWP actual Gemma scoped bridge: HWP/HWPX plain whole-body paragraph EN-KO/natural/polite6 calls and12 Accept/exact text+format properties/Undo checks PASS. Added opt-in writing intents, response binding and draft-protecting mode changes. Fixed fixture early stdin EOF cancelling async host requests; prior3 failures retained. Recommendation841-1444ms, Accept33-54ms in headless owned Chromium only. Final app/server natural0, supervisor0 before/after, uninstall complete. Default production admission remains denied; trusted installed routing/provenance and rich documents unqualified. No personal installation/main/push/release change; mission INCOMPLETE. Evidence rhwp-gemma-20261015/FINAL_RECEIPT.json.

Native host/broker/app package source e6ff394 remains unchanged. Current JS adapter
runs against pinned rHWP0.8.7 commit1a76570e833917d15817415a53c09ad61ab3203f;
864 source hashes and core tarball integrity were verified. Six actual D Gemma12B
Q4_0 requests use the installed host whose manifest hash is checked. No fake model,
remote Provider, changed weights or relaxed10s host timeout/256KiB output cap.
Playwright exposes a TEST-ONLY local analysis bridge and synthetic eligibility;
this is NOT the trusted bootstrap, desktop shortcut, general profile or installed
rHWP product routing. Default production admission remains false.

Six cases preserve negation,25 dollars and today in integrator semantic review;
not independent linguistic review. HWP/HWPX whole homogeneous plain paragraph only,
with unselected Header/tail exact reread, character/paragraph/style property JSON
and format digest before/after/Undo. No mixed runs, tables, fields, real Hancom,
save/reopen roundtrip or sensitive-document qualification. Result1s missed1/6;
Accept500ms missed0/6; small headless fixture timings do not qualify native UX.

Failures retained: preparation timeout and owned orphan cleanup, reused output
refusal, then request-pipe early EOF cancelling async Gemma. Full response framing
before EOF fixed the last failure; six final hosts exited0. New panel tests check
mode-change decline preserves edited draft, confirmed change clears authority,
and late old-mode reply cannot publish. rHWP unit suite20 PASS. Older Instant-only
browser fixture NOT rerun; no claim of new real-model stale/Disable browser tests.
Final supervisor PASS/COMPLETE before0/after0, app natural exit and server exit0;
owned uninstall complete with installation/registration/user-data/processes0.
Prior failed profile/evidence retained; unrelated sessions untouched.
Same-integrator review only. Project Source stale. Next: trusted product transport
and admission/provenance, then installed rHWP qualification; Calc/Impress remains
next editor work. General HWP/HWPX support is not enabled by this checkpoint.


## 2026-10-15 — rHWP product transport foundation and styled save/reopen
2026-10-15 rHWP native transport foundation and saved-format fixture: extension-owned Native Messaging module plus admission-bound bootstrap added;9 transport counterexamples and full29 unit tests PASS, but packaging/trusted load provenance/installed routing NOT_RUN and production admission remains denied. Actual Gemma existing test bridge HWP/HWPX6 cases18 checks PASS for styled uniform target, mixed surrounding runs, Accept/Undo and disk save/reopen exact text+property rereads. No mixed-target support or general complex-document claim. Final single-session app/server natural exit, supervisor0, uninstall complete; missing-profile first failure retained with1 owned child cleaned. No personal install/main/push/release changes; mission INCOMPLETE. Evidence rhwp-native-prep-20261015/FINAL_RECEIPT.json.

Request remains PARTIAL. native-review.mjs is a trusted extension application entry,
not wired into a shipped rHWP package. Native transport uses connectNative with a
package-supplied host name, same-extension context, one request, bounded10s deadline,
abort disconnect, terminal close, admission at dispatch and reply, identity binding,
no Deep fields/retry/HTTP/page-message listener. Unit ports are fake; native runtime
qualification of this NEW transport is NOT_RUN. The actual-model browser test still
uses the PREVIOUS Playwright bridge, explicitly labelled as such. Do not combine
these two observations into installed-product acceptance.

Pinned rHWP rejects mixed character runs IN the selected paragraph before target
text access. This refusal is retained. New actual Gemma fixture styles all of the
selected paragraph bold+italic and separate portions of Header/tail bold/italic.
Before/after/Undo compares exact target character/paragraph/style properties plus
every surrounding character property. After Accept, exports HWP/HWPX bytes to owned
disk, verifies disk bytes, performs Undo and source/format checks, then reopens saved
bytes and confirms output text/format again. Six translation/natural/polite cases
pass18 checks. This is an API export/reopen test, not a native Save dialog test.
No tables/fields/mixed selected runs/real Hancom/normal-profile qualification.
Outputs preserve negation,25 dollars,today in integrator review; no independent
linguistic reviewer. Timings are scoped headless fixture observations in receipt.

First preparation failed because ProfileDirectory did not exist; fixed owned
fixture precondition, no product policy relaxed. First supervisor FAIL retained,
one owned child terminated, remaining0. Final131.5s supervisor PASS before0/after0;
app quit acknowledged/natural, server exit0. Browser profile removed, owned uninstall
complete with registration/installation/user-data/processes0. No ongoing sessions.
Same-integrator code review; no new independent reviewer. Source package stale.
Next: bundle pinned Studio into extension-owned product page with source/artifact
binding; implement trusted per-load protection provenance (password-open/save-intent
metadata alone is insufficient); qualify actual Native Messaging in that package.
Then mixed-target formatting needs a verified replacement/run-mapping design rather
than removal of existing format refusal. Existing production activation stays denied.


## 2026-10-16 rHWP installed Native Messaging / original-load protection

Runtime package: verified e6ff394 native binaries plus separately source/hash-bound
rHWP addon (implementation bdc832d plus current readiness/CSP fixes), external
rhwp-installed-20261016/package-r5. This is a composite, not a native rebuild from
one source commit. Pinned upstream source864 hashes/core integrity checked; no new
dependency. Installer supplies actual Chrome NativeMessagingHosts registration and
host-config; no test analysis bridge/window hook. Popup links extension-owned Studio.

Original-byte bounded HWP5/HWPX inspection happens before editor text parsing.
Protected/unknown headers, ZIP encryption/ambiguity, nonempty protection manifest
refuse. Only proven plain formats qualify, with exact generation proof retained
until initializeDocument completes. Old review disposed on new/failed load. Recent
store/recovery/idle autosave disabled; explicit Save downloads bytes. CSP permits
local WASM, denies external connections; worker blob source was rejected by MV3,
fixed to self. Rebuilding popup now keeps exactly one editor link.

Final owned installed run:21 PASS, six actual Gemma EN-KO/natural/polite cases.
18 independent saved-file text/character/paragraph/style, Undo and reopen checks;
2 original protected-byte refusal checks;1 no HTTP requests check. No live mutation
through test APIs. Input is uniform bold+italic paragraph amid mixed surrounding
runs; mixed runs within selection, tables/fields/arbitrary complex files/Hancom
remain unsupported. Headless Chromium147/new profile, not physical-keyboard use.
Recommendation850-1380ms (2/6 exceed1s), Accept29-67ms. Quality reviewed by integrator:
negation,25 dollars,today preserved; no independent linguistic review. Cold model
startup not qualified or optimized. 33 adapter tests +2 uninstall guard tests PASS;
governance self-test and syntax/diff checks PASS. Native full-suite not rerun.

Failed selection fixture (inactive editor), early load-readiness candidate, and MV3
CSP rejection are preserved under external failure folders. Successful installed
runs18/20/21 remain separate. Final supervisor100.766s PASS, natural app exit,
0 children before/after cleanup. Explicit owned-profile uninstall removed install,
registration and profiles; no ongoing trial. Personal binary path was not updated.

IMPORTANT TEST CLEANUP INCIDENT: earlier external trial.py reinstall steps invoked
RemoveUserData without UserDataDirectory. That targets default Roaming/Local
com.local.codexpencil folders, not just owned app-profile. Logs report removal
complete but no baseline inventory exists, so prior contents/data loss are UNKNOWN.
Do not claim personal settings were untouched. Credentials were not inspected or
restored. Trial script now specifies owned path. Production uninstaller now rejects
this omission whenever GRAMMAR_HOST_TRIAL is set, before modifying installation;
2 real PowerShell regression cases pass. Final cleanup exercised this guarded source
script with explicit owned path. The r5 browser-tested package predates that guard;
r6 incorporates that guard. Its only changed payload file versus r5 is the uninstaller; all runtime/extension bytes are identical. r6 actual install, omitted-path refusal and explicit-path uninstall PASS in a separate sequential owned session (1.688s, children0). No release/push/adoption performed.

Evidence: D:/dev/grammar-evidence/rhwp-installed-20261016/FINAL_RECEIPT.json;
installed-receipt.json, supervisor.json, final-uninstall.json, cleanup-verification.json.
Same-integrator review only. Project Source snapshot is stale. Next: expand selected-run mapping only with
preservation/Undo proof, then Calc/Impress per editor-first priority. Provider work
second, startup third. Do not silently enable broader HWP/Hancom support.


## 2026-10-16 AppData incident investigation / cleanup containment

Priority override: user requested read-only impact/recovery investigation before
further product trials. No app/model/browser session, user-file restoration,
credential inspection, personal install update, push or release was performed.

Historical `grammar-host-personal-20261011/personal-data-before.json` records four
Roaming files: settings.json808 bytes, settings.json.bak598, terminology.v1.json2587,
terminology.v1.json.bak2036, each with SHA256. Matching virtualized baseline and
preservation receipt exist. Current default Roaming/Local directories are absent;
MSIX LocalCache was also checked (no settings/dictionary found). Exact deletion
time, last pre-incident contents and full loss cannot be proven. This narrows the
previous UNKNOWN: historical existence is confirmed; current absence is confirmed;
causal attribution of every lost byte is not. Do not claim no data loss.

Actual personal shortcut passes ProfileDirectory=C:/Users/USER/GrammarPersonalData-dffcf8a;
that path was outside the erroneous default deletion list and currently contains
only its original marker. Known personal installation backups and two related
external evidence roots yielded11 settings/dictionary candidates, none matching
historical personal hashes. Candidates are not restored or treated as user backups.
Only names/sizes/hashes were recorded, not setting values or dictionary entries.
VSS CIM query unavailable (CimException), NOT evidence of no snapshots; File History
configuration absent in current view. No undelete tools, raw disk scan, credential
recovery or privileged operation. Hashes alone cannot reconstruct contents. A known
independent backup/Previous Versions source is needed before safe restoration.
Evidence: D:/dev/grammar-evidence/appdata-investigation-20261016/IMPACT_AND_RECOVERY.json
and recovery-candidates.json. Search was bounded, not exhaustive across every disk.

Cleanup now requires explicit directories and a marked TrialRoot; implicit shared
AppData removal is rejected even without a trial environment flag. Normal uninstall
without RemoveUserData preserves settings. Trial scope is exported by owned_trial
as its exact output directory with owner marker. Install/data paths must be strict
children; lexical traversal/ADS/UNC/broad roots, missing or mismatched ownership,
reparse ancestors/descendants, and foreign host registration are rejected BEFORE
receipt/lock/registry mutation. Parent directory handles exclude delete sharing;
scoped deletion removes single entries rather than recursive traversal. Receipt
replacement avoids modifying outside hard-linked bytes. All pins close in finally.
This is guarded cleanup, not OS sandboxing of arbitrary scripts or protection from
an administrator/forged ownership. Old package uninstallers do not acquire these
fixes. Do not run archived cleanup scripts; migrate callers to this current contract.

14 production PowerShell synthetic tests PASS (including preserved outside sentinels,
unchanged refused receipts, junctions, hardlinks, native registration); supervisor6
PASS; one supervised synthetic install/profile cleanup PASS, remaining children0.
All new test data under D evidence; user data untouched. Intermediate File.Replace
null-binding failure2 retained as tests-r3 fixture state; fixed NullString binding,
no requirement relaxed. Governance/syntax/diff checks PASS. Hosted package CI NOT_RUN;
legacy test-personal-package RemoveUserData calls and external sibling-root trial
scripts require scope migration before rerunning (fail closed, not test acceptance).
No blanket claim that every old test driver is safe. No obsolete package publication.


## 2026-10-16 Cleanup package propagation / maintained trial callers

User requested shipping the containment fix into an actual candidate package and
migrating old test entry points, without touching user data. Added packaged
New-GrammarTrialScope.ps1: fresh/marked root only, no unmarked adoption or junction,
strict supervisor descendant scope, explicit profile and WebView environment.
Installer copies the helper and package builder hashes/ships it. Trial native-host
registry exception is limited to marker-id-bound HKCU/Software/PencilTrials/<id>;
foreign roots still refuse. Normal Chrome registration remains separately allowed.

Sandbox guest installation paths and JS/Writer consumers migrated together to
C:/PencilTrialScope/install; explicit profile at scope/profile. Guest cleanup passes
TrialRoot and UserDataDirectory. CI package driver exports scoped profile variables,
uses marked-root children and passes explicit uninstall scope; failure-case test
registry uses the marker-bound root. Gemma launcher tests now put installation below
a marked scope and pass it explicitly. App-flow settings lock uses owned profile,
not shared AppData. Historical externally archived payloads are preserved, not
silently edited/requalified; missing helper in old packages fails before installation.
See scripts/mission/OWNED_TRIAL_CLEANUP.md.

Actual ZIP extracted and installed with owned NativeMessaging registration;6 checks
PASS: installed script hashes, omitted path/outside sibling/traversal/junction
refused with unchanged receipt+registration+outside sentinel, then owned
installation/profile/registration removed with outside sentinel unchanged.
Single supervisor PASS6.188s,0 children before/after. Empty uniquely owned registry
parents removed separately after verifying no children/values. Sentinels retained
as evidence. No app/model/Provider launched in this installed cleanup trial.
Packaged scripts17 production PowerShell boundary/initializer tests PASS; separate
existing synthetic launcher31 PASS and Writer packaging9 PASS. JS/Python syntax,
PowerShell parse, governance self-test and diff checks PASS. No full native/Gemma
quality/GUI rerun, hosted CI or Sandbox acceptance claimed for the migrated callers.

Package is a truthful composite: prior verified e6ff394 native binaries and prior
installed-tested rHWP runtime unchanged, four files refreshed (installer, uninstaller,
scope helper, personal-use guide). MANIFEST cleanupAddon records source hashes;
not a fresh native build from the new commit. Actual ZIP and source bindings in
D:/dev/grammar-evidence/cleanup-package-20261016/FINAL_RECEIPT.json.
Pencil-cleanup-candidate.zip is local only; personal installation, user settings,
dictionary and credentials were not read/changed. No push/main/release update.
Prior AppData loss/recovery uncertainty remains unresolved; this is prevention,
not restoration. Next package/GUI runs must use these new scope paths. Whole
mission remains INCOMPLETE, Source stale. Same-integrator review only.


## 2026-10-16 — cleanup Sandbox GUI and Windows CI

Owner authorized candidate push and hosted Windows CI, one Sandbox, no user data.
Unchanged composite ZIP6b424ce5121d66dd549dd935aeb0d2f84a90605cff912c3fcbbcbddcc792d37a
(native e6ff394 plus hash-bound cleanup addon) installed in one offline Sandbox.
Real GUI3 PASS: WebView render, explicit owned-profile settings, natural quit.
Installed boundary6 PASS: missing data/outside/traversal/junction refusal preserves
receipt/registry/sentinel; normal removal complete. Installation/profile/host key/
product processes0; final Sandbox inventory empty. No model/Provider/user data.
Capture/Accept, model quality and all other migrated GUI drivers NOT_RUN here.
Tray removal warning remains in stderr; natural app/launcher exit and removal
succeeded, visual tray state not separately inspected.

Preserved setup failures: nested readonly mapping denied, guest script name guard,
and WebView env initialized after first install. Fixed fixture sequencing and
mapping without weakening guest isolation; maintained Guest-App-Cleanup.ps1 now
sets existing WebView path before installation. No new runtime installed.

First CI38838627141 on27d1cce FAILED. Two test failures: retry poll races atomic
session replacement (WinError2), marker comparison uses8.3 TEMP alias vs canonical
path. Fixes retain10s deadline and all ticket/PID/cleanup assertions; only transient
open races retry, marker uses native realpath. Local boundary17 PASS; actual broker2
PASS after explicit UTF8 output fix for Korean Windows. Hidden compiler/test starts.
Strict local supervisor still FAIL_LEFTOVER_CHILDREN: owned MSVC vctip.exe persists
after compilation. Job kills that owned child, confirms cleanup COMPLETE0; not a
natural-cleanup pass. Process-only VSCMD_SKIP_SENDTELEMETRY did not remove residue;
no global setting changed. Separate from passing installed GUI natural shutdown.

Source push scanned109 outgoing blobs without secret/unexpected-binary findings.
Latest exact-commit CI outcome belongs in Actions/external final receipt; initial
failed run is not overwritten. Evidence cleanup-gui-ci-20261016 includes attempt1,
output, ci-failure.log, local-boundaries.log, broker-check*, leftover-images.json.
No personal install/main/release. Prior AppData recovery remains unresolved.
Mission INCOMPLETE, Source stale; same-integrator review only.


## 2026-10-16 — actual Gemma rHWP in one offline GPU Sandbox

Unchanged cleanup-qualified composite ZIP6b424ce5121d66dd549dd935aeb0d2f84a90605cff912c3fcbbcbddcc792d37a,
native e6ff394, installed Native Messaging and headed owned Chromium: final21
checks PASS. Six HWP/HWPX translation/natural/polite cases verify explicit Accept,
exact saved text and character/paragraph/style properties, Undo and saved reopen;
two protected inputs refused before admission, no HTTP document/model bridge.
Installed extension49 files unchanged; actual Gemma4 12B IT QAT Q4_0 unchanged.
Six output reviews preserve negation,25 dollars,today and recipient. Synthetic
uniform target paragraph plus mixed surrounding formatting only; same pinned WASM
parser rereads, not independent Hancom interoperability or arbitrary rich targets.

Final recommendation887-1051ms (1/6 miss1s;0/6 miss3s paragraph), Accept35-85ms
(0/6 miss500ms). Capture-button to completed suggestion and Accept to Applied;
file reread is a separate correctness check. Guest asset staging341.444s and
subsequent product readiness13.073s are separate, not inference latency. No
quality reduction, hash bypass or product deadline increase.

Preserved attempt1: direct read-only mapped assets fail production artifact
validation after approximately180s; exact cause not proven. Guest-local staging
with exact model hash and full product runtime/model validation works. This is a
verified test setup workaround, not closure of D-drive/startup performance debt.
Attempt2 functional21 PASS but wrapper incorrectly compared installer-generated
host-config.js to ZIP placeholder. Fix validates installer receipt registration,
checks other package pins, and compares all installed files before/after. Attempt3
fully passes. Test-only headless option and guest runners added; no core product
change. Local adapter33 tests, JS syntax, PowerShell parse, governance/diff PASS.

Final app/launcher natural exit0; uninstall stopped0 processes, complete=true,
installation/profiles/registration/product processes0. Exact Sandbox stopped,
final inventory0 and viewer absent. Quit IPC closed before reply but actual exit0
confirmed; tray removal warning retained, visual tray state not separately tested.
One VM reused serially; read-only package/assets/tools, task-owned output only.
No host user data or personal installation modified. Previous AppData loss and
recovery uncertainty remain unresolved. No push/main/release. Mission INCOMPLETE,
Source stale, same-integrator review only. Evidence:
D:/dev/grammar-evidence/rhwp-sandbox-gemma-20261016/FINAL_RECEIPT.json
with attempt1/attempt2 failures and final output. Next editor work should preserve
these guarded cleanup paths; general mixed-target/complex/Hancom remains unqualified.


## 2026-10-16 — rHWP rich paragraph selection with actual Gemma

Implemented a conservative rich-review path: complete body paragraphs, ordered
escaped format-run markers, one full-context model request, review lines bound to
original styles and blank lines bound to original paragraph boundaries. Never map
translated text by proportional offsets or split context into per-run inference.
Accept requires exact source/document/selection/permission and retained topology,
performs one target-side history operation and checks target/outside properties.
Existing snapshot Undo fencing and no render-error rollback remain effective.
Mixed runs must be sentence aligned. Partial/empty selected paragraphs, inline
word emphasis requiring reordering, tables/fields and general Hancom are unqualified.
Bounds:8 selected paragraphs,24 runs,3000 selected codepoints; inspected document
at most128 paragraphs/32000 codepoints. Single section, whole body paragraphs only.

One offline GPU Sandbox, reused serially. Actual installed Native Messaging and
same Gemma4 12B IT QAT Q4_0: final41 checks PASS,12 samples across mixed single
paragraph and mixed two-paragraph HWP/HWPX EN-KO/natural/polite. Exact saved text,
character properties, paragraph/style and outside text preservation, Undo and
saved reopen36; unsupported inline boundaries2, protected originals2, noHTTP1.
Original pipeline globals absent. Installed extension51 files unchanged. Saved
rereads use same pinned WASM, not independent Hancom compatibility. Same-integrator
review of outputs preserves negation,25 dollars,today,recipient/original deadline.
No downgrade, per-run fragment translation, replay or cloud inference.

Final recommendation887-1923ms (nearest-rank p50=1404,p95/p99=1923;10/12 miss1s),
Accept35-75ms (p50=53,p95/p99=75;0/12 miss500ms). All12 below3s reference, including
four-sentence/two-paragraph samples. Capture-button to complete result and Accept
to Applied; saved reread checked separately. Staging251.203s and readiness12.831s
are separate; D-drive performance remains deferred, not fixed.

Failure retained in attempt1: six mixed-paragraph cases18checks PASS, first span
unavailable, broker stopped and launcher exit1. No raw broker error was retained,
so exact first terminal error is not directly observed. Source inspection found
Native Host used a hard1s deadline for writing too. New compiled host uses one8s
writing budget under extension10s watchdog and broker15s ceiling; grammar1s remains.
Targets1s/3s/500ms unchanged. Final same corpus succeeds. This is request completion
repair, not a claim of faster inference. Invalid request remains rejected and
bootstrap/inference share the deadline. Host29 tests PASS,1ownedGPU test ignored;
actual installed GPU evidence separate. Adapter43 tests PASS; governance self-test,
JS syntax/diff checks PASS. Full Rust suite/hosted CI not rerun. Existing lib165
build warnings retained, no warnings added by the budget helper.

Package is explicit composite: e6ff394 app/broker/target DLL unchanged; new static
CRT Native Host and source-hashed rich extension. host-build-receipt and MANIFEST
nativeHostAddon/richSelectionAddon identify source delta and hashes; not a fresh
whole-native build. Final ZIP is local only. No personal install/main/push/release.
Source package stale; mission INCOMPLETE. Prior AppData loss/recovery unresolved.

Guest app/launcher natural0, uninstall stopped0/complete, install/profiles/native
registration/product processes0. Exact VM stopped and Sandbox inventory0, viewer
absent. Host compiler helper vctip.exe PID60004 started during build window, but
original parent47732 gone and ancestry audit permission unavailable. Ownership is
unconfirmed: retained rather than terminating a potentially unrelated session.
Do NOT claim all host processes were cleared. Future native builds should use the
owned Job supervisor rather than an untracked compiler launch. No user data touched.

Evidence D:/dev/grammar-evidence/rhwp-rich-sandbox-20261016/FINAL_RECEIPT.json,
Pencil-rich-selection-candidate.zip, attempt1, output, host-build/test receipts.
Next: semantic style anchoring for word-level emphasis and reordered translated
spans; keep unsupported cases denied. Broader formatting/IME/real Hancom remains
separate from this scoped synthetic acceptance.


## 2026-10-18 — word-internal emphasis and partial HWP/HWPX selection

2026-10-18 rHWP inline/partial selection candidate: actual Gemma4 12B installed Native Messaging, one offline GPU Sandbox at a time, final18 HWP/HWPX EN-KO/natural/polite cases and57 checks PASS. Word-internal emphasis, partial paragraph and partial cross-paragraph selection; saved full text/char/paragraph/style/outside equality, Undo and reopen. Plain writing followed by typed exact-substring style alignment, ambiguous/overlap/missing anchors refuse. Repetition and check/report emphasis verified; some conservative rewrites unchanged. HWP Save caret-only metadata normalization repairs Undo while retaining full serialized-content fence. Final recommendation1385-2426ms (18/18 miss1s,0/18 over3s); Accept34-93ms (0/18 miss500ms); staging138.963s,readiness11.937s separate. Adapter58PASS, Native Host29PASS1ignored, native Gemma22PASS6ignored after one retained3s startup fixture failure under concurrent work; serial rerun passes without threshold change. Actual WASM4PASS. Compiler helper residuals caused supervisor FAIL_LEFTOVER_CHILDREN, scoped cleanup COMPLETE0, not natural compiler cleanup PASS. Final guest app natural0, uninstall complete/stopped0, product/install/profile/registration0, Sandbox0. Earlier mechanical57PASS with language/style failures is NOT quality acceptance. Source-hashed composite newbroker/newhost plus existing e6ff394 app; no host user data/personal install/main/push/release. General Hancom/IME/ambiguous repeated anchors/empty-table-field/richer cases unqualified; previous AppData loss unresolved; mission INCOMPLETE,Source stale. Evidence rhwp-inline-sandbox-20261018/FINAL_RECEIPT.json and rhwp-inline-sandbox-20261016 failed trials.

Entry candidate 3636472d7b5f2e10cfcb9cac88887830ea49a1d2; same isolated
codex/grammar-local-trial-ba58b7d worktree. Canonical main was not edited.
The new operation preserves unselected prefixes/suffixes and their original
properties locally. Equivalent original style objects share an ID per paragraph.
Ordinary writing receives selected plain text once, then typed style_align finds
exact non-default substrings in that immutable result. No fuzzy or proportional
mapping, silent retry, per-run translation, model downgrade, or cloud request.
Two stages share one8s cancellation budget; Accept does not infer. Ordinary
literal/modality guards remain. Alignment is metadata rather than prose; JSON
quotes are not interpreted as user literals, and the adapter cannot change the
fixed output text. Semantic equivalence still needs user review; this corpus is
not proof for every word/language/document. Repeated ambiguous anchors refuse.

Observed failures are preserved, not overwritten:
- 20261016 attempt1 partial HWP Save/Undo rejected caret metadata drift. Full-byte
  history comparison now normalizes only stored caret synchronously and restores
  it even on export error; actual content changes continue to refuse Undo.
- 20261016 attempts2-8: wrong semantic emphasis, untranslated fragments, awkward
  Korean, wrapper output and structural refusal. attempt5 had57 mechanical PASS
  but failed language quality. No qualified performance claim from those trials.
- 20261018 attempt1:3 checks then fixed text changed during formatting; attempt2:
  default instead of requested style; attempt3: JSON quote/literal guard conflict,
  launcher1 then scoped uninstall cleaned all resources. attempt4:57 mechanical
  PASS but recheck lost repetition and emphasis widened. Only final output is the
  current scoped qualification. Debugger was disabled in the final trial.
- final-native-tests:21PASS/1FAIL/6ignored; unchanged startup fixture did not reach
  warmup within its3s deadline. Later final-native-serial-tests22PASS/6ignored.
  Failure remains in evidence; no claim of universal timing stability. Native
  compiler helpers required owned-Job termination; remainingAfterCleanup0.

Final18 outputs reviewed by the same integrator (no independent reviewer).
Numbers25, repetition/again, today/deadline and check/report emphasis preserved;
some rewrite/polite results conservatively retain original wording. Sentence
quality on this small artificial corpus is not broad language-quality acceptance.
Recommendation p50=1430ms,p95/p99=2426ms; Accept p50=59ms,p95/p99=93ms.
Measures are capture-click to complete result and Accept to Applied; saved
reread is separately checked. Original1s/3s/500ms targets remain unchanged.

Package-final is composite: existing e6ff394 app/target DLL, rebuilt host/broker,
source-hashed rHWP extension. Not a fresh whole-native build or release. Manifest
addon hashes and build receipts distinguish the layers. Full Rust suite/hosted CI,
previous broad language corpora and real Hancom/physical IME were not rerun.
Current Source package is stale. Historical AppData recovery is still unresolved;
this work never read/changed host settings/dictionary/user documents. Guest cleanup
was restricted to the marked C:/PencilTrialScope tree; installation/profiles and
registration0, product processes0, exact Sandbox stopped and inventory0.

Next: broaden semantic-style corpus (negation, quoted/duplicated words, additional
character properties/languages) without weakening ambiguous-anchor refusal;
qualify remaining document adapters and optional Provider stability per roadmap.
Do not silently promote this candidate into personal installation/main/release.


## 2026-10-18 repeated/quoted/negated rHWP selection — NOT QUALIFIED

Owner requested actual Gemma HWP/HWPX translation/natural/polite writing with
repeated words, quotes, negation, word-internal emphasis, partial paragraph and
partial-span selection. Evidence: `D:/dev/grammar-evidence/rhwp-repeat-sandbox-20261018`.
One GPU Sandbox was reused sequentially and stopped; final inventory0. No host
user settings/dictionary/documents or personal installation were read/modified.

Actual installed attempts failed; there is NO new18-case/57-check acceptance.
Retained failures include literal_changed, placeholder-induced Korean particle
error, truncated segmented output, first-occurrence instead of second-occurrence
emphasis, altered fixed text, and invalid/missing style anchors. Earlier12-case
mechanical success is NOT semantic acceptance. Saved files left in reused output
folders have old timestamps; only each attempt's own receipt identifies its
results. Raw diagnostics also carry stale installed receipts: their scope is
RAW_SYNTHETIC_DIAGNOSTIC_NOT_PRODUCT_ACCEPTANCE, never an installed test PASS.

Confirmed cause: JSON serialization quotes could be confused with source literals,
producing the whole unchanged source wrapped in quotes. The retained writing
prompt separates decoded text from verbatimLiterals. Exact quote contents,
delimiters, order and count are still guarded. Three synthetic Korean translations
preserved quoted DRAFT twice, negation, before/after approval and repetition in raw
model diagnostics; this small corpus does not prove broad quality.

Style alignment is unresolved. Numeric occurrence output chose the wrong repeat.
Unique-context metadata avoided model-counted offsets, but actual outputs included
invented suffixes. A subsequent schema/prompt variant removed these suffixes yet
expanded emphasis to a Korean verb ending and omitted the second paragraph's style.
The joint change does not isolate schema versus prompt causality. Failed alignment
implementations and packages remain external evidence, NOT a deployable candidate.

Retained product change: ambiguous repeated non-default source spans now refuse
with RICH_REPEATED_STYLE_UNQUALIFIED before inference. Prior sparse-anchor product
protocol is preserved. Experimental context decoder/prompt is cfg(test) only;
the ignored raw diagnostic requires WDAGUtilityAccount, explicit owned-synthetic
marker, pinned runtime and full model hash, and fixed synthetic inputs. It is not
callable through the release product. A partial metadata response now fails the
experimental decoder if any requested style is missing. Mutation protocol, original
8s bound, model12B Q4_0 and1s/3s/500ms targets were not weakened.

Final focused adapter59PASS; native Gemma25PASS7ignored (including preserved prior prompt contract). Native host results and
compiler-helper cleanup receipts are in FINAL_RECEIPT.json. This is not a full Rust
suite, Windows CI, final installed-package qualification or release. Earlier
experimental adapter62PASS and host tests must not be substituted for retained-code
results. Compiler helper residuals were terminated by the owned Job supervisor;
FAIL_LEFTOVER_CHILDREN with cleanup COMPLETE0 is not natural compiler cleanup PASS.

No main adoption, push, release, personal install or refreshed Project Source.
Mission remains INCOMPLETE and Source stale; prior AppData recovery remains
unresolved. Review was by the same integrator; no independent reviewer.

Next: replace free-form style allocation with required per-style semantic anchors,
verify minimal emphasis boundaries and clause identity, then rerun the18 installed
cases plus previous non-repeated corpus. Do not enable repeated-style inference
until actual model semantic review and Accept/Undo/save/reopen checks pass. Current
retained guard is unit-tested; its new GUI refusal was NOT_RUN after final cleanup.


## 2026-10-18 required style slots — repeated selection scoped qualification

Supersedes the earlier repeated-selection NOT_QUALIFIED checkpoint only for the
following actual-model synthetic corpus. Evidence: `D:/dev/grammar-evidence/rhwp-slots-20261018/FINAL_RECEIPT.json`.
Final installed Gemma4 12B IT QAT Q4_0, pinned rHWP Native Messaging, unchanged
installed extension, offline GPU Sandbox: repeated corpus18 cases/57 checks PASS.
HWP/HWPX x inline/partial/partial-span x Korean translation/natural/polite writing.
Quote DRAFT twice, prohibited-before versus permitted-after approval, and second
check/report emphasis preserved. Korean check maps only to 확인 (not the imperative
ending); report maps only to 보고서 (not its particle). Exact saved whole-document
text/character/paragraph/style/outside-selection equality, Undo and reopen pass.
Protected admission2 and no HTTP requests are included. No test analysis bridge.
Same final product bytes also passed prior non-repeated corpus18 cases/57 checks.

Implementation: required response key for each source style item; lexical and
unique-clause context choices come from immutable fixed output. The model selects
meaning, while code computes occurrence positions. Missing/unknown keys, missing
or ambiguous context, invalid lexical candidates, overlaps and altered fixed text
refuse. Source markings escape document markup. Repeated source spans alone select
this new path; existing non-repeated style protocol remains. Narrow transparent
Korean ending/particle handling is NOT a general morphological analyzer. Unknown
languages/morphology, identical clauses without unique context and arbitrary rich
Hancom documents remain unqualified. Human review before Accept remains required.

Initial installed attempt refused paraphrase because context copied original
'after approval' while fixed output said 'following approval'. Context enumeration
repairs this without fuzzy repair, fallback or extra inference. The original unit
failure on 보내세요 is retained and fixed by the bounded ending table. Two subsequent
installed failures were fixture errors: before/after-only matching rejected valid
prior-to/following, and 하지-only negation counting missed 보내지. Their receipts are
retained; the final fixture allows those equivalents, keeps negation counts and
order checks, and STRENGTHENS Korean emphasis to exact lexical spans. No deadline,
model size, quote guard or mutation guard was relaxed. All final36 outputs were
reviewed by the same integrator; some natural/polite outputs conservatively retain
wording. This is not broad language quality or independent human review.

Repeated recommendation1897-2463ms, p50=1938,p95/p99=2463;1s misses18/18,3s misses0.
Accept38-76ms,p50=63,p95/p99=76;500ms misses0. Targets unchanged. Staging20.493s and
readiness11.926s are separate measured phases, not total install/cold-start time.
Final focused adapter61PASS, Gemma26PASS7ignored, host29PASS1ignored; release native
build success. Not a full Rust suite or hosted CI. Native compiler helpers remained
and the owned supervisor cleaned them: FAIL_LEFTOVER_CHILDREN, cleanup COMPLETE0,
not natural compiler cleanup PASS. Product app exited naturally with launcher0;
both final guest runs uninstalled with process/install/profile0. Exact single
Sandbox7178ebc1-6fa6-48b0-8ada-543dd9a62638 stopped; inventory0.

Source/runtime provenance: package-scoped is a composite of existing e6ff394 app/
target DLL and newly built host/broker plus source-hashed rHWP extension. Its file
manifest matches every runtime byte; native source hash matches this implementation.
Final verification harness hash is in FINAL_RECEIPT separately (fixture assertions
changed after package build). model-ready.json retains a preliminary FAIL default;
its ready phase is a timing snapshot only. Final app-result/installed/guest receipts
all PASS; no status was rewritten. Saved fixtures re-read with pinned rHWP WASM,
not independent Microsoft/Hancom application compatibility.

No host user data/settings/dictionary, personal install, main adoption, push or
release. Prior AppData recovery unresolved. Mission INCOMPLETE and Project Source
stale. Next: broader semantic/format corpus and remaining application adapters,
then optional Provider stability; do not extrapolate this corpus to general HWP.


## 2026-10-18 mixed repeated-word refusal checkpoint

2026-10-18 mixed repeated-word guard: reproduced unsafe candidate forcing when only some repeated source words survive rewriting (red unit FAIL retained). Production schema and decoder now refuse changed nonzero occurrence counts; this narrows coverage, does not implement mixed-synonym mapping. Same-count and all-replaced paths unchanged, identical clauses remain ambiguous. New actual-Gemma installed rHWP repeated corpus18 cases57 checks PASS on updated host/broker: emphasis, translation/natural/polite, Accept/Undo/full-format save/reopen and protected admission. Recommendation1915-2481ms, p50=1951,p95/p99=2481,1s misses18/18,3s0; Accept41-104ms,p50=61,p95/p99=104,500ms0. Asset staging78.590s and readiness12.216s separate. Adapter61, Gemma27PASS7ignored and nativeHost29PASS1ignored; native build exit0, compiler-helper residuals scoped cleanup COMPLETE0 (supervisor FAIL_LEFTOVER_CHILDREN). Single Sandbox8711ebb7-146b-4446-8812-b1cf0ba9fb4b stopped; product natural exit0, uninstall/process/profile/registration0. No host user data/personal install/push/main/release. Evidence rhwp-mixed-20261018/FINAL_RECEIPT.json; mission INCOMPLETE,Source stale.

Review: same integrator, not independent review. The new regression fails on the
previous implementation and passes unchanged after repair. Both schema construction
(before alignment inference) and response decoding call alignment_slots. Ordinary
writing may already have run; this is not a claim of zero total inference. No
mutation protocol, prompt, model, token cap or timing threshold changed. Mixed
substitution/deletion/addition stays unqualified and may reject valid rewrites.
The existing non-repeated corpus was NOT rerun on these bytes (previous evidence
remains historical); current actual installed qualification is repeated18 only.
Actual outputs preserve negation/order/quotes/emphasis in this synthetic corpus.
Korean literal 'tail' wording remains unnatural and some rewrites remain close to
source. Mechanical PASS is not broad language-quality acceptance or Hancom support.
Next: occurrence-aware mixed-synonym alignment with adversarial meaning checks,
including identical clauses, before expanding support. Then qualify against both
corpora on one package. Saved reread uses pinned rHWP, not independent Hancom.
Composite package retains e6ff394 app/target DLL and prior extension; only rebuilt
host/broker are new, with local_gemma.rs hash and file hashes in MANIFEST. No claim
that the entire app was rebuilt. Previous AppData recovery remains unresolved.


## 2026-10-18 unique-clause mixed-synonym checkpoint

2026-10-18 mixed-synonym unique-clause alignment: source marked text reconstructs original exactly; only a unique unchanged clause frame can bind a partially substituted repeated word. Schema and decoder share the binding; missing/ambiguous source, changed negation/context, duplicate/deleted identical clauses refuse. Actual Gemma installed repeated HWP/HWPX18 cases57 checks PASS plus mixed-synonym HWP/HWPX2 cases9 checks PASS: literal check preserved, styled check became review, exact Accept/format/Undo/save/reopen. Separate actual-model fixed-stage alignment3/3 PASS is NOT end-to-end inference evidence. Recommendation repeated1912-2967ms,mixed1914-1938ms;1s misses20/20,3s0. Accept repeated45-101ms,mixed68-73ms;500ms0. Adapter62,Gemma27PASS7ignored,host29PASS1ignored; native build exit0. One offline Sandbox stopped,guest product/install/profile/registration0, app natural exit0. No user data/personal install/main/push/release. Scoped English unique-frame support only,not arbitrary synonym/translation alignment or general Hancom. Evidence rhwp-frame-20261018/FINAL_RECEIPT.json; mission INCOMPLETE,Source stale.

The previous blanket nonzero count-change refusal is replaced only for unique
unchanged English clause frames. This does not prove the generated replacement is
a synonym: ordinary writing, explicit user review and scoped quality evaluation
remain necessary. Wider paraphrases that change the frame still refuse; equal-count
semantic remapping, translated frames retaining source words inside literals,
other morphology and arbitrary styled phrases remain unqualified. Same integrator
review only, not independent reviewer. No prompt/model/token/deadline relaxation.

Retained failure: attempt3-mixed-fixture-FAIL matched `not` inside `note` and stopped
before Accept. The corrected word-boundary assertion still rejects actual not,
never and tomorrow. Attempt4 repeated the same actual model/package and passed;
no output substitution, replay, alignment bridge or inference stub. New mixed cases
are natural rewriting, not mixed-synonym translation qualification. The existing18
cases cover actual translation/natural/polite with repetition/quotes/negation.
Raw attempt1 uses three prescribed fixed-stage inputs and a test executable; it is
separate from both installed journeys. No ordinary non-repeated corpus rerun this
turn, no full Rust suite/hosted CI/physical IME/Hancom acceptance. Preparatory
ExistingLogin launch initially returned 0x80070520; retry after login initialization
used the SAME Sandbox, with no second session. Preliminary/stale output files are
not acceptance; use the archived attempt receipts with their explicit scopes.

Composite package retains prior e6ff394 app/target DLL and extension, with rebuilt
host/broker tied to local_gemma.rs SHA256 in manifest; all file hashes checked.
Harness updated after packaging has its own evidence hash. Compiler helpers caused
FAIL_LEFTOVER_CHILDREN with build/test exit0, supervisor cleanup COMPLETE0; not
natural compiler cleanup PASS. Product app natural exit and guest uninstall were
separately checked. Previous host AppData recovery remains unresolved; nothing in
host user data was read/modified for this task.

Next: broaden mixed-synonym clause rewriting only with source/meaning/occurrence
counterexamples; retained-source literals in translated clauses need separate
qualification. Preserve exact-context rejection until that evidence exists. Global
adapter work and Provider stabilization remain priorities; HDD latency is deferred.


## 2026-10-18 rewritten request-context checkpoint

2026-10-18 bounded rewritten-request context: mixed check/verify/review can align through polite/article changes and today placement while ordered content, negation/modality/numbers, case-sensitive identifiers, question status and literals remain checked. Union exact and normalized candidates; ambiguity refuses. Final actual Gemma installed HWP/HWPX: repeated18/57 PASS, changed-prefix2/9 PASS, both-side-context2/9 PASS (22 successful cases75 checks). In final both-side case Kindly check today's report, please became Please kindly review today's report; literal check unchanged and emphasis only on review. Accept/full format/Undo/save/reopen confirmed. Recommendation1914-2568ms,1s misses22/22,3s0;Accept40-81ms,500ms0. New scope English requests only,not arbitrary/multilingual semantic mapping; Please kindly redundancy remains a fluency limit. One other case retained check and failed the strict synonym-exercised fixture before Accept; not counted as new-feature PASS. Unit ambiguity failure repaired; final adapter62,Gemma28PASS7ignored,host29PASS1ignored. Build/tests exit0 with compiler helper FAIL_LEFTOVER_CHILDREN, supervisor cleanup COMPLETE0. Single Sandbox stopped; installed product natural exit0, owned install/profile/process/registration0. No user data/personal install/main/push/release. Evidence rhwp-context-20261018/FINAL_RECEIPT.json; mission INCOMPLETE,Source stale.

Current qualification is scoped: prefix and both-side changes exercised natural
English rewriting in installed HWP/HWPX; the previous18 corpus supplies Korean
translation/natural/polite regression evidence. Do not describe this as new mixed
Korean-frame qualification. The normalized rule is deliberately finite, not fuzzy
semantic similarity or unrestricted synonym inference. Before/after markers,
role tokens, numbers and negation cannot be discarded; meaningful token case and
uppercase identifiers (A, US) are retained. Exact-frame and normalized candidates
are combined and deduplicated before requiring one match. Tests cover added
negation, changed day/object/numbers/modality, swapped actors, question changes and
multiple plausible clauses. Question punctuation retained on broadened path.

Initial unit failure selected an exact-frame candidate while another normalized
candidate existed; fixed before installation. Raw actual-model alignment5 cases
passed before the case-sensitive identifier tightening; those receipts are marked
pre-case-review and are NOT final runtime qualification. Final build/source hashes
and installed tests are separate. No model/prompt/token/deadline relaxation, added
inference request or synthetic response bridge. The unchanged-word trial is
attempt4-synonym-not-exercised: model kept check, so strict coverage assertion failed
before Accept (HWPX not reached). Its source files and failure remain. A distinct
Kindly...please synthetic input was then tested, with assertions still requiring
both changed target context sides AND a synonym. Never count the retained-word
case as that feature's PASS. All recorded output language checked by the same
integrator, not an independent reviewer; Please kindly and Korean literal tail
phrasing show fluency limitations. No broad language-quality acceptance claim.

Final runtime is source-hashed new host/broker plus unchanged prior e6ff394 app,
DLL and extension, not a full app rebuild. Package files and source hash verified;
final harness hash is recorded separately. Asset staging and readiness are in
receipts separately from recommendation/Accept timing. Adapter62 + nativeGemma28
(7 ignored) + host29 (1 ignored), not full Rust/hosted CI. Non-repeated corpus,
physical IME, user profiles, arbitrary rich documents and actual Hancom NOT_RUN.
Single Sandbox d1fe3683-4c70-4b5c-b486-6e9c50a56d9f reused serially and stopped;
guest cleanup0 after every attempt. Host AppData recovery remains unresolved.

Next: wider clause paraphrases and bilingual retained-literal alignment need
separate meaning/occurrence evidence before activation; current unknown/ambiguous
cases refuse. Improve phrasing quality independently of format/Apply checks.
Keep document compatibility ahead of Provider stabilization and deferred HDD tuning.


## 2026-10-18 Korean retained-literal and approval context checkpoint

2026-10-18 Korean retained-literal/context checkpoint: a unique nonliteral styled occurrence can align to Korean without inheriting emphasis from a preserved quoted label; source reconstruction and quote-overlap rejection remain mandatory. Bounded Korean action contractions retain only 확인/검토/점검 stems. Simple approval requests allow before/prior-to and after/following/once-approved equivalents while retaining event direction, role/order, negation, numbers and ambiguity refusal; compound predicate movement is unqualified. Fixed a leading-clause-space mismatch in this new rule. Prompt changes remove adjacent and separated courtesy duplication and clarify full-body translation; literal-only body loss now refuses before publication. Actual Gemma final installed HWP/HWPX new18/57 + existing repeated18/57 PASS (36 cases114 checks): translation/natural/polite, exact emphasis, numeric tokens/literals/meaning fixtures, explicit Accept, full char/paragraph/style/outside-selection equality, Undo and saved/reopened equality. Recommendation1408-2466ms,p50=1928,p95=2455,1s misses36/36,3s0;Accept38-102ms,p50=64,p95=93,500ms0. Final raw diagnostic18 alignments +5 fixed alignment checks PASS separately, not installed acceptance. Adapter62,Gemma29PASS7ignored,host29PASS1ignored; final supervised native tests/build PASS and cleanup COMPLETE0. Single Sandbox c02233e3-af03-422f-9fb0-bb96ac1304b6 stopped; natural app exit0, guest install/profile/process/registration0. Host user data untouched. Local composite candidate only, mission INCOMPLETE,Source stale. Evidence rhwp-multilingual-20261018/FINAL_RECEIPT.json.

Evidence: `D:/dev/grammar-evidence/rhwp-multilingual-20261018/FINAL_RECEIPT.json`;
`attempt6-final-PASS` has installed receipts, actual synthetic model responses,
saved accepted/Undo/reopened documents, and uninstall evidence. `package` retains
the e6ff394 base app/DLL/extension; only host/broker rebuilt from source-hashed
local_gemma.rs. This is not a fresh full desktop-app build or personal installation.
The committed `scripts/mission/rhwp-multilingual-cases.json` records synthetic inputs
and semantic checks; the installed runner now supports named fixtures and tests
one courtesy marker per clause, excluding immutable quoted literals.

Retained failures: attempt1 styled 확인해 instead of 확인 (before Accept); attempt2
returned only the quoted label for approval translation and still used separated
Kindly ..., please; attempts4/5 refused an otherwise valid approval paraphrase.
The final raw diagnostic identified the exact Once approved/review response;
whole-clause unit coverage exposed leading-space normalization mismatch. All these
logs remain failures. Initial compile-quoting and unit regressions remain in tests1,
tests2; tests8's last-line-JSON test was corrected to require one exact decoded JSON
input plus the new trailing full-body instruction. No semantic assertion was removed.
Raw diagnostic test process success alone does not assert all response quality:
attempt3 had a translation/alignment error despite its process exit0. Final all18
alignment results and actual installed semantic assertions were checked separately.

Qualification remains bounded: Korean retained-literal path requires one unquoted
source occurrence and no remaining unquoted ASCII word in translated output.
Semantic alignment is model-assisted, not a proof of arbitrary equivalence. English
new event normalization is only for simple request frames; unknown or ambiguous
frames refuse. Politeness improvement is confirmed for these fixtures, not every
language/register. General multilingual/compound-predicate rewriting, general
Hancom, physical IME/user profiles, cloud Provider and full Rust/hosted CI NOT_RUN.
Review was a separate same-integrator diff pass, not an independent reviewer.

Final asset staging15.486s and app readiness9.841s followed the diagnostic run in
that same VM; these are not cold-start benchmarks. Earlier stage times23.404,
197.519,173.627,108.831,103.062s are preserved. No HDD tuning or timing threshold
change. Only owned guest profiles were removed; host AppData recovery remains
unresolved. No personal install, main adoption, push or release this checkpoint.
Next: broaden semantic/occurrence evidence for mixed-language names, more complex
clauses and Korean rewrite contexts without allowing quote-based misalignment;
retain document compatibility priority before Provider stabilization/HDD work.


## 2026-10-18 Korean compound names / same-clause emphasis

2026-10-18 Korean compound/name checkpoint: quoted label and styled action in the same clause now disambiguate through a validated unique nonliteral source occurrence and exact unquoted fixed contexts; quoted overlap, duplicate nonliteral ambiguity and target mutation fences remain. Korean rewriting preserves case-sensitive ordered ASCII identifier tokens, conservatively refusing changes, not general entity recognition. Actual Gemma installed HWP/HWPX Korean natural/polite12/39 + prior multilingual18/57 + repeated18/57 PASS (48 cases153 checks), full saved char/paragraph/style/outside selection, explicit Accept, Undo and reopen. Output review confirms Pencil/Gemma/김민수/이서연, numeric25, approval-before/after, prohibition and sender/reviewer roles in three templates. New live cases retained 검토; no new live-synonym claim. Natural roles rewrite becomes more formal; Q18 TODO. Recommendation1901-2967ms,p50=1942,p95=2955,1s misses48/48,3s0;Accept40-104ms,p50=58,p95=83,500ms0. Stage34.308s, readiness9.038s after raw diagnostic, not cold start. Raw6/6 alignment and guard PASS separately; Gemma31PASS8ignored,host29PASS1ignored,adapter62PASS; supervised native build/tests PASS cleanup COMPLETE0. Red alignment regression and two refused installed attempts preserved. Final app natural0, owned install/profile/registration/process0, single Sandbox cf84966b-655c-4030-ad04-5cdd6884fe26 stopped. User data untouched; prior AppData recovery unresolved. Composite rebuilt host/broker plus unchanged e6ff394 app/extension, not full app rebuild. No push/main/release/personal install. Evidence rhwp-korean-names-20261018/FINAL_RECEIPT.json; mission INCOMPLETE,Source stale.

- Reproduction: actual model kept both quoted 검토 and action 검토 within one
  sentence. The old full-clause uniqueness check rejected all six raw rows and
  stopped the installed path before Accept. `same-clause-red` preserves the failing
  production schema test. Fixed schema uses only exact unquoted substrings after
  source reconstruction, and decoder independently rejects quote overlap.
- Mixed ASCII names are admitted only for Korean source with exact ordered ASCII
  tokens. These rules can conservatively refuse legitimate reorderings. They do
  not prove every Korean name, role or meaning is unchanged; actual corpus review
  is in `korean-quality-review.json`, limited to three named templates.
- `승인하면` is accepted as the equivalent approval-conditioned variant of
  `승인한 뒤` in this request; it is not permission to drop or invert approval.
  Corpus assertions added that equivalent after the raw diagnostic, before the
  final installed run. No test deadline, model, integrity check or mutation fence
  was relaxed. Before-approval prohibition remains independently checked.
- Final raw diagnostic and installed test are separate. The model is staged and
  checked on every attempt; three stages were 15.286s,17.006s,34.308s. Final app
  readiness follows the raw test, so it is not a cold-start measurement.
- Package manifest binds new broker/host to local_gemma.rs SHA and build2 receipt;
  e6ff394 base app/extension remain unchanged. All payload file hashes verified.
  External FINAL_RECEIPT records final source commit after local commit; no
  self-referential HEAD is put in tracked state. Project Source remains stale.
- Review: same integrator reviewed source/schema/decoder and all12 actual Korean
  outputs; no independent reviewer. Existing compiler warnings remain; none point
  to touched local_gemma code. No frontend changes or full-app build this turn.
- Next: Q18 preserve natural Korean register on held-out names, nested conditions
  and larger selections; then qualify wider mixed-name translations/rewrites with
  the same full-document checks. Unknown semantic alignment stays fail-closed.
  Do not infer general Hancom support, user-data recovery or desktop bug closure.


## 2026-10-18 Korean request register and wider conditions

2026-10-18 Korean natural-request register checkpoint: natural paraphrase now carries recognized source terminal request endings in its prompt and rejects changed ending sequences before publication; no destructive postprocessing/retry, polite mode unchanged. Actual prior7258aaf roles case 해 주세요->해 주시기 바랍니다 is repaired on installed HWP/HWPX. Wider synthetic names with conjunction, missing-condition disjunction, nested approval and concessive prohibition: new8/27 + prior Korean12/39 + multilingual18/57 + repeated18/57 PASS (56 cases180 checks). Actual Gemma4 12B via installed Native Messaging; exact emphasis/full saved char/paragraph/style/outside selection, explicit Accept, Undo and reopen. New endings 해 주세요/하지 마세요/해 줘/하지 마십시오 and tested names/roles/numbers/conditions preserved. Raw10/10 guard+alignment checked separately, not installed acceptance. Recommendation1409-2960ms,p50=1963,p95=2959,1s misses56/56,3s0;Accept35-96ms,p50=61,p95=80,500ms0. Final stage19.511s,ready12.443s,not cold-storage proof. Gemma32PASS8ignored,host29PASS1ignored,adapter62PASS; native supervisors PASS cleanup COMPLETE0. First fixture FAIL lacked equivalent 없다면; retained and broadened only to reviewed equivalent conditional expressions, with negation/roles/endings retained. One overlapping build attempt was refused before launch; next serial build passed. Single Sandbox5acd098d-ec31-4f82-b973-4d2576097a60 stopped, viewer0; final app natural0, owned product/install/profile/registration0. Host user data untouched. Bounded endings are not full Korean register/entity/semantic qualification; Q18 remains IN_PROGRESS. Composite new host/broker with e6ff394 app/extension unchanged; no main/push/release/personal install. Evidence rhwp-korean-register-20261018/FINAL_RECEIPT.json; mission INCOMPLETE,Source stale.

- Before/after: `register-before-after.json` binds actual installed roles-case
  outputs from7258aaf and this candidate. Natural rewrite now ends 해 주세요;
  explicitly selected Polite still produces 해 주시기 바랍니다, as expected.
- Implementation is a bounded list of terminal request endings, extracted outside
  straight-quote/backtick literals. Only Paraphrase gets the prompt constraint and
  guard. Unknown declarative/question endings and interior honorific changes are
  not covered. No post-generation ending replacement or automatic model retry.
- New fixtures use Naver Cloud/Orion-7, 한빛연구소/Mira-2, 서울연구팀/Atlas and
  OpenAI/Vega-3 with synthetic Korean names. The actual output review checks who
  sends/approves/reviews, AND completion, missing either prerequisite, approval
  ordering and prohibition despite sending. Full saved text/format/Undo/reopen
  checks are unchanged. All action stems stayed 검토; no new synonym claim.
- First installed run passed two cases then the test rejected 없다면 although
  equivalent to source 없으면. Raw output review also justified 보내고 and 없는 경우
  as condition-preserving variants. Added these alternatives before the final full
  run; did not drop negation, only-if, name, numeric or ending assertions. The
  failed receipt is retained in attempt1-fixture-FAIL. This is a fixture failure,
  not evidence of a product mutation defect or a passed initial run.
- Raw diagnostic10/10 was observed in the first attempt; the final installed run
  did not repeat it. Model staging61.567s then19.511s remains separate from ready
  and request timing; no HDD/cold-start improvement claim. One early build attempt
  returned SESSION_ALREADY_RUNNING with no child launch; serialized build2 passed.
- Final package manifest hashes all files; unchanged e6ff394 app/extension plus
  local_gemma.rs-SHA-bound host/broker from build2. Final source commit is written
  only to the external receipt after commit. No full frontend/app rebuild or CI.
- Same-integrator source/guard and output review, no independent human review.
  Existing compiler warnings remain; this is not a lint-clean/full-suite claim.
- Next Q18: held-out declarative/question registers, mixed speech levels and
  interior honorifics; qualify with real local model and the same full-document
  acceptance checks. Preserve unknown-alignment refusal. General Hancom, physical
  IME, real user documents and earlier AppData recovery remain unqualified/open.


## 2026-10-18 Korean statement/question and honorific checkpoint

2026-10-18 Korean statement/question/honorific checkpoint: bounded terminal predicate plus question punctuation and honorific-word sequence preservation for natural rewrite; explicit Polite unchanged. Actual installed local Gemma HWP/HWPX new8/27 + prior56/180 PASS (64 cases207 checks), explicit Accept/full saved char+paragraph+style/outside-selection/Undo/reopen. Tested 검토했어요., 검토하셨나요?, mixed 검토하셨습니다./확인했어., 선생님께/드렸고. Unknown endings/interior inflections and general semantics remain unqualified; conservative exact predicates may refuse valid synonyms. First fixture failed on style-preview separators; second run refused 드렸고->드렸으며, diagnosed with real raw output and repaired by stronger prompt without relaxing guard; later formal fixture added equivalent 없다면. All failures preserved. Final recommendation1896-4993ms,p50=2426,p95=2966,1s misses64/64,3s misses2/64;Accept37-111ms,p50=60,p95=104,500ms misses0. Stage56.933s,ready11.944s,not cold-storage proof. Gemma33PASS8ignored,host29PASS1ignored,adapter62PASS; native supervisors COMPLETE0. App natural0, extension65files unchanged, owned install/profile/registration/process0; single Sandbox9673017b-5b81-4d80-8853-91ffdc99c2cb stopped, viewer0. Host user data untouched; prior recovery unresolved. Composite rebuilt host/broker with unchanged e6ff394 app/extension. No main/push/release/personal install/CI. Evidence rhwp-korean-speech-20261018/FINAL_RECEIPT.json. Q18 IN_PROGRESS; mission INCOMPLETE, Project Source stale.


## 2026-10-18 Korean closed-equivalence and synonym-anchor checkpoint

2026-10-18 Korean closed-equivalence checkpoint: natural rewriting now compares five exact equivalent pairs while preserving tense/question/speech level and ordered honorifics; source-derived allowed choices replace contradictory immutable/exception instructions. Actual Gemma changed 드렸고->드렸으며 in2 installed cases and 송신->전송 in10, preserving quoted terms, names,27, channels and honorific predicates. A destination synonym colliding with quoted 전송 exposed style alignment refusal; failed regression and raw model response prove source-only routing and independently selectable duplicate contexts. Final known-pair routing requires one nonliteral action, restricts synonym candidate and unique unquoted context, rejects quote overlap and ambiguous duplicates. Final candidate Korean40 cases132 checks PASS: new12/39 + speech8/27 + conditions8/27 + names12/39, explicit Accept/full saved text,char+paragraph+style/outside selection/Undo/reopen. Earlier build2 English-source36/114 PASS separately; not final-binary monolithic76. Final recommendation2404-2971ms,p50=2444,p95=2951,1s misses40/40,3s0;Accept41-104ms,p50=62,p95=87,500ms0. Stage16.411s,ready11.939s,not cold-start/storage proof. Gemma35PASS8ignored,host29PASS1ignored,adapter62PASS; final native supervisors PASS COMPLETE0. Original 님 refusal, fixture 때만 failure, alignment failures and intermediate red tests preserved. Equivalence comparison deliberately broadened only for the five named pairs; target/privacy/time boundaries unchanged. Same-integrator review only. App natural0,65 extension files unchanged, guest install/profile/registration/process0; single Sandboxca621d98-7d0b-43ca-ab37-aabd97f17870 stopped, viewer0. Host user data untouched; prior recovery unresolved. Composite new host/broker plus unchanged e6ff394 app/extension. Q18 IN_PROGRESS for broader semantic/morphological qualification; 말씀하셨고/말씀하셨으며 has unit coverage only. No main/push/release/personal install/CI. Evidence rhwp-korean-equivalence-20261018/FINAL_RECEIPT.json; mission INCOMPLETE, Project Source stale.

## 2026-10-18 Korean semantic-boundary checkpoint

2026-10-18 Korean bounded-semantic checkpoint: closed reply/question synonyms and additive/conditional forms preserve recognised tense, negation and honorifics; ordered action/negative/condition fences for reply/question scope plus known connector fences across Korean natural rewriting. Actual final build4 installed local Gemma HWP/HWPX60 cases/198 checks PASS, including20 new cases and40 regressions, explicit Accept/full saved text+char+paragraph+style/outside selection/Undo/reopen. Real new styled choices: {'답신': 8, '회신': 4, '질의': 6, '질문': 2}; unchanged words are not synonym-change proof. Recommendation1906-2983ms,p50=2434,p95=2957,1s misses60/60,3s misses0/60;Accept36-110ms,p50=61,p95=81,500ms misses0/60. Stage49.334s,ready12.119s,not cold-storage proof. Gemma37PASS8ignored,host29PASS1ignored,adapter62PASS; native supervisors COMPLETE0. Baseline red proves condition/negation drift previously accepted. Two earlier automatic document-PASS cohorts failed semantic review (added request/honorific and additive-to-restrictive connector); preserved and repaired, not counted as final acceptance. One exact Sandbox stopped, app natural0,65 extension files unchanged, guest install/profile/registration/process0. Host user data untouched; prior AppData recovery unresolved. Composite rebuilt host/broker, unchanged e6ff394 app/extension. Same-integrator review only. No main/push/release/personal install/CI. Evidence rhwp-korean-semantic-20261018/FINAL_RECEIPT.json; Q18 IN_PROGRESS, mission INCOMPLETE, Source stale.

- Added 답신/회신 and 질의/질문 for enumerated inflections, retaining 송신/전송.
  Connector classes include 확인했고/확인했으며, 확인하셨고/확인하셨으며,
  승인하면/승인한다면, 승인했으면/승인했다면, 유지하고/유지하며.
  These are bounded contextual choices, not a universal morphology system.
- Recognised connectors are checked across natural Korean rewriting. Reply/question
  scope additionally keeps ordered action/negative/conditional tokens, including
  unknown inflections exactly. Request endings, terminal predicates, question
  punctuation and honorifics remain checked. Explicit Polite mode is unchanged;
  no post-generation repair or silent retry.
- Early actual outputs inserted 유지해 주시고 or changed 유지하고 to 유지하되.
  The latter also appeared in existing ko-formal outside the initial narrow scope.
  Both intermediate document-PASS results remain semantic-review FAIL. The stronger
  ko-formal assertion and production regressions preserve this history.
- New anchor tests reject quoted labels and ambiguous duplicate actions for both
  new pairs. Installed tests compare full saved text/format, Undo and reopen/resave.
- Test-development failures: tests2 originally changed no JSON bytes because of
  escaping; corrected test asserts its mutation. tests4 expectation omitted newly
  protected 유지하고; fixed without removing quoted-text exclusion.
- Intermediate reply run logged a tray-icon removal error; app natural exit0 and
  cleanup0 were independently checked. Not claimed repaired by this language work.
- Next Q18: held-out nested conditions/roles and unlisted inflections, with
  independent linguistic review when available. Unknown expressions can refuse.
  General Hancom/user documents/physical IME remain unqualified; Q04 is deferred.

## 2026-10-18 Nested Korean roles and performance checkpoint

Korean nested-role checkpoint: ordered full surface frames preserve actors, recipients, objects, nested condition attachment, negation, tense, honorifics, punctuation and literal positions within bounded reply/question natural rewrites. Added 확인하면/확인한다면 and 확인했으면/확인했다면 after actual-model diagnosis of a valid form being refused. Final installed actual Gemma HWP/HWPX36 cases/120 checks PASS (8 new,28 regressions), explicit Accept/full saved text+char+paragraph+style/outside selection/Undo/reopen; all8 new styled actions became 회신 while quoted 답신 stayed unchanged. Recommendation1891-2981ms,p50=2421,p95=2960,1s misses36/36,3s misses0/36;Accept37-95ms,p95=83,500ms misses0/36. Host free RAM45.9-49.8GiB during final run, guest free13.6GiB snapshot, GPU aggregate10.6-11.1GiB/15.9GiB. Capacity exhaustion not observed; aggregate/coarse counters do not exclude contention. Raw separate pre-repair diagnostic shows writing1.26-1.34s and alignment1.03-1.17s of server prompt+decode time, not installed acceptance. Stage66.562s and ready12.771s separate. Gemma38/host29 assertions pass but strict native supervisors FAIL_LEFTOVER_CHILDREN (vctip.exe identified in diagnostics), complete cleanup0; focused Korean9 PASS1ignored and unchanged build confirmation PASS with clean supervisors; adapter62PASS. Earlier fixture/oracle failures and partial4 then over-refusal remain FAIL. Single exact Sandbox stopped, app natural0,extension unchanged,guest install/profile/process0,host sampler/owned process/viewer0. Composite unchanged e6ff394 app/extension plus rebuilt broker/host; not a full app rebuild. Host user data untouched, prior AppData recovery unresolved. Same-integrator review; no general Korean/Hancom acceptance, no cloud/CI/main/push/release. Evidence rhwp-korean-nested-20261018/FINAL_RECEIPT.json. Mission INCOMPLETE, Source stale, Q18 IN_PROGRESS.

- Scope is intentionally conservative: natural Korean reply/question text with at
  least two recognised condition/concessive markers. Closed substitutions preserve
  the entire ordered word/particle/punctuation frame, including quoted positions.
  This is not a Korean dependency parser; legitimate clause reordering or unlisted
  synonyms can refuse. Explicit Polite mode retains its existing different contract.
- Regression counterexamples cover actor exchange, recipient/topic and object/subject
  particle changes, moved negation, tense loss, cause replacing condition, added else
  branches, sentence-boundary changes and relocated quotations. Full-body installed
  oracles supplement exact names/numbers, emphasis and complete saved-format checks.
- Actual diagnostic preserved the failed 과거형 output: 확인했으면 became 확인했다면.
  Added the reviewed closed pair, not arbitrary stem stripping or a removed guard.
  The previous refusal stops the local model under the existing lifecycle contract;
  no crash/root-cause claim or lifecycle redesign is made by this language change.
- Test-development failures: generated fixtures initially went to the prior evidence
  payload; exact newly-created duplicates were hash-checked against this task's files
  and removed. Next oracle escaped regex syntax and matched review-line separators
  incorrectly; the joined full-frame oracle fixes that without weakening semantics.
  A separate raw diagnostic is never counted as installed editor acceptance.
- Native assertions passed but compilation/fixture activity left MSVC vctip.exe in
  owned Jobs. Strict supervisor failures remain failures; every owned Job terminated
  and reaped its actual leftovers. No unowned vctip or other host process was killed.
- Performance follow-up: prioritise a deterministic emphasis alignment fast path
  only for proven unique ordered substitutions, retaining current model alignment
  on ambiguity. Validate complete formatting/Undo/reopen and held-out meaning before
  activation; no weaker model, output truncation or persistent document cache.
  RAM upgrade alone is not supported by these measurements. Shared-load effects,
  per-request GPU attribution and cold-storage causality remain unproven.
- See the external PERFORMANCE_REVIEW.md, resource-summary.json and numeric raw
  diagnostic-phase-metrics.json. New CUDA/speculative/prefix-cache options are NOT_RUN.


## 2026-10-18 Deterministic alignment and Pencil publication candidate
Owner authorizes current source push and safe main merge; no new binary release.
Proven ordered lexical frames can compute emphasis without polling model inference;
all existing alignment decoding, source binding, meaning/register checks remain.
Unproven translation/rewrite retains the model path. Two new Rust tests cover closed
Korean forms, role/number changes, repeated words and lazy inference (zero polls for
proven frames; one for fallback). First test attempts failed on a missing internal
completion envelope field, repaired without changing assertions. Gemma unit suite
40 passed/8 ignored, but strict native supervisor FAIL_LEFTOVER_CHILDREN with one
owned child, actual cleanup COMPLETE/0; rHWP 62 passed, frontend/branding/governance passed. Installed
actual Gemma and hosted CI are pending, not yet qualified by this change.
Public display name is Pencil; legacy executable/settings/native-host IDs and the
Grammar correction mode remain compatible. Remote main has two README commits
(31a927a, df48758); preserve them during integration, no force push.
Evidence: D:/dev/grammar-evidence/rhwp-deterministic-pencil-20261018.


## 2026-10-20 Resumed publication checkpoint: actual Gemma and cleanup

Supersedes the pending qualification in the 2026-10-18 preparation entry above.
Built source dccc246d793816aade916e261d9c7ab2ecdf7b4f; fresh app/host/broker/target
build and source-bound final-package. Installed app and extension display Pencil;
legacy runtime/settings/host identifiers remain compatible. Owner authorizes source
commit/push/safe main merge; no new executable release in this checkpoint.

- Actual offline Sandbox Gemma 12B QAT Q4_0: first 54 cases/177 checks PASS;
  sequential same-package repeat 16 cases/54 checks PASS. HWP/HWPX synthetic rHWP
  Accept, complete text/character/paragraph/style and outside-selection preservation,
  Undo, save/reopen and protected-file refusal. Total70 cases/231 checks.
- First36 paired Korean outputs exactly match previous checkpoint. Recommendation
  median2421->1430ms, but p95 worsened2960->4693ms and max5199ms. First54 contain
  three >3s requests; all miss1s. Accept41-186ms, no500ms misses.
- Repeat16 outputs exactly match first trial. Recommendation p50=1974ms,p95/max=3019ms,
  one >3s and all16 >1s; Accept46-82ms. This does not erase first-run outliers.
  Shared-load observations are not a controlled hardware A/B or general quality
  qualification. No per-request model-call counter was collected in installed runs;
  lazy production dispatch regression proves zero inference polls for proven mappings.
- Stage69.760s/66.568s and readiness11.991s/11.558s remain separate cold costs.
  No smaller model, output truncation or weaker safety/meaning/format checks.
- Both product runs naturally exited0; extension files unchanged; owned installs,
  profiles and guest processes0. Same Sandbox stopped, inventory empty. Numeric
  sampler exited. No user data or other sessions touched. Repeat tray-removal warning
  remains unresolved despite verified natural exit/cleanup; do not claim it repaired.
- Native assertions40 passed/8 ignored and two builds compiler-exit0, but strict
  supervisors FAIL_LEFTOVER_CHILDREN (one owned child each, cleanup COMPLETE/0).
  Current leftover executable not identified. These are not clean supervisor PASSes.
  Final packaging PASS; rHWP62, branding, frontend, provider/worker, cleanup17 and
  governance checks PASS. Hosted Windows CI39109153303 at dccc246 PASS (also
  39108791792 at e8ad029); final documentation/main CI recorded externally after push.
- Remote main README commits31a927a/df48758 preserved in mergee8ad029. Outgoing
  source objects scanned; no evidence/model/runtime binary or credential payload.
  Integration uses non-force fast-forward only and preserves canonical untracked files.
- Same-integrator review, no new independent reviewer. Mission INCOMPLETE; Project
  Source stale. General Hancom/physical IME/live cloud remain unqualified. Prototype2
  release unchanged. Q18 remains open for held-out language coverage and latency tails;
  owned compiler child and tray warning are separate maintenance follow-ups.

Evidence: D:/dev/grammar-evidence/rhwp-deterministic-pencil-20261018/FINAL_RECEIPT.json,
REPEAT_RECEIPT.json, attempt1-54-PASS-latency-misses, attempt2-16-repeat-PASS,
final-build-receipt.json and final-package. Git/CI publication receipts are external
so the tracked document does not claim its own hash.


### 2026-10-20 Final-main CI retry observation repair
Main run39385229545 at3cb8f78 failed on FileNotFoundError in the launcher's test-only
retry-ticket poll. Candidate run39385177422 at the identical source passed. The
existing wait_phase already permits transient Windows replacement-open errors;
two bespoke fresh-ticket loops did not. Both now use a bounded helper retaining
original8s/10s budgets, requiring failed phase plus changed ticket; all later
non-null ticket, app/sidecar and cleanup assertions remain. Three deterministic
regressions cover replacement errors/stale tickets, fixed deadline on persistent
absence and malformed JSON refusal. No launcher/product code or package changed.
Local helper3 PASS. Local full34 run FAILED (17 subprocess UTF-8 decoding errors
and one owned-uninstall refusal); strict owned supervisor cleanup COMPLETE/0.
Do not relabel this as local integration PASS. Evidence launcher-ci-repair/trial.log
and supervisor-result.json; fresh hosted Windows results recorded externally.


## 2026-10-20 Incremental proactive pipeline candidate (Q19)
Owner requests changed-range analysis, obsolete-work cancellation and cached instant
recommendations. Chromium already had line windows/cache, but discarded replies
forced full-field refresh and pending replacement lost earlier unpublished edits.
DocumentSession now carries a bounded dirty interval through revisions, including
insertions/deletions, and clears it only on a current covering publication. Content
keeps one in-flight/latest pending request, cancels obsolete pre-admission work,
discards stale active replies, and renders retained cache before new inference.
Busy retries retain the dirty range (250ms cadence, bounded150 attempts), not full
field. Input/IME/field wipe/mode change revoke publication authority immediately;
no inference is triggered by cached suggestion lookup. Accept/source checks unchanged.

Important discovered limitation: native pipe disconnect currently causes Gemma's
hard-cancel owner to stop the model Job. A disconnect-per-keystroke implementation
was rejected before commit. Current local supersession retains the original bounded
port/deadline until the active inference ends, discards its result, then runs latest
work. This is NOT physical inference cancellation or a 1s performance qualification.
Next: request-level interruption with independently confirmed idle while preserving
warm model; then actual installed single-session typing/Gemma timing/Accept tests.
No runtime/model/quality prompt changes, no general editor monitoring activation.

Production JS tests: core227 existing assertions plus unpublished-window cases;
worker79, scheduling7 (including rapid-edit pump, stale response and cache-only lookup).
Native transport is inspected, not requalified by mocked worker transport. Actual
Gemma/GPU/installed browser for this candidate NOT_RUN; prior package is unchanged.
No Sandbox or user profile launched; no user data touched. Same-integrator review.
Mission INCOMPLETE, Source stale; retained worktree and existing main remain distinct.
## 2026-10-20 Q19 warm supersession engine checkpoint

Backend-only candidate: observe the owned single slot processing the request,
drop the HTTP operation, then confirm idle within the original deadline before
reusing the model. Unknown slot state or timeout retains hard Job cleanup.
If admission was not observed, finish and discard instead: idle before admission
cannot fence a queued POST. Hard cancellation/revocation remains unchanged.

Actual owned-host Gemma12B QAT Q4_0: three idle-confirmed cycles with the same PID,
request-start-to-idle 2890/2809/2801ms. Cancellation was scheduled at200ms, not
measured at receipt. Next grammar request300ms returned "She has two books."
This is NOT a 1s result or proof of tokens avoided versus natural completion.
No model/prompt/quality settings changed. Model/runtime pins were verified.
Model Job processes0, workspace removed, engine.stop acknowledged.

Gemma42 tests PASS/9 ignored; actual GPU assertions PASS. Strict supervisors
FAIL_LEFTOVER_CHILDREN with one owned child each, cleanup COMPLETE/0. Binary-only
unit repetition also had a leftover: identity unproven, not attributed to Cargo.
No remaining model process or Sandbox. User data untouched.

Chromium remains on active-drain: explicit supersession must still be carried
through native/pipe boundaries separately from hard disable/navigation, followed
by installed rapid-input/cache/Accept timing tests. No install/main/push performed.
Same-integrator review, mission INCOMPLETE, Source stale.
Evidence: D:/dev/grammar-evidence/gemma-warm-cancel-20261020, including
gpu/warm-cancel.json and per-run supervisor-result.json and trial.log.
Runtime cancellation semantics reviewed against upstream llama.cpp b11461
tools/server/server-queue.cpp and tools/server/server-context.cpp.

## 2026-10-20 Q19 Chromium explicit supersession transport

Input changes now send an exact id/epoch/revision supersede frame through the
production native actor, broker grant and per-request Windows pipe. The service
keeps soft supersession separate from hard waiter loss: EOF, permission revocation
and the original deadline still stop the engine even during idle confirmation.
Only the original matching operation is signalled; duplicate/late controls do not
cancel newer work. The native actor has an outer original-budget fence so repeated
control messages cannot starve deadline polling. No deadline/quality/model change.

Review found and repaired a field-disable gap: content already clears localActive
on source supersession, so field revocation also sends a document-scoped stop-local
without needing the retired request ID. It closes the local port synchronously.
Existing cached publication/explicit Accept source/revision checks remain unchanged.

Final focused results: Windows Gemma/service/pipe59 PASS14 ignored; host31 PASS1
ignored; JS worker+scheduling88 PASS, core227 plus incremental cases; frontend
typecheck/build and governance PASS. Actual Windows pipe tests cover soft completion,
EOF and permit revocation while draining; service deadline remains hard.
Actual Gemma12B pipe/service trial2 soft cancellations: request-to-ack2934/2898ms,
then grammar302ms ("She has two books.") through the same resident service.
8s writing budget used; next grammar used1s. This is NOT installed Chromium
typing-to-suggestion latency or proof of token savings. GPU test preceded final
preservation of the service-closed error label and added deadline/revocation tests;
final focused suites validate those follow-up changes. No browser/Accept qualification.

Every native supervisor invocation so far reports FAIL_LEFTOVER_CHILDREN despite
test exit0, then COMPLETE cleanup with0 remaining. Preserve this separate failure.
Real engine owner close succeeded and workspace was removed; leftover child identity
was not established in this round. No Sandbox, user profile, documents, installation,
push/main/release changes. Same-integrator review only. Mission INCOMPLETE; Source stale.
Evidence: D:/dev/grammar-evidence/chromium-soft-cancel-20261020 (gpu/pipe-supersession.json,
per-run supervisor-result.json/trial.log, js.log).

Next: qualify the paired extension/native candidate in one installed synthetic
browser session. Specifically test the proactive1s ceiling against measured~2.9s
soft cancellation: it can still require hard cleanup instead of warm reuse.
Do not activate or advertise a1s improvement based on the8s writing trial.
Investigate the strict test-supervisor leftover separately; do not count its
forced cleanup as natural product exit.

## 2026-10-20 Q19 actual Chromium incremental fixture

Updated the owned browser runner to require the supervisor's marked trial root,
use the hash-pinned D:/PencilAssets model/runtime, and default to release binaries.
The original fixture uses a generated extension key, loopback host permission and
a content timestamp observer. This is NOT an unmodified installed package or
activeTab toolbar qualification. All document text/profile/registration is synthetic.
No installed app, general browser profile or user settings changed.

Production binaries from05c7b97 were built in debug and release. Debug browser-r1:
functional PASS (including2 supersessions), latency FAIL (incremental1322-1330ms;
baseline max1095ms), natural cleanup0. Preserve that failure. Release browser-r2:
functional/timing PASS,2 supersessions816-818ms. Release browser-r3 adds quantity,
negation and past-date cases in both textarea/simple contenteditable:6/6 exact
latest-source suggestions and Accept rereads PASS. Recommendation805-827ms,
p50=818,p95/p99/max=827; Accept33-37ms,p50=35,p95/p99/max=37. Two baseline
recommendations661/928ms and Accept14/15ms. Zero target misses in final small corpus.
Cached hover/click and Accept made no new analysis requests. Six supersede frames
and six cancelled replies observed. Source change at Accept preserved external
edit; password refused; disable removed UI. No quality/model/prompt changes.

Do not equate short-request cancellation with the earlier long-paraphrase~2.9s
measurements: different input/workload, no controlled before/after speedup claim.
Debug model pin logs separate read9.32s/hash99.80s; release-r2 read1.26s/hash1.42s.
This demonstrates substantial debug hash overhead in that trial, not a general
resolution of D-drive contention or startup performance. Both retain full hash
verification. Release-r3 source-to-native timing remains recorded per request.

Browser-r1/r2/r3 all completed natural browser/broker exit, unique registration
removal and profile deletion, supervisor remainingBefore/After0. The build
supervisors still FAIL_LEFTOVER_CHILDREN then cleanup0; do not hide them.
No Sandbox or second concurrent trial was launched. Same-integrator review only.
Evidence: D:/dev/grammar-evidence/chromium-incremental-20261020; per-run browser
receipt.json contains binary/model/script hashes, timings and cleanup; build logs
identify profiles. Release host9cdcab4d827bef45283e3dc839cf62d72961c67c8f53af6d00612c44a48e4948,
broker47f33e3462fd97346b95921bc9dc13115bb8f73d6322268d6352073ae69142cd.
Node syntax and governance validation PASS. No product runtime changes this round.

Next: unmodified installed extension/native pair and actual toolbar opt-in,
continuous typing/IME and longer changed windows; measure cold/warm tails with
the same fixed targets. Current6-case fixture is not general1s qualification,
installed acceptance, main adoption or release. Mission INCOMPLETE; Source stale.

## 2026-10-20 Q19 unmodified installed Chromium qualification

Built clean bf2260a0daab7e7be5904c769c2aa8d39197cb45 into a fresh release target,
then generated the source-bound trial package. Build and package supervisors PASS,
natural remainingBefore/After0. Package ZIP SHA256
7fc863f3d450032c8d070ead2a3617d0150ad3e292c201c5bc4302c8d8ee00f1.
This Chromium-scoped trial package has no Writer binding: it is NOT a general
replacement personal release. Existing user installation/settings were untouched.

One Sandbox at a time, networking/clipboard redirection disabled, GPU enabled,
read-only source assets, explicit marked C:/PencilTrialScope install/profile/assets.
Final r5 loads the installer-written extension unchanged, with packaged key/ID,
activeTab/no host permissions and exact installer registration/host hash. Both
activations used real keyboard toolbar and popup actions (no permission injection).
Worker observation wrappers count messages; extension files stay unchanged.
Real Gemma12B: quantity, negation and past-date in textarea/simple contenteditable,
6/6 latest-source suggestions and exact Accept rereads PASS, six supersessions and
cancelled replies, no new inference for cached card/Accept. Disable/storage,
cross-origin revocation, fresh field opt-in and same-origin reload checks PASS.

Timing is NOT PASS: recommendations816-1330ms, p50=826,p95/p99/max=1330;
1s misses1/6 (contenteditable past-date). Accept62-80ms,p50=63,p95/max=80,
500ms misses0/6. Browser receipt status=PASS denotes functional checks only;
its incremental.latencyPass=false, child exit1 and app/guest/sandbox result FAIL
preserve the overall timing failure. Do not remove that case or infer its cause
from the approximately500ms difference. Next fixture records bounded native
request/supersession/response timestamps to distinguish scheduling/drain/inference.
No product timeout, model, prompt, integrity check or quality setting changed.

Environment failures preserved: initial overlapping read-only mappings returned
E_ACCESSDENIED; independent mappings started. Captured connect stdout stayed open
in the viewer, hanging subprocess communication: controller now uses file output.
r1 module directory lacked node_modules naming, r2 login session was not ready,
r3 mapped-asset validation failed before runtime launch (specific broker error
was discarded by launcher; not proven hash mismatch or proven storage cause),
r4 PowerShell5.1 policy blocked gesture script. No policy change: r5 uses existing
PowerShell7 for the helper. Same assets staged inside guest before normal full
product validation: copy69.420s/r4,211.653s/r5; app ready19.623s/r4,17.317s/r5.
These are deployment/startup timings, not recommendation timings or a resolved
D-drive contention claim. Earlier failures are separate from final functionality.

Final browser closed, native host0, profile removed, extension unchanged; app
launcher exit0. Uninstaller stopped0 processes and reports installation,
registration, explicit profile/assets and processes0, complete=true. Exact Sandbox
stopped, inventory0. No concurrent test sessions or user data cleanup. Initial
failed supervised Sandbox start had one child then cleanup0; do not conflate it
with the final natural product cleanup. Same-integrator review only.
Evidence: D:/dev/grammar-evidence/chromium-installed-incremental-20261020/
FINAL_RECEIPT.json, build-receipt.json, guest-r1 through guest-r5 (first guest is
named guest), sandbox-result*.json, fixture/, trial.wsb, run-sandbox.py.

Repository installed-package test now optionally runs the shared incremental
corpus via GRAMMAR_INSTALLED_INCREMENTAL=1 against an already prepared real-Gemma
installation; default CI spelling checks and owned-CI guards remain. Optional
GRAMMAR_GESTURE_POWERSHELL selects a trusted test shell without policy changes.
Native timestamps are bounded to64 entries with no document text; functional and
overall timing status are distinct. Syntax/diff/governance and both unauthorized
mode refusal checks PASS. Exact newly parameterized harness Windows CI NOT_RUN;
actual Sandbox used the recorded equivalent generated fixture, not spoofed CI.

Next: reproduce the1.33s tail with timing probes, then continuous typing/IME and
longer changed windows, preserving fixed1s/500ms targets. No general editor or
whole-product completion, no push/main/release; mission INCOMPLETE, Source stale.

## 2026-10-20 Q19 input-clock correction and real Gemma input stress

Reused unchanged bf2260a production package (ZIP7fc863f3d450032c8d070ead2a3617d0150ad3e292c201c5bc4302c8d8ee00f1).
Two sequential owned Sandbox trials, never concurrent. User data/install untouched.
Fixed the latency measurement, not model quality: input event -> layout-visible
suggestion at requestAnimationFrame in the page clock; DOM-availability and old
Node/locator wait are retained separately. This is not physical display scan-out.
Installed Playwright1.62.1 coreBundle.js retryWithProgressAndBackoff uses
[20,50,100,100,500]ms polling. Paired trial reproduced old1327.9ms while the same
suggestion was layout-visible at872.6ms; nine of18 old waits exceeded1s while all
18 page-clock observations were <=968.6ms. Historical1330ms FAIL remains; this
explains reproduced measurement inflation, not a claimed faster model or proof
of every historical tail cause. No production code/prompt/model/deadline change.

Final guest-r2:18 supersession cases (quantity/negation/past-date x2 fields x3)
PASS, recommendation728.8-968.6ms (p50=825.7,p95=968.6), Accept62.9-73.3ms.
Six additional stress cases PASS: continuous30ms typing, Korean CDP composition,
and196-character changed paragraph inside272-character synthetic document,
each in textarea/simple contenteditable. Recommendation672.4-761.1ms,
Accept58.4-68.0ms. Preedit800ms produced zero requests/annotations; commit produced
one. Continuous typing produced one final-source request. Longer-window case
analyzed only the changed complete line; two requests including settled baseline.
Cache hover/click and Accept caused no inference; exact complete field rereads
preserved outside text, quantities, negation, dates, price and expected grammar.
All24 cases meet1s/500ms in this warm GPU sample, not general latency acceptance.
Korean IME evidence is trusted Chromium CDP composition, physical IME NOT_RUN.
Longer paragraph evidence is grammar correction, not free-form long translation.

First guest trial FAIL retained: multiline locator.fill created #text,DIV,DIV in
contenteditable, outside supported simple-text adapter; it was refused before
analysis. Final fixture seeds a plain text node then uses Chromium keyboard word
replacement. Actual node inventory confirms cause; production guard unchanged.
General rich contenteditable support is not added or claimed.

Real keyboard toolbar opt-in twice, disable/navigation/reload and installer-written
extension unchanged checks PASS. Final browser closed/host0/profile removed;
launcher exit0, guest install/profile/process0, exact Sandbox stopped/inventory0.
Assets staged128.185s, app ready17.319s, separate from warm recommendation. D-drive
contention deferred. Same-integrator review only. Syntax/scheduling/governance
checks recorded separately; exact canonical updated CI harness not executed here
(the Sandbox adapter executes the same corpus modules). No push/main/release.
Evidence: D:/dev/grammar-evidence/chromium-input-latency-20261020/FINAL_RECEIPT.json,
with guest and guest-r2 raw receipts. Next: broader changed-window/IME corpus and
other adapter incremental integration; physical IME and cold/low-GPU qualification
remain open. Q19 IN_PROGRESS, mission INCOMPLETE, Project Source stale.

Final local validation: node syntax3 modules PASS, scheduling7/7 PASS, governance
PASS, git diff --check PASS. Host Sandbox viewer/client process inventory0.

## 2026-10-20 Q19 four-sentence translation/rewrite checkpoint — PARTIAL, NOT ALL PASS

Entry14b93bf; production remains unchanged bf2260a package, ZIP
7fc863f3d450032c8d070ead2a3617d0150ad3e292c201c5bc4302c8d8ee00f1.
Actual managed Gemma4 12B QAT Q4_0 in installed Chromium, synthetic textarea and
simple contenteditable. Four-sentence sources: English183/237 characters and
Korean107/80 characters (see exact corpus). Paragraph target3s; Accept500ms.
Review-click to layout-visible rAF, not physical display scan-out; warm times.
No model/prompt/quality guard/deadline or production change.

Final r2: English->Korean2 and English natural rewrite2 passed functional Accept
and same-integrator semantic review. Korean->English2 passed functional Accept
but FAILED fidelity review: explicit second-person You added to an implicit-subject
prohibition in Minsu context; current budget 유지됩니다 rendered will be maintained.
This is a conservative source-fidelity judgement, not a general translation score.
All six functional rows: recommendation1026.4-1153.0ms, Accept55.2-62.1ms; all
within3s/500ms, none establishes general1s or broader paragraph acceptance.
Korean natural rewrite in textarea returned analysis_unavailable and stopped the
managed model, reproduced across both trials. Subsequent contenteditable returned
gemma_required after shutdown: NOT independent Korean quality evidence, no Accept.
Exact internal refusal reason/raw rejected output remains unresolved; generic
analysis_unavailable alone cannot prove a specific register/meaning guard caused it.
Production error path stops the engine; this was confirmed cleanup, not success.

Four stale checks PASS before the Korean refusal: in-flight paragraph translation
plus two source edits, and source mutation on Accept mousedown, each in both fields.
Old cache absent after edits; fresh explicit review reflects5 files/140 dollars
instead of3/120 and exact applied text is reread. Final-gesture stale Accept retains
the newer English source byte-for-byte. Cache and Accept do not infer. Updated
latest-source translations preserve actors, recipients, negation and condition.

R1 fixture mistake retained: selectOption generated untrusted change; product
correctly kept paraphrase even though select DOM value changed. Replaced with
Home/ArrowDown keyboard events, without weakening product trusted-input guard.
R2 executes safety cases and other supported modes before the known refusal to
separate independent results. All original cases retained. Additional case-specific
forbidden-output checks now reject the two recorded KO->EN outputs offline;
final strengthened corpus has NOT been rerun installed. New optional
GRAMMAR_INSTALLED_PARAGRAPH=1 requires existing explicit installed Gemma mode.
Harness results remain functional/quality-pending until separate semantic review.

Both Sandboxes sequential, exact IDs stopped and inventory0. Final browser/host0,
profile removed, installed extension unchanged; guest install/profile/process0.
Launcher exit1 (model failed, cleanup confirmed), NOT natural successful exit0.
Final staging157.667s, model readiness12.324s reported separately. User data and
personal install untouched. No additional worker/Command windows left. No push,
main, release or CI. Syntax2 modules, scheduling7/7, governance and diff checks PASS.
Same-integrator review only; physical IME/other hardware/general editors NOT_RUN.

Evidence: D:/dev/grammar-evidence/chromium-paragraph-20261020/FINAL_RECEIPT.json,
actual-output-review.json, oracle-regression.json, original guest and guest-r2.
Next priority within Q18/Q19: retain exact failed corpus; capture bounded refusal
codes and synthetic rejected output, repair Korean paragraph availability without
loosening semantic/register checks; preserve implicit actors and tense in translation,
then repeat full installed corpus including stale races. Mission INCOMPLETE;
Project Source stale; test completion is not paragraph feature acceptance.


## Paragraph refusal / translation repair — 2026-10-20 (partial, not accepted)

Baseline fbc1896, intermediate implementation/package82c007e. Actual owned-Sandbox
raw output reproduces Korean gemma_semantic_changed: 없으면->없다면 and
답신을->회신을 were not in the closed equivalence inventory although the prompt
preferred the latter. Added only present absence condition and same object-particle
reply pairs. Counterexamples still reject positive existence, past absence,
changed subject/recipient/particle, prohibition, tense and register; literal guards
remain. Completed parsed responses rejected by semantic guards may retain the
engine only after confirmed idle, no cancellation and the original deadline.
Refusal still returns an error, never a suggestion or retry. Transport/schema/
partial/deadline failures keep terminal cleanup. Forced actual-model semantic
refusal followed by same-process successful inference remains NOT_RUN; reuse
conditions have unit evidence and same-integrator production-path review.

Actual intermediate raw Gemma four-paragraph results: KO->EN preserves omitted
actor via passive and current budget (no invented You or future budget); Korean
rewrite now passes unchanged quality guards with original mixed polite endings;
English rewrite retains facts/conditions. EN->KO semantic review FAIL: applying
passive/omitted-subject guidance in both directions dropped explicit She. All four
structural guards passed, showing why semantic review remains separate. Restored
Korean-target instruction exactly to the previously tested baseline and kept new
omitted-subject/tense guidance English-target-only; added target-scope regression.
FINAL scoped source has NOT been rebuilt or run with the real model. Intermediate
ZIP82c007e is DO_NOT_ADOPT; do not deploy it. Raw response1168/1214/1228/1473ms is
NOT UI recommendation/Accept timing or proof of paragraph target acceptance.

Installed attempts sequential, never concurrent: first blocked before corpus by
Windows foreground denial. Second failed in our external focus-assist fixture
because a Python backslash escape produced BEL in a file path; retained FAIL,
fixed path and checked control characters. Repository keyboard helper now fails
before initial key injection when foreground activation is denied. Third session
completed raw actual-model diagnostic, but GUI NOT_RUN because Owner activation
availability was not confirmed. No permission injection. Actual final candidate
8 paragraph Accept cases and4 stale races remain NOT_RUN; old results do not
transfer. Each guest install/profile/process removed; every exact Sandbox stopped,
inventory0. User data/personal install untouched; no push/main/release/Provider call.

Evidence: D:/dev/grammar-evidence/paragraph-repair-20261020/FINAL_RECEIPT.json,
actual-output-review.json, diagnosis.json, build-receipt.json, raw diagnostics and
all failed installed receipts. Full source-bound intermediate build succeeded;
strict compiler-child supervisors reported vctip.exe leftovers, all cleaned0.
Final runtime suite initially61pass/1fail/15ignored: pre-existing startup fixture
missed3s warmup entry (spawn2242ms, health2442ms); no threshold increase. Same-binary
confirmation62pass/0fail/15ignored; supervisor still FAIL_LEFTOVER_CHILDREN for
vctip.exe, cleanup complete0 (final-runtime-confirm). Model/quantization,
request deadlines, paragraph3s and Accept500ms targets unchanged. Same-integrator
review only. Mission INCOMPLETE; Source stale. Next: build final scoped source,
repeat actual four-paragraph semantic review and installed eight cases/four stale
races in one owned desktop once browser activation is available. Do not bypass
activeTab or source/revision/Accept fences to finish the test.


## 2026-10-21 Final paragraph installed real Gemma qualification

Fresh clean source `5cf40a5a40839cf89439052580e9680716444a7f` built and packaged;
ZIP cdbd2f1838ae74d0fcbcbb543d42165b4c1e087a5162b640a12387cf89351ccd.
One Sandbox, actual unchanged Gemma12B QAT Q4_0 and installed extension/host.
Eight EN->KO/KO->EN/natural EN/KO textarea+simple-contenteditable cases passed
exact Accept reread and same-integrator bounded semantic review. EN->KO now keeps
explicit she; KO->EN has no invented You/future budget. Korean reply/absence
synonyms retain conditions, prohibition and original mixed speech levels. This is
four synthetic texts in two field types, not general translation qualification.
Four stale races passed: in-flight repeated edits and source change at Accept for
both fields. Cache/Accept did not add inference. Real keyboard toolbar/popup grants,
Disable/navigation/re-enable/reload checks passed; extension bytes unchanged.
Automatic foreground worked this time; no Owner click or permission injection.

Warm explicit Review-click to rAF-visible complete recommendation:1059-1371ms,
p50=1152,p95/p99/max=1371(n8 nearest rank); Accept full reread54-66ms,
p50=59,p95/p99/max=66. Paragraph3s misses0/8, Accept500ms misses0/8; all8 exceed
short1s. This does not measure committed-input proactive latency. Model staging
284.047s and app/model readiness15.365s are separate, not recommendation latency.

Build exit0/source receipt valid; strict supervisor FAIL_LEFTOVER_CHILDREN for
vctip.exe, owned cleanup COMPLETE0. Preserve this and previous startup/semantic/
foreground/fixture failures. App natural exit0; browser/host/model processes0,
install/profile removed, exact Sandbox stopped and final inventory0. Personal
install/user data untouched. No cloud/CI/push/main/release. Source remains stale.
Evidence: D:/dev/grammar-evidence/paragraph-final-20261021/FINAL_RECEIPT.json,
actual-output-review.json, build-receipt.json, guest/installed-browser/result.json,
guest/result.json, sandbox-result.json. Package source remains5cf40a5; this is a
later evidence-only documentation checkpoint. No independent reviewer this turn.

Next: actual forced semantic-refusal then successful request in the same model
process (current guarded reuse has unit coverage, this valid paragraph no longer
refuses); broaden subject/tense/register corpus and committed-input paragraph
latency. Physical IME, general profiles, Edge and other adapters are NOT_RUN here.
Q18/Q19 IN_PROGRESS; mission INCOMPLETE. Do not weaken quality or timing targets.


## 2026-10-21 Actual refusal recovery remains unqualified

2026-10-21 actual quality-refusal recovery qualification INCOMPLETE: added owned browser regression asserting typed refusal/no suggestion/no mutation/no replay and model+broker PID/start-time identity before follow-up8 paragraph Accepts/4 stale races. First actual Gemma run6 Korean probes did not refuse; NOT a recovery PASS. Same model4508/broker8732 survived all6; follow-up Accept not reached. Revised8 modality/nested probes were NOT_RUN: second sequential Sandbox foreground activation denied after120s. Both natural launcher0, install/profile/process/Sandbox0. Unchanged5cf40a5 package binaries/extension; external diagnostic launcher copy only redirects stderr and enables timing, not unmodified-launcher qualification. No product patch/quality weakening/user data/personal install/push/main/release. Evidence refusal-recovery-r2-20261021/FINAL_RECEIPT.json and first-run refusal-recovery-20261021. Next reproduce actual refusal without foreground dependency before UI repetition; same-process post-refusal acceptance remains open. Q18/Q19 IN_PROGRESS; mission INCOMPLETE, Source stale.

Reusable test: scripts/mission/gemma-refusal-recovery-browser.mjs. Six original probes and their actual outputs remain in first-run guest/installed-browser/result.json. Final revised probe list has syntax checks only; do not describe it as executed. Observer receives only task-owned process identity and content-free refusal codes, never supplies model output. Existing final paragraph8/4 PASS is separate prior evidence, not post-refusal recovery. Same-integrator review only.


## 2026-10-21 Actual refusal then same-process Accept — bounded PASS

2026-10-21 actual refusal recovery PASS_BOUNDED: installed5cf40a5 native host processed real Gemma request Would you mind sending the report to Mina when you have a moment? and quality guard modality_changed returned analysis_unavailable with empty suggestions. Broker7988/model9000 PID+start times identical before refusal, after refusal and after headless follow-up8 translation/rewrite Accept cases plus4 stale races. Explicit popup/field opt-in and exact reread retained; copied extension changes only localhost manifest permission, production JS unchanged. NOT OS-toolbar/unmodified-launcher qualification. Warm recommendation1058-1371ms p50=1153 p95/p99/max=1371(n8), Accept58-61ms p50=59 p95/p99/max=61;3s/500ms misses0/8, short1s exceedances8/8. Same-integrator semantic review bounded PASS. First automated run FAIL preserved: diagnostic FileStream buffered actual3 refusal codes until closure; repaired external observer with buffer1 WriteThrough, no production/quality change. Model staging301.254s,ready20.481s separate. Both sequential Sandbox natural launcher0,install/profile/process/Sandbox0; user data untouched. Evidence refusal-native-headless-r2-20261021/FINAL_RECEIPT.json corrects overbroad legacy outer receipt label; nested browser scope is explicit modified fixture. New native probe regression added. Q18/Q19 IN_PROGRESS; mission INCOMPLETE, Source stale. Next broaden actual refusal types and browser-visible refusal messaging/physical scope; no Cloud/push/main/release.

Reproduction: scripts/mission/gemma-refusal-native-probe.mjs exports
probeActualRefusal(hostPath, exactOrigin, observe, evidence). Run only inside owned
WDAG with installed host and resident pinned broker; observers return task-owned
PID/start metadata and existing content-free refusal codes. The successful external
runner/WSB/fixture and hash manifest are under the evidence directory above. It
runs the native probe before launching headless Chromium, then existing
runParagraphUI with all8 cases/4 stale races, and compares final process identities.
Never substitute a mock refusal, write injected model output, or treat ordinary
unchanged output as rejection. On first run3 real refusals were present in final
logs and same process continued, but observer missed them; automated FAIL remains.
On final run the first request is sufficient, native host exits0 while resident
model stays alive. Refused raw replacement is not captured, so guard rejection is
proven but a material semantic error in that output is not independently adjudicated.
Accept output semantic review is separate actual-output-review.json, same integrator.

Validation: native-probe module Node syntax and governance PASS, git diff check PASS;
no production code change or full Rust rerun required. Package source5cf40a5 is
unchanged; later commit contains test/evidence only. Refusal itself was native
protocol, not browser-visible UI; physical keyboard/IME, normal profile, Edge,
other adapters, Cloud Provider and general language quality are NOT_RUN here.
The external diagnostic launcher and manifest-only fixture permissions are test
instrumentation, never changes to the personal installation or release package.


## 2026-10-23 Installed extension actual refusal recovery

2026-10-23 installed-extension refusal recovery PASS_BOUNDED: unchanged installed5cf40a5 extension (activeTab/scripting/nativeMessaging/storage, no host_permissions) activated via actual OS keyboard toolbar+popup in one owned Sandbox. Actual browser paraphrase refusal modality_changed returned analysis_unavailable, empty suggestions, no Accept, no mutation/replay. Broker4932/model5664 PID+start times identical before refusal, after refusal and after8 four-sentence translation/rewrite Accept cases plus4 stale races. Exact reread and same-integrator semantic review PASS in bounded EN/KO corpus. Warm recommendation997-1310ms p50=1091 p95/p99/max=1310; Accept54-63ms p50=59 p95/p99/max=63 (nearest rank,n8). Paragraph3s/Accept500ms misses0/8, short1s exceedances7/8; explicit Review click timing, not proactive committed-input latency. Staging242.977s, readiness14.665s separate. Disable/navigation/re-enable checks PASS; extension unchanged; natural launcher0, owned host/install/registration/profile/process/Sandbox0. External diagnostic launcher retains content-free stderr; not unmodified-launcher qualification. No product patch or new build needed, quality/model pins unchanged. Console output encoding is lossy; UTF-8 result JSON used for review. Tray-icon diagnostic remains, shutdown confirmed complete. Evidence refusal-installed-20261023/FINAL_RECEIPT.json. User data untouched; no personal install/Cloud/push/main/release. Q18/Q19 IN_PROGRESS; mission INCOMPLETE, Source stale. Next broaden refusal classes and Korean paragraph quality; physical IME/general profiles remain NOT_RUN.

Reproduction: D:/dev/grammar-evidence/refusal-installed-20261023/run.py and
trial.wsb use preserved fixture/ and fresh guest/ output. Single-Sandbox controller
refuses a second session and stops its exact ID in finally. Reuse the source-bound
paragraph-final-20261021 package only while product paths are unchanged from5cf40a5;
this checkpoint verified no src/src-tauri/adapters diff. Browser module is
scripts/mission/gemma-refusal-recovery-browser.mjs followed by
scripts/mission/gemma-paragraph-browser.mjs. Real OS keyboard grants activeTab;
no manifest edit or worker permission injection. Each helper refuses input unless
the owned window is foreground. Both toolbar activations succeeded without Owner
click. External Start-Diagnostic.ps1 uses buffer1 WriteThrough to expose existing
refusal codes immediately; installed binaries and extension files are unchanged.
Raw guest receipts are preserved; FINAL_RECEIPT.json separates external launcher
instrumentation from installed extension qualification. actual-output-review.json
contains all8 manually reviewed outputs, and evidence-hashes.json pins receipts
and reproduction files. Rejected replacement itself is not logged, so the guard's
linguistic correctness is not independently adjudicated. No new failure was found
in the requested recovery flow; prior failed attempts remain historical failures.
Validation: actual installed scenario PASS, Node module syntax, governance and diff
checks; no full Rust rerun because no production code changed. No independent
reviewer used. Continue broader quality work without claiming whole-product acceptance.


## 2026-10-23 Korean request translation repair candidate

2026-10-23 Korean expansion PARTIAL: actual installed5cf40a5 extension observed modality_changed and semantic_changed refusals4 with immediate8 recovery Accepts in identical model process. Expanded rows20 Accept/4 refusals/4 no-change, baseline8 Accept+4 stale functional PASS; manual review FAIL for2 question-en outputs: polite listener request became passive factual question. Quoted Korean retained by verbatim-literal contract, not full quotation translation. First Sandbox failed ExistingLogin0x80070520 before product start; second completed, all owned processes/install/profile/Sandbox0. Repair scoped to three unquoted Korean polite request-question endings for English translation, with modality rejection if addressed request disappears; ordinary declarative guidance and model quality unchanged. Lib local_gemma tests46PASS11ignored, strict supervisor FAIL_LEFTOVER_CHILDREN1 then cleanup0. Initial wrong broker test filter ran0, not qualification. Fresh repaired package/actual-model retest pending. Evidence korean-refusal-installed-r2-20261023 and korean-request-repair-20261023. No user data/Cloud/push/release; mission INCOMPLETE.


## 2026-10-23 Korean request repair installed qualification

2026-10-23 Korean paragraph/refusal expansion repaired and PASS_BOUNDED: new source ceb6298 fresh package, unchanged installed extension permissions (no host_permissions), real OS keyboard toolbar+popup. Initial5cf40a5 run passed functionality but manual review found2 polite request translations changed into passive factual questions. Scoped Korean request-question English prompt and modality fence repair this without weakening ordinary omitted-subject guidance or quality/model pins. Final question-en says could you send a reply to Min-su and exact Accept succeeds in both fields. Expanded28rows:20 Accept (including8 immediate post-refusal translation/rewrite Accepts),4 actual refusals across modality_changed/semantic_changed,4 unchanged results NOT Accept proof. Baseline8 Accept+4 stale races pass. Broker6616/model7744 PID+start times unchanged throughout. Same-integrator semantic review bounded PASS; quoted Korean remains verbatim intentionally, not full quote translation. Expanded response observation1090-1568ms p50=1235 p95=1535 p99/max=1568(n20), includes100ms observation wait and is NOT paint/proactive latency; Accept61-71ms p50=64 p95=69 p99/max=71. Baseline visible recommendation966-1279ms,short1s exceedances6/8;3s/500ms misses0. Model staging369.545s/readiness22.855s separate. First Sandbox ExistingLogin0x80070520 before product start preserved; sequential runs only, all stopped. Lib tests46PASS11ignored; initial wrong broker filter ran0, NOT qualification. Test/build child-residue supervisors FAIL_LEFTOVER_CHILDREN1, cleanup0; buildexit0 with source-bound receipt. Final natural launcher0, extension unchanged, owned install/registration/profile/process/Sandbox0; user data untouched. Evidence korean-refusal-final-20261023/FINAL_RECEIPT.json and korean-request-repair-20261023 build receipt. No personal install/Cloud/push/main/release. Mission INCOMPLETE, Q18/Q19 IN_PROGRESS, Source stale; next broader request/quotation translation quality and general-profile/IME boundaries remain open.

Reproduce with scripts/mission/gemma-korean-recovery-browser.mjs and its JSON
corpus in the preserved korean-refusal-final-20261023 fixture/run.py/trial.wsb.
The final external runner additionally requires two actual refusal types and
question-en suggestions accepted in both fields. It loads the newly built
korean-request-repair-20261023/package, not the historical5cf40a5 package.
Fresh model source/runtime pins unchanged; content-free diagnostic stderr only.
Raw automated PENDING quality labels remain intact; actual-output-review.json
records separate manual review. All baseline8 outputs are byte-identical to
prior reviewed outputs; expanded outputs changed only the2 faulty request rows.
Three supported request endings: 주시겠어요?, 주시겠습니까?, 주실래요? outside
verbatim literals. This bounded fence is not a general Korean speech-act proof;
unrecognised variants remain unqualified. Quote contents stay in their original
language by the existing verbatim-literal contract. Null paraphrases are not
quality failures or successful rewriting evidence. Rejected raw replacements
are not captured, so the linguistic correctness of those refusals is not proven.
No independent reviewer; same-integrator code/diff and actual-output review.
Console log encoding is lossy; UTF-8 receipt JSON is the authoritative text.
Cleanup only targeted C:/PencilTrialScope and the task-owned browser profile;
uninstaller removedUserData refers to that synthetic data, never user AppData.


## 2026-10-23 request/question expansion repair pending installed retest

Expanded installed ceb6298 corpus found a real quality failure despite functional
PASS: explicit speaker permission (제가 ...도 될까요?) translated passively, losing
speaker identity and commitment. Both textarea/contenteditable accepted it.
Evidence korean-requests-20261023/installed/actual-output-review.json preserves
this failure. 32 expanded Accepts,4 actual refusals,8 no-change; baseline8 Accept
and4 stale cases; same model PID/start; complete owned cleanup and Sandbox0.
New bounded English prompt/fence preserves explicit speaker permission separately
from addressed requests and factual questions. Red regression fails before repair;
47 local_gemma tests PASS11ignored after repair, strict test cleanup0. Actual-model
repair qualification remains PENDING until new source-bound package retest.

Child diagnosis reproduced successful MSVC compilation leaving exact installed
vctip.exe. owned_build.py now explicitly owns this helper until build completion,
reaps only its Job tree and awaits exit; BUILD_COMPLETE_HELPERS_REAPED is not a
natural shutdown PASS. No registry/global telemetry change, no name-based kill.
Child-only VSCMD_SKIP_SENDTELEMETRY suppresses VsDevCmd's separate PowerShell helper,
not compiler vctip. Undeclared/query-failed children still fail. Product tests keep
strict owned_trial CLI, with no helper exception.10 supervisor regressions PASS;
real target-service DLL build exit0, helper reaped, remaining0. Initial diagnostic
API error and test assumptions are recorded in diagnostic-attempts.json.


## 2026-10-23 installed request/question and build ownership checkpoint

2026-10-23 Korean request/question expansion PASS_BOUNDED_MEANING_REGISTER: installed ceb6298 test exposed speaker loss in 제가 ...도 될까요? permission translation (2 fields); functional PASS was not quality PASS. Source9112e4b adds bounded first-person permission prompt/fence, distinct from addressed request and factual questions. Red production-helper regression FAIL before repair;47 Gemma tests PASS11ignored after, strict execution natural cleanup0. New source-bound package tested in one fresh Sandbox after previous session stopped:18 corpus cases across2 fields,32 Accept including8 immediate recovery Accepts,4 actual modality_changed/semantic_changed refusals,8 no-change NOT Accept proof; baseline8 Accept+4 stale races PASS. Same broker4752/model7836 PID+start throughout. Permission now may I send and I will not send; other outputs byte-identical to previous run. Same-integrator semantic/register review only; English article/fluency polish remains, quoted literals remain untranslated and rejected raw outputs not adjudicated. Expanded observation886-1527ms,p50=1156,p95=1514 includes100ms wait, not paint;Accept59-70ms,p50=64,p95=67;baselinevisible997-1278ms,1s misses6/8,3s/500ms misses0. Staging427.067s/readiness19.776s separate. MSVC vctip residue reproduced; owned_build declares exact installed helper and reaps only owned Job members, records BUILD_COMPLETE_HELPERS_REAPED rather than natural PASS. Real target DLL compile helper reaped0; final full release build420.625s natural owned cleanup0. Product-test supervisor remains strict;10 supervisor tests PASS. Child-only VsDevCmd telemetry suppression, no registry/global process changes. Final app/launcher0,owned install/registration/profile/process/Sandbox0,user data untouched. Evidence korean-requests-20261023/FINAL_RECEIPT.json; initial quality failure/red test/diagnostic failures retained. No Cloud/personal install/push/main/release. Mission INCOMPLETE,Source stale,Q18/Q19 ongoing; next broader speaker/register boundaries and English fluency, general-profile/physical IME qualification.

Product/package source9112e4bdb79d6f2e7ed20f681d65c95bef653bab;
ZIP13bbb22fed6671f77fb5d808ff532ff7af34e41699c5cf7c050c61822849b9ce;
extensioniahehghbabhakjnchcneoilmajgpdkpe. Evidence docs commit is not binary source.
Reproduce using preserved installed-repaired/run.py/trial.wsb/fixture under the
above evidence directory and package. Fixture asserts two real refusal types,
question-en and permission-question-en Accepts. Extension bytes/permissions are
unchanged; external launcher only redirects diagnostic stderr. No extra host
permissions. Final raw receipt quality labels remain pending; separate manual
actual-output-review.json adjudicates bounded meaning/register, with fluency
limits explicitly retained. The permission output's bare Professor article choice
is not perfect-English qualification. No-change rows are not rewritten outputs.

For future builds use owned_build.py as in docs/PROTOTYPE.md. Compile Rust tests
with owned_build.py cargo test --no-run; run the emitted executable separately
under strict owned_trial.py. This separates intentionally managed MSVC helper
teardown from product natural-child-exit requirements. Never add product processes
to the helper declaration. Only installed vctip is declared by the build wrapper;
other images or failed image queries still fail. Direct, unsupervised third-party
compiler commands are not claimed to shut down their services naturally.
Uninstaller removedUserData=true concerns only C:/PencilTrialScope synthetic data;
no host user AppData, dictionary or normal browser profile was read or removed.


## 2026-10-23 English fluency candidate pending actual model qualification

English-only translation guidance now distinguishes specific role articles,
unnamed roles and named titles, and discourages literal temporal preposition
chains without changing subject, speech act or commitment. Existing meaning,
register and speaker/request guards remain unchanged. Corpus adds named-title
and indefinite-role counterexamples; required/forbidden wording is checked before
Accept. Actual installed-package output qualification remains pending.
Focused Gemma assertions47PASS11ignored; strict supervisor reported leftover
MSVC vctip despite test exit0, then confirmed cleanup0. This is NOT natural test
cleanup PASS. Compile supervisor explicitly reaped its declared helper, cleanup0.
First compile preceded edits after an encoding error, so compile-r2 is the actual
modified-source compile. Evidence english-fluency-20261023. No user data touched.

## 2026-10-23 English fluency first installed run FAIL — repair pending rerun

Source e8b3b8a package is DO_NOT_ADOPT. The single installed Sandbox run
completed 48 rows but four wording assertions failed (bare Professor and within
today, two fields each). Manual review additionally found an invented Kim copied
from a prompt example and Korean factual-question paraphrase omitted 받지,
changing not receiving a reply into not replying. The latter also exists in the
previous 9112e4b raw output: its prior broad meaning PASS is corrected for that
case; immutable receipts remain unchanged. Functional Accept was not semantic
proof. Baseline/stale stage was NOT_RUN after failure. Same broker/model survived
four real refusals and follow-ups; all owned guest processes/install/profile and
Sandbox were removed. Evidence english-fluency-20261023/actual-output-review.json.

Repair removes personal-name examples, puts positive English article/time rules
after the input, and preserves receive/send action tokens in bounded Korean reply
rewriting. Exact observed receive-loss production-helper regression failed before
repair and passes after;48 Gemma tests PASS11ignored, supervisor natural cleanup0.
Red test failure and unknown helper-image query failure are preserved, owned
cleanup0. Real-model qualification of this repair remains pending. No user data.

## 2026-10-23 English fluency second installed run — named title still FAIL

Source ef836a4 fixes the observed bare-role article, today phrase and Korean
receipt-loss outputs in both fields, but drops the explicit surname in 김 교수님.
Named-title assertions blocked those two Accepts. Overall FAIL retained,34 Accept,
4 actual refusals,8 no-change; baseline/stale stage NOT_RUN. All owned cleanup0.
The next candidate adds only source-derived adjacent title context, no invented
name example. Same-integrator review; final installed qualification pending.


## 2026-10-23 Final English fluency installed checkpoint

2026-10-23 English article/fluency qualification PASS_BOUNDED at source cae1d354b3923cc1cfbf89ad9a53e24cba3a8b57: installed unchanged extension permissions/files and actual OS keyboard toolbar/popup, Gemma12B same pins. Three sequential Sandboxes, never concurrent. First e8b3b8a FAIL: bare Professor/within today, invented Kim from prompt example, and Korean received-reply action loss. Second ef836a4 FAIL: explicit Kim omitted. Final source-bound title context retains Professor Kim; unnamed the professor and indefinite a professor remain distinct; by the end of today replaces within today; speaker May I/I will not and Korean 회신을 받지 못했어요 retain meaning/register. Prior 9112e4b factual-question-paraphrase quality PASS was overbroad and is corrected; original raw receipts untouched. Production-helper receive-loss regression RED before repair, then PASS. Final expanded48rows:36 actual Accept including8 immediate post-refusal Accepts,4 real modality_changed/semantic_changed refusals,8 no-change NOT Accept proof; additional baseline8 Accept and4 stale races PASS. Broker5896/model5996 PID+start unchanged. Same-integrator manual review only, not independent linguistic certification. Expanded response observation948-1575ms p50=1174 p95=1536 includes100ms wait, not paint;Accept61-67ms p50=65 p95=67. Baseline visible1028-1309ms p50=1060;paragraph3s/Accept500ms misses0, short1s misses8/8. Model staging571.487s/readiness14.534s separate. Final build487.718s explicitly reaped declared MSVC helper, cleanup0, not natural-child-exit PASS. All three guest app/install/registration/profile/process/Sandbox cleanup0, no user data/personal install/Cloud/push/main/release. Evidence english-fluency-20261023/FINAL_RECEIPT.json and manual reviews retain failures. Quotes remain verbatim, some English impersonal phrasing remains stiff, no-change not rewrite qualification. Title guidance is bounded source surface context, not general entity verification. Mission INCOMPLETE, Source stale, Q18/Q19 ongoing. Next expand name/title and quoted-context cases without relaxing literal/meaning guards; physical IME/general profiles and strict1s goal remain unqualified.

Reproduce from preserved installed-r3/run.py, trial.wsb and fixture; no extra
extension permissions or output/error injection. External diagnostic launcher
redirects content-free broker stderr, so this is not unmodified-launcher evidence.
Manual quality review is actual-output-review-r3.json; original browser receipt
keeps QUALITY_PENDING labels, deliberately distinct from manual adjudication.
Comparison-installed-r3.json is automated comparison only. First and second manual
reviews remain FAIL. UTF-8 result JSON is authoritative; console Korean is lossy.

Final package-r3.zip SHA256
84268115f8009878ce9e817b405c76ae09ee19e9f5c8a16743e8a43fc92128f9;
extension johmcjbinepekondfmgaaijhamiilcap. Source commit is cae1d35, not the later
evidence documentation commit. App/host/broker hashes and source tree are in the
manifest and final receipt. Uninstaller removedUserData=true refers exclusively
to C:/PencilTrialScope owned synthetic assets/profile, never host user AppData.

Earlier strict focused test reported leftover vctip (then cleanup0); compiler
helper declaration is not a relaxation of product natural-cleanup requirements.
The red test also records an image-query failure and exit101; do not call it PASS.
No general editor or overall mission acceptance is claimed by this checkpoint.

Final source library assertions177PASS26ignored, but strict supervisor FAIL_LEFTOVER_CHILDREN for owned vctip, then confirmed cleanup0. This is not natural test cleanup PASS. Previous ef836a4 full-library execution had natural cleanup0; intermittent compiler-helper lifetime remains explicitly managed, not erased. Final product app/model/browser cleanup was natural and separate.


## 2026-10-23 Names, titles and quoted-context verification

2026-10-23 Names/titles/quotation checkpoint at product e68b4122ce8f1764658ad983ce752202b9827a1f: PASS_BOUNDED_MEANING_REGISTER_FLUENCY_PARTIAL. Existing20 plus6 new corpus cases retained. Initial cae1d35 functional PASS was semantic FAIL: three English scenarios in both fields turned explicit speaker negative promises into passive predictions. Actual-output-review.json retains all6 failures. Added source-bounded same/immediately-previous sentence 제가 + 보내지 않겠습니다 guidance and unquoted I will not send/resend fence; quoted I cannot satisfy it. Actual observed-output regression RED then PASS. Final actual installed extension unchanged/no extra permissions; real keyboard toolbar/popup; Gemma12B same pins. Expanded64rows:50 Accept including12 immediate post-refusal Accepts,6 real modality_changed/semantic_changed refusals,8 no-change NOT Accept proof; baseline8 Accept and4 stale races. Same broker3476/model5412 PID+start, no automatic replay/cache inference. Final changed outputs only the6 repaired promises; old corpus outputs unchanged.58 accepted rows manually reviewed by same integrator, no independent linguistic review. Complete Park Ji-hoon/Lee Seo-yeon/Mina Chen/Alex Rivera/Jiyoon Park names, roles, recipients, conditions, tense, respectful register and immutable quotes retained within tested cases. Quoted-request Korean paraphrase refused in both fields, NOT accepted rewrite proof. Fluency remains PARTIAL: Before approval, I will not send omits object; Teacher Lee/Team Leader Park literal role titles and earlier impersonal wording remain stiff. No general entity/subject/translation guarantee. Expanded response observed749-1646ms,p50=1249,p95=1602 includes100ms wait, not proactive paint; Accept60-69ms,p50=66,p95=68. Baseline visible1091-1372ms;3s/500ms misses0,short1s misses8/8. Model copy464.056s/readiness15.962s separate. Final focused49PASS11ignored. Intermediate full-library177PASS1FAIL26ignored: startup_failure_receipt_requires_actual_native_cleanup hit gemma_start_timeout;runtime_cleanup_deadline; root cause unresolved, supervisor cleaned owned processes0. Later focused PASS does not erase failure or establish full-suite PASS. Build547.766s explicitly reaped declared MSVC helper,cleanup0. Two sequential Sandboxes, never concurrent; both actual product exits natural0, install/registry/profile/app/model/browser/Sandbox residues0. User data untouched; no personal install/Cloud/push/main/release. Evidence names-quotes-20261023/FINAL_RECEIPT.json; package.zip SHA256 a4a7a058e4b927ae1b5efbc909d0bd223a0ebe37712dd6ccca914dc1ca6cc4e7; extension kaogbnemglkkdcgcbhhlaamcldbkkgpp. External stderr diagnostic launcher remains, not unmodified-launcher qualification. Mission INCOMPLETE, Source stale. Next: isolate startup-timeout cleanup failure without relaxing budget; refine object completion/role-title fluency without invented subjects/gender; expand held-out cases with same-process installed evidence.


## 2026-10-23 Compact guidance: held-out comparison rejected

2026-10-23 Compact Gemma guidance experiment: REJECTED / DO_NOT_ADOPT. Candidate042dc78685b48f2b2b3bb49c9e47ad74ca8046a5 consolidated English source-specific request/permission/title/commitment instructions into general principles/checklist and shortened paraphrase guidance; output safety guards unchanged. Froze8 unseen cases before outputs, retained26 regressions; no retuning after heldout inspection. Baseline e68b412 installed extension:16 heldout Accepts,61 total Accepts,8 no-change,5 actual refusals,4 FAIL rows from PowerShell process-observation commands (reason unconfirmed). Responses existed, but failures retained; paragraph/stale stage NOT_RUN. Candidate installed package integrity passed, then Windows foreground activation failed before requests; candidate UI translation/rewrite/Accept NOT_RUN. Both sequential Sandboxes naturally closed app/model/host/browser and uninstalled owned profile/registry/install; remaining0. Separate real GPU production-engine diagnostic used identical frozen candidate prompt/guards and pinned model/runtime, not UI acceptance:33 valid requests including all8 heldout, same model,cleanup/workspace/job0. Fixture first row used revision0 and was invalid_binding; this is not a model quality result. Revision and outcome checks repaired in ignored diagnostic; initial0-test filter invocation also NOT_RUN. On7 English heldout cases, same-integrator unblinded scores improved3/equal4, but3 heldout role/gender failures: invented he for Han Jiho, I for unspecified checker, We for unspecified discussants. Existing multiple-titles case invented male Mr and lost teacher role; factual-question introduced I. within today also returned. Fluency gains do not compensate meaning/role regressions; no general quality or causal speed claim (baseline build overlap/uncontrolled host load; engine vs UI path differs). Candidate package ZIP0b49463c99a0e07a2c9abd8b323074d7349c3f31454830f3fcf210cfe533a50b is DO_NOT_INSTALL_OR_RELEASE. Production restored exactly to e68b412; only reusable frozen evaluation corpus and ignored real-engine diagnostic retained. Initial restoration missed the prompt and2 contract assertions caught it; corrected exact function, final focused49PASS12ignored, governancePASS. Prior startup cleanup deadline failure remains unresolved. Evidence concise-guidance-20261023/FINAL_RECEIPT.json,quality-comparison.json,protocol.json; failed runs preserved. No user data/personal install/Cloud/push/main/release; all owned processes and Sandboxes0. Mission INCOMPLETE,Source stale. Next: general source-grounded unknown-subject/gender/title validation before another compact prompt; a NEW unseen fold (these8 are now used evaluation data), then actual installed Accept with foreground activation. No sentence-specific prompt patch was adopted.


## 2026-10-23 Source-person output fence candidate

Added an output-side lexical fence for invented English first-person/plural/gender pronouns and marked titles; prompt unchanged. Quoted speakers do not license narrative persons. Korean determiner 저/그 does not license I/he; US abbreviation excluded. Conservative explicit role evidence permits Dr/Professor. This is not general entity binding: an explicit person elsewhere may still license wrong attribution, neutral they and arbitrary invented names remain unqualified. Six new heldout cases frozen before outputs, previous evaluation fold remains used data. Focused initial51PASS12ignored; actual model and installed qualification pending. Evidence source-roles-20261023. No user data, one Sandbox at a time, no personal install/push/release.

Actual first engine run40 completed, same model and cleanup0, but quality not PASS: compound title particle 께서는 rejected valid Dr/Professor; explicit plural sending promise became passive. Candidate df12e6f/package is DO_NOT_ADOPT. Generic particle handling and explicit future-speaker guard repaired without prompt changes. First six heldout now used; three additional unseen cases frozen before rerun. repair-focused51PASS1FAIL12ignored: startup fixture failed provider_spawn_deadline before warmup; owned cleanup0. Earlier two baseline installed attempts blocked before requests at toolbar activation, cleanup0, not model/Accept proof. Final candidate qualification pending.


## 2026-10-23 Source-person containment final checkpoint

2026-10-23 Source-grounded person/title containment at product4a9fdf5 (not main adoption). Prompt section exactly equals e68b412: no sentence/name examples added. Output fences reject unsupported English I/we/he/she groups and marked titles using unquoted source evidence, distinguish Korean determiners/US, support compound role particles, and preserve explicit future and declarative negative speaker promises. Conservative lexical scope, not general coreference: wrong entity attribution with existing pronouns, they/you and arbitrary invented names remain unqualified; valid implicit speakers/synonyms may be refused.

First df12e6f actual40 requests completed same model/cleanup0 but candidate rejected: compound-title false-positive risk reproduced independently (raw refused output not retained), plus accepted plural sender omission. Second02e0ea3 actual43 fixed title cases and rejected plural loss but accepted intervening-sentence negative promise as passive forecast. Both packages DO_NOT_ADOPT. Final4a9fdf5 actual43:31 changed returns,5 unchanged (not rewrite/Accept proof),7 actual quality refusals, same PID+ready throughout and natural workspace/job/process cleanup0. Nine newly introduced paragraph cases: first6 used for repair, second3 frozen before first use at02e0ea3; final rerun is regression, not fresh unseen evaluation. Final9 give4 changed English returns,1 unchanged Korean,4 refusals. Same-integrator unblinded review finds bounded meaning/politeness/person preservation in returned new cases, fluency partial/stiff1of2. Refusals receive no linguistic score. Common36 returned outputs unchanged versus02e0ea3, so no fluency improvement claimed. Warm paragraph engine653.5-990.9ms, not UI publication/Accept or causal latency proof; integrity/staging separate.

Installed extension: two e68b412 baseline attempts and one02e0ea3 candidate, sequential max1 Sandbox. All installed files/key/permissions/host registration verified; toolbar keyboard focus blocked, mouse hit Pencil but Enable popup unavailable. Candidate added owned UIA SetFocus fallback with Space gated on verified focus; activation still failed; no permission or worker activation injection. No installed translation/rewrite/Accept reached. Browser/app/model/host/profile/registration/install and Sandbox all cleaned0. Final4a9fdf5 package is for a future attended activation retry only; personal install/push/merge/release NOT_PERFORMED. User documents/settings/profiles untouched.

Final focused52PASS12ignored. Intermediate full-library02e0ea3:180PASS1FAIL27ignored, existing startup_failure_receipt_requires_actual_native_cleanup failed gemma_start_timeout before warmup. Earlier repair-focused provider_spawn_deadline and initial missing-particle assertion failures retained; reruns do not erase them. Declared MSVC build helper explicitly reaped, not natural compiler-child exit. Product model test exits natural0. Evidence source-roles-20261023/final-quality-review.json and FINAL_RECEIPT.json. Mission INCOMPLETE, Source stale. Next: attended actual installed activation and Accept on final package; isolate existing startup fixture timing failure; improve English fluency using general guidance/unseen folds without weakening fences or trading errors for refusal-only coverage.

Final package source4a9fdf5b66337e9bc2ce24e0a0434d784e422459; ZIP747bbe6884cacacf1f057e1057a7a76b68513f58346586a602634351ed628a79; extension gaodnbggfhaaojdalceeabfabjbmnfmk. Final build531.14s explicitly reaped declared MSVC helper,cleanup0. Package build is not installed qualification. Uninstaller removedUserData=true refers only to C:/PencilTrialScope synthetic profile/assets, never host AppData.


## 2026-10-23 GitHub candidate synchronization

2026-10-23 GitHub candidate synchronization: reviewed and non-force pushed 29 commits b30ae48..4ef212f to codex/grammar-local-trial-ba58b7d; 115 outgoing blobs scanned without credential/binary/model/archive matches. First Windows run39963370820 FAIL: launcher status observer WinError32. Repair c2f3245 uses existing bounded wait_phase, preserves failed-state/app/cleanup assertions, adds forced-race and deadline/malformed-status regressions. Run39965158557 on c2f3245 PASS. Local library181PASS27ignored and startup standalone PASS are reruns, not resolution of prior native startup failures. Final local launcher37PASS after explicit UTF8 parent output, per-fixture marked scope binding and explicit bounded app-release handshake; all earlier local failures preserved. Supervised cleanup COMPLETE0 with declared compiler helper reaped, not natural exit. Final follow-up test/docs commit requires its own CI result. Latest installed-extension Gemma Accept NOT_RUN; lexical semantic/fluency limits, mission INCOMPLETE and Source stale remain. No main merge/release/personal install/Sandbox/user data/cloud call. See docs/control/GITHUB_CANDIDATE_REVIEW_2026-10-23.md and local github-sync-20261023 evidence.


## 2026-10-23 Prototype 3 publication

2026-10-23 Prototype3 published: source/tag5d43b2b, candidate Windows CI39968529938 PASS, main safely fast-forwarded from b30ae48 with existing untracked audit files preserved. Fresh source-bound release build,654 file hashes, extracted ZIP owned install/basic Instant/removal PASS; build helper explicitly reaped, all owned residual processes0. Release v0.1.0-prototype.3 ZIP SHA2564c42f69b9c26c06b59463ac2d200392c19adfbf8f004c5db4423014d9ca95fb8; GitHub asset digests matched. No model/runtime weights or user data bundled. Latest actual Gemma installed UI/Accept NOT_RUN, startup failures and language/hardware/editor limits retained. Mission INCOMPLETE; Project Source stale. Public VALIDATION.json and local prototype3-20261023 evidence distinguish package smoke from product acceptance.

Release: https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.3
This post-publication record commit changes documentation/state only; the executable ZIP remains bound to tag5d43b2b, not later record commits. Source CI is not real-model UI qualification. Next: attended installed activation/Accept, unresolved startup failures, broader independent language evaluation.


## 2026-10-23 Prototype 3 actual Gemma and CI refusal repair

2026-10-23 Prototype3 actual installed Gemma Chromium qualification: original released ZIP5d43b2b SHA2564c42f69b9c26c06b59463ac2d200392c19adfbf8f004c5db4423014d9ca95fb8, original launcher/extension/permissions; four reused paragraph cases on textarea/contenteditable,8 exact Accept and4 stale checks PASS. Review-to-visible1089.3-1372ms,Accept58.1-63.1ms; deployment98.682s and ready12.310s separate. Bounded manual meaning/politeness review acceptable, English fluency limited. First preparation failed missing host-machine file, preserved; second serial Sandbox PASS and stopped, natural owned processes0, installed/profile/registry0, user data untouched. Desktop/IME/ordinary profiles/live Provider remain NOT_RUN. Main record CI39970685154 failed active-launcher uninstall timeout10s; exact hosted cause unknown. Added early read-only lock refusal before tree/compilation, all mutation safety checks and10s deadline retained; deterministic old-code RED, repaired local38 PASS, first console-encoding17 errors preserved. Hosted repair result separately recorded. No change to published ZIP; mission INCOMPLETE, Source stale. See PROTOTYPE_3_POST_RELEASE_VALIDATION.md and prototype3-gemma-20261023 evidence.


## 2026-10-24 repaired package revision r1

2026-10-24 Prototype3 r1 package revision published from clean5e18309 (main Windows CI39975857078 PASS). ZIP44798a7b584e4a1ee4bfd25e43a163a65aa18324ff9fce93f53189543935a5c8, 654 manifest files verified, repaired uninstaller hash equals source. Existing release tag/original ZIP retained; four distinct r1 assets added and GitHub SHA256 verified. Final owned Sandbox actual Gemma desktop RichEdit EN-KO/KO-EN/natural EN/natural KO4 captures+candidates+Accept+exact rereads/UI confirmations PASS; recommendation1151.8-1443.0ms, Accept75.3-110.4ms. Extra EM_UNDO checks FAIL4, CANUNDO0, original not restored; no all-checks PASS and no general undo claim. Final classification PARTIAL_UNDO_FAILED. Root cause/physical Ctrl+Z remain open. Outside-trial cleanup refused with synthetic sentinel intact; active launcher uninstall refused636.3ms with receipt preserved; normal uninstall complete. App/editor natural exit0, model/owned process/install/profile/registration residual0, all Sandboxes stopped. Build helper vctip explicitly reaped, not natural cleanup. Earlier launcher-path failure, premature UI assertion, Undo failure and read-only Sandbox mapping artifact-validation failure preserved. Final copy309.0s and model-ready14.43s separate from recommendations. No user documents/settings touched. Four reused paragraphs manual meaning review acceptable with stiff wording; not general/unseen quality. r1 Chromium/physical IME/ordinary profiles/Edge/live Cloud/reproducible builds NOT_RUN. Mission INCOMPLETE; Project Source stale. Evidence prototype3-r1-20261024, release assets PROTOTYPE_3_R1_VALIDATION.json and PROTOTYPE_3_R1_INSTALL.md.


## 2026-10-24 RichEdit Undo local repair qualification

2026-10-24 RichEdit Undo repair LOCAL CANDIDATE (runtime 2583623, no main/push/release): qualified RichEdit TSF SetText reproduced CANUNDO0; correction flag did not fix it. Bound same-thread TOM replacement now records editor Undo, with TSF identity/revision/privacy guards retained and actual TOM whole-story length/text and selection/range checked immediately before write and after write. No rollback/replay or automatic Undo. Late binding invalidation and source/selection changes are refused; full-length comparisons cannot accept truncated prefixes. Native transaction17 cases,2 Undo restorations and2 replay refusals PASS (including65534-unit source). Deferred EN_CHANGE was measured after DoEditSession completion; later application edit is preserved, not incorrectly reported as an in-transaction failure. Separate real-binding fixture FAIL18 identically before/after (input-scope VT_EMPTY before capture); this is unresolved and not counted as PASS. Final unmodified installed package actual Gemma4 reused paragraphs (EN-KO,KO-EN,natural EN,natural KO): capture+candidate+Accept+exact reread+UI Applied+EM_CANUNDO1+EM_UNDO exact source restoration all PASS. Recommendation1185.1-1457.2ms,Accept87.4-129.5ms;1s target misses4/4,paragraph3s misses0/4. Deployment217.926s/model-ready15.967s separate. Bounded manual meaning/subject/condition/politeness review acceptable; stiff English remains, not unseen/general quality qualification. First guest focus failed automatically; later activation acquired. Outside-trial deletion refused with synthetic sentinel intact; active uninstall refused713.9ms, final uninstall complete. App/editor/launcher natural exit0, guest process/install/profile/registration0; the same single Sandbox was stopped, remaining0. Host user settings/documents untouched. Final clean full build and654 package hashes verified; ZIP a1544be66032d6abb95a7901e8998a39fb3c4ecace07553ac8c4bdb8c30c1cf5. Two intermediate builds deliberately aborted for review hardening, owned cleanup0; old failures remain. Same-integrator review only. Physical Ctrl+Z/IME, general editors/rich formatting, Chromium, live Cloud, full Rust suite and Windows CI NOT_RUN this checkpoint. Published r1 still has its historical Undo limitation; this candidate has not been published. Mission INCOMPLETE, Project Source stale. Evidence D:/dev/grammar-evidence/richedit-undo-20261024/FINAL_RECEIPT.json.


## 2026-10-24 capture fixture activation and candidate CI

2026-10-24 capture fixture activation repair: the same unmodified fixture passed25 cases as the first owned Sandbox process, while later inactive processes reproduced18 failures with input-scope VT_EMPTY. Diagnostic showed SetForegroundWindow=false but thread focus=true; after actual foreground activation the same process passed25. Production privacy refusal was correct; no production guard or inference changed. Fixture now requires visible owned foreground+focus before capture, bounded30s readiness, and returns ENVIRONMENT_NOT_READY/78 rather than cascading18 product failures. Hidden-window negative returns78 without capture; positive same-process repeat3 gives75 capture scenarios,9 explicit Accept/Undo exact restorations and3 replay refusals PASS. Transaction17 cases also PASS. Helper cursor click failed, but foreground activation was observed; click is not claimed successful. Original failures/compile typo preserved. One Sandbox only, owned process0 at guest completion; supervisor shutdown recorded separately. Windows CI now compiles both C++ probes with W4/WX in addition to existing Rust/Node/TypeScript checks, explicitly build-only for GUI scope. Previously installed actual Gemma4-case qualification remains bound to runtime2583623, not rerun or relabeled here. Outgoing source is for codex/grammar-local-trial-ba58b7d; no main/release/personal install. Hosted outcome must be tied to exact pushed SHA and is recorded separately in capture-fixture-20261024/CI_RECEIPT.json. Mission INCOMPLETE; Project Source stale.


## 2026-10-24 Codex CLI compatibility investigation

2026-10-24 Codex compatibility candidate: installed runtime2583623 + CLI0.144.6 authenticated and captured synthetic RichEdit selection, but gpt-6-luna failed with codex_model_capability_unavailable; UI cancel preserved draft/source (618.180ms), no server-cancel claim from that UI evidence, installed Accept NOT_RUN. Same Sandbox/account with integrity-verified official CLI0.164.0 listed gpt-6-luna and completed actual protocol translation3249ms/rewrite2658ms/interrupt74ms with reported model gpt-6-luna, no tools, exit0. Source exact CLI pin updated to0.164.0, new generated stable schema320 files retained alongside immutable0.144.6 history, hardening unchanged. Updated production Rust client live safety: interrupt acknowledged1ms, INTERRUPTED, follow-up grammar2276ms, shutdown/runtime removal true. Production corpus10/10 responses, 2242-9114ms median2844ms,4 above3s; bounded review9 acceptable/1 temporal meaning failure (translate-context-03: checking moved before speech instead of before promised reply). Not general quality PASS. Installed-app Accept/new-version device login/new package/Windows CI NOT_RUN for this candidate. Raw model-capability UI error now explains compatibility without fallback. Initial harness failures preserved (page readiness, forbidden window-show IPC, login-wait expiry, missing native editor dependency); final preflight native editor write/reread/exit PASS. Cleanup and final evidence in D:/dev/grammar-evidence/codex-provider-20261024-r5/FINAL_RECEIPT.json. No host user data/credentials copied, no main/push/release. Mission INCOMPLETE, Project Source stale. Next: source-bound package with0.164.0, installed translation/rewrite/cancel/Accept, temporal-meaning defect.


## API provider preparation — 2026-10-24
Owner prioritizes Claude/Google/OpenRouter/xAI API connections. Reused existing
session-bound desktop API Deep flow and canonical writing contract. Added four
UI endpoint presets, exact Anthropic Messages protocol/header/parser support,
Google compatibility payload, OpenRouter no-fallback/ZDR routing restrictions.
HTTPS, no redirects/retries, bounded response/deadline, consent, cancellation,
credential memory lifetime and stale capture protection remain in force.
No dependency changes. Preset edits clear draft key/model/consent; saved setup is
explicitly displayed and only replaced by Save or revoked by Remove.
Validation: provider Rust 46 passed/3 live ignored; settings original-component
mock-Tauri headless Chrome 5 checks passed, zero external requests; TypeScript and
no-fallback contract passed. Evidence: D:/dev/grammar-evidence/api-providers-20261024.
This is source candidate implementation, not installed/new release acceptance.
Live API accounts/quality/latency/Accept remain NOT_RUN. Next: approved account
qualification of one selected API provider in a single owned synthetic session;
keep Codex CLI 0.164.0 installed-package qualification separately pending.


## API catalog selection — 2026-10-24
Added explicit authenticated catalog lookup for all four official presets and
manual dropdown selection. Provider-reported list presence is separate from
actual invocation (always unverified by this lookup). Fixed destinations, sensitive
headers, no inference payload, no retries/redirects, overall deadline and aggregate
limits; pagination cursors are encoded on the same destination. Unsupported rows
stay visible/disabled. Catalog changes never grant inference consent. Key/provider
edits and revocation invalidate transient UI results; late lookup responses cannot
restore a revoked list. Pending GET is bounded, not claimed remotely cancelled.
Review pass: examined credential destinations, pagination loops, malformed/oversize
responses, no automatic model selection and stale UI response handling. Same-agent
review only. No changes to Gemma, writing contract or Accept protections.
Validation: 51 Rust Provider PASS / 3 live ignored; 9 settings UI checks PASS;
TypeScript/frontend build and cargo check binary PASS (unused-code warnings remain).
Initial test-helper lifetime compile error was corrected; first UI assertion used
Playwright isDisabled on an option, corrected to check the native disabled attribute.
Evidence: D:/dev/grammar-evidence/api-providers-20261024/catalog-*.
Live API account/quality/inference/installed Accept and release remain NOT_RUN.
Next: source-bound package verification, then separately authorized live qualification.


## Prototype 3 r2 publication preparation — 2026-10-24
Owner authorizes merging CI-green API candidate d193456 into main and updating
GitHub Release. origin/main 5e18309 is an ancestor; existing untracked main audit
files must remain intact. Source CI 40063548607 passed at d193456. Its source-bound
package passed installed Settings selection/save/remove/restart in a network-disabled
Sandbox using catalog-response-only fixtures. First fixture injection failed because
Tauri invoke is non-writable; rerun changed only the external harness and passed.
Both attempts preserved; no product repair inferred. Final trial processes/VMs zero.
Release r2 adds explicit model-list instructions and live-API NOT_RUN limitations;
old assets/tag retained. New source-bound r2 build and main CI are required before
publication, with exact post-build receipts stored in the r2 release evidence.


## 2026-10-24 API secure persistence candidate

Owner explicitly requests Windows secure API-key storage, restart restoration of
Provider/model, key deletion and consent revocation, using synthetic keys only.
This narrowly supersedes the old session-only setup design; it is not permission
to inspect existing user credentials or perform API inference. Work remains on
codex/grammar-local-trial-ba58b7d from 5511f68; personal r2 install is unchanged.

One Generic Credential per Windows user/profile stores the selected endpoint,
model, consent and key together; the target is Pencil/API/v1 plus normalized
profile-path SHA-256. LOCAL_MACHINE persistence means same user on this computer,
not a system-wide credential. No vault enumeration, plaintext settings, fallback,
startup network request or key-return IPC. Same-user software is outside the
protection boundary. Keys are zeroized in owned runtime/serialization buffers.
Catalog lookup may explicitly reuse a saved key only for the exact endpoint.

Revoke retains key/model but durably removes consent and cancels the active
writing ticket. Delete cancels and deletes the credential. Shutdown only clears
memory. Failed storage mutations disable requests locally and report durable
revocation/deletion as unconfirmed; retry before restarting. The previous durable
record may survive a failed deletion. Oversized credential blobs fail closed;
there is no plaintext fallback. This is one selected setup, not a multi-account
key ring. Profile relocation changes the vault namespace; remove the saved key
before uninstalling/abandoning a profile. Uninstall never enumerates credentials.

Validation evidence: D:/dev/grammar-evidence/api-secure-storage-20261024.
Initial focused native17 PASS plus one child-only ignored entry; a parent test
actually launches that child process to read the UUID-scoped synthetic Windows
credential and revoke permission, then checks deletion in the parent. Original
React component with mocked Tauri10 PASS, frontend typecheck/build PASS. Full Rust
intermediate292 PASS33 ignored. Final strengthened regression results recorded
below. Windows tests address only a fresh random synthetic credential; personal
settings, dictionary, keys and the running personal r2 app remain untouched.

Review: separate same-integrator diff pass (no independent reviewer). Checked
startup restoration, no-secret status, endpoint binding, shutdown vs deletion,
stale save/cancel, storage errors, and raw Win32 ownership/free. New regression
covers cross-endpoint blank-key refusal, failed writes, credential-size limit,
no ordinary profile file creation and remounted UI restoration. Live API calls,
new installed-package/Sandbox acceptance, push/main/release are NOT_RUN this turn.
Mission remains INCOMPLETE; Project Source is stale for this persistence change.

Final: Rust293 PASS,0 FAIL,33 ignored (the vault child-only test is executed by
its passing parent); React/mock-Tauri11 PASS with remount/provider/model/consent
restoration and blank key field. Final frontend build/typecheck, no-fallback and
governance/self-test PASS. No live API or installed candidate qualification.
Native build supervisors report BUILD_COMPLETE_HELPERS_REAPED: one owned MSVC
helper per Rust run was reaped, all remainingAfterCleanup=0. UI supervisors PASS,
natural child exit, remaining0. No Sandbox launched and existing personal app
left intact. Next: build/package and isolated installed settings persistence trial
before updating the personal installation or publishing this candidate.


## 2026-10-24 Secure API installed-package and personal adoption

Fresh source7ddf7a6 release build/package installed side-by-side at
C:/Users/USER/PencilSecureAPI-7ddf7a6. Installed unmodified app, diagnostic WebView
port only, real Windows Credential Manager and synthetic keys: four presets
(Claude/Google/OpenRouter/xAI),13 sequential app launches, each verifies save,
Provider/model/consent restoration, blank-key same-endpoint model update, durable
revoke with key retained, durable delete with no storage error, and no key echo.
No API catalog/generation/network operation invoked; no successful live API claim.
No key in ordinary settings files. Synthetic credential deletion confirmed;
owned supervisor PASS/natural app+launcher shutdown/remaining0. No Sandbox used.

Initial build FAIL: Tauri normalized Cargo.toml CRLF to LF and the clean-source
receipt refused the changed checkout. Normalized bytes exactly matched HEAD;
index stat refreshed without content change. Fresh rerun PASS, source/binary
receipt verified, natural build cleanup0. Original failure retained (first
owned MSVC helper was reaped, remaining0). No threshold/check weakening.

Package ZIP SHA256 b9528320797b5f6f12202cd7e659b20777659cb0fef28d78c15ec1ba9b311c8c;
app SHA256 0aecbf2859f10eb39ce0c635621d410525d40182070b20fdb32e4aa9720ff781;
extension mggjcmnljkkpmjgedojjmifobkdaddjl. Evidence:
D:/dev/grammar-evidence/api-secure-package-20261024, build-receipt.json,
package-result.json, installed-result.json, installed-supervisor/supervisor-result.json,
personal-result.json and FINAL_RECEIPT.json. Test script retained alongside.

Owner-authorized personal adoption occurs only after PASS: new desktop shortcuts
Pencil Secure API - API/Gemma reference the original personal profile
C:/Users/USER/GrammarPersonalData-dffcf8a. Gemma shortcut retains D:/PencilAssets
runtime/model paths; no model copied or invoked. Original installation and
shortcuts retained. Checked settings/dictionary files were already absent in
known profile locations; existence/hash snapshot unchanged, no invented restore.
No synthetic setup copied to personal profile, and new normal personal app was
not started. Existing r2 PID33096 was present before this task; automatic normal
Quit failed because Computer Use could not obtain input geometry. It is retained
rather than force-killed (possible unsaved session state); Owner should Quit r2
before using the new shortcut. All newly created trial/build processes exited.

No source push/main/release update. Published r2 stays historical session-only.
Mission INCOMPLETE; Source materiality stale. Next optional work: candidate CI and
release refresh, or Owner-entered real API setup under a separate live-call scope.

## 2026-10-24 API catalog live setup and automatic lookup candidate

On the personal installed r3 package (source 743a06c), the Owner-entered Claude key successfully listed 13 provider models through the product UI. Claude Haiku 5.5 was present with ID claude-haiku-5-5. The original entered shorthand haiku-5.5 was replaced by dropdown selection. Windows secure save succeeded; after normal quit/relaunch the UI restored the exact Anthropic endpoint and model. Key material was not exported or logged. Generation consent remains denied pending Owner UI action; no paid writing request or API Accept was run. Catalog access is not generation acceptance.

Local source candidate: ApiSettings loads official-provider catalogs once when an edited nonempty key field loses focus. Buttons retain explicit behavior; moving to endpoint/provider editing does not send the old draft key. Startup/restoration does not contact a provider. Manual refresh remains. New React/ mocked Tauri coverage exercises once-per-entry behavior, no per-keystroke request, provider-switch key clearing and no remount request. Typecheck PASS; UI 13 checks PASS; governance PASS. Initial UI runner failed before launch because its default Playwright executable was absent; rerun used installed Chrome with sandbox enabled and a temporary headless profile. This code is not packaged/installed/CI-qualified yet. Current installed r3 continues to require Load model list.

Evidence: D:/dev/grammar-evidence/api-autoload-20261024/ui-result.json. The personal r3 remains open for Owner setup, not a residual synthetic trial.
