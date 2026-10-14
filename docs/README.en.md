<div align="center">

# ✏️ Pencil

### Write where you already write.

**A review-first AI writing assistant for Windows.**  
Select text, review the suggestion, and apply it only when you choose.

<p>
  <img alt="Release" src="https://img.shields.io/github/v/release/CAPTW/pencil?include_prereleases&sort=semver&style=flat-square&label=release">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20x64-0078D4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Status" src="https://img.shields.io/badge/status-prototype-F59E0B?style=flat-square">
  <img alt="Local first" src="https://img.shields.io/badge/AI-local--first-4F46E5?style=flat-square">
</p>

[**Download Prototype 2**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2) · [한국어](../README.md) · [Getting started](GETTING_STARTED.ko.md) · [Report a bug](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

</div>

> [!IMPORTANT]
> Pencil is currently an **unsigned Windows x64 prerelease**. It does not claim universal editor support. Start with short, non-sensitive test text before using it with important documents.

## Why Pencil?

Pencil is designed to start from the text you are already writing instead of making you move your work into a separate AI chat.

| | |
| --- | --- |
| ⚡ **Local-first** | Optional local inference can handle translation and rewriting on your device. |
| ✅ **Review first** | Suggestions are not silently written back. Review, edit, then choose Accept or Copy. |
| 🎯 **Scoped Apply** | Replacement can be refused when the target, source text, or range has changed. |
| 🌐 **Browser opt-in** | Browser assistance runs only in explicitly enabled Chromium documents and supported fields. |
| ☁️ **Deep is optional** | Deeper rewriting uses a selected Provider/API only after explicit consent. |

## How it works

**Desktop**  
`Select text` → `Ctrl + Shift + G` → `Instant suggestion` → `Review / edit` → `Accept` or `Copy`

**Chromium**  
`Enable document` → `Enable field` → `Review suggestion` → `Accept`

The core rule is simple: **AI suggests; you decide what changes.**

## Highlights

- **Rewrite & translate** with fixed writing intents such as natural, concise, and polite.
- **Editable review** before anything is applied.
- **Explicit replacement** of a verified source range on supported targets.
- **Inline browser assist** for opted-in textarea and supported simple contenteditable fields.
- **Optional Deep** through a selected CLI Provider or OpenAI-compatible API path.
- **Privacy boundaries** for sensitive targets, opt-in scope, and cloud consent.

## Quick start

### 1. Download

Get the Windows x64 ZIP and `SHA256SUMS.txt` from the [**Prototype 2 release**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2). GitHub’s auto-generated **Source code (zip)** is not the executable package.

Requirements: **Windows x64 · PowerShell 7 · Microsoft WebView2**.

### 2. Install

~~~powershell
.\Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA\PencilPrototype1"
Set-Location "$env:LOCALAPPDATA\PencilPrototype1"
~~~

### 3. Choose how to run

Prepare local AI once, then start Pencil:

~~~powershell
.\Prepare-GrammarLocal.ps1
.\Start-Grammar.ps1
~~~

Or start without loading a local model:

~~~powershell
.\Start-Grammar.ps1 -WithoutLocalModel
~~~

See the [getting-started guide](GETTING_STARTED.ko.md) for model, hardware, installation, troubleshooting, and uninstall details.

## Current support

| Target | Status |
| --- | --- |
| Qualified Windows Edit / RichEdit controls | 🟢 Scoped prototype support |
| Chromium textarea / simple contenteditable | 🟢 Explicit opt-in, scoped support |
| Additional desktop application adapters | 🧪 In development / bounded qualification |
| Arbitrary editors, Office/Hancom in general, every website | ⚪ Not generally qualified |
| macOS / Linux / Windows ARM | ⚪ No current release package |

> [!NOTE]
> A file extension alone is not treated as authority to edit a document. Editor adapters are qualified separately.

## Privacy & control

- Browser documents and fields require explicit user opt-in.
- Password, sensitive, and uncertain targets are refused.
- Local Instant and cloud Deep are separate paths.
- Cloud Deep is limited to the selected Provider and granted consent.
- Suggestions are never applied automatically.
- Release ZIPs do not bundle model weights or downloaded GPU runtime binaries.

See the [Prototype & qualification reference](PROTOTYPE.md) for detailed boundaries and validation notes.

## Project status

Pencil is an **experimental prototype for personal testing**. The current build is a public unsigned prerelease. Broad editor compatibility, physical IME coverage, normal-user-profile qualification, and live acceptance for every cloud Provider are not complete.

No project-wide open-source license has been assigned yet. Public source availability does not itself grant additional use or redistribution rights, and third-party models and dependencies retain their own terms.

## Documentation

- 📘 [Getting started](GETTING_STARTED.ko.md)
- 🧪 [Prototype / build / qualification](PROTOTYPE.md)
- 🇰🇷 [Korean overview](../README.md)
- 🐞 [Bug report](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

<details>
<summary><strong>For developers</strong></summary>

### Stack

**Tauri 2 · Rust · React · TypeScript · Vite · Windows native integration · Chromium MV3**

### Common checks

~~~powershell
npm ci
npm run typecheck
npm run build:frontend
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
~~~

Read [AGENTS.md](../AGENTS.md) before development work. Current semantic state is tracked in [control/state.json](../control/state.json), with detailed development history in [HANDOFF](control/HANDOFF.md).

</details>

---

<div align="center">

**Pencil — AI suggestions, human decisions.**

</div>
