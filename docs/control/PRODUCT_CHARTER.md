> Active mission: see MULTI_AGENT_CONTRACT.md v1.1.0 override and control/state.json. Historical no-execution/one-step approval restrictions below are superseded only inside this mission; acceptance claims are not promoted.

# Product Charter — P3-B target, P3-A preserved

CURRENT_PRODUCT_TARGET=P3A_PRESERVED_PLUS_P3B_PROACTIVE_INLINE_ASSIST

이 문서는 제품 목표에 대한 규범적 결정이다. 현재 구현·검증·Git identity·단일 next task는 canonical current-state와 current audit가 결정한다. 목표 동결은 구현 완료나 whole-product acceptance가 아니다.

## 정체성과 경계

Grammar는 개인용 Windows promptless writing assistant다. 자유형 AI prompt 입력창을 핵심 UI로 만들지 않고 고정 writing intent, language, format, terminology profile, Provider를 선택한다. 다른 제품의 proprietary code, model, asset, wording, visual trade dress는 복제하지 않는다.

영구 보존하는 P3-A 흐름은 `explicit selection → shortcut → local Instant → optional selected-Provider Deep → review/edit → explicit Apply or Copy`다. 지원하지 않는 앱, adapter 실패, monitoring disable에서는 사용자 재선택에 의한 P3-A 또는 Copy-only를 제공한다. 보호·민감 필드는 P3-A에도 우회 허용하지 않으며 텍스트를 처리하지 않는다.

## 동결한 P3-B 목표

1. 사용자가 opt-in한 앱/도메인과 현재 문서에서만 adapter text/document event를 받는다. 보이지 않는 monitoring은 금지한다.
2. ephemeral document identity, monotonic revision, changed range, bounded incremental snapshot을 유지한다. typing 후 bounded debounce로 변경 범위만 local Instant 분석한다.
3. inline underline 또는 동등 annotation과 anchor를 제공한다. hover/click은 이미 계산한 memory cache만 표시하며 hover inference/network count는 반드시 0이다.
4. suggestion card는 Accept, Edit, Dismiss, Ignore, Copy와 keyboard/accessibility 동작을 제공한다. 실제 문서 변경은 explicit user action 뒤 exact identity/revision/range 검증을 통과한 경우에만 수행한다.
5. 사용자 편집과 composition을 보존한다. 범위 이동은 증명 가능한 경우에만 rebase하고 모호하면 stale invalidation, Copy-only 또는 P3-A로 돌아간다. 자동 replacement는 없다.
6. Deep는 local proactive와 별도다. per-app/per-document cloud opt-in과 disclosure 뒤 명시적으로 선택한 Provider 하나만 호출한다. 한 요청은 정확히 한 Provider이며 실패 시 silent replay/fallback은 없다. matched approved terminology만 전송한다.
7. bounded memory-only document/suggestion cache를 app/document close, adapter detach, logout, revision reset에서 지운다. pause, emergency disable, app/domain denylist와 visible state는 첫 monitoring 전에 필수다.

## 현재와 목표 구분

`src/captureContract.ts:1`은 capture session/generation이며 document identity가 아니다. `src/instantSelectionRuntime.ts:10`의 draftRevision은 UI draft 편집 번호다. `src/App.tsx:578`은 selection-scoped Instant event listener이며 document monitoring이 아니다. `src/App.tsx:657`은 기존 autoRewrite/ack 조건에서 capture 후 Deep를 자동 요청할 수 있다. 이것을 새 P3-B per-document cloud 허가로 확대 해석하지 않는다. `src/App.tsx:1434`의 Instant user-edit Apply 공백 등은 current audit에 따라 먼저 안정화한다.

FIRST_ADAPTER_DECISION=CHROMIUM_TEXTAREA_CONTENTEDITABLE_MV3

결정 근거와 제한은 FIRST_ADAPTER_DECISION.md에 있다. HWP 지원은 약속하지 않는다. adapter는 하나씩 별도 수용하며 첫 adapter 구현 전 P3-A 안전 Gate를 통과해야 한다.

## 불변조건

screenshot, OCR, screen recording, keylogging, password/secure/sensitive field 처리, full-document history 영구 저장, selected/generated text history 영구 저장, plaintext credential 저장, raw Provider stderr 사용자 노출, 전체 terminology store 전송은 금지한다. 명시적 사용자 관리 terminology 저장은 텍스트 history 저장과 구분한다. local-only Instant는 cloud enable과 독립적으로 사용 가능해야 한다.

이번 authority reset은 docs/governance/tooling만 허용한다. P3-B 기능 구현, 다음 개발 task 실행, remote publication은 허용하지 않는다. NEXT_GATE_EXECUTED=false; AUTOMATIC_CONTINUATION=false.
