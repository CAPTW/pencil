# Architecture, Privacy and Threat Model

이 문서는 현재 구조와 미래 설계를 구분한다. current identity와 acceptance를 복제하지 않으며 current audit 및 canonical state를 소비한다. 미래 경로명은 계획으로서 아직 존재하거나 구현됐다고 주장하지 않는다.

## 현재 구조 — CODE_PROVEN

`src/main.tsx:6` React entry → `src/App.tsx:279` UI state → Tauri invoke/event → `src-tauri/src/main.rs:135` capture/Instant/provider state → Windows target/clipboard/input boundary다. `src/App.tsx:430` Deep, `:490` Cancel, `:499` Copy, `:580` Instant result, `:606` capture, `:1286` Apply, `:1399` dismiss가 주요 연결점이다. `src/promptlessContract.ts:20` settings schema와 `src/terminologyContract.ts:63` terminology revision은 document revision이 아니다. `src/instantSelectionRuntime.ts:46` stale generation 및 dirty draft backstop, `src/resultReview.ts:30` bounded diff를 재사용 후보로 둔다.

P3-A generation/intent/cache와 future document/rebase를 동일 객체로 가장하지 않는다. F-01~F-06 및 D-020의 current verdict가 safety baseline이며 기존 historical acceptance를 현재 전체제품 PASS로 승계하지 않는다.

## 미래 구조 — DESIGN_FROZEN, NOT_IMPLEMENTED

`opt-in editor adapter → validated capability/event boundary → document identity/revision + changed ranges → bounded local scheduler → local Instant → memory suggestion cache → annotation/cached card → explicit mutation gate → adapter mutation receipt`.

Deep는 이 파이프라인의 명시 요청 분기다. per-app AND per-document cloud permission, selected provider, approved matched terminology subset, bounded process lifetime, cancellation을 검증한다. hover는 이 분기를 호출할 권한이 없다. cache hit/miss 모두 hover inference/network 0이다.

문서 identity는 adapter instance/browser tab + frame + navigation epoch + editable element identity 조합을 세션 내에서 구분한다. snapshot은 bounded memory-only, revision은 monotonic, unicode offset 규약과 composition epoch를 명시한다. 이벤트 누락/역전/overflow/worker restart는 reset하고 모든 이전 suggestion을 무효화한다. identity나 정확한 source slice를 재검증하지 못하면 mutation은 0이다.

## Trust boundary와 threat table

|경계/위협|필수 통제와 검증|실패시 containment|
|---|---|---|
|Web page→content script: forged event, XSS, DOM mutation|페이지는 불신 입력; text-only parsing, length limits, origin/frame/document token 검사; page instruction 실행 금지|detach 및 cache wipe; P3-A 재선택|
|Content script→extension service worker→native host|sender origin/tab/frame와 current opt-in 확인, versioned typed messages, nonce/session/revision, payload bounds, allowlisted extension origin; 임의 shell/path 명령 금지|port close, owned process cleanup, no document mutation|
|Native host→app|same-user local IPC, request allowlist, one owner/session, backpressure/deadline; localhost network bridge 기본 채택 금지|monitor off, explicit reconnect|
|Core→Provider|explicit cloud consent, selected provider only, approved matched terms only; no credentials in payload/logs; raw stderr redaction|typed error, no other-provider retry|
|Cache/log/artifact|TTL/bytes/count bound, no raw text persistent log, close/logout/detach/reset wipe|memory clear; content-free diagnostics|
|Suggestion→editor|exact identity/revision/source slice, explicit action, no ambiguous rebase; mutation receipt from reread not input-event count|stale rejection, Copy-only; no automatic retry|
|Sensitive/secure/denylisted fields|deny before reading or scheduling; unknown sensitivity => unsupported; test DOM type/attribute transition before read|no text collection; no P3-A bypass|
|UI deception/focus stealing|visible monitoring indicator, keyboard pause/emergency stop, accessible cached card, composition-aware focus preservation|stop listeners/tasks, erase annotations/cache|

## Privacy acceptance sequencing

Phase 2 freezes the rules; Phase 3 implements minimum opt-in, denylist, sensitive detection, visible state, pause/emergency disable and cache wipe BEFORE any monitoring. Phase 8 hardens and adversarially verifies existing controls; it does not defer their introduction. Disable must prevent queued work, annotation and Deep, close owned ports/processes, and never restart silently. No screenshots/OCR/recording/keylogging are test shortcuts. Tests use synthetic documents only.

P3-A Copy/Apply uses explicit clipboard paths; future browser adapter has no clipboard permission requirement. Protected fields remain unavailable even if P3-A works elsewhere. In unsupported ordinary editors P3-A capture is a new explicit user gesture, never conversion of a full monitored document into a cloud request.

## Unresolved design gates

Extension/native-host installation and controlled-editor mutation/undo semantics require separate live prototypes. Isolated worlds do not make DOM trustworthy. Browser version, accessibility, IME, native-host disconnect, allowlist enforcement, teardown race and mutation receipts must be measured. No new adapter, IPC schema or monitoring permission is implemented by this task.
