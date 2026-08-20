# P1-03 Personal MVP Acceptance Ledger

This ledger is deliberately content-free. Acceptance fixtures use synthetic,
non-sensitive data, and reports record only state, counts, reason codes, paths
that contain no user content, and command exit status.

| ID | Required claim | Pre-change probe | Final evidence | Gate |
|---|---|---|---|---|
| ISO-01 | Every writing turn uses an app-owned empty working directory outside repository and user-document roots. | Inspect `thread/start` and `turn/start` request contracts. | Rust contract test plus bounded real App Server request audit. | Required |
| ISO-02 | Writing threads are ephemeral and selected text is absent from persistent thread listings. | Inspect current `thread/start`; query a content-free synthetic marker through the pinned server. | Schema-contract test and live `thread/list` marker-count assertion. | Required |
| ISO-03 | Approval is `never`, sandbox is stable `readOnly`, network/tool expansion is disabled, and unexpected tool/permission activity fails closed. | Inspect request JSON and fake-server event handling. | Fake-server negative tests and adversarial live turn with zero forbidden item count. | Required |
| ISO-04 | Owned runtime directories have an ownership marker, bounded stale cleanup, and safe normal cleanup. | No current runtime directory owner exists. | Filesystem contract tests in an owned temporary root. | Required |
| INT-01 | Cancel, dismiss, recapture, intent/profile/revision change, and shutdown invalidate the active intent and issue at most one bounded interrupt when identifiers exist. | Inspect capture lifecycle and `turn/interrupt` use. | State/fake-server tests plus live cancel/recapture evidence. | Required |
| INT-02 | Interrupt failure, timeout, or child death cannot make a stale result Ready or Apply. | Existing stale guard plus missing interrupt probe. | Negative tests and child-kill recovery scenario. | Required |
| LIM-01 | Source is limited to 12,000 Unicode scalars and 48 KiB before any model request. | Direct command-contract probe. | Exact-boundary Rust tests with ASCII, Korean/CJK, emoji, and CRLF. | Required |
| LIM-02 | Model replacement is limited to 24,000 Unicode scalars and 96 KiB before Ready. | Structured-result parser probe. | Parser/schema boundary tests. | Required |
| LIM-03 | Final locally formatted Apply text is limited to 36,000 Unicode scalars and 144 KiB before clipboard/input mutation. | Apply command probe. | Fake-platform zero-mutation boundary tests. | Required |
| DISC-01 | A versioned cloud-processing acknowledgement is required before the first model turn and persists without content history. | Inspect settings and current selection listener. | Settings migration/command/UI contract tests and restart acceptance. | Required |
| URL-01 | Only the exact pinned HTTPS device URL authority can be opened; the frontend receives no arbitrary URL. | Inspect current Rust response and opener call. | URL allowlist negatives, IPC contract scan, and content-free device-flow evidence. | Required |
| CSP-01 | Production CSP is non-null and least privilege; arbitrary remote navigation/opener permission is absent. | Config/capability scan. | Static contract test and production executable smoke. | Required |
| CLIP-01 | A pure non-text clipboard aborts before sentinel or `Ctrl+C` and remains unchanged. | Current clipboard capture probe. | Classifier unit tests and ignored Windows image-only clipboard acceptance. | Required |
| CLIP-02 | Empty, text, mixed rich+text, external mutation, and copy fallback retain the documented text-only policy. | Existing P0 tests plus mixed-format gap probe. | P0 regressions and focused P1-03 clipboard tests. | Required |
| REG-01 | All focused/full Rust, ignored P0/P1, TypeScript contracts, typecheck, build, check, fmt, diff, hash, privacy, and scope gates pass. | Inventory existing commands and tests. | Recorded command/exit/pass-count matrix using external build targets. | Required |
| PROD-01 | A no-installer production executable builds outside tracked source, starts tray/widget, and exits cleanly. | External production build/start probe. | Process-bounded smoke with no bundle artifacts. | Required |
| LIVE-01 | Authenticated pinned App Server completes bounded synthetic correction, rewrite, translation, terminology, and adversarial turns. | Content-free `account/read` precondition. | Live scenario counters only; no prompt/output in logs or report. | Required |
| EDIT-01 | Real Notepad replaces only the captured selection after explicit Apply. | Existing target-bound harness and actual-app probe. | Target-local equality count and wrong-target paste count zero. | Required |
| EDIT-02 | Real Edge/Chrome local-file textarea passes the same target-bound contract. | Installed-app and local-file probe. | Target-local bounded equality comparison; owned temp file removed. | Required |
| EDIT-03 | Real Word or Hancom passes the same target-bound contract without saving a user document. | Installed-editor probe. | New unsaved synthetic document, target-local comparison, close without save. | Required |
| UX-01 | Non-default shortcut restart, conflict rollback, translation formats, terminology status, two-monitor placement, and keyboard-only controls work. | Existing ignored tests plus actual-app matrix. | Live matrix with content-free outcomes. | Required |
| PROC-01 | App Server child death invalidates the client and the next current request reconnects exactly once without replay. | Existing reconnect contracts. | Focused fake/live recovery test. | Required |
| PERS-01 | Preferences and terminology persist; capture, content, clipboard, result, and turn identifiers do not. | Store schema/static scan. | Owned app-data restart and corruption-recovery matrix. | Required |
| PRIV-01 | Forbidden capture, history, listener, telemetry, logging, arbitrary URL, and artifact counts are zero. | Static scans and runtime output capture. | Final privacy/data-retention scan with counts only. | Required |

## Conditional rules

- `MON-01`: two monitors are present on the acceptance machine, so the
  non-primary-monitor scenario is required rather than
  `NOT_APPLICABLE_SINGLE_MONITOR`.
- `EDITOR-01`: Microsoft Word is installed, so the Word/Hancom category is
  required and cannot be waived.
- Any missing authenticated pinned runtime, unavailable production executable,
  failed editor category, privacy isolation failure, or authority drift blocks
  PASS using the exact task-defined token family.

## Final evidence snapshot — 2026-08-20

All counts below are content-free. No selected text, rewrite output, clipboard
value, terminology text, account value, device code, URL, or process identifier
is recorded here.

| Area | Executed evidence | Result |
|---|---|---|
| Authority | `main` began clean at `0b150e40c03dcb76fffd909dd6f9ea809aba4e70`; all four accepted P0/P1 commits are ancestors. The 267-file protocol bundle recomputed to its retained canonical fingerprint, and both tracked lockfiles retained their authorized hashes with zero diff. | Green |
| Rust regression | Fresh external `CARGO_TARGET_DIR` full run discovered 160 tests: 149 passed, 0 failed, 11 ignored. The P1-03 contract group contains 20 passing tests, including independent settings/terminology backup recovery in one owned temporary app-data root. | Green |
| Ignored acceptance | All 11 ignored tests were separately invoked with `--exact --ignored --test-threads=1` and passed, including the pinned CLI resolver/handshake/live inference and every retained P0/P1 Windows gate. | Green |
| Frontend and static contracts | Four zero-dependency TypeScript contract scripts, `npm run typecheck`, external frontend build, `cargo check`, scoped/repository-wide format checks, and `git diff --check` passed. | Green |
| Pinned live inference | The authenticated exact supported App Server completed bounded synthetic correction, natural rewrite, both translation directions, matched terminology, and adversarial turns. Ephemeral thread-list matches and forbidden tool/MCP/command/approval/permission items were all zero. | Green |
| Notepad | Explicit Apply changed only the captured synthetic target; invalid/closed target produced no paste and used the typed fallback. The required non-primary-monitor scenario stayed on-screen and returned to the captured target. | Green |
| Browser | A temporary local-file textarea completed explicit Apply and closed-target fallback with no wrong-target paste. The owned temporary browser target was closed and no helper process remains. | Green |
| Word | A new unsaved synthetic document completed correction, both translation formats, approved terminology, inert suggested terminology, explicit Apply, and fallback. Equality checks passed without saving or reading a real document. | Green |
| Shortcut and accessibility | Persisted `Ctrl+Shift+H` survived restart and worked in Word. A live `Ctrl+Shift+J` registration conflict was rejected while H remained active. Keyboard-only cancel, Settings, Close, and Apply paths were exercised; saved terminology entries became visible before the add/edit form with distinct control labels. | Green |
| Process recovery and cancellation | Terminating only the owned App Server leaf invalidated the cached client; the next request created exactly one clean tree and succeeded without replay. A delayed current request was keyboard-cancelled; the old intent caused no target or clipboard mutation. | Green |
| Persistence and recovery | Shortcut, mode, translation target/format, disclosure acknowledgement, terminology flags/profile/entries/revision survived actual restart while session/content/clipboard/result/turn fields remained absent. Corrupt main files with valid independent backups recovered in one owned temporary app-data root. | Green |
| Privacy and retention | Production logging calls, local network listeners, telemetry/crash upload, forbidden capture/keylogging implementations, persisted forbidden fields, full selection fixtures, staged generated/runtime artifacts, and production stdout/stderr bytes were all zero. | Green |
| Production smoke | The external no-installer release executable started, owned the persisted H shortcut, left settings unchanged, emitted zero stdout/stderr bytes, and reduced its seven-process owned descendant tree to zero on shutdown. | Green |
| Cleanup | Codex Pencil, Word, Notepad, owned browser fixtures, shortcut-conflict helpers, and current marked runtime sessions all ended at zero. One historical empty unmarked runtime directory from 2026-08-14 was preserved because fail-closed ownership could not be proven. | Green with preserved pre-existing artifact |

The evidence is eligible for the task-defined PASS decision only after the final
scoped commit exists and the normal worktree, protocol fingerprint, and lockfile
hashes are reconfirmed clean.
