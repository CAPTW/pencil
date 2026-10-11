# Grammar 개인용 패키지 안내

이 패키지는 한 Windows 사용자 계정에서 개인적으로 쓰기 위한 후보 빌드입니다. 공개 배포본이 아니며 서명되지 않았습니다. `MANIFEST.json`에 이 패키지를 만든 소스 커밋, 모든 파일의 SHA-256, 고정된 확장 ID, 지원 범위가 기록되어 있고, `BUILD_RECEIPT.json`에 깨끗한 소스에서 실행한 release 빌드 영수증이 있습니다.

## 현재 실험 후보의 검증 상태

현재 검증된 제품 커밋은 **84df647**입니다. 격리된 Windows Sandbox에서 표준 한 줄 Edit·명시적 자동 스크롤 다중행 Edit·RichEdit의 검토 후 Accept를 확인했고, 이 패키지의 다중행 Edit 회귀 15개와 Chromium 전역 위젯 흐름 10개가 통과했습니다. 원문 교체 후 내용을 실제로 다시 읽어 확인합니다. 검증은 합성 문서와 정확한 지원 컨트롤에 한정됩니다. Word·Excel·PowerPoint·한컴 앱 adapter는 아직 구현·검증되지 않았습니다.

## 지원 범위

| 기능 | 상태 |
|---|---|
| Windows 표준 `Edit` 컨트롤(예: 클래식 메모장, 대화상자 입력칸)의 선택문 읽기 | 켜짐. 비밀번호·자격 증명·알 수 없는 필드는 텍스트를 읽기 전에 거부. 명시적 비민감 입력 범위와 대상 서비스 조건을 충족한 컨트롤만 직접 Accept 시험 범위 |
| 데스크톱 결과 사용 | 검증 조건을 충족한 x64 한 줄 `Edit`, `ES_MULTILINE`+`ES_AUTOVSCROLL` Edit, `RichEdit50W`에서 **Accept**와 **Copy**를 제공합니다. Accept는 검토한 초안을 대상 내부에서 재검증한 뒤 선택 범위만 교체합니다. 입력 범위가 불명확하거나 보호·조합 입력 등 조건이 맞지 않으면 읽기/교체를 거부합니다. |
| 직접 Apply 제한 | 현재 캡처 권한은 최대 30초입니다. 원문·선택·권한이 바뀌거나 시간이 지나면 거부합니다. 결과가 불명확하면 원문을 직접 확인하도록 안내하며 자동 재시도·자동 Copy·원문 복원을 하지 않습니다. Copy는 별도로 명시적으로 선택합니다. |
| 그 밖의 편집기 | 선택문 읽기도 하지 않음. 결과는 Copy로만 사용 |
| Chromium 확장: textarea / 단순 contenteditable | 문서·필드 opt-in 후 로컬 inline 제안. 선택 후 Ctrl+Shift+G로 데스크톱 위젯의 검토·편집·Accept도 가능. 일반 사이트 전체의 호환성을 뜻하지 않음 |
| Deep (클라우드 Provider) | 선택한 Provider 하나와 명시적 동의가 있을 때만. 실제 Provider 호출은 검증되지 않음(NOT_RUN) |

## Gemma 시작 연결 후보

`Prepare-GrammarLocal.ps1`은 고정된 Gemma 12B Q4_0 모델(약 7GB)과 llama.cpp CUDA 12.4 런타임을 `local` 폴더에 준비합니다. 첫 실행은 다운로드와 SHA-256 검증 때문에 오래 걸릴 수 있습니다. 공식 파일을 이미 보관했다면 `-AssetCache <폴더>`로 네트워크 없이 복사·검증합니다. 이 모드에서 파일이 없으면 다운로드로 전환하지 않습니다. GPU 드라이버를 설치하거나 변경하지 않습니다.

**실제 모델 실행 전제조건:** 이 llama.cpp 배포본은 Microsoft Visual C++ 런타임 DLL도 필요합니다. 깨끗한 Windows Sandbox에서 DLL이 없으면 모델 실행 전 `0xC0000135`로 종료되는 것을 확인했습니다. 현재 패키지는 이 의존성을 자동 설치하거나 사전에 구체적으로 진단하지 않습니다. 실제 모델 Sandbox 시험에서는 서명을 확인한 Visual Studio redistributable DLL을 시험 프로세스에만 제공했으며 시스템 설정은 변경하지 않았습니다. 이 시험 조건은 일반 설치 패키지에 의존성이 포함됐다는 뜻이 아닙니다. [Microsoft의 배포 방식 안내](https://learn.microsoft.com/en-us/cpp/windows/choosing-a-deployment-method?view=msvc-170)를 참고하십시오.

준비 후 `pwsh -NoProfile -File .\Start-Grammar.ps1`로 실행합니다. 앱을 먼저 열고 모델 준비 상태를 표시하며, 사용자가 켠 Chromium 필드에만 로컬 Gemma 연결을 제공합니다. 앱을 트레이에서 종료하면 연결 정보를 회수하고 브로커 정리 완료를 기다립니다. 실행 중에는 제거를 거부합니다. 강제로 실행기를 종료해 연결 정보가 남았다면 자동으로 덮어쓰지 않고 오류를 표시합니다.

**현재 범위:** `Start-Grammar.ps1`은 데스크톱 문법 모드의 Gemma 연결을 기본으로 켭니다. 명시적 `-EnableDesktopGemma:$false`로 브라우저 전용 연결을 사용할 수 있습니다. 데스크톱 로컬 연결은 문법, 자연스럽게, 간결하게, 공손하게 및 영어·한국어 번역을 지원합니다. 자동 대상 언어·일본어·중국어 번역은 로컬에서 거부하고 안내합니다. 필요한 경우 지원하는 언어를 선택하거나 별도로 동의한 Deep을 사용합니다. 모델 준비·기동 검증과 실제 설치 앱/브라우저 사용자 흐름 수용은 별개입니다. API endpoint/model/key의 세션 전용 설정과 명시적 API Deep은 구현돼 있습니다. 실제 외부 API 성공 응답은 별도 검증 대상입니다. 이 중간 패키지를 완성된 Gemma 기본 프로토타입으로 배포하지 않습니다. 현재 로컬 추론의 강제 취소·시간 초과는 모델 프로세스를 종료할 수 있습니다. 모델 종료 시 브로커는 새 요청을 받지 않고 종료합니다. 실행기는 앱을 강제 종료하지 않으며, 앱이 열려 있는 동안 설치 잠금과 중복 실행·제거 차단을 유지합니다. 자동 재시작하지 않습니다. 정리 완료가 확인되면 Retry local model로 다시 준비할 수 있으며, 버튼이 제공되지 않으면 초안을 확인한 뒤 트레이에서 종료하고 실행기로 다시 시작합니다. 작업 실패와 정리 완료는 별도로 보고합니다. 편집 초안은 분석 실패로 덮어쓰지 않습니다.

## 설치

Microsoft WebView2 Runtime이 필요합니다. 설치기는 패키지 앱의 실제 런타임 검색을 먼저 실행하며, 검색 실패·비정상 종료·시간 초과 시 폴더나 등록을 만들지 않습니다. 런타임을 자동 설치하지 않습니다. [Microsoft 공식 WebView2 안내](https://developer.microsoft.com/microsoft-edge/webview2/)에서 설치한 후 다시 시도하세요. 이 검색 성공만으로 UI 동작 전체가 검증되는 것은 아닙니다. Chromium 호스트는 MSVC CRT를 정적으로 연결합니다.

PowerShell 7(`pwsh`)에서 실행합니다. Windows PowerShell 5.1(탐색기의 "PowerShell로 실행")로는 설치 스크립트가 시작되지 않고 PowerShell 7이 필요하다고 알립니다.

```powershell
Get-FileHash .\<패키지>.zip -Algorithm SHA256   # package.ps1 이 출력한 zipSha256 과 같아야 합니다
Expand-Archive .\<패키지>.zip -DestinationPath .\grammar-package   # 빈 새 폴더에 풉니다
pwsh -NoProfile -File .\grammar-package\Install-Grammar.ps1
```

- 기본 설치 위치는 `%LOCALAPPDATA%\GrammarPersonal` 입니다. 다른 위치는 `-InstallRoot <경로>` 로 지정합니다. 이미 있는 폴더에는 설치하지 않습니다.
- 설치 스크립트는 복사 전에 모든 파일의 해시를 `MANIFEST.json` 과 비교합니다. 하나라도 다르거나 `MANIFEST.json` 에 없는 파일이 있으면 아무것도 만들지 않고 중단합니다.
- Chrome 네이티브 호스트를 현재 사용자(HKCU)에만 등록합니다. Chrome에 아직 네이티브 호스트 키가 없으면 그 키를 만듭니다. 브라우저 확장을 쓰지 않으려면 `-SkipBrowserHost` 를 붙입니다.
- 설치 중 오류가 나면 만든 폴더와 등록을 되돌리고 "Nothing was left installed" 로 끝납니다. 되돌리기가 일부 실패하면 오류 문구가 알려 주는 `<설치 위치>\Uninstall-Grammar.ps1` 을 실행해 마무리합니다.
- Chrome에서 `chrome://extensions` → 개발자 모드 → **압축해제된 확장 프로그램 로드** → `<설치 위치>\extension` 을 선택합니다. 확장 ID는 `MANIFEST.json` 의 `extensionId` 와 같아야 합니다.

## 사용

- 데스크톱: PowerShell 7에서 `pwsh -NoProfile -File "<설치 위치>\Start-Grammar.ps1"`로 실행합니다. exe를 직접 실행하면 관리되는 Gemma 연결이 제공되지 않습니다. 검증 조건을 충족한 필드에서 텍스트를 선택하고 `Ctrl+Shift+G`를 누르면 로컬 Instant 초안이 나타납니다. 초안을 검토·편집한 뒤 **Accept**를 누르면 정확한 선택 범위만 교체합니다. **Copy**를 선택하면 원문을 바꾸지 않고 클립보드에 복사하며 "Copied — paste it into the field"가 표시됩니다. 초안은 창에 남고, 다음 선택을 캡처할 때까지 Copy는 비활성화됩니다. 번역의 "원문 + 번역" 형식도 Copy할 때 적용됩니다(한 줄 원문은 `원문 (번역)`, 여러 줄 원문은 빈 줄 뒤 `(번역)`). 클라우드 동의는 Deep을 요청할 때만 묻습니다.
  - Deep 동의는 Provider 별로 저장되어 재시작 뒤에도 유지됩니다. 동의한 뒤에는 **Auto rewrite**(기본 켜짐)가 켜져 있는 한 캡처할 때마다 선택문이 그 Provider 로 전송됩니다. 캡처마다 직접 정하려면 설정에서 Auto rewrite 를 끄고 필요할 때 **Run Deep** 을 누릅니다. 저장된 동의를 되돌리는 화면은 없습니다(`-RemoveUserData` 로 사용자 데이터를 지우면 초기화됩니다).
  - Deep 이 실행되는 동안에는 Copy 가 비활성화됩니다. Instant 초안을 바로 쓰려면 **Cancel**(또는 결과 영역에서 Escape)을 누릅니다. Cancel 은 Deep만 멈추고, 캡처와 (고친) 초안은 그대로 남아 Copy할 수 있습니다. 창을 닫거나 단축키·트레이로 숨기면 그 캡처는 끝나며 더 이상 Copy할 수 없습니다.
  - 초안을 직접 고친 뒤 모드·번역 언어·사전을 바꾸면, 고친 초안을 버릴지 먼저 묻습니다. 취소하면 아무것도 바뀌지 않습니다.
  - Claude Deep 은 네이티브 `claude.exe` 가 필요합니다. npm 으로 설치된 `claude.cmd` 만 있으면 여러 줄 요청을 전달할 수 없어 상태가 "사용할 수 없음"으로 표시됩니다. Antigravity 도 같은 이유로 `agy.exe` 가 필요합니다.
- 브라우저: 먼저 확장 아이콘을 툴바에 고정합니다(퍼즐 아이콘 → 핀. 시험은 버튼이 미리 고정된 새 프로필을 썼습니다). 검증된 경로는 키보드입니다. `Alt+Shift+T` 로 툴바로 이동하고, 오른쪽 방향키(필요하면 Tab)로 Grammar 버튼에 초점을 맞춘 뒤 Space 를 누릅니다. 팝업에서 Tab 으로 **Enable this document** 에 초점을 맞추고 Space 를 누릅니다. 마우스로 누르는 경로는 검증되지 않았습니다. 그다음 필드를 켭니다. 패널의 "Field to enable: <필드 이름>" 이 켜질 필드를 알려 주며, **Enable this field** 를 누르거나 그 필드 안에서 `Alt+Shift+E` 를 누르면 바로 그 필드만 켜집니다. 제안 목록에서 하나를 고르면(클릭, Enter, Space) 그 제안의 카드가 열리고, Accept / Apply edit / Dismiss / Ignore / Copy 는 그 제안에만 적용됩니다. 목록을 지나가거나 마우스를 올리는 것만으로는 카드가 바뀌지 않습니다. Accept 뒤에도 다른 제안은 남습니다.

## 제거

먼저 앱 상단의 전원 버튼(**Quit Grammar**)을 누르고 확인하거나, 트레이 메뉴의 Quit으로 Grammar를 끄세요. 전원 버튼의 확인을 취소하면 앱과 진행 중인 작업이 유지됩니다. Close는 창만 숨깁니다. 종료 시 현재 초안은 저장되지 않으며, 정리를 완료하지 못하면 앱이 열린 채 오류를 표시합니다. 앱이 종료된 뒤 설치 폴더를 연 탐색기·터미널 창을 닫습니다.

```powershell
pwsh -NoProfile -File "$env:LOCALAPPDATA\GrammarPersonal\Uninstall-Grammar.ps1"
```

- 설치 위치에서 실행 중인 프로세스만 종료하고, 설치 영수증에 기록된 네이티브 호스트 등록만 지운 뒤 설치 폴더를 삭제합니다. 다른 위치에 설치했다면 그 폴더의 `Uninstall-Grammar.ps1` 을 실행합니다(자기 폴더를 지웁니다).
- 파일이 잠겨 일부가 남으면 결과에 `complete: false` 와 남은 항목이 나옵니다. 영수증과 제거 스크립트는 마지막까지 남으므로, 잠근 프로그램을 닫고 같은 명령을 다시 실행하면 이어서 지웁니다.
- 설정과 개인 사전은 남습니다. 모두 지우려면 `-RemoveUserData` 를 붙입니다. 이 옵션은 설정, 개인 사전, WebView 데이터, 앱 전용 Codex 로그인(`%APPDATA%\com.local.codexpencil`, `%LOCALAPPDATA%\com.local.codexpencil`)을 지우며, 같은 Windows 사용자의 다른 Grammar 사본(이전 번들 포함)도 같은 데이터를 씁니다. 다른 Grammar 가 실행 중이면 거부합니다.
- Chrome의 확장은 `chrome://extensions` 에서 직접 제거합니다.

## 새 패키지로 바꾸기

1. 위의 제거 명령을 `-RemoveUserData` 없이 실행합니다. 설정과 사전은 유지됩니다.
2. 새 패키지를 같은 위치에 설치합니다.
3. 패키지마다 확장 ID가 새로 정해집니다. `chrome://extensions` 에서 이전 Grammar 확장을 제거하고 새 `<설치 위치>\extension` 을 다시 로드합니다.

## 남은 제한 (자세한 PASS/FAIL/NOT_RUN 은 HANDOFF.md)

- 실제 Provider(Codex/Claude/Antigravity) 호출, 계정 로그인, 비용은 검증하지 않았습니다(NOT_RUN).
- Claude·Antigravity Deep 은 선택문과 일치한 용어가 담긴 요청을 명령줄 인수로 전달합니다. 요청이 실행되는 동안 같은 Windows 사용자의 다른 프로그램(또는 관리자)이 프로세스 목록에서 그 내용을 볼 수 있습니다. Codex 는 표준 입력으로 전달합니다.
- 데스크톱 창은 Escape 로 닫히지 않습니다(Deep 실행 중 결과 영역의 Escape 는 Deep 만 취소합니다). Dismiss, 창의 닫기 버튼 또는 단축키를 씁니다.
- 브라우저 페이지가 주소만 바꾸는 이동(history API, `#` 이동)을 하면 패널이 이유 표시 없이 사라집니다. 문서를 다시 켜면 됩니다.
- 제거 후에도 `%TEMP%\codex-pencil-runtime-v1` 폴더가 남습니다. Deep 을 실행할 때 Provider 의 작업 폴더로 쓰이며, 요청마다 만든 하위 폴더는 끝나면 지웁니다. Grammar 자신은 여기에 표시 파일만 쓰지만, 실제 Provider 가 작업 중 무엇을 쓰는지는 검증하지 않았습니다(NOT_RUN). Grammar 가 실행 중이 아니면 지워도 됩니다.
- 브라우저 Deep 의 정리 기록 파일이 비정상 종료 등으로 불완전하게 남으면, 안전을 위해 브라우저 Deep 이 계속 막힙니다. 팝업의 **Check completed cleanup** 도 "Cleanup check failed; Deep remains blocked" 로 끝납니다. 기록은 설치 폴더에 있으므로 확장을 다시 로드해도 풀리지 않습니다. 패키지를 제거한 뒤 다시 설치합니다. Instant 는 영향을 받지 않습니다.
- 물리 키보드·실제 IME·확대/축소·다중 모니터·장시간 실사용은 이번 Apply 복구에서 검증하지 않았습니다.
- 명시적 Accept는 지원 컨트롤의 대상 내부 교체 또는 opt-in한 Chromium 필드의 검증된 교체 경로에서 수행합니다. 이전 외부 잠금·전체 원문 복원 경로는 삭제된 상태입니다. 일반 앱에 무조건 적용하지 않으며, 정확한 대상 상태를 증명할 수 없으면 변경하지 않습니다. 자동 Apply는 없습니다.
- Copy는 클립보드의 기존 내용(이미지, 파일, 서식 있는 텍스트 포함)을 결과 텍스트로 바꾸며, 이전 내용을 되돌리지 않습니다.
- Chrome 확장의 DOM 교체는 브라우저 실행 취소를 보장하지 않습니다. Edge 등록과 공개 웹스토어 설치는 검증 범위 밖입니다.
- 설치된 확장을 그대로 쓰는 경로는 한 가지만 검증되었습니다(R3, Owner가 이 범위로만 수용). Chromium의 새 작업용 프로필에서 확장 버튼을 툴바에 고정한 뒤, 그 버튼과 팝업의 **Enable this document** 를 실제 키보드 입력으로 눌렀습니다(Alt+Shift+T 로 툴바 이동, 오른쪽 방향키로 버튼, 팝업에서 Tab, 초점이 맞은 대상에만 Space). 마우스로 확장 메뉴(퍼즐 아이콘)나 팝업을 누르는 경로, 평소 쓰는 일반 프로필, Edge는 검증되지 않았습니다. 자세한 구분은 HANDOFF.md 를 보세요.

### 직접 Accept 사용 범위 (2026-10-05)

표준 한 줄 Edit와 명시적 자동 스크롤(`ES_AUTOVSCROLL`)이 있는 다중행 Edit, RichEdit의 정확한 비민감 입력 범위에서 격리 시험을 통과했습니다. 다중행 자동 스크롤이 없는 Edit는 Undo 결함이 남아 직접 교체를 켜지 않습니다. custom subclass나 입력 범위를 확인하지 못한 필드도 거부됩니다. 일반 메모장이나 모든 Text 앱 지원을 이 시험으로 보장하지 않습니다.

Chromium에서는 데스크톱 앱을 실행한 상태로 위의 문서·필드 opt-in을 마친 뒤, 지원 필드에서 교체할 문장을 선택하고 `Ctrl+Shift+G`를 누릅니다. 위젯에서 초안을 검토·편집하고 **Accept**를 누르면 선택 밖의 내용은 보존하고 원문을 교체합니다. 권한은 최대 30초이며 원문·선택·문서·권한이 바뀌면 거부됩니다. 동작을 확인하지 못하면 원문을 검사하고 새로 선택하십시오. 자동 재시도나 원문 전체 복원은 하지 않습니다. 브라우저 Undo는 보장하지 않습니다.

설치기는 앱·host·인증된 DLL을 같은 설치 폴더에 두고 해당 host를 등록합니다. `host` 하위 폴더에는 등록 메타데이터가 있고, 원본 식별·정리 파일은 실행 파일 옆에 있습니다. 파일을 임의로 옮기지 마십시오.

실제 사람의 키보드·IME, 일반 프로필, Edge, Office(DOCX/XLSX/PPTX) 및 한컴(HWP/HWPX) 앱은 이 범위에 포함되지 않습니다. 현재 후보의 근거와 이전 실패는 `docs/control/HANDOFF.md`에 있습니다. 전체 개발 목표는 미완료입니다.

### 데스크톱 준비 상태 시험 연결

기본 `Start-Grammar.ps1` 실행에서는 실행 파일 검증 후 앱이 모델보다 먼저 열립니다. 앱 상단에서 준비 중·사용 가능·실패 상태를 확인합니다. 준비 중 캡처는 모델 요청으로 예약되지 않습니다. 준비가 끝난 뒤 텍스트를 다시 캡처하십시오. 실패해도 앱과 초안은 열린 채로 남습니다. 예열 또는 실행 중 모델이 실패한 뒤 프로세스·작업 폴더 정리가 확인되면 **Retry local model** 버튼으로 앱을 닫지 않고 모델을 다시 준비할 수 있습니다. 이 버튼은 이전 캡처를 재전송하지 않습니다. 준비 완료 뒤 새로 캡처하십시오. 파일 검증 실패나 정리 미확인 상태에서는 버튼이 제공되지 않으므로 필요한 초안을 복사한 뒤 트레이에서 종료하고 실행기를 다시 실행하십시오.

임시 세션 파일은 상태·로컬 pipe 이름·일회용 재시작 식별자만 담고 문서나 생성 결과를 저장하지 않습니다. 정상 종료 때 제거합니다. 실행기가 비정상 종료되면 앱은 보유한 실행기 프로세스 핸들로 종료를 감지하여 연결을 거부합니다. 이는 설치 위젯 전체 수용을 뜻하지 않으며, 기본 활성화는 여전히 보류 상태입니다.


### 모델 파일이 없는 시험 설치에서의 복구

기본 실행에서는 패키지 무결성 확인 후 모델·런타임 파일이 없어도 앱을 엽니다. 모델은 실패 상태로 표시되며 초안은 열린 앱에 유지됩니다. 별도의 PowerShell 7 창에서 설치 폴더의 `Prepare-GrammarLocal.ps1`을 명시적으로 실행하고, 성공한 뒤 앱의 **Retry local model**을 누르십시오. 기존 `local` 폴더가 일부만 남아 준비 스크립트가 거부하면 임의 삭제하지 말고 설치 상태를 점검하십시오. 파일 준비나 Retry는 이전 캡처를 자동으로 전송하지 않습니다. 모델 준비 완료 후 새로 캡처합니다.

파일이 아직 없을 때 Retry를 눌러도 런타임을 생성하지 않고 다시 실패 상태로 돌아갑니다. 준비하지 않고 앱을 종료하면 실행기는 모델 사용 불가를 오류로 보고하고 세션을 정리합니다. 패키지 무결성 실패는 이 복구 경로로 우회되지 않습니다. 실제 모델 품질·속도 수용은 실행기 시험과 별개입니다.


## Private CRT packaging candidate (2026-10-11; installed verification pending)

The package builder now locates the x64 release Visual Studio Redist CRT (or
accepts an explicit `-VCRuntimeDirectory`), verifies Microsoft signatures before
and after copying, and includes the DLLs plus their manifest hashes. Start-Grammar
pins the declared DLLs, refuses missing/changed/extra files, and adds this directory
only to the owned broker's process PATH. No global PATH or runtime installer is used.
The upstream llama runtime bundle digest is unchanged. Older packages without this
metadata retain their existing system-runtime prerequisite; they are not upgraded
by these source changes. Private CRT security updates require rebuilding the package.

This is currently a source candidate, not a newly qualified installation: local
PowerShell startup and the Windows test linker stalled before execution. The new
launcher regressions and clean Sandbox package test still need to run. Do not treat
the earlier process-PATH workaround as qualification of this packaging change.
