# Grammar Chromium candidate

This candidate implements local Instant assistance for an explicitly enabled current document and non-sensitive field. It is not whole-product acceptance. Deep is unavailable; it makes no Provider request. Native desktop capture is withheld pending isolated positive/sensitivity acceptance, and native Apply is Copy-only.

Supported candidate fields: plain textarea and simple `contenteditable="true"` containing at most16 text nodes, in the top document. Rich editors, frames, shadow roots, hidden/readonly/password/sensitive fields and oversized documents are unavailable. Field labels/metadata are a conservative guard, not a promise to identify every semantic secret; enable only non-sensitive writing fields. No automatic text mutation or background cloud analysis occurs.

The extension action enables one current document. Select a supported field and click **Enable this field**. The adjacent annotation card shows precomputed suggestions. Accept and Apply edit recheck the current source/range and reread content; Dismiss/Ignore do not edit. Copy uses an explicit click, with select-and-Ctrl+C fallback if browser clipboard access is unavailable. Pause clears the field cache; Disable document or Alt+Shift+G disables it. The popup offers a domain block and emergency disable. Navigation requires fresh document opt-in. The default package has no native host configured.

Native browser undo is not guaranteed for these DOM replacements. Contenteditable caret/IME/device/zoom/long-session behavior and production toolbar activeTab permission still require qualification. Complex controlled editors may override input; a mismatched reread produces an error without retry. The card can overlap other wide elements; keyboard navigation remains available. Do not treat a synthetic success as support for an arbitrary website.

## Reproduce the isolated test

Use PowerShell7, Node, Rust and the repository lockfiles. From the mission worktree:

```powershell
npm ci --offline --ignore-scripts
cargo build --manifest-path src-tauri/Cargo.toml --locked --offline --bin grammar-chromium-host
node adapters/chromium/tests/core.test.mjs
node adapters/chromium/tests/worker.test.mjs
$env:GRAMMAR_PLAYWRIGHT = '<installed playwright package directory>'
$env:GRAMMAR_CHROMIUM = '<matching Chromium chrome.exe>'
$env:GRAMMAR_EVIDENCE = '<owned external evidence directory>'
node adapters/chromium/tests/browser.test.mjs
```

The browser harness creates its own profile, extension copy and synthetic localhost server (port18437). Only that copied test extension receives the localhost host permission; production uses activeTab. It registers a unique current-user native host and removes only that registration in `finally`, keeping evidence files. No existing browser profile, clipboard, account or document is used. Do not run against a port already owned by another process. The test emits content-free counts and synthetic result receipts, not screenshots.

For manual installation, `host/Prepare-Host.ps1` can prepare a fresh directory using a built executable and exact extension ID. It does not register unless `-Register` is explicitly supplied. Its receipt contains a unique host name; set that value in the prepared extension's `host-config.js` and reload it. `Remove-Registration.ps1` checks the exact owned manifest path before removal. Normal-profile installation remains outside this mission's executed actions; no such installation was performed.

## Resource and trust contract

8192 UTF-16 units/document;64 suggestions;60s cache TTL;250ms debounce;four active documents;one native request/document;5s Instant response timeout;128KiB native input frame. Text and suggestions stay in process memory. Only non-content denied-origin preferences persist. Host framing/origin/operation validation has no arbitrary shell/path execution. Hover/card focus does not invoke analysis. A service-worker restart loses permission and requires an explicit enable.

The host uses the existing Rust Instant engine. Deep requires a shared headless Provider facade, consent-bound document protocol and cancellable transport; it must not be implemented by copying CLI logic or enlarging the Instant timeout. Supported Provider choices remain codex, antigravity and claude in the desktop implementation. No real Provider inference was executed for this candidate.

Official implementation references: [Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging), [activeTab](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab), [Playwright extension testing](https://playwright.dev/docs/chrome-extensions). Checked2026-09-25; local synthetic results, not documentation, establish the tested behavior.
