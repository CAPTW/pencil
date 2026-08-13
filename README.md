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
|   `-- verify-windows.ps1
|-- src
|   |-- App.tsx
|   |-- main.tsx
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
    |   |-- clipboard.rs
    |   |-- codex_client.rs
    |   |-- main.rs
    |   |-- prerequisites.rs
    |   `-- settings.rs
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
- `Ctrl+Shift+G` is registered by the Rust global-shortcut plugin.
- The tray menu includes Show, Hide, Login / Account, and Quit.
- On hotkey press, Rust saves the current text clipboard when possible, writes a sentinel, sends `Ctrl+C`, reads the copied selection, then restores the previous text clipboard when possible.
- The full selected text is kept in Rust memory for the current capture and is not stored in settings or logged. The local React preview receives the replacement, summary, confidence, and edit snippets (`before`/`after`/`reason`) returned by Codex so the user can review before applying.
- Rewrite modes: `grammar`, `natural`, `concise`, `polite`, `translate_en`, `translate_ko`.
- The Rust backend starts `codex app-server` with piped stdin/stdout/stderr. The default app-server transport is stdio, which is local to the child process.
- The app must not be changed to launch Codex app-server with a websocket, TCP listener, or non-local network transport.
- Rewrites use ephemeral Codex threads with `approvalPolicy: "never"` and a strict JSON schema requiring `{ replacement, changed, summary, confidence }`. Optional `edits` remain available for the existing review UI, and `additionalProperties` is `false`.
- Applying a rewrite writes the replacement to the clipboard, sends `Ctrl+V`, and restores the previous text clipboard when available and enabled.
- Settings are stored locally in the Tauri app config directory as `settings.json`.

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

## Privacy Behavior

- No continuous clipboard monitoring.
- No rewrite history storage.
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
- Password-field detection is not reliable in the current MVP, so the app does not claim to detect password fields. Do not use the hotkey in password or secret fields.
- Some apps block simulated `Ctrl+C` or `Ctrl+V`, run elevated, or use custom editors that do not expose selected text through the clipboard.
- Codex CLI must be installed and authenticated-capable.

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
- Select text in Notepad or another text field and press `Ctrl+Shift+G`.
- Confirm the floating window appears near the cursor and does not show the full selected source text.
- Verify each mode produces a non-empty replacement, summary, confidence, and optional local edit details.
- Confirm Apply is disabled until a rewrite result exists and remains disabled while a rewrite is pending.
- Click Apply and confirm the selected text in the original app is replaced.
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

`Ctrl+Shift+G` is fixed in the MVP. If registration fails because another app owns that shortcut, Codex Pencil shows a setup warning; shortcut customization is not implemented yet.
