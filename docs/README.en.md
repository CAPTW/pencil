<div align="center">

# ✏️ Pencil

### Write where you already write.

**A local-first AI writing copilot for Windows — rewrite, polish & translate right where you type.**  
On-device Gemma by default, with optional OpenAI-compatible API and Provider CLI connections.  
AI suggests, you decide.

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
| ⚡ **Local-first** | On-device Gemma is the default engine, so rewrite and translation suggestions arrive fast without a cloud round trip. |
| 🔌 **Bring your own AI** | Need a deeper rewrite? Select a supported Provider CLI or OpenAI-compatible API; account and runtime restrictions apply. |
| ✅ **Review first** | Suggestions are not silently written back. Review, edit, then choose Accept or Copy. |
| 🎯 **Scoped Apply** | Replacement is refused when the target, source text, or range has changed. |
| 🌐 **Browser opt-in** | Browser assistance runs only in explicitly enabled Chromium documents and supported fields. |

## How it works

**Desktop**  
`Select text` → `Ctrl + Shift + G` → `Instant suggestion` → `Review / edit` → `Accept` or `Copy`

**Chromium**  
`Enable document` → `Enable field` → `Review suggestion` → `Accept`

The core rule is simple: **AI suggests; you decide what changes.**

## Highlights

- **One-shortcut writing modes** — grammar fix, natural, concise, polite, and Korean ↔ English translation. No prompt writing needed.
- **Terminology profiles** — register, import and export your terms per profile. Cloud requests carry only the approved terms that actually appear in the selected text.
- **Editable review** before anything is applied.
- **Explicit replacement** of a verified source range on supported targets.
- **Inline browser assist** for opted-in textarea and supported simple contenteditable fields.
- **Privacy boundaries** for sensitive targets, opt-in scope, and cloud consent.

## Bring your own AI

| Route | How to connect | Best for |
| --- | --- | --- |
| 🖥️ **Local Gemma** (default) | Run the one-time prepare script | Fast Instant suggestions, offline processing |
| 🔑 **Account sign-in** | Official sign-in for ChatGPT (Codex) · Claude (Claude Code) · Google (Antigravity) | Deep rewrites using your existing subscription |
| 🧩 **API key** | OpenAI-compatible endpoint · model · key | Any cloud or self-hosted model |

- Each request goes to exactly one Provider you chose; failures never silently retry on another Provider.
- API keys stay in app memory only and are never written to settings. Account sign-in is managed by each official tool.
- Cloud routes run only after explicit consent. Local Gemma works independently of any cloud setup.

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

To use local Gemma, prepare it once, then start Pencil. This downloads a ~7 GB model and an NVIDIA GPU runtime, so check free space and GPU requirements first.

~~~powershell
.\Prepare-GrammarLocal.ps1
.\Start-Grammar.ps1
~~~

No GPU, or cloud Providers only? Start without loading a local model:

~~~powershell
.\Start-Grammar.ps1 -WithoutLocalModel
~~~

See the [getting-started guide](GETTING_STARTED.ko.md) for model, hardware, installation, troubleshooting, and uninstall details.

## Current support

| Target | Status |
| --- | --- |
| Qualified Windows Edit / RichEdit controls | 🟢 Scoped prototype support |
| Chromium textarea / simple contenteditable | 🟢 Explicit opt-in, scoped support |
| Microsoft Word · LibreOffice Writer · HWP/HWPX adapters | 🧪 In development / bounded qualification |
| Arbitrary editors, Office/Hancom in general, every website | ⚪ Not generally qualified |
| macOS / Linux / Windows ARM | ⚪ No current release package |

> [!NOTE]
> A file extension alone is not treated as authority to edit a document. Editor adapters are qualified separately.

## What's next

Pencil is actively evolving. These are directions in progress, not shipped support.

- 🧩 **More editors** — Word, LibreOffice Writer and HWP/HWPX adapters, each qualified one at a time
- ✍️ **Inline suggestions while typing** — underline suggestions in opted-in documents; applying still takes your explicit Accept
- 🔌 **More providers** — broader sign-in and API routes and model choices
- ⏱️ **Faster local suggestions** — targeting suggestions within one second of typing

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

Provider CLI sign-in and model availability depend on the installed version and account. This is not live qualification of every provider. See [current scope](PROTOTYPE.md) for Claude/Antigravity privacy and authentication limitations.
