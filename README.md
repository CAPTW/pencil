<div align="center">

# ✏️ Pencil

### Write where you already write.

**쓰던 그 자리에서 바로 다듬는 Windows용 AI 글쓰기 도우미.**  
로컬 Gemma가 기본이고, 필요하면 클라우드 API(Claude · Gemini · OpenRouter · xAI · OpenAI 호환)나 Provider CLI를 연결할 수 있습니다.  
제안은 AI가, 결정은 내가.

<p>
  <img alt="Release" src="https://img.shields.io/github/v/release/CAPTW/pencil?include_prereleases&sort=semver&style=flat-square&label=release">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20x64-0078D4?style=flat-square&logo=windows11&logoColor=white">
  <img alt="Status" src="https://img.shields.io/badge/status-prototype-F59E0B?style=flat-square">
  <img alt="Local first" src="https://img.shields.io/badge/AI-local--first-4F46E5?style=flat-square">
</p>

[**Download Prototype 3 (r4)**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.3) · [빠른 시작](docs/GETTING_STARTED.ko.md) · [English](docs/README.en.md) · [Report a bug](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

</div>

> [!IMPORTANT]
> Pencil은 현재 **Windows x64용 unsigned prerelease**입니다. 검증된 범위 밖의 편집기까지 일반 지원한다고 주장하지 않습니다. 중요한 문서보다 짧은 비민감 시험 문장으로 먼저 확인하세요.

## Why Pencil?

Pencil은 별도의 AI 채팅창에서 글을 다시 복사하는 대신, **지금 작성 중인 문장**에서 시작하도록 설계했습니다.

| | |
| --- | --- |
| ⚡ **Local-first** | 기본 엔진은 기기 안에서 도는 Gemma입니다. 윤문·번역 제안을 클라우드 왕복 없이 받습니다. |
| 🔌 **Bring your own AI** | 더 깊은 재작성이 필요하면 클라우드 API나 지원하는 Provider CLI를 직접 고릅니다. 계정·실행 환경에 따라 제한이 있습니다. |
| ✅ **Review first** | 제안을 바로 덮어쓰지 않습니다. 검토하고 수정한 뒤 Accept 또는 Copy를 선택합니다. |
| 🎯 **Scoped Apply** | 대상·원문·범위가 바뀌었거나 검토 권한이 만료되면 적용을 거부합니다. |
| 🌐 **Browser opt-in** | 사용자가 켠 Chromium 문서와 지원 입력 필드에서만 동작합니다. |

## How it works

**Desktop**  
`문장 선택` → `Ctrl + Shift + G` (기본값, 설정에서 변경 가능) → `Instant 제안` → `검토 / 수정` → `Accept` 또는 `Copy`

**Chromium**  
`Enable this document` → `Enable this field` (또는 필드 안에서 `Alt + Shift + E`) → `제안 검토` → `Accept`

Accept 권한은 **캡처 후 최대 30초**입니다(모델 응답과 검토 시간 포함). 시간이 지났거나 원문·선택이 바뀌었으면 다시 선택해 캡처하거나 Copy를 쓰세요.

Pencil의 기본 원칙은 간단합니다. **AI는 제안하고, 변경은 사용자가 결정합니다.**

## What you get

- **One-shortcut writing modes** — Grammar(문법 교정) · Natural(자연스럽게) · Concise(간결하게) · Polite(공손하게) · Translate(번역). 프롬프트를 쓸 필요가 없습니다.
- **Translation** — 로컬 Gemma는 한국어 ↔ 영어를 지원합니다. 클라우드 Deep에서는 일본어와 중국어(간체·번체) 대상도 고를 수 있습니다.
- **Terminology profiles** — 자주 쓰는 용어를 프로필별로 등록하고 JSON/CSV로 가져오거나 내보냅니다. 클라우드 요청에는 해당 문장에 실제로 쓰인 승인 용어만 보냅니다.
- **Editable review** — 결과를 그대로 쓰지 않고 검토 창에서 직접 수정합니다.
- **Explicit replacement** — 지원 대상에서는 검증된 원문 범위만 명시적으로 교체합니다.
- **Inline browser assist** — opt-in한 textarea와 지원되는 simple contenteditable에 제안을 표시하고, `Alt + Shift + R`로 선택 범위를 검토합니다.
- **Privacy boundaries** — password/sensitive target 거부, opt-in 범위, 명시적 cloud consent.

## Bring your own AI

| 경로 | 연결 방식 | 쓰임 |
| --- | --- | --- |
| 🖥️ **Local Gemma** (기본) | 최초 1회 준비 스크립트 실행 | 빠른 Instant 제안, 준비 후 오프라인 처리 |
| 🔑 **Provider CLI** | Codex · Claude · Antigravity 중 선택. 인증·개인정보 제한 확인 필요 | 선택한 도구가 지원하는 범위의 명시적 Deep |
| 🧩 **API key** | Claude(Anthropic Messages) · Google Gemini · OpenRouter · xAI 프리셋, 또는 HTTPS Chat Completions endpoint 직접 입력 | 원하는 클라우드/사내 모델 연결 |

- 한 요청은 사용자가 고른 Provider 하나에만 보내며, 실패해도 다른 Provider로 몰래 재시도하지 않습니다.
- API는 **Settings → Optional API setup**에서 Provider 선택 → 키 입력 → 모델 목록 선택 → 전송 동의 → **Save API setup securely** 순서로 설정하고, **Run API Deep**으로만 요청합니다. CLI의 Auto rewrite는 API를 자동 호출하지 않습니다.
- API 키·모델·동의는 일반 설정 파일이 아닌 **Windows Credential Manager**에 Windows 사용자·Pencil 프로필별로 저장되어 재시작 후 복원됩니다. **Revoke API consent**는 동의만 철회하고, **Remove API key and permission**은 저장된 키까지 삭제합니다.
- 직접 입력하는 endpoint는 HTTPS만 허용합니다. 모델 목록 조회는 공식 프리셋에서만 지원하며 키만 보내고 문서는 보내지 않습니다.
- 목록 조회 성공은 실제 호출 가능을 뜻하지 않습니다. 실제 생성은 Claude `claude-haiku-5-5`의 짧은 합성 문장으로만 확인했고, Google·OpenRouter·xAI 실제 생성은 미검증입니다.
- Provider CLI의 계정 로그인·모델 이용 가능 여부는 설치 버전과 계정에 따라 다릅니다. Antigravity는 개인정보 qualification 제한, Claude CLI는 bare 모드 OAuth 제한이 있습니다.
- 클라우드 경로는 명시적 동의 후에만 사용합니다. 로컬 Gemma는 클라우드 설정과 무관하게 동작합니다.

## Quick start

### 1. Download

[**Prototype 3 release**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.3)의 **Assets**에서 `Pencil-0.1.0-prototype.3-r4-windows-x64.zip`과 `PROTOTYPE_3_R4_SHA256SUMS.txt`를 받아 SHA-256을 확인하세요. 같은 릴리스의 이전 원본·r1–r3 ZIP은 기록 보존용입니다. GitHub의 자동 생성 **Source code (zip)**은 실행 패키지가 아닙니다.

필수 환경은 **Windows x64 · PowerShell 7 · Microsoft WebView2**입니다.

### 2. Install

압축을 푼 폴더에서 PowerShell 7로 실행합니다.

~~~powershell
.\Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA\PencilPrototype3"
Set-Location "$env:LOCALAPPDATA\PencilPrototype3"
~~~

### 3. Choose how to run

로컬 Gemma를 사용할 경우 최초 한 번 준비한 뒤 시작합니다. 약 7 GB 모델과 NVIDIA GPU 런타임(약 0.6 GB)을 받으므로 여유 공간과 GPU 사양을 먼저 확인하세요.

~~~powershell
.\Prepare-GrammarLocal.ps1
.\Start-Grammar.ps1
~~~

GPU가 없거나 클라우드 Provider만 쓰려면 로컬 모델 없이 시작할 수 있습니다.

~~~powershell
.\Start-Grammar.ps1 -WithoutLocalModel
~~~

하드웨어·모델·설치 세부 조건과 제거 방법은 [설치부터 첫 사용까지](docs/GETTING_STARTED.ko.md), r4 API 설정과 Accept 30초·Copy-only 안내는 [r4 설치 안내](docs/releases/PROTOTYPE_3_R4_INSTALL.md)를 확인하세요.

## Current support

| Target | Status |
| --- | --- |
| Qualified Windows Edit / RichEdit controls | 🟢 Scoped prototype support (자동 가로 스크롤이 없는 한 줄 Edit는 Copy-only) |
| Chromium textarea / simple contenteditable | 🟢 Explicit opt-in, scoped support |
| Microsoft Word · LibreOffice Writer · HWP/HWPX(rHWP) adapters | 🧪 In development / bounded qualification, 기본 ZIP 미포함 |
| Arbitrary editors, Office/Hancom in general, every website | ⚪ Not generally qualified |
| macOS / Linux / Windows ARM | ⚪ No current release package |

> [!NOTE]
> 파일 확장자가 같다는 이유만으로 Pencil이 그 편집기를 안전하게 수정할 수 있다고 가정하지 않습니다. 각 편집기 adapter는 별도로 검증합니다.

## What's next

Pencil은 계속 업데이트됩니다. 아래는 진행 중인 방향이며, 지원 완료를 뜻하지 않습니다.

- 🧩 **더 많은 편집기** — Word, LibreOffice Writer, HWP/HWPX 어댑터를 하나씩 검증해 추가
- ✍️ **입력 중 인라인 제안 확대** — 브라우저 밖 편집기에서도 타이핑하는 동안 제안 표시, 적용은 여전히 사용자가 결정
- 🔌 **Provider 확장** — 계정 로그인·API 연결 경로와 모델 선택지 확대, 미검증 Provider의 실제 호출 검증
- ⏱️ **더 빠른 로컬 제안** — 입력 후 1초 이내 제안을 목표로 성능 개선

## Privacy & control

- 브라우저는 **문서와 필드를 사용자가 직접 활성화**해야 합니다.
- password와 민감하거나 불명확한 대상은 읽지 않습니다.
- Local Instant와 cloud Deep은 별도 경로입니다.
- cloud Deep은 선택한 Provider와 동의 범위 안에서만 실행합니다.
- 제안은 자동으로 문서에 적용되지 않습니다.
- API 키는 Windows Credential Manager에 저장하며 일반 설정 파일에 평문으로 쓰지 않습니다. 제거(uninstall)는 저장된 키를 지우지 않으므로 먼저 앱에서 **Remove API key and permission**을 실행하세요.
- release ZIP에는 모델 weight와 다운로드형 GPU runtime을 포함하지 않습니다.

자세한 보안·검증 범위는 [Prototype & qualification reference](docs/PROTOTYPE.md)를 확인하세요.

## Project status

Pencil은 **개인 시험 사용을 위한 experimental prototype**입니다. 현재 릴리스는 공개 prerelease이며 코드 서명과 자동 업데이트가 없습니다. 폭넓은 편집기 호환성, 물리 IME, 일반 사용자 프로필, 모든 cloud Provider의 live acceptance는 아직 완료된 것으로 보지 않습니다. 추천 1초·Accept 0.5초·단락 3초는 목표이며 보장값이 아닙니다.

프로젝트 전체에 적용되는 오픈소스 라이선스는 아직 지정되지 않았습니다. 공개 저장소라는 사실만으로 별도 사용·재배포 권한을 의미하지 않으며, 모델과 의존성에는 각 upstream 조건이 적용됩니다.

## Documentation

- 📘 [설치부터 첫 사용까지](docs/GETTING_STARTED.ko.md)
- 🔑 [Prototype 3 r4 설치·API 안내](docs/releases/PROTOTYPE_3_R4_INSTALL.md) · [릴리스 노트](docs/releases/v0.1.0-prototype.3.md) · [Changelog](CHANGELOG.md)
- 🧪 [Prototype / build / qualification](docs/PROTOTYPE.md)
- 🌍 [English overview](docs/README.en.md)
- 🐞 [Bug report](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

<details>
<summary><strong>For developers</strong></summary>

### Stack

**Tauri 2 · Rust · React · TypeScript · Vite · Win32 C++ (TSF / RichEdit) · Chromium MV3 · llama.cpp**

### Repository layout

| 경로 | 내용 |
| --- | --- |
| `src/` | React + TypeScript 검토 창·설정 UI |
| `src-tauri/src/` | Rust 앱 코어: 캡처·Apply 안전성, Provider(`provider/`: Codex·Claude·Antigravity CLI, HTTP API), 로컬 Gemma, 용어사전, Instant 분석 엔진 |
| `src-tauri/src/bin/` | `grammar-chromium-host`(Chromium Native Messaging host), `grammar-gemma-broker`(로컬 모델 프로세스 관리) |
| `src-tauri/native/` | Win32 C++: Edit/RichEdit/TSF 캡처·교체(`target-apply.dll`), Word·Writer·브라우저 브리지 |
| `src-tauri/protocol/` | 고정 버전 Codex CLI app-server JSON 스키마 |
| `adapters/chromium/` | Chromium MV3 확장과 native host 도구 |
| `adapters/libreoffice/` · `adapters/rhwp/` | LibreOffice Writer · rHWP(HWP/HWPX) adapter — 별도 배포 |
| `scripts/package/` | 릴리스 ZIP에 들어가는 설치·실행·Gemma 준비·제거 스크립트 |
| `scripts/mission/` | 빌드(`build-receipt.ps1`)·패키징(`package.ps1`)·검증 harness |
| `scripts/local/` | 격리 Windows Sandbox 시험 도구 |
| `benchmarks/instant-selection/` | 합성 벤치마크 corpus와 기준 |
| `control/` · `docs/control/` | 개발 거버넌스 계약·현재 상태·상세 이력 |

`scripts/daily-use/`, `scripts/build-personal-bundle.ps1`, [docs/PERSONAL_BUNDLE.md](docs/PERSONAL_BUNDLE.md)와 `docs/P*_*.md`는 이전 단계 기록입니다. 현재 패키지는 `scripts/mission/package.ps1`로 만듭니다.

### Common checks

GitHub의 [Prototype source checks](.github/workflows/prototype-source.yml)와 같은 주요 검사입니다. Rust/C++ 빌드는 Windows(MSVC)가 필요합니다.

~~~powershell
python scripts/validate_governance.py --root . --self-test
npm ci --ignore-scripts
npm run build:frontend          # tsc + vite build
npm run test:provider
node scripts/test-capture-contract.mjs
node adapters/chromium/tests/core.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --test-threads=1
~~~

소스에서 패키지를 빌드하는 방법은 [Build and install from source](docs/PROTOTYPE.md#build-and-install-from-source)를 참고하세요.

개발 작업 전 [AGENTS.md](AGENTS.md)를 확인하세요. 현재 의미 상태는 [control/state.json](control/state.json), 상세 개발 이력은 [HANDOFF](docs/control/HANDOFF.md), 개발 우선순위와 완료조건은 [지속 품질 개선 백로그](docs/control/IMPROVEMENT_BACKLOG.md)에서 관리합니다.

</details>

---

<div align="center">

**Pencil — AI suggestions, human decisions.**

</div>
