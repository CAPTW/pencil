# Pencil 설치부터 첫 사용까지

[소개로 돌아가기](../README.md) · [Prototype 2 릴리스](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2)

이 안내는 **Windows x64 Prototype 2**용입니다. 개발 환경을 설치하거나 소스를 빌드하지 않아도 실행 ZIP으로 시작할 수 있습니다. 아직 모든 PC·편집기에 대한 호환성이 검증된 제품은 아닙니다.

## 1. 실행 환경 준비

- **PowerShell 7:** [Microsoft 공식 설치 안내](https://learn.microsoft.com/en-us/powershell/scripting/install/install-powershell-on-windows)를 따라 준비하세요. 이번 시험은 일반 MSI 설치의 PowerShell 7에서 진행했습니다. 시작 메뉴에서 **PowerShell 7**을 열고 `$PSVersionTable.PSVersion`으로 버전을 확인합니다. Windows 기본 **Windows PowerShell 5.1**과 다릅니다.
- **WebView2 Runtime:** [Microsoft 공식 다운로드](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)에서 준비합니다. 이미 설치돼 있다면 중복 설치할 필요는 없습니다.
- **로컬 Gemma:** 약 7 GB 모델과 GPU 런타임, 준비 중 임시 공간이 필요합니다. 기본 CUDA 13.4 구성은 RTX 5070 Ti에서 검증했습니다. 모든 NVIDIA GPU를 지원한다고 보장하지 않습니다.

Pencil은 사용자별 폴더에 설치합니다. 선행 프로그램 설치에는 해당 환경의 권한 정책이 적용됩니다. 모델 없이 시험하려면 Gemma 준비 단계를 건너뛸 수 있습니다.

## 2. 다운로드와 무결성 확인

릴리스의 **Assets**에서 실행 ZIP과 `SHA256SUMS.txt`를 받으세요. **Source code (zip)**은 소스 개발용입니다.

PowerShell에서 다운로드한 폴더로 이동한 뒤 확인합니다. 저장 위치를 바꿨다면 실제 폴더를 사용하세요.

```powershell
Set-Location "$env:USERPROFILE\Downloads"
Get-FileHash .\Pencil-0.1.0-prototype.2-windows-x64.zip -Algorithm SHA256
Get-Content .\SHA256SUMS.txt
```

ZIP 항목의 해시가 대소문자를 제외하고 정확히 같아야 합니다. 다르면 설치하지 말고 다시 다운로드하세요. 이 값은 파일 무결성 확인이며 코드 서명은 아닙니다.

일치한 파일을 새 폴더에 압축 해제합니다. 다운로드 차단 때문에 스크립트 실행이 거부되는 경우에는 **출처와 해시를 확인한 이 ZIP만** 속성에서 차단 해제한 뒤 새 폴더에 다시 풀 수 있습니다. 조직 정책이 unsigned 앱을 금지한다면 사용을 보류하세요. 시스템 실행 정책이나 보안 프로그램을 끄는 방식으로 우회하지 마세요.

## 3. 새 폴더에 설치

**PowerShell 7**을 열고 `Install-Grammar.ps1`이 보이는 압축 해제 폴더로 이동합니다. 탐색기의 주소 표시줄에서 경로를 복사할 수 있습니다.

```powershell
# 실제 압축 해제 폴더 경로로 바꿔 주세요.
Set-Location 'C:\경로\압축을 푼 폴더'
.\Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA\PencilPrototype1"
```

성공하면 설치 경로와 결과가 표시됩니다. 설치 폴더가 이미 존재하면 덮어쓰지 않습니다. 기존 설치가 있다면 다른 새 경로로 시험하고, 동시에 실행하지 마세요. 설치기는 Chromium 연결을 위한 사용자 범위 native host 등록도 수행합니다.

이후 명령은 **압축 해제 폴더가 아닌 설치 폴더**에서 실행합니다.

```powershell
Set-Location "$env:LOCALAPPDATA\PencilPrototype1"
```

## 4. 시작 방법 선택

### A. 로컬 Gemma 번역·재작성

최초 한 번, 모델과 런타임을 다운로드하고 SHA-256으로 검증합니다.

```powershell
.\Prepare-GrammarLocal.ps1
```

네트워크와 저장장치에 따라 오래 걸릴 수 있습니다. 다운로드 창을 중복 실행하지 마세요. 완료된 뒤 앱을 시작합니다.

```powershell
.\Start-Grammar.ps1
```

다음 실행부터는 `Start-Grammar.ps1`만 사용합니다. 모델 준비 중에는 기다려 주세요. 짧은 문장의 응답 시간과 첫 모델 로딩 시간은 다릅니다. 사용하는 동안 실행한 PowerShell 창을 유지하고 종료는 앱 트레이 메뉴에서 합니다.

### B. 모델 없이 시작

```powershell
.\Start-Grammar.ps1 -WithoutLocalModel
```

모델을 내려받거나 실행하지 않습니다. 기본 로컬 규칙은 사용할 수 있지만, Gemma의 번역·재작성 결과가 제공되는 모드는 아닙니다. 클라우드 Deep을 쓰려면 Provider 또는 API를 따로 설정하고 전송에 동의해야 합니다. CPU에서 동일 품질·속도를 보장하는 옵션으로 해석하지 마세요.

## 5. 데스크톱에서 첫 문장 시험

1. 지원되는 표준 Edit/RichEdit 입력칸에 직접 만든 짧은 비민감 문장을 입력합니다.
2. 문장을 선택하고 **Ctrl+Shift+G**를 누릅니다.
3. 결과를 읽고, 필요하면 초안을 수정합니다.
4. **Accept**로 선택 범위를 교체하거나 **Copy** 후 직접 붙여넣습니다.
5. 실제 원문이 의도대로 바뀌었는지 확인합니다.

Windows 11의 새 메모장, Word, Excel, PowerPoint, Hancom을 일반 지원 대상으로 보지 마세요. 지원되지 않는 입력칸을 거부하는 것은 정상입니다. 원문이나 선택이 바뀌었거나 검토 권한이 만료됐다면 다시 선택하고 캡처하세요. Apply를 강제로 반복하거나 검증을 우회하지 마세요.

## 6. Chromium에서 첫 제안 시험

먼저 별도 브라우저 프로필과 직접 만든 시험 문장을 사용하세요.

1. Chrome/Chromium의 `chrome://extensions`를 엽니다.
2. **개발자 모드**를 켜고 **압축해제된 확장 프로그램을 로드합니다**를 선택합니다.
3. 설치 폴더 안의 `extension` 폴더를 선택합니다. ZIP이나 그 위 폴더가 아닙니다.
4. 확장 아이콘을 툴바에 고정합니다. 앱도 설치 폴더의 launcher로 실행해 둡니다.
5. 시험 페이지에서 확장 팝업의 **Enable this document**를 선택합니다.
6. textarea 또는 지원되는 단순 contenteditable을 선택하고 **Enable this field**를 누릅니다. **Alt+Shift+E**도 해당 필드 활성화에 사용할 수 있습니다.
7. 문장을 입력하고 제안을 선택해 검토합니다. **Accept** 또는 **Apply edit** 후 실제 내용을 확인합니다.

필드를 잠시 멈추려면 **Pause**, 문서를 끄려면 **Disable document** 또는 **Alt+Shift+G**를 사용합니다. 팝업에서 도메인 차단과 emergency disable도 제공됩니다. 페이지 이동 뒤에는 다시 문서 opt-in이 필요합니다. 비밀번호·민감 필드는 대상이 아닙니다.

팝업의 마우스 경로가 작동하지 않으면 검증된 키보드 경로를 시도할 수 있습니다. **Alt+Shift+T**로 툴바에 들어가 방향키로 고정한 확장 아이콘을 선택하고 Space, 팝업에서는 Tab으로 원하는 버튼의 초점을 확인한 뒤 Space를 누릅니다. 버전별 UI 차이와 일반 사이트 전체 호환성은 보장하지 않습니다. DOM 교체의 브라우저 Undo도 보장되지 않습니다.

## 7. 클라우드 Provider와 API는 선택 사항

로컬 Gemma를 쓰기 위해 클라우드 계정에 로그인할 필요는 없습니다.

- **CLI Deep:** Settings에서 Provider를 선택하고 준비 상태와 전송 동의를 확인합니다. 동의는 Provider별로 저장되며 재시작 후에도 남습니다. **Auto rewrite**를 끄면 캡처마다 자동 Deep이 시작되는 것을 막을 수 있습니다. **Revoke all Deep consent**로 동의를 철회할 수 있지만 이미 보낸 내용은 회수할 수 없습니다.
- **API:** Settings에서 OpenAI-compatible Chat Completions endpoint·model·API key와 권한을 설정하고 **Run API Deep**으로 요청합니다. API 키는 세션용이며 **Remove API key and permission**으로 제거할 수 있습니다. CLI의 Auto rewrite는 이 API를 자동 호출하지 않습니다.
- **한계:** 모든 서비스의 API 형식을 지원하는 것은 아닙니다. 실제 Provider별 호출·계정 수용 범위는 별도입니다. Antigravity는 개인정보 qualification 제한, Claude는 bare 모드 OAuth 제한이 기록돼 있습니다.

비용과 데이터 정책은 선택한 제공업체를 확인하세요. 민감한 문장으로 시험하지 마세요.

## 8. 자주 막히는 곳

| 증상 | 먼저 확인할 내용 |
| --- | --- |
| 스크립트 실행 또는 버전 오류 | PowerShell 7인지 확인. 검증된 다운로드의 차단 여부 확인. 보안 정책 변경으로 우회하지 않기 |
| 설치 경로가 이미 존재함 | 기존 폴더를 강제로 덮어쓰지 말고 새 경로 사용. 기존 설치는 그 설치본의 제거 스크립트로 관리 |
| 모델 파일이 없다고 나옴 | 설치 폴더에서 Prepare를 한 번 완료했는지 확인. 이후 Start 실행 |
| 모델 준비가 중단됨 / `local.pending` 존재 | 기존 파일을 보존하고 오류 문구로 문의. 반복 Prepare나 무작위 파일 삭제 금지. 새 시험 설치 또는 모델 없는 시작 검토 |
| 첫 응답이 오래 걸림 | 다운로드·모델 예열과 응답을 구분. GPU 요구 사항 확인. 여러 세션을 띄우지 않기 |
| 단축키가 반응하지 않음 | 앱 실행, 지원 컨트롤, 비어 있지 않은 선택, 단축키 충돌 확인 |
| Accept 거부 | 원문·커서·선택 변경이나 권한 만료 확인. 다시 선택해 검토. 지원되지 않는 앱에서는 강제 Apply 불가 |
| 확장이 host에 연결되지 않음 | 설치된 extension인지, 설치가 성공했는지 확인. 다른 버전의 확장·host를 섞지 않기 |
| 종료 후 다시 시작이 막힘 | 화면이 닫힌 뒤 WebView 정리는 최대 30초 기다립니다. launcher가 끝나기 전 재실행하지 마세요. 정리 오류가 나면 여러 번 실행하거나 무관한 프로세스를 종료하지 말고 보고 |

## 9. 종료·업데이트·제거

종료는 Pencil 트레이 메뉴를 사용합니다. 새 버전은 별도 경로에 설치해 먼저 시험하세요. 자동 업데이트·설정/모델 자동 이전을 보장하지 않습니다. 기존 `local` 모델 폴더와 설정을 보존하고, 다른 버전 파일을 임의로 섞지 마세요.

제거는 **제거할 설치본의 폴더**에서 실행합니다.

```powershell
.\Uninstall-Grammar.ps1
```

기본 제거는 사용자 데이터를 남깁니다. `-RemoveUserData`는 설정·용어사전·WebView 데이터·앱 소유 Codex 로그인 데이터를 삭제하며, **같은 Windows 사용자의 다른 설치본과 공유하는 데이터에도 영향을 줄 수 있습니다.** 보존 여부를 확인하고 모든 Pencil을 종료한 뒤 필요한 경우에만 사용하세요. 설치 폴더 내부의 다운로드 모델도 제거 대상이므로 재사용할 모델은 제거 전에 보존하세요. 외부 캐시나 임시 runtime 폴더까지 모두 제거된다고 가정하지 마세요.

## 10. 문제 신고하기

[버그 보고 양식](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md)에 릴리스 버전, Windows/GPU, 실행 모드, 편집기 종류, 재현 단계와 오류 문구를 적어 주세요. 직접 만든 비민감 예시만 사용하고 API 키·인증 코드·실제 문서·전체 프로필·로그 원본은 첨부하지 마세요.

설치 실패, 언어 품질, Apply 거부, 지연 중 무엇인지 구분하면 도움이 됩니다. 대기 시간을 보고할 때는 **최초 다운로드 / 예열 / 추천 / Accept** 중 어느 구간인지 적어 주세요.
