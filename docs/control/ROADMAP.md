> Active mission: see MULTI_AGENT_CONTRACT.md v1.1.0 override and control/state.json. Historical no-execution/one-step approval restrictions below are superseded only inside this mission; acceptance claims are not promoted.

# Phase-Step Roadmap — historical phases, active mission continuation

EXACT_NEXT_TASK=GRAMMAR-P3B-P1-SHARED-DEEP-RUNTIME-AND-CONSENT-BOUND-HOST

아래 Phase 표는 R0 당시 계획 기록이다. 현재 후보의 실행 결과·미완료 범위는 HANDOFF.md와 control/state.json을 따른다.

current audit의 확인된 결함만 repair 범위로 채택한다. 아래는 미래 candidate 계획이며 구현 완료 또는 실행 권한이 아니다. exact next task는 **GRAMMAR-P3A-R1-CURRENT-HEAD-SAFETY-STABILIZATION-CONFIRMED-CANCEL-PROCESS-DRAFT-AND-APPLY-BOUNDARY-DEFECTS-BOUNDED-REPAIR** 하나다. 이 task를 여기서 실행하지 않는다. Phase 1 safety baseline 전에 P3-B monitoring을 시작하지 않는다.

FIRST_ADAPTER_DECISION=CHROMIUM_TEXTAREA_CONTENTEDITABLE_MV3

## 모든 Step에 적용되는 필수 execution fields

아래 공통 필드는 각 표의 모든 Step에 포함되는 계약이다. 행별 항목이 추가 제한한다.

- **Forbidden scope:** 허가된 path claim 밖 편집, dependency/protocol 확장, 타 agent 수정 되돌리기, 실제 사용자 문서/clipboard 수집, 계정 변경, remote publication, 다음 Step 실행 금지. Phase 0에서는 모든 product source/config/dependency 변경 금지. 이후 product 변경은 별도 Owner task packet의 exact allowlist에 한해서만 가능하다.
- **Completion/failure token:** 각 ID `PxxSyy`에 대해 정확히 `PASS_GRAMMAR_<ID>` / `BLOCKED_GRAMMAR_<ID>`를 쓴다. 예: P01S01은 `PASS_GRAMMAR_P01S01` / `BLOCKED_GRAMMAR_P01S01`. PASS는 그 행의 unit/integration/native-live/privacy 조건 모두 충족했을 때만 가능하다. NOT_RUN/BLOCKED live를 PASS로 승격하지 않는다.
- **Rollback/containment:** candidate 격리 보존; 실패한 기능은 disabled/Copy-only/fail-closed, no automatic retry. canonical rollback/reset 금지. owned task/host/process만 종료하고 raw text cache 폐기. 실패 evidence와 최소 successor 범위 기록, 넓은 repair로 확장 금지.
- **Project Source refresh:** 각 Step 종료 시 materiality 판정 필수. current behavior/authority/acceptance/next-gate가 바뀌면 canonical main 채택 뒤 exact source refresh 필수. pure fixture/doc wording 변경은 no-refresh 사유를 result packet에 남긴다. Phase 0/9 source package Step은 refresh 필수.
- **Next gate:** 표의 다음 Step은 후보 Gate이며 새 Owner authorization 없이는 실행하지 않는다. 열의 `Owner →`는 독립 review와 authorization을 뜻한다.
- **Automatic continuation:** 모든 Phase/Step `automatic_continuation=false`, `next_gate_executed=false`.
- **Commit boundary:** 각 Step은 응집된 candidate commit 하나 또는 작은 stack. entry identity, diff/path check, independent verifier, acceptance evidence를 반환한다. 서로 겹치는 핵심 state/App/main path는 동시 소유하지 않는다.
- **Evidence:** unit/fixture/native/live/privacy를 분리하고 native mutation은 document reread로 확인한다. 실행 시작/SendInput count는 product success가 아니다.

## PHASE 0 — Current Authority Reset

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P00S01|현재 main 감사, F-01~06/D-020/readiness 재판정. 선행: clean identity와 격리 evidence|제품 delta 0; docs/control audit·charter·architecture·roadmap|U: actual module regression; I: locked build/test; N: current native smoke와 blocked 구분; P: synthetic-only, product diff 0|Owner → P00S02|
|P00S02|tool-neutral 계약·exact 12 Source successor. 선행 P00S01 판정|docs/control, root wrappers, scripts/control validators, external package|U: schema negative tests; I: wrapper/source digest·fresh extraction; N: FF 안전 조건과 canonical identity; P: raw text/credential 없음|Owner → exact next task (P01S01부터)|

## PHASE 1 — P3-A Current-Head Safety and Acceptance Baseline

가장 높은 레버리지의 첫 한 수는 **P01S01**: confirmed cancellation/process ownership을 실제 production-import synthetic child test로 재현하여 시간 한계·잔류 PID 계약을 먼저 고정한다. 아래 P01S01~03은 exact next task 내부의 순차 bounded steps이며 새 task를 세 개 자동 생성하지 않는다. 확인되지 않은 가설은 repair하지 않는다.

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P01S01|confirmed Cancel/process 결함 최소 수리. 선행 P00 결과·새 exact-base task packet|cancel authority를 long-held lock 밖에 두고 owned child/job·drain deadline 일원화; src-tauri/src/provider/**, process_job.rs, active_turn.rs, main.rs; 해당 test|U: cancel/timeout/assignment-failure; I: production process path 느린 synthetic child·pipe-holding descendant·byte cap; N: Windows owned residual PID 0·bounded cancel; P: raw stderr/text 노출 0|Owner → P01S02|
|P01S02|confirmed edited-draft/Apply boundary 수리. 선행 P01S01 review|capture/intention/terminology/draft binding·current selection exact revalidation; src/App.tsx, instantSelectionRuntime.ts, src-tauri/src/main.rs, capture_session.rs, windows_apply.rs, apply_safety.rs|U: Instant edit·late result·stale token; I: 실제 module import + IPC state; N: synthetic editor same HWND changed selection/no selection에서 paste 0·정상 mutation reread; P: clipboard restore/ownership·secure field no read|Owner → P01S03|
|P01S03|P3-A baseline 및 provenance 결함의 별도 bounded disposition. 선행 P01S02|acceptance matrix·관련 native scripts; F03 repair 필요시 packaging tooling 범위를 새로 claim; P3-B 금지|U: counterexample regressions; I: full related build/test; N: supported selection/copy/apply/provider typed-status matrix, binary hash; P: synthetic-only·residual 0·artifact source measured|Owner → P02S01|

## PHASE 2 — Adapter-Neutral Document Core

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P02S01|identity/revision/offset/event/cache contract design freeze. 선행 P01 baseline|docs/control/document-core-design.md (planned); capability envelope, unicode/IME/rebase algebra; product 구현 금지|U: table counterexamples; I: capture-vs-document authority review; N: installed Chromium capability read-only check; P: sensitivity/opt-in/TTL policy freeze|Owner → P02S02|
|P02S02|bounded core 구현, 외부 monitoring 없음. 선행 P02S01|src-tauri/src/document_core/** (planned), neutral tests; epochs, delta/rebase/cache limits|U: unicode/overlap/reorder/overflow property tests; I: synthetic event producer→core invalidation; N: Windows core harness; P: memory bound·wipe·no persistent history/network|Owner → P03S01|

## PHASE 3 — First Editor Adapter

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P03S01|permission/trust/disable shell, text intake 전 gate. 선행 P02S02|adapters/chromium/**, src-tauri/src/adapter_host/** (planned); MV3 native bridge, opt-in/denylist/visible state/pause/emergency controls|U: sender/nonce/size/secure-field rejection; I: forged origin/disconnect/restart; N: synthetic Windows tab enable→disable, owned host cleanup; P: disable 상태 text read·network 0, sensitive before-read deny|Owner → P03S02|
|P03S02|plain textarea read-only delta adapter. 선행 P03S01 privacy PASS|same planned adapter/core paths; document epoch, bounded diff/composition; mutation/annotation/cloud 금지|U: input/paste/undo/programmatic change deltas; I: event gap→reset; N: synthetic textarea+navigation+IME; P: no full-page scan, deny/close wipe, content-free diagnostics|Owner → P03S03|
|P03S03|simple contenteditable read-only 범위 확장. 선행 P03S02|same adapter namespace; DOM text mapping/capability probe; complex rich editor 금지|U: split/merge text nodes; I: detached range/unsupported rich state; N: simple editable + unsupported matrix; P: cross-origin/closed-shadow reject·no hidden monitoring|Owner → P04S01|

## PHASE 4 — Annotation and Cached Suggestion UI

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P04S01|read-only annotation geometry. 선행 P03S03|adapters/chromium/annotation/** (planned); textarea mirror/equivalent, editable range highlight|U: offset→rect mapping; I: revision invalidation removes stale marks; N: DPI/zoom/scroll/font/viewport tests; P: no editor text/format mutation or screenshots|Owner → P04S02|
|P04S02|cached card·accessibility. 선행 P04S01|adapters/chromium/card/** (planned); keyboard/hover focus and cached actions, Apply disabled until P07|U: cache miss/hit both inference 0; I: card→cache only, network spy 0; N: keyboard/IME/focus/document switch; P: page injection sanitization·hover network 0|Owner → P05S01|

## PHASE 5 — Incremental Local Instant Suggestion Pipeline

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P05S01|changed-range local scheduler. 선행 P04S02|document_core scheduler + existing Instant engine seams; bounded debounce/backpressure|U: rapid edits/cancel/dedup; I: actual Instant core import + changed ranges; N: synthetic typing p95 CPU/memory budget measured and frozen before PASS; P: network 0·memory-only snapshot|Owner → P05S02|
|P05S02|suggestion lifecycle·false-positive suppression. 선행 P05S01|document_core suggestions + adapter annotation; Ignore/Dismiss lifetime, no persisted text|U: stale/repeated suggestion suppression; I: event→Instant→cache→mark; N: multilingual synthetic corpus and focus coexistence; P: close/reset wipe·no full-history logs|Owner → P06S01|

## PHASE 6 — Explicit Deep Provider Enrichment

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P06S01|per-app/document cloud permission + disclosure. 선행 P05S02|document_core consent, src/promptlessContract.ts, settings.rs, adapter card (planned schema change separately authorized)|U: selected provider/consent revocation; I: missing consent sends 0; N: synthetic per-document enable/disable; P: matched approved terminology only, no other-provider replay|Owner → P06S02|
|P06S02|bounded explicit Deep request integration. 선행 P06S01 + P01 process safety|provider manager + document request binding; P3-A separate|U: timeout/cancel/stale result; I: actual process/provider seams with fixture labels; N: existing-account synthetic selected-provider live or typed unavailable, residual PID 0; P: no background transmission/credentials/raw stderr|Owner → P07S01|

## PHASE 7 — Apply, Rebase and User-Edit Protection

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P07S01|explicit Accept/Edit mutation gate. 선행 P06S02|document_core mutation authorization + textarea adapter mutation receipt|U: identity/revision/slice/overlap rejects; I: concurrent user edit before commit; N: synthetic textarea mutation reread + undo/selection preservation, wrong target mutation 0; P: no clipboard read/default cloud|Owner → P07S02|
|P07S02|simple editable safe rebase + fallback. 선행 P07S01|same adapter + P3-A fallback UI; unknown editing model Copy-only|U: non-overlap rebase vs ambiguity; I: Accept/Edit/Dismiss/Ignore/Copy + cache invalidation; N: IME/rich editor unsupported/navigation during Apply; P: protected fields no fallback bypass·automatic retry 0|Owner → P08S01|

## PHASE 8 — Privacy, Threat and Emergency Controls

여기서는 Phase 3의 필수 통제를 강화한다. Phase 8까지 opt-in/denylist/pause/visible-state를 미루는 것은 금지한다.

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P08S01|adversarial privacy and abuse tests. 선행 P07S02|adapters/core negative tests + docs/control threat model; 새 adapter 금지|U: type→password·forged frame/session/flood; I: disable while queued/Deep/Apply; N: synthetic adversarial Windows pages; P: text read/transmit after deny 0·cache empty|Owner → P08S02|
|P08S02|emergency teardown/endurance hardening. 선행 P08S01|adapter/core/host lifecycle fixes only; settings migration scoped separately|U: idempotent teardown/reconnect deny; I: crash/logoff/navigation/revision reset; N: bounded soak and owned residual 0; P: filesystem/log/cache privacy audit, no plaintext credentials/history|Owner → P09S01|

## PHASE 9 — Current Product Acceptance and Personal Bundle

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P09S01|current exact-head acceptance matrix. 선행 P08S02|acceptance scripts + docs/control; product fixes는 separate bounded successor|U: all actual module regressions; I: clean-folder locked build; N: adapter + P3-A fallback + providers live/typed unavailable matrix, mutation receipts; P: selected-provider policy·residual 0|Owner → P09S02|
|P09S02|measured personal bundle/source refresh. 선행 P09S01|packaging verification/tooling, external bundle + exact Project Sources|U: forged provenance rejected; I: commit/tree/dirty measured build receipt + hash + fresh extraction; N: clean-folder Windows launch/use, no public release; P: user data/credentials excluded|Owner → P10S01 only with separate expansion authorization|

## PHASE 10 — Adapter Expansion and HWP Feasibility

|Step|목표 / prerequisites|Architecture delta / expected paths|Unit / integration / native-live / privacy checks|Next gate|
|---|---|---|---|---|
|P10S01|second adapter comparison, shared-core reuse 측정. 선행 P09S02 + Owner expansion authorization|docs/control/adapter-expansion-feasibility.md (planned); implementation 없음|U: capability gap cases; I: core reuse delta measured by paths/contracts; N: authorized synthetic read-only capability probes; P: installed-app consent/least permissions|Owner → P10S02|
|P10S02|HWP read-only feasibility와 go/no-go. 선행 P10S01 + 별도 Owner HWP probe authorization|docs/control/hwp-feasibility.md + evidence only; HWP adapter 구현 금지|U: proposed contract counterexamples; I: official API/event/version comparison; N: authorized synthetic HWP document read-only probe or BLOCKED (support not assumed); P: no security-module bypass/real documents, minimal permission|Owner decision required; no implementation task auto-created|

## Phase exit과 support 약속

P00는 governance reset의 완료 여부만 판정한다. P01 baseline은 P3-B 완료가 아니다. P03 adapter 읽기, P04 annotation, P05 local suggestion, P06 Deep, P07 mutation은 서로 다른 acceptance다. P09 이전에는 whole-product current acceptance를 선언하지 않는다. P10은 feasibility 완료이며 HWP 지원 완료가 아니다. 비용·browser/API 버전 변경으로 계획이 달라지면 next task packet에서 bounded revision을 승인받고 Source materiality를 갱신한다.
