# P3-01 Instant Selection Engine architecture, privacy, and benchmark baseline

## 1. Authority and scope

This architecture-only Gate is based on `main@fb988fe810bbb09804a59e40161e6d1aef1bca8b` (`feat: polish translation workflow and widget behavior`). It defines contracts and deterministic non-production benchmark tooling. It does not implement, install, execute, or claim qualification of an Instant Analyzer.

The sole independently accepted product authority remains `PASS_GRAMMAR_P1_03_PERSONAL_MVP_ACCEPTANCE`. Product launch, clipboard access, AI inference, network access, packaging, signing, publication, and Project Source refresh are outside this Gate.

## 2. Current implementation authority versus product acceptance authority

`fb988fe810bbb09804a59e40161e6d1aef1bca8b` is the current implementation authority. It is not a new product-acceptance authority. The three commits after the P2-01 workflow baseline are present in source, but their product acceptance is `NOT_INFERRED`.

The architecture may rely on fresh source inspection and green regression results, but those facts do not replace the accepted P1-03 authority or close historical native-smoke work.

## 3. Historical and current P2-02 status distinction

The historical owner disposition remains `DEFERRED_NOT_ACCEPTED_NOT_COMMITTED`. The live repository now contains P2-02-derived implementation bytes committed after that closure, so current implementation presence is `COMMITTED_POST_CLOSURE`. Current P2-02 acceptance remains `NOT_INDEPENDENTLY_ACCEPTED`.

P3-01 records both facts without rewriting history, accepting P2-02, or treating the P2-02 archive as current source.

## 4. Current selection-to-Apply architecture map

| Path and symbol | Role | Input owner | Output owner | Sensitive exposure | Persistence | Network | Concurrency and stale defense | P3 disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `src-tauri/src/main.rs::global_shortcut_plugin` | Routes one configured shortcut press to capture, or hides a visible Widget | shortcut gate | Tauri async task | no text until capture | none | none | press/repeat/release gate | preserve unchanged |
| `src-tauri/src/main.rs::capture_from_hotkey` | Captures foreground target before Widget focus, then selected text | Windows target and clipboard boundary | `CaptureSessionStore` and `selection-captured` event | exact selected text, target HWND/PID | active memory only | none | recapture cancels prior active turn; generation advances | reuse unchanged |
| `src-tauri/src/capture_session.rs::CaptureSessionStore` | Owns session ID, generation, selected source, target, lifecycle, bound intent, active turn | backend capture | backend analysis and Apply gates | selected text and target identity | process memory only | none | exact token, lifecycle, intent and terminology binding | extend orchestration around it; do not redesign capture token or target binding |
| `src/captureContract.ts` | Strictly parses capture and Apply IPC contracts | backend event/command | frontend refs and UI | selected text only in event payload | none | none | exact keys and safe integer generation | preserve token contract |
| `src/App.tsx::rewrite` | Builds current frontend intent and invokes Deep rewrite | current token/settings refs | result, draft and status state | token, mode and target selection | React memory | Deep command invokes cloud path | compares capture intent and terminology epoch before promotion | split future analysis orchestration at this seam |
| `src-tauri/src/main.rs::rewrite_selected_text` | Revalidates source/settings, matches terminology, binds intent, drives Codex, validates completion | backend session/configuration/store | `RewriteResult` and Ready lifecycle | exact selected text plus matched approved constraints | active memory only | through Codex App Server | token, bound intent, store revision, active turn, line structure | preserve as Deep Analyzer |
| `src-tauri/src/terminology_matcher.rs::match_terminology` | Selects deterministic approved constraints for exact selected source | local store and source | bounded matched set | selected text plus eligible approved entries | none | none | revision is later bound; deterministic overlap handling | reuse matcher boundary; convert needed spans explicitly for Instant contract |
| `src-tauri/src/codex_client.rs::CodexClient` | Maintains hardened stdio App Server and ephemeral Deep requests | bounded prompt | strict structured result | selected source and matched approved constraints | history disabled; ephemeral thread | cloud inference behind stdio client | active-turn identity and strict schema | preserve unchanged |
| `src/App.tsx` result/draft refs | Holds visible result and editable draft | accepted backend result or user edit | review surface and Apply command | replacement and edit details | React memory | none | current intent comparison; no explicit draft revision yet | add candidate/draft revisions in P3-02, never silent overwrite |
| `src-tauri/src/main.rs::apply_replacement` | Revalidates ready bound intent, settings and terminology revision | reviewed frontend draft | target-bound Apply outcome | final replacement and captured source for translation formatting | none | none | exact session/generation/intent/revision checks | reuse unchanged |
| `src-tauri/src/apply_safety.rs::apply_current_session` | Hides Widget, revalidates HWND/PID/foreground/clipboard ownership, sends one paste | ready backend session | target editor or safe copied fallback | replacement and supported text clipboard snapshot | none | none | fail closed; no automatic retry after uncertain input | preserve unchanged |
| `src-tauri/src/settings.rs` and `src-tauri/src/terminology_store.rs` | Persist preferences and terminology separately | explicit settings/terminology actions | app-owned JSON stores | preferences or terminology only | app-owned local files | none | validated backup and atomic replacement | no Instant content persistence |
| `src-tauri/src/main.rs::show_main_window`, `hide_main_window`, window config | Own Widget visibility, resizable borderless window and shortcut toggle | tray/shortcut/capture | main WebView window | none | configuration only | none | close/hide cancels active turn | preserve current P2-02-derived behavior without accepting P2-02 |

Current focused coverage includes generation supersession, stale completion, intent binding, terminology-revision rejection, user-editable draft parsing, target-bound Apply, clipboard ownership, translation intent, shortcut toggle, and window/result layout contracts.

## 5. P3 goals and non-goals

Goals are a fast first reviewable correction candidate, zero privacy regression, deterministic invalidation, explicit candidate reconciliation, protected user edits, and a reproducible qualification harness.

Non-goals are capture redesign, Apply redesign, automatic Apply, continuous observation, inline underlines, hover cards, document coordinates, editor integration, a bundled model, a network listener, or replacing the Deep Analyzer.

## 6. Instant Analyzer insertion seam

The seam is after the existing `selection-captured` event has established an exact `sessionId` and `generation`, and before `src/App.tsx::rewrite` commits a Deep result to visible draft state. P3-02 should introduce an analysis coordinator, not a second capture path.

The coordinator requests a local correction analysis by token. Backend code retrieves the exact captured source from `CaptureSessionStore`, computes source identity, obtains only matched approved terminology needed for that source, and calls a non-networked analyzer. The response is an immutable candidate; the editable draft remains separate.

The existing capture lifecycle may remain `Captured` while the local candidate is reviewed. Before an Instant draft is applied, a narrowly scoped candidate-authorization transition must revalidate every source-identity field and bind the same current rewrite/terminology intent used by the existing Ready gate. It then delegates to the unchanged `apply_replacement` and `apply_current_session` path. If Deep analysis is active, explicit Instant Apply first interrupts or invalidates that active turn, returns to a provable current capture state, then authorizes the selected Instant candidate. No replacement bypasses Ready validation.

This establishes `CURRENT_CAPTURE_AND_APPLY_PATH_REUSED_WITHOUT_REDESIGN`.

## 7. Deep Analyzer preservation plan

`rewrite_selected_text`, `CodexClient`, the hardened stdio App Server, current model/effort/service-tier selection, authentication isolation, disclosure gate, ephemeral thread, matched-approved-only terminology request, strict output schema, active-turn interruption, and line-structure validation remain the Deep Analyzer.

For correction, P3-02 may start Deep after Instant or concurrently only after the existing cloud disclosure is satisfied. For all Deep-only modes, the current route remains direct. P3-01 authorizes no model, protocol, auth, or runtime-isolation change.

## 8. Mode capability matrix

| P3 capability name | Current UI mode mapping | Instant eligibility | Deep status |
| --- | --- | --- | --- |
| `correction` | `grammar` | eligible | preserved alternate |
| `natural` | `natural` | not eligible | only |
| `professional` | current `polite` intent | not eligible | only |
| `concise` | `concise` | not eligible | only |
| `summary` | no current first-class mode | not eligible | only if separately introduced by a future Gate |
| `translation` | `translate` | not eligible | only |

Eligibility expansion requires a later owner-authorized Gate.

## 9. Request and result contracts

An Instant request contains source identity, correction mode, exact current source obtained from backend capture authority, an already available source-language hint, content-free analyzer settings, and only matched approved terminology required to protect or constrain that source.

An Instant result contains the same source identity, analyzer identity/version, bounded latency metadata, and non-overlapping suggestions. Results are data, not executable instructions. Unknown fields, unsupported kinds, invalid ranges, identity mismatch, network use, or persistence claims fail closed.

## 10. Unicode range representation

Ranges use `UTF16_CODE_UNIT`, `HALF_OPEN`, `startUtf16`, and `endUtf16`, with `0 <= startUtf16 < endUtf16 <= source.length`. A boundary cannot split a surrogate pair. Suggestions in one candidate cannot overlap. Multiple suggestions apply in descending `startUtf16` order after source identity matches.

UTF-16 matches JavaScript string indexing and the current Windows-facing frontend. Rust strings are UTF-8, so Rust must build checked UTF-16-to-UTF-8 boundary mappings from the exact source; it must never cast offsets or slice unchecked.

## 11. Source identity and invalidation

Identity fields are `sessionId`, `generation`, `intentRevision`, `terminologyRevision`, `sourceSha256`, `analyzerId`, `analyzerVersion`, and `mode`. `sourceSha256` is lowercase SHA-256 of exact UTF-8 source bytes. All fields participate in equality.

Recapture, mode change, target-language change, active-profile change, terminology revision, source change, analyzer version change, cancel, dismiss, successful Apply, or shutdown invalidates the candidate. A repeated source hash in a new session is not reusable.

## 12. Suggestion model

A suggestion has `suggestionId`, `sourceIdentity`, `kind`, `startUtf16`, `endUtf16`, `replacement`, `messageCode`, `ruleCode`, `confidenceBand`, `engineId`, and `engineVersion`.

Initial kinds are `spelling`, `spacing`, `basic_grammar`, and `punctuation`. A suggestion must change its exact source slice. Free-form model explanation is unnecessary. The UI may derive localized copy from bounded message/rule codes.

## 13. Cache contract

The cache is process memory only and belongs to the active `CaptureSession`. It stores at most one source identity, one Instant result set, and one optional Deep candidate. It writes no disk, settings, telemetry, history, or recovery file.

It clears on recapture, cancel, dismiss, successful Apply, and shutdown. It invalidates on mode, target, profile, terminology revision, user source, source identity, or analyzer version change.

## 14. Reconciliation state machine

Conceptual states are `Captured`, `InstantAnalyzing`, `InstantReady`, `DeepAnalyzing`, `DeepReady`, `UserEdited`, `Applied`, `Cancelled`, and `Stale`.

- Capture creates one new identity and clears prior candidates.
- Instant may become the first reviewable candidate.
- Deep may run only under existing disclosure/authorization constraints.
- Deep arrival is stored as an alternate; it never silently replaces visible Instant content.
- After `UserEdited`, any arriving result is alternate-only.
- Candidate switching is explicit and revalidates identity.
- Apply is explicit and delegates to target-bound Apply.
- Stale completion performs no UI promotion, clipboard action, input injection, or persistence.

## 15. User-edit protection

P3-02 must separate immutable candidates from `draftText`. It must maintain `draftRevision`, `draftOrigin`, and `isUserEdited`. The first keystroke after candidate adoption marks `UserEdited`; later Instant or Deep completion cannot call the draft setter.

Switching candidates requires an explicit control. The UI must either retain the prior user draft as a recoverable alternate for the active session or require explicit confirmation before replacement. Dismissal and successful Apply clear that memory.

## 16. UI-state implications without UI implementation

The future Widget needs content-free states for analyzing, Instant ready, Deep available, user edited, stale, and failed. It may show analyzer origin and bounded status codes. It must keep review-before-Apply and must not present a Deep arrival as an automatic upgrade.

P3-01 changes no React component, style, window geometry, shortcut, tray, or warning layout.

## 17. Privacy collection contract

Allowed Instant inputs are exact current selected source, source identity, correction mode, an already available language hint, matched approved terminology necessary for the selected source, and content-free behavior settings.

Forbidden inputs are the full terminology dictionary, suggested/disabled terms, profile notes, prior selections, document history, surrounding unselected text, clipboard history, auth data, Codex state, user account data, and HWND/PID without a separately proven need.

No screenshot, OCR, screen recording, key history, DOM scraping, Office/HWP scraping, or unselected-document access is authorized.

## 18. Memory and retention contract

Selected source, candidate replacements, suggestions, source hashes, and draft text are active-session memory only. Cancel, dismiss, Apply, recapture, and shutdown provide deterministic clearing points. Crash recovery intentionally does not restore Instant content.

Settings and terminology retain their existing independent boundaries. Instant artifacts must not be written into either store.

## 19. Network contract

The Instant path has no network permission and no HTTP, TCP, WebSocket, or IPC listener. It cannot invoke Codex, another model, telemetry, update, or provider services.

The Deep path remains cloud-backed through the existing local stdio App Server and remains subject to the current user disclosure. “Local App Server” does not mean local inference.

## 20. Logging and diagnostic contract

Logs may contain enum codes, booleans, counts, bounded timings, and content-free hashes when needed. They may not contain source, replacement, draft, prompt, terminology values, clipboard data, auth identity, device code, account label, or model output.

Production persistence of Instant diagnostics is forbidden. A future benchmark run may write synthetic case IDs and metrics to an explicit task-owned root.

## 21. Threat model

| Threat | Required control | Fail-closed result |
| --- | --- | --- |
| stale completion after recapture | full source identity equality | discard, no promotion |
| source hash collision used across sessions | session/generation equality plus no cross-session cache | discard |
| malformed or surrogate-splitting range | checked UTF-16 boundary conversion | reject result |
| overlapping suggestions | sorted interval validation | reject result |
| protected terminology mutation | protected-span gate and matched-approved-only constraints | reject candidate |
| prompt-like selected text | analyzer treats source as data; deterministic rules have no instruction channel | ordinary text only |
| Deep result overwrites visible/user-edited text | immutable alternate candidate plus draft revision | no draft mutation |
| silent Apply | explicit Apply command and current target-bound backend gate | no target mutation |
| content persistence | memory-only cache and content-free logs | no disk write |
| hidden network or listener | engine capability gate and process/network qualification | engine disqualified |
| resource exhaustion | 12,000 UTF-16 source ceiling and resource budgets | reject or fall back to Deep review path |

## 22. Engine candidate matrix

Scores are 1 (least favorable) through 5 (most favorable). Columns are privacy, offline, cold start, request latency, precision potential, Korean, English, mixed language, protected-term control, determinism, dependency impact, artifact size, memory, integration, licensing/provenance, maintenance, testability, and rollback.

| Candidate | Priv | Off | Cold | Req | Prec | KO | EN | Mix | Term | Det | Dep | Size | Mem | Int | Lic | Maint | Test | Roll |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| deterministic in-repository rules | 5 | 5 | 5 | 5 | 5 | 2 | 2 | 2 | 5 | 5 | 5 | 5 | 5 | 4 | 5 | 3 | 5 | 5 |
| Windows Spell Checking API adapter | 4 | 3 | 4 | 4 | 4 | 2 | 4 | 2 | 3 | 3 | 3 | 5 | 4 | 2 | 4 | 3 | 2 | 4 |
| hybrid rules plus Windows API | 4 | 3 | 4 | 4 | 5 | 3 | 4 | 3 | 5 | 3 | 3 | 4 | 4 | 1 | 4 | 2 | 3 | 3 |
| embedded small local model | 4 | 4 | 1 | 2 | 3 | 3 | 3 | 3 | 2 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 2 | 2 |
| current cloud Deep Analyzer only | 3 | 1 | 1 | 1 | 4 | 4 | 4 | 4 | 4 | 2 | 5 | 5 | 4 | 5 | 4 | 4 | 3 | 5 |

Repository-backed facts support the rule-engine and current Deep-path assessments. Windows language availability, offline package behavior, exact API latency, and external model licensing/performance are `UNVERIFIED_REQUIRES_SEPARATE_RESEARCH`; those scores are conservative planning assumptions, not measured facts.

## 23. Benchmark corpus design

The checked corpus contains exactly 120 synthetic cases: `KO_SPACING` 16; `KO_TYPO` 12; `KO_BASIC_GRAMMAR` 12; `EN_SPELLING` 12; `EN_BASIC_GRAMMAR` 12; `MIXED_LANGUAGE` 8; `PROTECTED_TERMINOLOGY` 12; `STRUCTURE_PRESERVATION` 12; `NO_CHANGE` 12; and `ADVERSARIAL_PROMPT_LIKE` 12.

Templates are task-authored and deterministic. They contain generic sentences, synthetic terminology, numbers, units, URLs, code literals, list/line structure, and prompt-like data. They use no user, clipboard, production-store, model, web, external-dataset, copyrighted-corpus, Prompt-body, or Project Source text.

## 24. Evaluator and metric contract

The dependency-free Node evaluator validates schema version, complete and unique case coverage, exact source hash, finite latency, UTF-16 boundaries, non-overlap, unique IDs and semantics, non-no-op replacement, supported kinds, network false, and persistent cache false.

Matching requires exact span, replacement, and kind against the canonical suggestion or an explicit allowed alternative. `ruleCode` may vary. Metrics cover exact precision/recall/F1, exact case output, no-change false positives, protected/literal violations, invalidity, source identity, latency percentiles and length buckets, cold start, RSS, artifact bytes, and four gate families.

## 25. Provisional safety, quality, latency, memory, and artifact-size budgets

Safety tolerates zero invalid suggestions, overlaps, source-identity mismatches, protected-span mutations, literal violations, network use, or persistent cache use.

Quality requires exact precision at least 0.98, exact recall at least 0.60, exact F1 at least 0.74, exact case-output rate at least 0.70, and no-change false-positive rate at most 0.01.

Latency p95 limits are 250 ms for UTF-16 length 1–512, 500 ms for 513–2048, and 750 ms for 2049–12000. Cold start is at most 1000 ms. Peak RSS is at most 384 MiB and additional engine artifact bytes at most 300 MiB.

These are P3-02 targets, not current-product results. `PRODUCT_ENGINE_BENCHMARK_NOT_EXECUTED`.

## 26. P3-02 recommended strategy

Select `DETERMINISTIC_RULE_ENGINE_FIRST`.

It best satisfies zero-network privacy, high precision, no required new dependency, small rollback surface, deterministic testability, and minimal insertion at the established seam. P3-02 should begin with a deliberately narrow correction rule set and permit abstention. Recall may remain modest while Deep remains available.

## 27. P3-02 implementation boundaries

P3-02 may implement only the correction-mode local analyzer, source identity, checked ranges, session cache, coordinator, candidate authorization, reconciliation, user-edit protection, and synthetic benchmark adapter needed to qualify the strategy.

It must preserve existing capture, target-bound Apply, clipboard, shortcut, auth, App Server, settings/terminology persistence, disclosure, and non-correction modes. A dependency, Windows API, model, editor adapter, or background observer needs separate authority.

## 28. Deferred inline-assist roadmap

Continuous observation, per-keystroke analysis, document revision tracking, coordinates, underlines, hover cards, Word add-ins, browser extensions, HWP integration, UI Automation ranges, DOM ranges, and editor adapters are deferred.

`INLINE_ASSIST_DEFERRED_NOT_AUTHORIZED_BY_P3_01`

## 29. Migration and rollback plan

Ship the future Instant capability behind an internal correction-only capability flag defaulting to disabled until qualification. With it disabled, current `rewrite_selected_text` and Apply behavior are the complete path. No persisted schema is required for the session cache.

Rollback removes coordinator routing and the local analyzer module, leaves no data migration, restores direct Deep orchestration, and retains all current capture and Apply contracts. Candidate state is discarded on restart.

## 30. Risks, unresolved assumptions, and required future research

- The current backend lifecycle represents one Deep-ready candidate; P3-02 must prove the candidate-authorization transition without weakening Ready or target binding.
- Current frontend state lacks explicit draft revision and candidate origin; P3-02 must add them with focused stale-arrival tests.
- Existing terminology matcher spans are Rust character-oriented, not the frozen UTF-16 contract; conversion must be independently tested with surrogate pairs and normalization-sensitive text.
- A small deterministic rule set may not reach recall targets. Failure to qualify keeps Deep-only behavior; it does not justify lower precision.
- Windows Spell Checking API availability, language quality, offline guarantees, package provenance, and test hermeticity remain `UNVERIFIED_REQUIRES_SEPARATE_RESEARCH`.
- Synthetic benchmark success does not establish real-editor, real-language, latency, privacy, or product acceptance. A future Gate needs bounded product-engine measurements and unchanged P1 safety/privacy regressions.
- The corrected Project Source remains historically valid but stale for this implementation and is not updated here.
