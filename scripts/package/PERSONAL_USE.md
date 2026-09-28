# Grammar 개인용 패키지 안내

이 패키지는 한 Windows 사용자 계정에서 개인적으로 쓰기 위한 후보 빌드입니다. 공개 배포본이 아니며 서명되지 않았습니다. `MANIFEST.json`에 이 패키지를 만든 소스 커밋, 모든 파일의 SHA-256, 고정된 확장 ID, 지원 범위가 기록되어 있고, `BUILD_RECEIPT.json`에 깨끗한 소스에서 실행한 release 빌드 영수증이 있습니다.

## 지원 범위 (합성 검증을 통과한 범위만 켜져 있음)

| 기능 | 상태 |
|---|---|
| Windows 표준 `Edit` 컨트롤(예: 클래식 메모장, 대화상자 입력칸)의 선택문 읽기 | 켜짐. 비밀번호·자격 증명·비표준(리치에디트, WinForms 파생, ANSI) 필드는 텍스트를 읽기 전에 거부 |
| 데스크톱 결과 사용 | **Copy** 버튼 하나. 결과를 클립보드에 복사하고, 붙여넣기는 사용자가 직접 합니다. 데스크톱 앱은 다른 앱 편집기의 텍스트를 바꾸지 않습니다. 그 앱이 같은 순간 스스로 텍스트를 바꾸는 경우를 배제할 방법이 없어 자동 교체는 하지 않습니다(Owner 결정: Copy-only 유지, 예전 Apply 버튼은 Copy로 통합) |
| 그 밖의 편집기 | 선택문 읽기도 하지 않음. 결과는 Copy로만 사용 |
| Chromium 확장: textarea / 단순 contenteditable | 문서와 필드를 각각 명시적으로 켠 경우에만 로컬 Instant 제안 |
| Deep (클라우드 Provider) | 선택한 Provider 하나와 명시적 동의가 있을 때만. 실제 Provider 호출은 검증되지 않음(NOT_RUN) |

## 설치

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

- 데스크톱: `<설치 위치>\codex-pencil.exe` 를 실행합니다. 표준 Edit 필드에서 텍스트를 선택하고 `Ctrl+Shift+G` 를 누르면 로컬 Instant 초안이 나타납니다. 초안을 고친 뒤 **Copy** 를 누릅니다. 결과가 클립보드에 복사되고 창에 "Copied — paste it into the field" 가 표시됩니다. 원래 필드는 바뀌지 않으므로 직접 붙여넣습니다. 초안은 창에 남고, 다음 선택을 캡처할 때까지 Copy는 비활성화됩니다. 번역의 "원문 + 번역" 형식도 Copy할 때 적용됩니다(한 줄 원문은 `원문 (번역)`, 여러 줄 원문은 빈 줄 뒤 `(번역)`). 클라우드 동의는 Deep을 요청할 때만 묻습니다.
  - Deep 동의는 Provider 별로 저장되어 재시작 뒤에도 유지됩니다. 동의한 뒤에는 **Auto rewrite**(기본 켜짐)가 켜져 있는 한 캡처할 때마다 선택문이 그 Provider 로 전송됩니다. 캡처마다 직접 정하려면 설정에서 Auto rewrite 를 끄고 필요할 때 **Run Deep** 을 누릅니다. 저장된 동의를 되돌리는 화면은 없습니다(`-RemoveUserData` 로 사용자 데이터를 지우면 초기화됩니다).
  - Deep 이 실행되는 동안에는 Copy 가 비활성화됩니다. Instant 초안을 바로 쓰려면 **Cancel**(또는 결과 영역에서 Escape)을 누릅니다. Cancel 은 Deep만 멈추고, 캡처와 (고친) 초안은 그대로 남아 Copy할 수 있습니다. 창을 닫거나 단축키·트레이로 숨기면 그 캡처는 끝나며 더 이상 Copy할 수 없습니다.
  - 초안을 직접 고친 뒤 모드·번역 언어·사전을 바꾸면, 고친 초안을 버릴지 먼저 묻습니다. 취소하면 아무것도 바뀌지 않습니다.
  - Claude Deep 은 네이티브 `claude.exe` 가 필요합니다. npm 으로 설치된 `claude.cmd` 만 있으면 여러 줄 요청을 전달할 수 없어 상태가 "사용할 수 없음"으로 표시됩니다. Antigravity 도 같은 이유로 `agy.exe` 가 필요합니다.
- 브라우저: 먼저 확장 아이콘을 툴바에 고정합니다(퍼즐 아이콘 → 핀. 시험은 버튼이 미리 고정된 새 프로필을 썼습니다). 검증된 경로는 키보드입니다. `Alt+Shift+T` 로 툴바로 이동하고, 오른쪽 방향키(필요하면 Tab)로 Grammar 버튼에 초점을 맞춘 뒤 Space 를 누릅니다. 팝업에서 Tab 으로 **Enable this document** 에 초점을 맞추고 Space 를 누릅니다. 마우스로 누르는 경로는 검증되지 않았습니다. 그다음 필드를 켭니다. 패널의 "Field to enable: <필드 이름>" 이 켜질 필드를 알려 주며, **Enable this field** 를 누르거나 그 필드 안에서 `Alt+Shift+E` 를 누르면 바로 그 필드만 켜집니다. 제안 목록에서 하나를 고르면(클릭, Enter, Space) 그 제안의 카드가 열리고, Accept / Apply edit / Dismiss / Ignore / Copy 는 그 제안에만 적용됩니다. 목록을 지나가거나 마우스를 올리는 것만으로는 카드가 바뀌지 않습니다. Accept 뒤에도 다른 제안은 남습니다.

## 제거

먼저 트레이 메뉴의 Quit 으로 Grammar 를 끄고, 설치 폴더를 연 탐색기·터미널 창을 닫습니다.

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
- 물리 키보드·IME·확대/축소·다중 모니터·장시간 사용은 자동 합성 환경에서만 확인했습니다.
- 데스크톱 편집기에 결과를 자동으로 넣는 기능은 없습니다(Owner 결정으로 Copy-only 유지). 다른 프로세스의 편집기에는 "확인한 범위가 그대로일 때만 바꾸기"를 한 번에 수행하는 방법이 없어, 확인 직후 그 앱이 텍스트를 바꾸면 덮어쓸 수 있기 때문입니다.
- Copy는 클립보드의 기존 내용(이미지, 파일, 서식 있는 텍스트 포함)을 결과 텍스트로 바꾸며, 이전 내용을 되돌리지 않습니다.
- Chrome 확장의 DOM 교체는 브라우저 실행 취소를 보장하지 않습니다. Edge 등록과 공개 웹스토어 설치는 검증 범위 밖입니다.
- 설치된 확장을 그대로 쓰는 경로는 한 가지만 검증되었습니다(R3, Owner가 이 범위로만 수용). Chromium의 새 작업용 프로필에서 확장 버튼을 툴바에 고정한 뒤, 그 버튼과 팝업의 **Enable this document** 를 실제 키보드 입력으로 눌렀습니다(Alt+Shift+T 로 툴바 이동, 오른쪽 방향키로 버튼, 팝업에서 Tab, 초점이 맞은 대상에만 Space). 마우스로 확장 메뉴(퍼즐 아이콘)나 팝업을 누르는 경로, 평소 쓰는 일반 프로필, Edge는 검증되지 않았습니다. 자세한 구분은 HANDOFF.md 를 보세요.
