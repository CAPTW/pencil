# Codex Pencil

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
    |   |-- main.rs
    |   |-- prerequisites.rs
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
- Saved settings use schema version 3 and recoverable same-directory temporary-file promotion with a previous-file backup. Schema version 2 migrates without changing the shortcut, mode, translation target/format, clipboard restore, or auto-rewrite preferences. Terminology defaults to enabled with `general` active, approved matches and suggestions enabled, and `autoSaveSuggestions` fixed to `false`.
- The tray menu includes Show, Hide, Login / Account, Settings, and Quit. Settings shows and focuses the existing widget and opens its focused settings view.
- On hotkey press, Rust saves the current text clipboard when possible, writes a sentinel, sends `Ctrl+C`, reads the copied selection, then restores the previous text clipboard when possible.
- Before the widget is shown or focused, Rust captures the foreground target window and its owning process for a backend-owned capture session. Raw target handles and process identifiers are never sent to React.
- The full selected text is kept in Rust memory for the current capture and is not stored in settings or logged. React receives the selected text in the one-shot capture event only to derive the displayed character count, then retains the opaque session token rather than the source text. The local React preview receives the replacement, summary, confidence, and edit snippets (`before`/`after`/`reason`) returned by Codex so the user can review before applying.
- Rewrite modes: `grammar`, `natural`, `concise`, `polite`, and one promptless `translate` mode.
- Translation infers the source language and supports exact targets `ko`, `en`, `ja`, `zh-Hans`, and `zh-Hant`. There is no free-form Prompt or chat input.
- The model is instructed to return translated text only. Rust binds the result to the exact capture session, generation, mode, and target, then locally applies either `translation_only` or `source_with_translation` using the backend-owned exact source.
- The Rust backend starts `codex app-server` with piped stdin/stdout/stderr. The default app-server transport is stdio, which is local to the child process.
- The app must not be changed to launch Codex app-server with a websocket, TCP listener, or non-local network transport.
- Rewrites use ephemeral Codex threads with `approvalPolicy: "never"` and a strict JSON schema requiring `{ replacement, changed, summary, confidence }`. Optional `edits` remain available for the existing review UI, and `additionalProperties` is `false`.
- Applying a rewrite requires the exact current session identifier and generation. Rust validates lifecycle state, target-window validity, owning process, and actual foreground restoration before writing the replacement or sending one `Ctrl+V` sequence.
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

6. Codex returns `loginId`, `verificationUrl`, and `userCode`.
7. The UI displays `userCode` and provides a button to open `verificationUrl`.
8. The app listens for `account/login/completed` and `account/updated` notifications, and also polls `account/read` while waiting.

Requests are rejected locally until the handshake completes. A closed stdout stream, unwritable stdin, malformed protocol JSON, or child-process exit invalidates the cached client and fails pending requests. The next user request performs at most one clean reconnect; an in-flight request is not silently replayed.

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
- A current `Ready` session first revalidates the captured window and process, hides the widget, performs bounded activation attempts, and verifies `GetForegroundWindow()` immediately before `SendInput`.
- If a valid current target is missing, belongs to a different process, or cannot be proven foreground, the approved replacement is copied without `Ctrl+V`. The widget reports that the user must paste manually.
- A zero or partial `SendInput` result is a typed failure and cancels the session so automatic retry cannot duplicate or redirect uncertain input.
- Clipboard restore is supported for text only. The app records clipboard sequence ownership, uses newer readable text as the restore candidate when another actor changed the clipboard before Apply, and restores only while the replacement sequence is still app-owned. A later external clipboard change is never overwritten. Copy-only fallback intentionally leaves the approved replacement on the clipboard.
- The ignored interactive harness uses only synthetic text and can be run with a separate target directory:

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
- Selected text is sent to Codex only after the user presses `Ctrl+Shift+G` and the app performs a rewrite.
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
- Password-field detection is not reliable in the current MVP, so the app does not claim to detect password fields. Do not use the hotkey in password or secret fields.
- Some apps block simulated `Ctrl+C` or `Ctrl+V`, run elevated, or use custom editors that do not expose selected text through the clipboard.
- Codex CLI must be installed and authenticated-capable.
- Superseded translation requests are discarded when they complete, but this gate does not interrupt an already-running Codex App Server turn.
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
- Select text in Notepad or another text field and press the currently displayed primary shortcut (default `Ctrl+Shift+G`).
- Open Settings from the widget and tray, register a non-default shortcut, confirm the old shortcut stops firing, restart, and confirm the saved shortcut remains active. Reset and confirm `Ctrl+Shift+G` is active again.
- Confirm the floating window appears near the cursor and does not show the full selected source text.
- Verify each rewrite mode produces a non-empty replacement, summary, confidence, and optional local edit details.
- In Translate mode, verify all five targets, `translation_only`, and exact local `source_with_translation` composition for single-line and CRLF multiline source.
- Confirm Apply is disabled until a rewrite result exists and remains disabled while a rewrite is pending.
- Click Apply and confirm the selected text in the original app is replaced.
- Close the captured target before Apply and confirm no automatic paste occurs, the replacement remains on the clipboard, and the widget instructs you to paste manually.
- Start a new capture before an older rewrite completes and confirm the older result cannot replace or Apply against the new capture.
- Confirm the previous text clipboard is restored when Restore clipboard is enabled and the previous clipboard was text.
- Disable Restore clipboard and confirm the replacement remains on the clipboard after Apply.
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
