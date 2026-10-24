<div align="center">

# ✏️ Pencil

### Write where you already write.

**A local-first AI writing copilot for Windows — rewrite, polish & translate right where you type.**  
On-device Gemma by default, with optional cloud APIs (Claude · Gemini · OpenRouter · xAI · OpenAI-compatible) and Provider CLI connections.  
AI suggests, you decide.

<p>
  <img alt="Release" src="https://img.shields.io/github/v/release/CAPTW/pencil?include_prereleases&sort=semver&style=flat-square&label=release">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20x64-0078D4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Status" src="https://img.shields.io/badge/status-prototype-F59E0B?style=flat-square">
  <img alt="Local first" src="https://img.shields.io/badge/AI-local--first-4F46E5?style=flat-square">
</p>

[**Download Prototype 3 (r4)**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.3) · [한국어](../README.md) · [Getting started (Korean)](GETTING_STARTED.ko.md) · [Report a bug](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

</div>

> [!IMPORTANT]
> Pencil is currently an **unsigned Windows x64 prerelease**. It does not claim universal editor support. Start with short, non-sensitive test text before using it with important documents.

## Why Pencil?

Pencil is designed to start from the text you are already writing instead of making you move your work into a separate AI chat.

| | |
| --- | --- |
| ⚡ **Local-first** | On-device Gemma is the default engine, so rewrite and translation suggestions arrive without a cloud round trip. |
| 🔌 **Bring your own AI** | Need a deeper rewrite? Choose a cloud API or a supported Provider CLI; account and runtime restrictions apply. |
| ✅ **Review first** | Suggestions are not silently written back. Review, edit, then choose Accept or Copy. |
| 🎯 **Scoped Apply** | Replacement is refused when the target, source text or range has changed, or the review authority has expired. |
| 🌐 **Browser opt-in** | Browser assistance runs only in explicitly enabled Chromium documents and supported fields. |

## How it works

**Desktop**  
`Select text` → `Ctrl + Shift + G` (default, configurable) → `Instant suggestion` → `Review / edit` → `Accept` or `Copy`

**Chromium**  
`Enable this document` → `Enable this field` (or `Alt + Shift + E` inside the field) → `Review suggestion` → `Accept`

Accept authority lasts **at most 30 seconds from capture**, including model and review time. If it has expired or the source/selection changed, select and capture again, or use Copy.

The core rule is simple: **AI suggests; you decide what changes.**

## Highlights

- **One-shortcut writing modes** — Grammar, Natural, Concise, Polite and Translate. No prompt writing needed.
- **Translation** — local Gemma covers Korean ↔ English; cloud Deep can also target Japanese and Chinese (Simplified/Traditional).
- **Terminology profiles** — register terms per profile and import/export them as JSON/CSV. Cloud requests carry only the approved terms that actually appear in the selected text.
- **Editable review** before anything is applied.
- **Explicit replacement** of a verified source range on supported targets.
- **Inline browser assist** for opted-in textarea and supported simple contenteditable fields, with `Alt + Shift + R` selected-range review.
- **Privacy boundaries** for sensitive targets, opt-in scope and cloud consent.

## Bring your own AI

| Route | How to connect | Best for |
| --- | --- | --- |
| 🖥️ **Local Gemma** (default) | Run the one-time prepare script | Fast Instant suggestions, offline after preparation |
| 🔑 **Provider CLI** | Choose Codex · Claude · Antigravity, subject to authentication/privacy restrictions | Explicit Deep within the selected tool’s supported scope |
| 🧩 **API key** | Claude (Anthropic Messages) · Google Gemini · OpenRouter · xAI presets, or a custom HTTPS Chat Completions endpoint | Your choice of cloud or self-hosted model |

- Each request goes to exactly one Provider you chose; failures never silently retry on another Provider.
- Set up an API in **Settings → Optional API setup**: choose a Provider → enter the key → pick a model from the list → grant consent → **Save API setup securely**. Requests are sent only through **Run API Deep**; CLI Auto rewrite never calls the API.
- The API key, model and consent are stored in **Windows Credential Manager** per Windows user and Pencil profile — not in ordinary settings files — and are restored after restart. **Revoke API consent** withdraws consent only; **Remove API key and permission** also deletes the stored key.
- Custom endpoints must use HTTPS. Model-list lookup is available for the official presets only and sends the key, never document text.
- A loaded model list does not prove a model can be called. Live generation has been checked only with Claude `claude-haiku-5-5` on short synthetic text; Google, OpenRouter and xAI live generation are unverified.
- Provider CLI sign-in and model availability depend on the installed version and account. Antigravity has a privacy-qualification limit and the Claude CLI has a bare-mode OAuth limit.
- Cloud routes run only after explicit consent. Local Gemma works independently of any cloud setup.

## Quick start

### 1. Download

From the [**Prototype 3 release**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.3) assets, download `Pencil-0.1.0-prototype.3-r4-windows-x64.zip` and `PROTOTYPE_3_R4_SHA256SUMS.txt`, then verify the SHA-256. The original and r1–r3 ZIPs in the same release are kept for traceability. GitHub’s auto-generated **Source code (zip)** is not the executable package.

Requirements: **Windows x64 · PowerShell 7 · Microsoft WebView2**.

### 2. Install

Run from the extracted folder in PowerShell 7:

~~~powershell
.\Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA\PencilPrototype3"
Set-Location "$env:LOCALAPPDATA\PencilPrototype3"
~~~

### 3. Choose how to run

To use local Gemma, prepare it once, then start Pencil. This downloads a ~7 GB model and an NVIDIA GPU runtime (~0.6 GB), so check free space and GPU requirements first.

~~~powershell
.\Prepare-GrammarLocal.ps1
.\Start-Grammar.ps1
~~~

No GPU, or cloud Providers only? Start without loading a local model:

~~~powershell
.\Start-Grammar.ps1 -WithoutLocalModel
~~~

See the [getting-started guide (Korean)](GETTING_STARTED.ko.md) for model, hardware, installation, troubleshooting and uninstall details, and the [r4 install guide](releases/PROTOTYPE_3_R4_INSTALL.md) for API setup and the 30-second Accept / Copy-only rules.

## Current support

| Target | Status |
| --- | --- |
| Qualified Windows Edit / RichEdit controls | 🟢 Scoped prototype support (single-line Edit without auto horizontal scroll is Copy-only) |
| Chromium textarea / simple contenteditable | 🟢 Explicit opt-in, scoped support |
| Microsoft Word · LibreOffice Writer · HWP/HWPX (rHWP) adapters | 🧪 In development / bounded qualification, not in the default ZIP |
| Arbitrary editors, Office/Hancom in general, every website | ⚪ Not generally qualified |
| macOS / Linux / Windows ARM | ⚪ No current release package |

> [!NOTE]
> A file extension alone is not treated as authority to edit a document. Editor adapters are qualified separately.

## What's next

Pencil is actively evolving. These are directions in progress, not shipped support.

- 🧩 **More editors** — Word, LibreOffice Writer and HWP/HWPX adapters, each qualified one at a time
- ✍️ **Wider inline suggestions while typing** — suggestions while typing outside the browser too; applying still takes your explicit Accept
- 🔌 **More providers** — broader sign-in and API routes and model choices, plus live checks for unverified Providers
- ⏱️ **Faster local suggestions** — targeting suggestions within one second of typing

## Privacy & control

- Browser documents and fields require explicit user opt-in.
- Password, sensitive and uncertain targets are not read.
- Local Instant and cloud Deep are separate paths.
- Cloud Deep is limited to the selected Provider and granted consent.
- Suggestions are never applied automatically.
- API keys are stored in Windows Credential Manager and never written in plain text to ordinary settings files. Uninstall does not delete a stored key, so run **Remove API key and permission** in the app first.
- Release ZIPs do not bundle model weights or downloaded GPU runtime binaries.

See the [Prototype & qualification reference](PROTOTYPE.md) for detailed boundaries and validation notes.

## Project status

Pencil is an **experimental prototype for personal testing**. The current build is a public unsigned prerelease without auto-update. Broad editor compatibility, physical IME coverage, normal-user-profile qualification and live acceptance for every cloud Provider are not complete. The 1 s recommendation, 0.5 s Accept and 3 s paragraph figures are targets, not guarantees.

No project-wide open-source license has been assigned yet. Public source availability does not itself grant additional use or redistribution rights, and third-party models and dependencies retain their own terms.

## Documentation

- 📘 [Getting started (Korean)](GETTING_STARTED.ko.md)
- 🔑 [Prototype 3 r4 install & API guide](releases/PROTOTYPE_3_R4_INSTALL.md) · [Release notes](releases/v0.1.0-prototype.3.md) · [Changelog](../CHANGELOG.md)
- 🧪 [Prototype / build / qualification](PROTOTYPE.md)
- 🇰🇷 [Korean overview](../README.md)
- 🐞 [Bug report](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

<details>
<summary><strong>For developers</strong></summary>

### Stack

**Tauri 2 · Rust · React · TypeScript · Vite · Win32 C++ (TSF / RichEdit) · Chromium MV3 · llama.cpp**

### Repository layout

| Path | Contents |
| --- | --- |
| `src/` | React + TypeScript review window and settings UI |
| `src-tauri/src/` | Rust core: capture and Apply safety, Providers (`provider/`: Codex·Claude·Antigravity CLI, HTTP API), local Gemma, terminology, Instant analysis engine |
| `src-tauri/src/bin/` | `grammar-chromium-host` (Chromium Native Messaging host), `grammar-gemma-broker` (local model process owner) |
| `src-tauri/native/` | Win32 C++: Edit/RichEdit/TSF capture and replacement (`target-apply.dll`), Word·Writer·browser bridges |
| `src-tauri/protocol/` | Pinned Codex CLI app-server JSON schemas |
| `adapters/chromium/` | Chromium MV3 extension and native host tools |
| `adapters/libreoffice/` · `adapters/rhwp/` | LibreOffice Writer · rHWP (HWP/HWPX) adapters — deployed separately |
| `scripts/package/` | Install, launch, Gemma preparation and uninstall scripts shipped in the release ZIP |
| `scripts/mission/` | Build (`build-receipt.ps1`), packaging (`package.ps1`) and qualification harnesses |
| `scripts/local/` | Isolated Windows Sandbox trial tooling |
| `benchmarks/instant-selection/` | Synthetic benchmark corpus and baseline |
| `control/` · `docs/control/` | Development governance contract, current state and detailed history |

`scripts/daily-use/`, `scripts/build-personal-bundle.ps1`, [PERSONAL_BUNDLE.md](PERSONAL_BUNDLE.md) and `docs/P*_*.md` are earlier-phase records. Current packages are built with `scripts/mission/package.ps1`.

### Common checks

These mirror the main steps of the GitHub [Prototype source checks](../.github/workflows/prototype-source.yml). The Rust/C++ build requires Windows (MSVC).

~~~powershell
python scripts/validate_governance.py --root . --self-test
npm ci --ignore-scripts
npm run build:frontend          # tsc + vite build
npm run test:provider
node scripts/test-capture-contract.mjs
node adapters/chromium/tests/core.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --test-threads=1
~~~

To build a package from source, see [Build and install from source](PROTOTYPE.md#build-and-install-from-source).

Read [AGENTS.md](../AGENTS.md) before development work. Current semantic state is tracked in [control/state.json](../control/state.json), detailed history in [HANDOFF](control/HANDOFF.md), and priorities with completion criteria in the [improvement backlog](control/IMPROVEMENT_BACKLOG.md).

</details>

---

<div align="center">

**Pencil — AI suggestions, human decisions.**

</div>
