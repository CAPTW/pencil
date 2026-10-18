<div align="center">

# ✏️ Pencil

### Write where you already write.

**쓰던 그 자리에서 바로 다듬는 Windows용 AI 글쓰기 도우미.**  
로컬 Gemma를 기본으로, 선택적 OpenAI 호환 API와 Provider CLI 연결을 제공합니다.  
제안은 AI가, 결정은 내가.

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
| ⚡ **Local-first** | 기본 엔진은 기기 안에서 도는 Gemma. 윤문·번역 제안을 클라우드 왕복 없이 빠르게 받습니다. |
| 🔌 **Bring your own AI** | 더 깊은 재작성이 필요하면 OpenAI 호환 API 또는 지원하는 Provider CLI를 선택합니다. 계정·실행 환경에 따른 제한이 있습니다. |
| ✅ **Review first** | 제안을 바로 덮어쓰지 않습니다. 검토하고 수정한 뒤 Accept 또는 Copy를 선택합니다. |
| 🎯 **Scoped Apply** | 대상·원문·범위가 바뀌었으면 적용을 거부합니다. |
| 🌐 **Browser opt-in** | 사용자가 켠 Chromium 문서와 지원 입력 필드에서만 동작합니다. |

## How it works

**Desktop**  
`문장 선택` → `Ctrl + Shift + G` → `Instant 제안` → `검토 / 수정` → `Accept` 또는 `Copy`

**Chromium**  
`Enable document` → `Enable field` → `제안 검토` → `Accept`

Pencil의 기본 원칙은 간단합니다. **AI는 제안하고, 변경은 사용자가 결정합니다.**

## What you get

- **One-shortcut writing modes** — 문법 교정 · 자연스럽게 · 간결하게 · 공손하게 · 한↔영 번역. 프롬프트를 쓸 필요가 없습니다.
- **Terminology profiles** — 자주 쓰는 용어를 프로필별로 등록·가져오기·내보내기. 클라우드 요청에는 해당 문장에 실제로 쓰인 승인 용어만 보냅니다.
- **Editable review** — 결과를 그대로 쓰지 않고 검토 창에서 직접 수정.
- **Explicit replacement** — 지원 대상에서는 검증된 원문 범위만 명시적으로 교체.
- **Inline browser assist** — opt-in한 textarea와 지원되는 simple contenteditable에 제안.
- **Privacy boundaries** — password/sensitive target 거부, opt-in 범위, 명시적 cloud consent.

## Bring your own AI

| 경로 | 연결 방식 | 쓰임 |
| --- | --- | --- |
| 🖥️ **Local Gemma** (기본) | 최초 1회 준비 스크립트 실행 | 빠른 Instant 제안, 오프라인 처리 |
| 🔑 **Account sign-in** | ChatGPT(Codex) · Claude(Claude Code) · Google(Antigravity) 공식 로그인 | 계정 구독을 활용한 Deep 재작성 |
| 🧩 **API key** | OpenAI 호환 endpoint · model · key 입력 | 원하는 클라우드/사내 모델 연결 |

- 한 요청은 사용자가 고른 Provider 하나에만 보내며, 실패해도 다른 Provider로 몰래 재시도하지 않습니다.
- API 키는 앱 메모리에만 두고 설정 파일에 저장하지 않습니다. 계정 로그인은 각 공식 도구가 관리합니다.
- 클라우드 경로는 명시적 동의 후에만 사용합니다. 로컬 Gemma는 클라우드 설정과 무관하게 동작합니다.

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

로컬 Gemma를 사용할 경우 최초 한 번 준비한 뒤 시작합니다. 약 7 GB 모델과 NVIDIA GPU 런타임을 받으므로 여유 공간과 GPU 사양을 먼저 확인하세요.

~~~powershell
.\Prepare-GrammarLocal.ps1
.\Start-Grammar.ps1
~~~

GPU가 없거나 클라우드 Provider만 쓰려면 로컬 모델 없이 시작할 수 있습니다.

~~~powershell
.\Start-Grammar.ps1 -WithoutLocalModel
~~~

하드웨어·모델·설치 세부 조건과 제거 방법은 [설치부터 첫 사용까지](docs/GETTING_STARTED.ko.md)를 확인하세요.

## Current support

| Target | Status |
| --- | --- |
| Qualified Windows Edit / RichEdit controls | 🟢 Scoped prototype support |
| Chromium textarea / simple contenteditable | 🟢 Explicit opt-in, scoped support |
| Microsoft Word · LibreOffice Writer · HWP/HWPX adapters | 🧪 In development / bounded qualification |
| Arbitrary editors, Office/Hancom in general, every website | ⚪ Not generally qualified |
| macOS / Linux / Windows ARM | ⚪ No current release package |

> [!NOTE]
> 파일 확장자가 같다는 이유만으로 Pencil이 그 편집기를 안전하게 수정할 수 있다고 가정하지 않습니다. 각 편집기 adapter는 별도로 검증합니다.

## What's next

Pencil은 계속 업데이트됩니다. 아래는 진행 중인 방향이며, 지원 완료를 뜻하지 않습니다.

- 🧩 **더 많은 편집기** — Word, LibreOffice Writer, HWP/HWPX 어댑터를 하나씩 검증해 추가
- ✍️ **입력 중 인라인 제안** — opt-in한 문서에서 타이핑하는 동안 밑줄 제안 표시, 적용은 여전히 사용자가 결정
- 🔌 **Provider 확장** — 계정 로그인·API 연결 경로와 모델 선택지 확대
- ⏱️ **더 빠른 로컬 제안** — 입력 후 1초 이내 제안을 목표로 성능 개선

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

개발 우선순위와 완료조건은 [지속 품질 개선 백로그](docs/control/IMPROVEMENT_BACKLOG.md)에서 관리합니다. 기존 기능 개발과 함께 CI·안정성·사용성·개인정보·배포 위험을 주요 단계마다 재평가합니다.

<div align="center">

**Pencil — AI suggestions, human decisions.**

</div>

Provider CLI의 계정 로그인·모델 이용 가능 여부는 설치 버전과 계정에 따라 달라집니다. 실제 Provider 전체를 검증했다는 의미가 아니며, Claude·Antigravity의 개인정보·인증 제한은 [현재 제품 범위](docs/PROTOTYPE.md)를 확인하세요.
