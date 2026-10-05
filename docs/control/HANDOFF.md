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
