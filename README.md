# Codex Pencil

Current product direction, audit boundaries and development authority are in [`docs/control/CURRENT_DECISIONS.md`](docs/control/CURRENT_DECISIONS.md) and [`control/state.json`](control/state.json). Read [`AGENTS.md`](AGENTS.md) before new work. P3-B proactive inline assist is a frozen future target, not an implemented capability; the setup and historical contract notes below describe the existing P3-A application.

Codex Pencil is a compact Windows tray/widget writing assistant built with Tauri v2, React, TypeScript, and Rust commands. It uses the local Codex app-server over stdio and ChatGPT managed Codex device-code login. It does not implement custom OpenAI OAuth, run a backend server, expose a network listener, add monetization, or create app-specific user accounts.

## File Tree

```text
.
|-- README.md
|-- index.html
|-- package-lock.json
|-- package.json
|-- tsconfig.json
|-- tsconfig.node.json
|-- vite.config.ts
|-- scripts
|   |-- build-tauri.ps1
|   |-- test-capture-contract.mjs
|   |-- test-promptless-contract.mjs
|   |-- test-terminology-contract.mjs
|   `-- verify-windows.ps1
|-- src
|   |-- App.tsx
|   |-- captureContract.ts
|   |-- main.tsx
|   |-- promptlessContract.ts
|   |-- terminologyContract.ts
|   |-- styles.css
|   `-- vite-env.d.ts
`-- src-tauri
    |-- Cargo.toml
    |-- build.rs
    |-- capabilities
    |   `-- default.json
    |-- icons
    |   |-- 32x32.png
    |   |-- icon.ico
    |   `-- icon.png
    |-- src
    |   |-- apply_safety.rs
    |   |-- capture_session.rs
    |   |-- clipboard.rs
    |   |-- codex_client.rs
    |   |-- codex_home.rs
    |   |-- content_limits.rs
    |   |-- device_login.rs
    |   |-- main.rs
    |   |-- process_job.rs
    |   |-- prerequisites.rs
    |   |-- runtime_isolation.rs
    |   |-- shortcut.rs
    |   |-- settings.rs
    |   |-- terminology.rs
    |   |-- terminology_import_export.rs
    |   |-- terminology_matcher.rs
    |   |-- terminology_service.rs
    |   |-- terminology_store.rs
    |   |-- terminology_validation.rs
    |   |-- translation.rs
    |   |-- windows_apply.rs
    |   `-- windows_target.rs
    `-- tauri.conf.json
```

## Windows Setup

Prerequisites:

- Windows 10/11.
- Node.js and npm.
- Exact supported Codex CLI: `codex-cli 0.144.6`, resolved as described below.
- Rust toolchain via `rustup` for building the Tauri app.
- Microsoft C++ Build Tools with the Desktop development with C++ workload.
- Microsoft Edge WebView2 Runtime if it is not already installed in the environment.

`cargo` and `rustc` are build-time requirements. They are not part of the intended runtime path for rewriting text; at runtime the app needs the bundled Tauri app and the exact supported Codex CLI. The runtime resolver uses a non-empty `CODEX_PENCIL_CODEX_BIN` first, then the first `codex.cmd` on `PATH`, and finally the first `codex.exe` on `PATH`. A configured override is authoritative and is not silently bypassed.

If Rust was just installed and `where cargo` or `where rustc` still fails, open a new terminal or add `%USERPROFILE%\.cargo\bin` to the current session PATH.

Install dependencies:

```powershell
cd D:\dev\repos\Grammar
npm install
```

Run locally:

```powershell
npm run dev
```

Build a Windows bundle:

```powershell
npm run build
```

Current release artifacts are written under `src-tauri\target\release`:

```text
src-tauri\target\release\codex-pencil.exe
src-tauri\target\release\bundle\msi\Codex Pencil_0.1.0_x64_en-US.msi
src-tauri\target\release\bundle\nsis\Codex Pencil_0.1.0_x64-setup.exe
```

### Private portable personal bundle

For current daily use, build the R8 portable folder with `scripts/build-daily-use-bundle.ps1`. The historical P2-01 personal-use deliverable below remains the reproducible portable ZIP used by that earlier packaging chain, not the developer bundle output above. It is unsigned, has no updater or public publication step, and keeps exact `codex-cli 0.144.6` as an external prerequisite. Packaging uses existing locked dependencies only; do not run an install or update command as part of this workflow.

After the packaging workflow source commit exists and the worktree is clean:

```powershell
$packagingHead = (git rev-parse HEAD).Trim()
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-personal-bundle.ps1 `
  -ExpectedSourceCommit $packagingHead `
  -OutputBase 'D:\dev\artifacts\Codex-Pencil\P2-01'
```

The script rejects an existing commit-bound output root, builds frontend and Rust release outputs externally with Cargo offline and locked, emits the portable directory and ZIP, and verifies both. See [`docs/PERSONAL_BUNDLE.md`](docs/PERSONAL_BUNDLE.md) for the artifact layout, verification command, private-use limitations, and content-free troubleshooting codes.

If the app cannot find `codex`, set an explicit path before running:

```powershell
$env:CODEX_PENCIL_CODEX_BIN = "C:\Users\USER\AppData\Roaming\npm\codex.cmd"
npm run dev
```

## Verification

Manual checks:

```powershell
cd D:\dev\repos\Grammar
where codex
where cargo
where rustc
codex --version
cargo --version
rustc --version
npm run typecheck
node scripts/test-capture-contract.mjs
node scripts/test-promptless-contract.mjs
node scripts/test-terminology-contract.mjs
npm run build:frontend
npm run build
```

Helper script:

```powershell
npm run verify:windows
```

The helper prints Node/npm/Codex/Rust tool versions, runs the frontend checks, and runs `npm run build` only when both `cargo` and `rustc` are available. If Rust is missing, it clearly skips the full Tauri build.

`npm run build` also adds the standard Rustup bin directory to its own build-session PATH when Rust is installed there but the current terminal has not picked it up yet.

## Implementation Notes

- The app runs as a Tauri tray app with one hidden floating window.
- The Rust global-shortcut plugin registers one primary shortcut. Its default is `Ctrl+Shift+G`; Settings can transactionally replace it or reset it to the default.
- A candidate shortcut is normalized and registered before the old binding is removed. Registration conflicts leave the old runtime binding and persisted setting unchanged; persistence failure attempts a bounded rollback to the old binding.
- Saved settings use schema version 4 and recoverable same-directory temporary-file promotion with a previous-file backup. Earlier schemas migrate without changing the shortcut, mode, translation target/format, clipboard restore, or terminology preferences. The only new persisted privacy field is the version number of the user's cloud-processing acknowledgement; selected text, results, and acknowledgement content are not stored. Terminology defaults to enabled with `general` active, approved matches and suggestions enabled, and `autoSaveSuggestions` fixed to `false`.
- The tray menu includes Show, Hide, Login / Account, Settings, and Quit. Settings shows and focuses the existing widget and opens its focused settings view.
- On hotkey press, Rust reads the selection directly from the focused control of the foreground window, and only when that control is a standard Windows `Edit` control: exact class `Edit`, Unicode, visible, enabled, not `ES_PASSWORD`, no password character, owned by the window's process, not a known password-manager or credential UI process, and under 65,535 UTF-16 units (for example classic Notepad or a dialog text box). Every other control, including Windows 11 Notepad, RichEdit, WinForms/WPF, browsers and Office, is denied before any text-bearing message. Capture never uses the clipboard or simulated keys.
- The primary shortcut is a toggle: pressed while the widget is visible it hides the widget and cancels the current capture. Press it again to capture a new selection.
- Before the widget is shown or focused, Rust captures the foreground target window and its owning process for a backend-owned capture session. Raw target handles and process identifiers are never sent to React.
- The full selected text is kept in Rust memory for the current capture and is not stored in settings or logged. React receives the selected text in the one-shot capture event only to derive the displayed character count, then retains the opaque session token rather than the source text. The local React preview receives the replacement, summary, confidence, and edit snippets (`before`/`after`/`reason`) returned by Codex so the user can review before applying.
- Rewrite modes: `grammar`, `natural`, `concise`, `polite`, and one promptless `translate` mode.
- Translation infers the source language and supports exact targets `ko`, `en`, `ja`, `zh-Hans`, and `zh-Hant`. There is no free-form Prompt or chat input.
- The model is instructed to return translated text only. Rust binds the result to the exact capture session, generation, mode, and target, then locally applies either `translation_only` or `source_with_translation` using the backend-owned exact source.
- The Rust backend starts `codex app-server` with piped stdin/stdout/stderr. The default app-server transport is stdio, which is local to the child process. On Windows, the launcher and every npm/native descendant are assigned to an app-owned Job Object so shutdown and reconnect terminate the complete process tree.
- The app must not be changed to launch Codex app-server with a websocket, TCP listener, or non-local network transport.
- App Server uses a marked, empty, per-client working directory under `%TEMP%\codex-pencil-runtime-v1`. It is never the repository, Documents, Desktop, OneDrive, or a user project, and it is removed after bounded process shutdown. A separate app-owned `%LOCALAPPDATA%\com.local.codexpencil\codex-home-v1` isolates Codex Pencil authentication from the user's general Codex config, MCP servers, plugins, skills, and hooks. Codex itself owns the credential payload; Codex Pencil neither reads nor copies token values. A `config.toml` in this dedicated home is rejected fail-closed.
- Rewrites use ephemeral Codex threads with `approvalPolicy: "never"`, the pinned stable `readOnly` sandbox policy with tool network disabled, and a strict JSON schema requiring `{ replacement, changed, summary, confidence }`. Optional `edits` remain available for the existing review UI, and `additionalProperties` is `false`. Shell, MCP, plugin, skill, browser, image, multi-agent, history, analytics, and telemetry surfaces are disabled at child startup; unexpected tool, approval, permission, or dynamic activity fails the turn.
- Source text is rejected before client/thread acquisition above 12,000 Unicode scalars or 48 KiB UTF-8. Model replacement is rejected above 24,000 scalars or 96 KiB, and locally formatted final Apply text is rejected before clipboard/input mutation above 36,000 scalars or 144 KiB. Text is never truncated or silently coerced.
- Applying a rewrite requires the exact current session identifier and generation. Rust validates lifecycle state, target-window validity and owning process. For a standard Edit capture it then locks the control read-only against typing, re-verifies the full-text hash and the exact selection, replaces that range with one undoable `EM_REPLACESEL`, and re-reads the exact expected text before unlocking. Anything else ends in Copy-only; no simulated `Ctrl+V` is sent.
- Settings are stored locally in the Tauri app config directory as `settings.json`; they contain preferences only, not selected text, replacements, clipboard contents, or history.

## Codex Auth Model

Codex Pencil uses Codex managed auth through the local app-server protocol:

1. The Rust backend starts `codex app-server`.
2. It sends one `initialize` request and awaits its response.
3. It sends an id-less `initialized` notification and only then marks the connection ready. Experimental API capability is not enabled.
4. It reads auth state with `account/read`.
5. Device-code login starts with:

```json
{
  "id": 1,
  "method": "account/login/start",
  "params": {
    "type": "chatgptDeviceCode"
  }
}
```

6. Codex returns `loginId`, `verificationUrl`, and `userCode`. Rust accepts only the exact pinned `https://auth.openai.com/codex/device` URL and keeps it in backend memory.
7. The UI receives only the opaque `loginId` and `userCode`. Opening the login page requires the matching backend-held validated login; arbitrary frontend URLs are not accepted.
8. The app listens for `account/login/completed` and `account/updated` notifications, and also polls `account/read` while waiting.

The app-specific Codex home is intentionally separate from the user's normal Codex CLI home, so the first production run may require one device login even when the general CLI is already authenticated. Requests are rejected locally until the handshake completes. A closed stdout stream, unwritable stdin, malformed protocol JSON, or child-process exit invalidates the cached client and fails pending requests. The next user request performs at most one clean reconnect; an in-flight request is not silently replayed.

## Codex App Server Protocol Authority

- Supported version: `codex-cli 0.144.6`.
- Retained schema bundle: `src-tauri/protocol/codex-cli-0.144.6/`.
- Schema generation: `& $resolvedCodex app-server generate-json-schema --out src-tauri/protocol/codex-cli-0.144.6`.
- Experimental fields were not requested; `--experimental` was not passed.
- Generated JSON files: 267.
- Canonical bundle SHA-256: `c954593823626b5194b7f687c27d542eb87fcfa63e5e11af3ca95f9b6c39c6e8`.

The fingerprint algorithm and executable resolution policy are recorded in `src-tauri/protocol/codex-cli-0.144.6/PROVENANCE.md`. Regenerate into a new versioned directory and update the resolver pin, tests, README, and fingerprint together when intentionally supporting a different Codex version.

The app does not implement custom OAuth, does not store or display tokens, does not use an OpenAI API key, and does not expose a network listener.

## Target-bound Apply Contract

- Each successful capture creates one in-memory lifecycle: `Captured -> Rewriting -> Ready -> Applying -> Completed`, with explicit retry/cancellation transitions for failures.
- A new capture or dismiss invalidates the old token. An asynchronous rewrite completion is accepted only if its session identifier and generation are still current; stale completions cannot become `Ready`.
- Translation additionally binds `Ready` and Apply to the exact mode and target language. A target/mode change invalidates an older result without changing the clipboard or sending input. Apply-format changes remain local and do not require another model request.
- Terminology-aware rewrites additionally bind `Ready` and Apply to the enabled flags, active profile, store revision, and deterministic matched entry IDs. A profile, dictionary, or terminology-setting change makes the older result stale before clipboard or input activity.
- Stale or mismatched Apply requests are rejected before any clipboard write, widget action, or keyboard input.
- A current `Ready` session first revalidates the captured window and process. A standard Edit capture is then applied with the locked verify-replace-reread protocol above. If a selection change slips in between selecting and replacing (for example a mouse click), the re-read proves that only the replacement moved, and the verified original text is restored under the same lock with the modification flag and the moved selection. This rare recovery clears that control's single-level undo and ends in Copy-only. An outcome that cannot be verified cancels the session without retry.
- Every other case copies the approved replacement without sending keys, and the widget reports that the user must paste manually. That includes a missing target or a changed process; a moved cursor or changed text or editor; a read-only or length-limited field; and any target not captured by the native reader.
- Verified native Apply never touches the clipboard. Copy-only fallback intentionally leaves the approved replacement on the clipboard. The stored Restore clipboard preference is kept in the settings schema but is no longer shown in the widget.
- The earlier clipboard-and-`SendInput` paste path is not reachable in production because no production platform grants it selection authority. Its ignored interactive harness uses only synthetic text and can still be run with a separate target directory:

```powershell
$env:CARGO_TARGET_DIR = "$PWD\src-tauri\target\p0-02-verification"
cargo test --manifest-path src-tauri/Cargo.toml p0_02_windows_live_tests::windows_live_target_bound_apply_acceptance -- --ignored --exact --nocapture
```

The harness requires an interactive Windows desktop where `SendInput` is actually delivered to the verified foreground editor. An environment that accepts the input records but does not deliver them cannot be treated as live acceptance evidence.

P1-01 ignored live checks use actual Windows global-hotkey registration, an actual Tauri tray/window runtime, and the existing target-bound editor harness with synthetic text only:

```powershell
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP "grammar-p1-01-target"
cargo test --manifest-path src-tauri/Cargo.toml p1_01_windows_live_tests::p1_01_windows_live_shortcut_registration_conflict_restart_and_reset -- --ignored --exact --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml p1_01_windows_live_tests::p1_01_windows_live_tray_settings_action_shows_focuses_and_emits_settings_mode -- --ignored --exact --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml p0_02_windows_live_tests::p1_01_windows_live_translation_formats_acceptance -- --ignored --exact --test-threads=1
```

## Local Terminology Memory

The personal dictionary is local product configuration, not document or rewrite history.

- The store is `terminology.v1.json` under Tauri `app_data_dir`; `terminology.v1.json.bak` retains a valid recovery source. Writes validate the complete next store, sync a same-directory temporary file, and promote it without partially updating in-memory state.
- A new store contains reserved `global` and `general` profiles. `global` is always enabled and implicitly active; settings select exactly one enabled non-global profile.
- Entry types are `translation`, `preferred`, and `protected`; states are `approved`, `suggested`, and `disabled`. Suggested and disabled entries are inert. Suggestions require an explicit Save as suggested action and a later explicit Approve action before matching.
- Settings shows the filtered saved-entry count and entry cards before the add/edit form. Filter and form controls have distinct accessible labels, and a hidden-entry hint appears when active filters exclude stored entries.
- Automatic matching is local, NFC-normalized, exact `whole_phrase` matching with aliases and deterministic profile/language/length/priority/time/ID precedence. It does not use fuzzy matching, embeddings, document scanning, or a database.
- Only approved entries from `global` and the active profile that actually match the backend-owned selected text are serialized as request constraints. The subset is capped at 50 entries and 16 KiB. Notes, profile names, usage counters, timestamps, unmatched entries, suggested/disabled entries, UI search text, and the full dictionary are not sent.
- Selected text and terminology constraints are encoded as untrusted JSON data in the existing stdio App Server request. Protected terms are preserve-exact constraints; translation and preferred entries carry only the matched source and preferred form.
- Result validation is local. Missing protected or preferred forms, unverified usage, matcher truncation, and conflicts produce review warnings; they do not silently rewrite or auto-Apply the result.
- `usageCount` is best-effort and increments only after the user approves an Apply or copy-only action. No sentence, result, clipboard value, Prompt, or model output is stored with it.
- JSON export is the versioned profile/entry store. CSV uses fixed RFC 4180 columns and CRLF records. Import accepts at most 2 MiB, performs an expiring dry run, reports duplicates/conflicts, applies only valid non-conflicting data in one revision, and rejects a stale plan if the store changed.
- If both main and backup are invalid, the app exposes a content-free unrecoverable state and does not overwrite either automatically. The user must explicitly reset or import a valid recovery file.

Focused checks:

```powershell
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP "grammar-p1-02-target"
cargo test --manifest-path src-tauri/Cargo.toml p1_02_contract_tests:: -- --test-threads=1
node scripts/test-terminology-contract.mjs
npm run typecheck
```

The ignored P1-02 Windows acceptance uses only owned temporary app data and bounded synthetic editor windows. It exercises persistence/recovery and terminology-bound target Apply without live model inference:

```powershell
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP "grammar-p1-02-live"
cargo test --manifest-path src-tauri/Cargo.toml p1_02_windows_live_tests::p1_02_windows_live_store_profile_suggestion_and_import_restart_acceptance -- --ignored --exact --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml p1_02_windows_live_tests::p1_02_windows_live_matched_validation_and_target_bound_apply_acceptance -- --ignored --exact --test-threads=1
```

## Privacy Behavior

- No continuous clipboard monitoring.
- No rewrite history storage.
- No document, clipboard, Prompt, or terminology-request history storage. Dictionary exports contain only local profiles and terminology entries.
- Before the first inference, the app explains that the selected text and only matched approved terminology are processed by Codex/ChatGPT in the cloud. Cancelling invalidates the capture and sends no model request; acknowledgement persists only as a versioned preference.
- Selected text is sent to Codex only after the user presses the configured shortcut, accepts the cloud disclosure, and the app performs a rewrite.
- The local preview UI may receive and display replacement text plus short edit metadata. Edit metadata can include snippets from the selected text.
- Prompts and selected text should not be logged. Codex app-server stderr is drained and discarded by the app.
- Rewrite application is always user-confirmed through the Apply button.

## Second-Pass Verification Notes

- Supported and verified `codex --version`: `codex-cli 0.144.6`.
- `rustc` and `cargo` were installed at `%USERPROFILE%\.cargo\bin`, but the original shell PATH did not include that directory.
- With `%USERPROFILE%\.cargo\bin` temporarily prepended to PATH, `npm run build` completed and produced MSI/NSIS bundles.
- With the same temporary PATH, `cargo test` passed all Rust tests.
- `npm run verify:windows` now adds the standard Rustup bin directory to its own session PATH when needed.

## Known Limitations

- Clipboard restore is text-only. Complex clipboard formats such as images, files, rich HTML/RTF, or app-private formats may not be preserved after the copy/paste workflow.
- Clipboard sequence checks narrow ownership races but cannot atomically preserve or reconstruct unsupported rich/non-text formats. Copy-only fallback deliberately replaces the clipboard text and does not restore it.
- Native capture denies `ES_PASSWORD` fields, fields that report a password character, and a fixed list of password-manager and credential UI processes before any text-bearing message. A secret typed into an ordinary Edit field of another app cannot be recognized. Do not use the hotkey in secret fields.
- Only standard Edit controls are supported natively. Rich or custom editors, elevated apps (UIPI) and apps that answer slowly are denied or end in Copy-only. Use the Chromium adapter for browser text.
- Codex CLI must be installed and authenticated-capable.
- Cancel, dismiss, recapture, mode/target/profile/revision changes, and application exit issue at most one bounded `turn/interrupt` for the exact active turn. A timeout or process failure still relies on the existing stale-result backstop; the app never retries an uncertain old turn or applies its completion.
- The pinned `codex-cli 0.144.6` `readOnly` schema exposes `networkAccess` but no stable custom readable-root list. Codex Pencil therefore combines the narrowest schema-valid sandbox with an empty isolated cwd, a dedicated config-free Codex home, disabled tool surfaces, and fail-closed event inspection. Supporting a newer CLI requires a new versioned schema authority and fresh acceptance.
- Terminology matching is intentionally exact and deterministic. This MVP has one active non-global profile, no automatic app/document profile switching, no fuzzy or semantic matching, and no cloud synchronization.
- The terminology backup is a recovery source, not a journal. Recovery can roll back the most recent semantic mutation if the newest main file becomes invalid.
- CSV is an entry-oriented interchange format: its fixed columns do not encode profile enablement or profiles with no entries. New CSV-only profiles are imported enabled but never become active automatically; use JSON for a full-fidelity local profile/store backup.

## Runtime Smoke Test Checklist

- Launch the release executable directly:

```powershell
cd D:\dev\repos\Grammar\src-tauri\target\release
.\codex-pencil.exe
```

- Confirm the app starts without a dev server and shows a system tray icon.
- Use the tray menu to show and hide the floating window.
- Use the tray Quit item and confirm `codex-pencil.exe` exits.
- After Quit, confirm no Codex Pencil-owned `codex app-server` child remains. Other Codex desktop app-server processes may exist separately.
- If not logged in, click Start device login, confirm the browser opens, enter the displayed device code, and wait for the app to show the ChatGPT account.
- Cancel a device login and confirm the UI shows Login cancelled.
- Select text in a standard Edit field (classic Notepad or a dialog text box) and press the currently displayed primary shortcut (default `Ctrl+Shift+G`). Confirm that an unsupported editor shows the unsupported-editor message without reading text.
- Open Settings from the widget and tray, register a non-default shortcut, confirm the old shortcut stops firing, restart, and confirm the saved shortcut remains active. Reset and confirm `Ctrl+Shift+G` is active again.
- Confirm the floating window appears near the cursor and does not show the full selected source text.
- Verify each rewrite mode produces a non-empty replacement, summary, confidence, and optional local edit details.
- In Translate mode, verify all five targets, `translation_only`, and exact local `source_with_translation` composition for single-line and CRLF multiline source.
- Confirm Apply is disabled until a rewrite result exists and remains disabled while a rewrite is pending.
- Click Apply and confirm the selected text in the original standard Edit field is replaced, the clipboard is unchanged, and `Ctrl+Z` in that field undoes the Apply.
- Close the captured target before Apply and confirm no automatic paste occurs, the replacement remains on the clipboard, and the widget instructs you to paste manually.
- Start a new capture before an older rewrite completes and confirm the older result cannot replace or Apply against the new capture.
- Move the cursor or edit the text after capture, click Apply, and confirm nothing in the field changes and the replacement is on the clipboard (Copy-only).
- Press the hotkey with no selected text and confirm the No text selected state.
- Copy an image or file reference to the clipboard, run a text rewrite, and confirm the documented text-only clipboard limitation is acceptable.
- Inspect stdout/stderr or any local diagnostic output and confirm selected text, prompts, raw model output, clipboard contents, and tokens are not logged.
- Smoke test one installer when appropriate:

```powershell
cd D:\dev\repos\Grammar\src-tauri\target\release\bundle
.\nsis\Codex` Pencil_0.1.0_x64-setup.exe
# or
msiexec /i ".\msi\Codex Pencil_0.1.0_x64_en-US.msi"
```

- Launch the installed app, confirm tray startup/login screen/quit behavior, confirm it can find `codex` on `PATH`, then verify the uninstall entry exists.

If a saved shortcut cannot be registered at startup, Codex Pencil attempts the default `Ctrl+Shift+G`, exposes a content-free recovery warning, and keeps Settings reachable from the tray.
