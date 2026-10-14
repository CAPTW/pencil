<div align="center">

# ✏️ Pencil

### Write where you already write.

**Windows용 review-first AI writing assistant.**  
문장을 선택하고, 제안을 검토하고, 원하는 경우에만 적용합니다.

<p>
  <img alt="Release" src="https://img.shields.io/github/v/release/CAPTW/pencil?include_prereleases&sort=semver&style=flat-square&label=release">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20x64-0078D4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Status" src="https://img.shields.io/badge/status-prototype-F59E0B?style=flat-square">
  <img alt="Local first" src="https://img.shields.io/badge/AI-local--first-4F46E5?style=flat-square">
</p>

[**Download Prototype 2**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2) · [빠른 시작](docs/GETTING_STARTED.ko.md) · [English](docs/README.en.md) · [Report a bug](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

</div>

> [!IMPORTANT]
> Pencil은 현재 **Windows x64용 unsigned prerelease**입니다. 검증된 범위 밖의 편집기까지 일반 지원한다고 주장하지 않습니다. 중요한 문서보다 짧은 비민감 시험 문장으로 먼저 확인하세요.

## Why Pencil?

Pencil은 별도의 AI 채팅창에서 글을 다시 복사하는 대신, **지금 작성 중인 문장**에서 시작하도록 설계했습니다.

| | |
| --- | --- |
| ⚡ **Local-first** | 로컬 추론을 사용하면 번역과 재작성을 기기 안에서 처리할 수 있습니다. |
| ✅ **Review first** | 제안을 바로 덮어쓰지 않습니다. 검토하고 수정한 뒤 Accept 또는 Copy를 선택합니다. |
| 🎯 **Scoped Apply** | 대상·원문·범위가 바뀌었으면 적용을 거부할 수 있습니다. |
| 🌐 **Browser opt-in** | 사용자가 켠 Chromium 문서와 지원 입력 필드에서만 동작합니다. |
| ☁️ **Deep is optional** | 더 깊은 재작성은 선택한 Provider/API와 명시적 동의가 있을 때만 사용합니다. |

## How it works

**Desktop**  
`문장 선택` → `Ctrl + Shift + G` → `Instant 제안` → `검토 / 수정` → `Accept` 또는 `Copy`

**Chromium**  
`Enable document` → `Enable field` → `제안 검토` → `Accept`

Pencil의 기본 원칙은 간단합니다. **AI는 제안하고, 변경은 사용자가 결정합니다.**

## What you get

- **Rewrite & translate** — 자연스럽게, 간결하게, 공손하게 등의 고정 writing intent와 번역.
- **Editable review** — 결과를 그대로 쓰지 않고 검토 창에서 직접 수정.
- **Explicit replacement** — 지원 대상에서는 검증된 원문 범위만 명시적으로 교체.
- **Inline browser assist** — opt-in한 textarea와 지원되는 simple contenteditable에 제안.
- **Optional Deep** — 선택한 CLI Provider 또는 OpenAI-compatible API 경로.
- **Privacy boundaries** — password/sensitive target 거부, opt-in 범위, 명시적 cloud consent.

## Quick start

### 1. Download

[**Prototype 2 release**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2)의 **Assets**에서 Windows x64 ZIP과 `SHA256SUMS.txt`를 받으세요. GitHub의 자동 생성 **Source code (zip)**은 실행 패키지가 아닙니다.

필수 환경은 **Windows x64 · PowerShell 7 · Microsoft WebView2**입니다.

### 2. Install

~~~powershell
.\Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA\PencilPrototype1"
Set-Location "$env:LOCALAPPDATA\PencilPrototype1"
~~~

### 3. Choose how to run

로컬 AI를 사용할 경우 최초 한 번 준비한 뒤 시작합니다.

~~~powershell
.\Prepare-GrammarLocal.ps1
.\Start-Grammar.ps1
~~~

로컬 모델 없이 시작할 수도 있습니다.

~~~powershell
.\Start-Grammar.ps1 -WithoutLocalModel
~~~

하드웨어·모델·설치 세부 조건과 제거 방법은 [설치부터 첫 사용까지](docs/GETTING_STARTED.ko.md)를 확인하세요.

## Current support

| Target | Status |
| --- | --- |
| Qualified Windows Edit / RichEdit controls | 🟢 Scoped prototype support |
| Chromium textarea / simple contenteditable | 🟢 Explicit opt-in, scoped support |
| Additional desktop application adapters | 🧪 In development / bounded qualification |
| Arbitrary editors, Office/Hancom in general, every website | ⚪ Not generally qualified |
| macOS / Linux / Windows ARM | ⚪ No current release package |

> [!NOTE]
> 파일 확장자가 같다는 이유만으로 Pencil이 그 편집기를 안전하게 수정할 수 있다고 가정하지 않습니다. 각 편집기 adapter는 별도로 검증합니다.

## Privacy & control

- 브라우저는 **문서와 필드를 사용자가 직접 활성화**해야 합니다.
- password와 민감하거나 불명확한 대상은 처리하지 않습니다.
- Local Instant와 cloud Deep은 별도 경로입니다.
- cloud Deep은 선택한 Provider와 동의 범위 안에서만 실행합니다.
- 제안은 자동으로 문서에 적용되지 않습니다.
- release ZIP에는 모델 weight와 다운로드형 GPU runtime을 포함하지 않습니다.

자세한 보안·검증 범위는 [Prototype & qualification reference](docs/PROTOTYPE.md)를 확인하세요.

## Project status

Pencil은 **개인 시험 사용을 위한 experimental prototype**입니다. 현재 릴리스는 공개 prerelease이며 코드 서명이 없습니다. 폭넓은 편집기 호환성, 물리 IME, 일반 사용자 프로필, 모든 cloud Provider의 live acceptance는 아직 완료된 것으로 보지 않습니다.

프로젝트 전체에 적용되는 오픈소스 라이선스도 아직 지정되지 않았습니다. 공개 저장소라는 사실만으로 별도 사용·재배포 권한을 의미하지 않으며, 모델과 의존성에는 각 upstream 조건이 적용됩니다.

## Documentation

- 📘 [설치부터 첫 사용까지](docs/GETTING_STARTED.ko.md)
- 🧪 [Prototype / build / qualification](docs/PROTOTYPE.md)
- 🌍 [English overview](docs/README.en.md)
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

개발 작업 전 [AGENTS.md](AGENTS.md)를 확인하세요. 현재 의미 상태는 [control/state.json](control/state.json), 상세 개발 이력은 [HANDOFF](docs/control/HANDOFF.md)에 기록합니다.

</details>

---

<div align="center">

**Pencil — AI suggestions, human decisions.**

</div>
