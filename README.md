# Pencil

**Windows에서 선택한 문장을 다듬고, 검토한 뒤 직접 적용하는 글쓰기 도우미.**

Local writing assistance · Translation · Explicit Accept

[**Prototype 2 다운로드**](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2) · [설치부터 첫 사용까지](docs/GETTING_STARTED.ko.md) · [English](docs/README.en.md) · [문제 신고](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

> **현재 버전: 0.1.0 Prototype 2 · Windows x64 · 서명되지 않은 사전 릴리스**
>
> 개인 시험 사용 단계입니다. 지원 범위를 먼저 확인하고 짧은 비민감 문장으로 시작하세요.

## Pencil로 무엇을 할 수 있나요?

새 프롬프트를 매번 작성하는 대신, 쓰던 문장에서 바로 시작합니다.

**문장 선택 → `Ctrl+Shift+G` → 제안 검토·수정 → `Accept` 또는 `Copy`**

- **로컬 글쓰기 지원:** Gemma를 이용한 번역과 자연스럽게·간결하게·공손하게 재작성.
- **브라우저 inline 제안:** 직접 활성화한 문서와 입력 필드에서 제안을 보고 선택해 적용.
- **검토 후 적용:** 자동으로 문서를 고치지 않습니다. 대상이나 원문이 바뀌면 Accept를 거부할 수 있습니다.
- **선택적 클라우드 Deep:** 원하는 Provider 또는 OpenAI-compatible API를 직접 설정하고 동의한 경우에 사용.

Gemma의 기본 모델은 **Gemma4 12B IT QAT Q4_0**입니다. 속도를 위해 작은 모델로 자동 전환하지 않습니다.

## 내 PC에서는 어떻게 시작하나요?

| 환경 | 시작 방법 | 기대할 수 있는 범위 |
| --- | --- | --- |
| RTX 5070 Ti에서 검증한 CUDA 13.4 구성 | Gemma 모델 준비 후 일반 실행 | 로컬 번역·재작성; 아래 실측 참고 |
| 다른 NVIDIA GPU | 호환성을 별도로 확인한 뒤 시험 | 같은 속도·안정성 보장 없음 |
| GPU가 부족하거나 로컬 모델을 쓰지 않을 때 | `-WithoutLocalModel`로 실행 | 기본 로컬 규칙 + 별도 설정한 Deep. Gemma 기능과 동일하지 않음 |
| macOS, Linux, Windows ARM | 이번 실행 패키지 대상 아님 | Windows x64 패키지만 제공 |

**필수:** Windows x64, PowerShell 7, Microsoft WebView2. Pencil 설치에는 개발 도구가 필요하지 않습니다. Gemma 모델은 약 **7 GB**이며 GPU 런타임과 준비 공간이 추가로 필요합니다. ZIP 자체에는 모델이 없습니다.

## 설치와 첫 실행

### 1. 릴리스 파일 받기

[릴리스 페이지](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2)의 **Assets**에서 다음을 받으세요.

| 파일 | 용도 |
| --- | --- |
| `Pencil-0.1.0-prototype.2-windows-x64.zip` | 실행 프로그램과 설치 스크립트, 약 9.4 MB |
| `SHA256SUMS.txt` | 다운로드 파일 무결성 확인 |
| `VALIDATION.json` | 이번 패키지의 검증 범위와 남은 제한 |

GitHub가 제공하는 **Source code (zip)**은 실행 패키지가 아닙니다.

### 2. 확인하고 설치하기

[상세 설치 안내](docs/GETTING_STARTED.ko.md)에 따라 PowerShell 7·WebView2를 준비하고 ZIP 해시를 확인하세요. ZIP을 새 폴더에 푼 다음, **PowerShell 7**에서 그 폴더로 이동해 실행합니다.

```powershell
.\Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA\PencilPrototype1"
Set-Location "$env:LOCALAPPDATA\PencilPrototype1"
```

### 3. 두 시작 방법 중 하나 선택하기

**Gemma를 쓸 때 — 최초 한 번 모델을 준비한 뒤 실행:**

```powershell
.\Prepare-GrammarLocal.ps1
.\Start-Grammar.ps1
```

다음부터는 설치 폴더에서 `Start-Grammar.ps1`만 실행하세요. 처음 다운로드·준비하는 시간과 앱 시작 시 모델 예열 시간은 문장 응답 시간과 별개입니다.

**모델 없이 시작할 때 — 모델 다운로드 불필요:**

```powershell
.\Start-Grammar.ps1 -WithoutLocalModel
```

실행한 PowerShell 창은 앱을 관리하므로 사용하는 동안 열어 두고, 종료는 Pencil 트레이 메뉴에서 하세요. 이미 실행 중이라면 두 번째 세션을 띄우지 마세요.

## 첫 문장을 다듬어 보기

**데스크톱:** 지원되는 입력칸에서 짧은 문장을 선택하고 `Ctrl+Shift+G`를 누르세요. 결과를 읽고 필요하면 수정한 뒤 **Accept**로 원문을 교체하거나 **Copy**로 직접 붙여넣으세요. 원문·선택이 바뀌었거나 시간이 지나 거부되면 다시 선택합니다.

**Chromium:** 설치 폴더의 `extension`을 개발자 모드로 로드하고, 확장 아이콘을 고정하세요. 팝업의 **Enable this document** → 지원 입력칸의 **Enable this field** 순으로 켭니다. 설치 위치 선택부터 중지 방법까지는 [상세 안내](docs/GETTING_STARTED.ko.md)를 참고하세요.

## 어디에서 사용할 수 있나요?

| 대상 | Prototype 2 범위 |
| --- | --- |
| Windows 표준 Edit / RichEdit | 검증 조건을 충족하는 특정 컨트롤에서 캡처·Accept. 모든 앱의 입력칸을 뜻하지 않음 |
| Chromium textarea / 단순 contenteditable | 명시적으로 켠 문서·필드에서 제안·Accept. 모든 웹사이트 호환 보장 아님 |
| Windows 11 새 메모장, Microsoft Office, Hancom | 일반 지원 미완료. 파일 확장자만으로 Apply를 보장하지 않음 |
| LibreOffice Writer | 별도 paired adapter의 제한적 개발 검증. 기본 ZIP의 지원 범위에 포함되지 않음 |
| Edge, 물리 IME, 혼합 DPI, 장시간 사용 | 광범위한 실사용 검증 미완료 |

## 속도와 검증 결과

아래는 **RTX 5070 Ti·합성 RichEdit·제품 커밋 `792032b`**에서 얻은 작은 표본입니다. 다른 PC의 성능 보장이나 언어 품질 전체 평가가 아닙니다.

| 항목 | 관측 결과 |
| --- | --- |
| 실제 Gemma 제안 → Accept → 원문 재조회 | 14/14 성공 |
| 실제 마우스 클릭 → 원문 변경 확인 | 113–332 ms, 7건 |
| 짧은 문장 응답 | 687–1371 ms; 10건 중 2건은 1초 목표 초과 |
| 4문장 단락 응답 | 987–1090 ms, 4건 |
| 모델 준비 | 16.164초 |

Prototype 2는 WebView 종료를 요청하고 런타임을 해제한 뒤 실제 브라우저 프로세스 종료를 기다립니다. 화면이 닫혀도 내부 정리는 이어질 수 있으며 WebView 정리 한도는 30초입니다. Prototype 1의 자식 프로세스 잔류 기록은 보존했습니다. 최신 패키지의 소스·검사 결과는 릴리스의 `VALIDATION.json`을 확인하세요. 이번 수리는 번역 품질이나 응답 속도를 새로 보증하는 변경이 아닙니다. [검증·개발 안내](docs/PROTOTYPE.md)

## 개인정보와 클라우드 사용

- 기본 Gemma 추론은 로컬에서 실행됩니다. 최초 모델·런타임 다운로드에는 인터넷이 필요합니다.
- 브라우저 분석은 opt-in한 문서·필드에서만 동작하며, 비밀번호·민감 필드는 처리하지 않습니다.
- CLI Deep 동의는 Provider별로 저장됩니다. **Auto rewrite**가 켜져 있으면 동의 후 캡처 시 전송될 수 있습니다. 수동으로 결정하려면 이를 끄세요.
- API 방식은 **Run API Deep**으로 명시적으로 요청합니다. 모든 API 형식 지원은 아닙니다. 클라우드 사용에는 제공업체의 데이터 정책과 비용이 적용됩니다.
- 로그·스크린샷·Issue에 원문, API 키, 인증 코드나 사용자 문서를 올리지 마세요.

## 도움이 필요하신가요?

- **설치 오류, 모델 준비 실패, Accept 거부:** [설치·사용·문제 해결 안내](docs/GETTING_STARTED.ko.md)
- **빌드하거나 검증 기록을 읽고 싶다면:** [개발자 안내](docs/PROTOTYPE.md)
- **재현 가능한 문제를 발견했다면:** [버그 보고](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)

현재 실행 파일은 서명되지 않았습니다. 보안 정책을 끄지 말고, unsigned 실행이 금지된 환경에서는 사용을 보류하세요. 공개 저장소라는 사실이 별도의 오픈소스 사용 허가를 뜻하지는 않습니다. 프로젝트 전체 라이선스는 아직 지정되지 않았고, 의존성과 모델에는 각 upstream 조건이 적용됩니다.

---

개발 참여 전 [AGENTS.md](AGENTS.md)를 확인하세요. 현재 상태는 [control/state.json](control/state.json), 상세 이력은 [HANDOFF](docs/control/HANDOFF.md)에 기록합니다.


개발 우선순위와 완료조건은 [지속 품질 개선 백로그](docs/control/IMPROVEMENT_BACKLOG.md)에서 관리합니다. 기존 기능 개발과 함께 CI·안정성·사용성·개인정보·배포 위험을 주요 단계마다 재평가합니다.
