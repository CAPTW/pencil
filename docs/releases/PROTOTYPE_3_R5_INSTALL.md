# Pencil Prototype 3 r5 — 설치와 제한

Release Assets에서 `Pencil-0.1.0-prototype.3-r5-windows-x64.zip`과
`PROTOTYPE_3_R5_SHA256SUMS.txt`를 받으세요. Source code ZIP은 실행 파일이 아닙니다.
SHA-256을 확인하고 빈 폴더에 압축을 푼 뒤 START_HERE.md를 따르세요.
Windows x64, PowerShell 7, WebView2가 필요하며 서명되지 않은 Prototype입니다.
기존 Pencil을 Quit한 뒤 새 폴더에 설치하세요. 기존 설정·사전·보안 저장 키를
지우지 마세요. 제거는 기본적으로 사용자 데이터를 보존합니다.

## API 연결

Settings → Optional API setup에서 Provider를 고르고 키를 입력하세요.
공식 Provider의 키 입력을 마치고 다른 입력칸으로 이동하면 모델 목록을 한 번
조회합니다. Load model list로 수동 조회할 수도 있습니다. 키 입력 중 매 글자나
앱 재시작 때 자동 요청하지 않습니다. 목록 조회에는 키만 전송하며 문서는 보내지 않습니다.
Available models에서 선택하고 요청 동의를 확인한 뒤 Save API setup securely를 누르세요.
키는 Windows Credential Manager에 저장하며 재시작 후 Provider·모델을 복원합니다.
Revoke API consent는 동의만 철회하고, Remove API key and permission은 키도 삭제합니다.
목록에 보이는 모델도 과금·할당량·생성 권한·응답 형식 때문에 실제 호출은 실패할 수 있습니다.

이 코드의 선행 설치 후보에서 Claude `claude-haiku-5-5`의 짧은 합성 문장 번역·재작성과
Accept 후 원문 재조회, 번역 Undo를 확인했습니다. 모든 Provider/API가 검증된 것은 아닙니다.
Google·OpenRouter·xAI 실제 생성과 이 새 패키지의 실제 API 재호출은 NOT_RUN입니다.
새 패키지의 빌드·설치·CI 결과와 선행 실제 호출 결과는 VALIDATION 파일에서 구분합니다.

## 고정 Accept 버튼, 남은 시간 및 Copy-only

결과 창 아래의 Accept·Copy 버튼은 본문을 스크롤해도 표시됩니다.
Accept 남은 시간은 캡처부터 계산하며 초안 편집이나 결과 도착으로 갱신되지 않습니다.
만료되면 Accept가 비활성화되고 다시 캡처하거나 Copy하라는 안내가 표시됩니다.
화면의 시간은 보수적인 안내이며 실제 대상 검증은 적용 직전에 다시 수행됩니다.
선행 b1a3831 설치본에서 로컬 Instant Accept·Undo·만료 후 Copy와 원문 보존을
확인했습니다. 새 r5 패키지의 실제 API/Gemma GUI 재시험은 NOT_RUN입니다.


- 캡처 권한은 **캡처 시점부터 최대 30초**입니다. 모델 응답 대기와 검토 시간도 포함됩니다.
  시간이 지났거나 원문·선택·대상이 바뀌면 Accept를 거부합니다. 권한을 자동 연장하거나
  자동 재시도하지 않습니다. 다시 선택·캡처해 결과를 검토하거나 Copy를 직접 선택하세요.
- 직접 교체를 지원하지 않는 입력칸은 안전하게 캡처 가능한 경우에만 **Copy-only**로
  사용합니다. Copy 후 사용자가 직접 붙여넣습니다. Accept가 거부돼도 자동 복사하지 않습니다.
- 특히 자동 가로 스크롤이 없는 Windows 한 줄 Edit 입력칸은 긴 결과가 잘릴 수 있어
  직접 Accept를 차단합니다. 버튼이 표시되더라도 거부될 수 있으므로 Copy를 사용하세요.
- 비밀번호·보호·민감 필드 또는 안전한 캡처가 불가능한 입력칸은 읽지 않습니다.
  Copy-only가 이러한 필드를 읽기 위한 우회 기능은 아닙니다.
- 결과가 불명확하다는 안내가 나오면 원문부터 확인하세요. 자동 복원·재적용은 하지 않습니다.

Gemma 모델·런타임은 포함되지 않습니다. 명시적 준비 단계에서 별도 다운로드합니다.
GPU가 맞지 않으면 `Start-Grammar.ps1 -WithoutLocalModel`로 기본 규칙과 선택적 API를
사용할 수 있지만 Gemma와 같은 품질·속도를 보장하지 않습니다.
추천 1초·Accept 0.5초·단락 3초는 목표이며 보장값이 아닙니다.
일반 Office/Hancom 입력칸, 물리 IME, Edge, 다른 GPU는 일반 지원으로 수용되지 않았습니다.
Writer/rHWP는 별도 adapter 배포가 필요합니다. 자동 업데이트·서명은 제공하지 않습니다.
