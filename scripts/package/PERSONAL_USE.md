# Grammar 개인용 패키지 안내

이 패키지는 한 Windows 사용자 계정에서 개인적으로 쓰기 위한 후보 빌드입니다. 공개 배포본이 아니며 서명되지 않았습니다. `MANIFEST.json`에 이 패키지를 만든 소스 커밋, 모든 파일의 SHA-256, 고정된 확장 ID, 지원 범위가 기록되어 있고, `BUILD_RECEIPT.json`에 깨끗한 소스에서 실행한 release 빌드 영수증이 있습니다.

## 지원 범위 (합성 검증을 통과한 범위만 켜져 있음)

| 기능 | 상태 |
|---|---|
| Windows 표준 `Edit` 컨트롤(예: 클래식 메모장, 대화상자 입력칸)의 선택문 읽기 | 켜짐. 비밀번호·자격 증명·비표준(리치에디트, WinForms 파생, ANSI) 필드는 텍스트를 읽기 전에 거부 |
| 데스크톱 편집기에 대한 적용(Apply) | Copy-only. Grammar는 다른 앱 편집기의 텍스트를 바꾸지 않습니다. **Apply** 를 누르면 결과가 클립보드에 복사되고, 붙여넣기는 사용자가 직접 합니다. 그 앱이 같은 순간 스스로 텍스트를 바꾸는 경우를 배제할 방법이 없어 자동 교체는 꺼져 있습니다(native Apply 기능 미완료) |
| 그 밖의 편집기 | 선택문 읽기도 하지 않음. 결과는 Copy로만 사용 |
| Chromium 확장: textarea / 단순 contenteditable | 문서와 필드를 각각 명시적으로 켠 경우에만 로컬 Instant 제안 |
| Deep (클라우드 Provider) | 선택한 Provider 하나와 명시적 동의가 있을 때만. 실제 Provider 호출은 검증되지 않음(NOT_RUN) |

## 설치

PowerShell 7에서 실행합니다.

```powershell
Expand-Archive .\<패키지>.zip -DestinationPath .\grammar-package
pwsh -NoProfile -File .\grammar-package\Install-Grammar.ps1
```

- 기본 설치 위치는 `%LOCALAPPDATA%\GrammarPersonal` 입니다. 다른 위치는 `-InstallRoot <경로>` 로 지정합니다. 이미 있는 폴더에는 설치하지 않습니다.
- 설치 스크립트는 복사 전에 모든 파일의 해시를 `MANIFEST.json` 과 비교하고, 하나라도 다르면 중단합니다.
- Chrome 네이티브 호스트를 현재 사용자(HKCU)에만 등록합니다. 브라우저 확장을 쓰지 않으려면 `-SkipBrowserHost` 를 붙입니다.
- Chrome에서 `chrome://extensions` → 개발자 모드 → **압축해제된 확장 프로그램 로드** → `<설치 위치>\extension` 을 선택합니다. 확장 ID는 `MANIFEST.json` 의 `extensionId` 와 같아야 합니다.

## 사용

- 데스크톱: `<설치 위치>\codex-pencil.exe` 를 실행합니다. 표준 Edit 필드에서 텍스트를 선택하고 `Ctrl+Shift+G` 를 누르면 로컬 Instant 초안이 나타납니다. 초안을 고친 뒤 **Apply** 또는 **Copy** 를 누릅니다. 데스크톱에서 Apply는 결과를 클립보드에 복사하고 창에 "Copied — paste manually" 를 표시하며, 원래 필드는 바꾸지 않으므로 직접 붙여넣습니다. 클라우드 동의는 Deep을 요청할 때만 묻습니다.
- 브라우저: 확장 팝업에서 **Enable this document**, 필드에서 **Enable this field** 를 누릅니다. 제안 카드에서 Accept / Apply edit / Dismiss / Ignore / Copy 를 키보드나 마우스로 선택합니다.

## 제거

```powershell
pwsh -NoProfile -File "$env:LOCALAPPDATA\GrammarPersonal\Uninstall-Grammar.ps1" -RemoveUserData
```

- 설치 위치에서 실행 중인 프로세스만 종료하고, 설치 영수증에 기록된 네이티브 호스트 등록만 지운 뒤 설치 폴더를 삭제합니다.
- `-RemoveUserData` 는 설정, 개인 사전, WebView 데이터, 앱 전용 Codex 홈(`%APPDATA%\com.local.codexpencil`, `%LOCALAPPDATA%\com.local.codexpencil`)도 삭제합니다.
- Chrome의 확장은 `chrome://extensions` 에서 직접 제거합니다.

## 남은 제한 (자세한 PASS/FAIL/NOT_RUN 은 HANDOFF.md)

- 실제 Provider(Codex/Claude/Antigravity) 호출, 계정 로그인, 비용은 검증하지 않았습니다(NOT_RUN).
- 물리 키보드·IME·확대/축소·다중 모니터·장시간 사용은 자동 합성 환경에서만 확인했습니다.
- 데스크톱 편집기에 결과를 자동으로 넣는 기능(native Apply)은 완성되지 않았습니다. 다른 프로세스의 편집기에는 "확인한 범위가 그대로일 때만 바꾸기"를 한 번에 수행하는 방법이 없어, 확인 직후 그 앱이 텍스트를 바꾸면 덮어쓸 수 있기 때문입니다. 그래서 Apply는 항상 Copy-only입니다.
- Chrome 확장의 DOM 교체는 브라우저 실행 취소를 보장하지 않습니다. Edge 등록과 공개 웹스토어 설치는 검증 범위 밖입니다.
- 설치된 확장을 그대로 쓰는 경로(툴바 클릭 또는 단축키로 activeTab 권한 부여)는 자동 시험에서 실제 사용자 제스처를 만들 수 없으면 NOT_RUN 으로 기록됩니다. 자세한 구분은 HANDOFF.md 를 보세요.
