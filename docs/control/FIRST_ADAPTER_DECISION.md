# First Editor Adapter Decision

FIRST_ADAPTER_DECISION=CHROMIUM_TEXTAREA_CONTENTEDITABLE_MV3

선택은 정확히 하나의 Chromium MV3 browser adapter family다. 첫 live acceptance는 opt-in한 plain `textarea`, 다음 동일 adapter의 bounded milestone은 검증된 simple `contenteditable`이다. complex rich editor, closed shadow DOM, cross-origin frames, privileged browser pages는 초기 지원에서 제외한다. 이 결정은 구현 완료·설치 허가·제품 acceptance가 아니다.

## 네 후보 비교

|후보|현재 코드와 재사용|event/annotation/Apply 판단|배포·사용자 가치·결정|
|---|---|---|---|
|Chromium textarea/contenteditable|TypeScript parser/state/diff seams (`src/instantSelectionRuntime.ts:46`, `src/resultReview.ts:30`), Rust local engine는 재사용 후보; extension은 없다|DOM input event와 range mapping을 synthetic page에서 재현 가능. textarea 내부 substring underline은 가정하지 않고 geometry mirror overlay/equivalent annotation; simple editable은 range highlight/overlay. exact source/range reread가 가능하나 controlled-editor undo는 별도 검증|MV3 extension + native host 추가 배포 비용이 있지만 browser writing field 가치와 좁은 권한·재현성이 가장 명확. **선택**|
|Microsoft Word|현재 Word/Office add-in 코드 없음; shared core 후보만 있음|공식 Word API는 paragraph/annotation events를 제공한다. 요구 API set·Word 버전에서 해당 event와 annotation 지원 확인 필요; 단순히 기능 없다고 기각하지 않음|Office add-in 배포/requirement-set/host integration 추가; 현재 설치·license·live harness 미확인. rich document 가치는 높으나 첫 adapter에서는 보류|
|Win32/UI Automation text control|기존 Windows target/clipboard/SendInput 경계를 재사용 가능하나 document event adapter는 없음|TextPattern read-only API와 text changed notification은 존재; 정확한 changed range·annotation 및 안전 mutation은 provider별 추가 작업 필요|범용 앱 가치 크지만 UIA provider 편차와 focus/input fallback 비용; existing capture가 safe document adapter인 것은 아님. 보류|
|HWP/Hancom|현재 HWP adapter 없음|공식 Hancom automation 문서/보안모듈 안내는 존재. 설치 버전별 document event, anchored annotation, reversible bounded range mutation은 미확인|한국어 문서 가치 높음. 별도 API/배포·security feasibility 필요; 웹한글 API를 Windows HWP 증거로 쓰지 않음. Phase 10 read-only feasibility에 보류|

비교는 primary documentation와 current code에 대한 engineering inference다. 네 후보의 설치/live performance를 실측 비교했다는 뜻이 아니다.

## 선택 adapter 계약

|필수 필드|동결한 방향|
|---|---|
|Integration form|MV3 extension: isolated content script → service worker → allowlisted native messaging host → existing local core. 별도 extension namespace/host protocol, product UI에 page가 privileged invoke하지 못함|
|최소 권한|`activeTab`, `scripting`, `nativeMessaging`; per-user native host 등록. `<all_urls>`, screenshot, clipboard, history, broad tabs permission 불필요. 저장이 필요하면 non-content opt-in 설정만 별도 검토. persistent optional host permission은 초기 범위 밖|
|Opt-in lifecycle|사용자 browser action 뒤 current tab/document에 명시 enable, visible indicator. same-origin navigation도 새 document epoch/permission decision, cross-origin navigation/close/worker disconnect는 detach. 앱/도메인 denylist가 우선|
|Changed range|`beforeinput`/`input`/composition events + bounded previous/current text diff; event target ranges는 hint일 뿐 authority가 아님. programmatic mutation 누락은 bounded reconciliation에서 revision reset. whole-page polling/keypress logging 금지|
|Annotation|textarea는 cloned geometry 기반 non-editing overlay/equivalent annotation; scrolling/zoom/font/IME에서 검증. simple contenteditable은 CSS Custom Highlight/Range 또는 non-invasive overlay를 capability probe. editor DOM wrapping과 자동 formatting 변경 금지|
|Card|revision-keyed memory cache만 읽기. hover/click network/inference 0, keyboard로 같은 actions 접근. anchor 없어지면 표시 제거|
|Apply/rebase|explicit Accept/Edit 후 identity/navigation epoch/revision/source slice/current caret/composition 검증. non-overlapping bounded edits만 deterministic rebase; ambiguous overlap는 reject. editor text reread로 mutation 확인, undo/selection 유지, no automatic retry|
|Native test|isolated Windows browser profile + synthetic local test origins, textarea/simple editable/controlled unsupported case, IME/undo/zoom/scroll/navigation/frame detach. event receipt·content reread·network counter·owned host PID를 기록. 사용자 clipboard/문서·화면 캡처 수집 금지|
|Trust boundary|Web page text/events는 불신. service worker validates sender origin/tab/frame; native host accepts only exact extension origins and bounded typed operations. no arbitrary exec, no page credential access, no raw text diagnostics|
|Fallback|unsupported ordinary app/field는 monitoring off 후 명시 P3-A 재선택 또는 Copy-only. protected/sensitive/denylisted field는 처리 금지; fallback으로 우회 금지|
|Implementation blockers|P3-A safety baseline; neutral identity/revision/offset rules; native host authentication/ownership; IME/undo mutation receipt; bounded cache/scheduler; early privacy controls; browser installed-version capability acceptance|

`activeTab`의 플랫폼 허가는 같은 origin navigation에서 유지될 수 있으므로 앱은 자체 document epoch opt-in을 추가한다. browser API capability는 제품 permission을 대체하지 않는다. Chrome native messaging가 host process를 시작한다는 사실만으로 cleanup나 safety를 입증하지 않는다.

## 확인한 primary sources (retrieved 2026-09-18)

- [Chrome activeTab](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab): 사용자 gesture 기반 임시 접근 및 scripting 연동, origin navigation 제한.
- [Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging): host manifest/allowed_origins, Windows 등록, content-script→service-worker 경유, sender validation 필요.
- [W3C Input Events](https://w3c.github.io/input-events/): beforeinput/input 및 target range 계약. 모든 editor에서 신뢰 가능한 delta를 준다는 주장은 하지 않는다.
- [CSS Custom Highlight specification](https://drafts.csswg.org/css-highlight-api-1/): Range 기반 highlight model. textarea native text에 동일 적용을 가정하지 않는다.
- [Microsoft Word events](https://learn.microsoft.com/en-us/office/dev/add-ins/word/word-add-ins-events): paragraph changes와 annotation event 목록. 실제 host 지원은 후속 capability test 사항.
- [Microsoft UI Automation TextPattern](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/ui-automation-textpattern-overview): read-only text provider interface와 text-change notification.
- [Hancom automation guide](https://developer.hancom.com/hwpautomation): 공식 automation 매뉴얼과 파일 접근 보안 승인 안내. 별도 HWP feasibility 필요성의 근거이며 live 지원 판정은 아님.

공식 자료가 바뀌거나 구현 Phase에 진입하면 primary source와 installed-version capability를 다시 확인한다. 라이브러리 채택이나 API 안정성·배포 허가를 지금 승인하지 않는다.
