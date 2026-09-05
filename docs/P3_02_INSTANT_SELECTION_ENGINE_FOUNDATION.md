# P3-02 Instant Selection Engine Foundation

## Status and boundary

This commit implements and qualifies a pure, deterministic local correction engine foundation. Its runtime classification is:

`IMPLEMENTED_AND_QUALIFIED_NOT_RUNTIME_WIRED`

`RUNTIME_WIRING_NOT_AUTHORIZED_NOT_IMPLEMENTED`

`PRODUCT_ACCEPTANCE_NOT_INFERRED`

`P2_02_ACCEPTANCE_NOT_INFERRED`

The library is not connected to Tauri commands, capture, the Widget, Settings, Deep analysis, clipboard access, Apply, product startup, or any background monitor. A separately authorized P3-03 task would be required for that work.

## Engine descriptor

- ID: `deterministic-rule-engine`
- Version: `0.1.0`
- Mode: correction only
- Network used: false
- Persistent cache used: false
- Runtime wired: false
- Maximum accepted suggestions: 64
- Maximum protected spans: 64

## Request and identity contracts

The request accepts only the exact selected source, full frozen identity, correction mode, bounded protected ranges, and an optional content-free language hint. It does not accept surrounding document text, history, clipboard history, account/auth data, prompts, Deep results, terminology dictionaries, paths, URLs, handles, process identity, or network endpoints.

The complete identity is `sessionId`, `generation`, `intentRevision`, `terminologyRevision`, `sourceSha256`, `mode`, `analyzerId`, and `analyzerVersion`. Every field participates in equality. The engine recomputes lowercase SHA-256 from the exact UTF-8 source bytes and rejects a mismatch. Identical text in another session is not reusable.

Input is fail-closed above 12,000 Unicode scalars or 48 KiB UTF-8. It is never truncated.

## Range and candidate contracts

All ranges use `UTF16_CODE_UNIT` offsets and half-open intervals. Conversion to Rust UTF-8 byte ranges is checked. Zero-length, reversed, out-of-bounds, surrogate-splitting, duplicate, no-op, and overlapping suggestions are rejected.

Protected inputs carry a range and bounded content-free label code only. Invalid or overlapping spans fail closed. Intersecting suggestions are suppressed, and diagnostics retain only the suppressed count.

Accepted suggestions are deterministically sorted, conflicts are resolved by stable priority, and candidate edits are applied in descending start-offset order. Untouched bytes, CRLF/LF choice, leading and trailing whitespace, URLs, code, identifiers, numbers, and units remain exact unless an accepted suggestion explicitly targets their range. Zero suggestions return the original source exactly.

## Retained rule inventory

Each rule has at least two positive tests, two negative tests, one or more off-corpus tests, and the stated false-positive boundary. The combined suite includes 24 dedicated off-corpus cases.

| Rule ID | Category | Bounded purpose | False-positive boundary |
| --- | --- | --- | --- |
| `KO_SPACING_CONFIRM` | spacing | Join the reusable Korean confirmation predicate phrase. | Requires the exact phrase and a lexical end boundary. |
| `KO_TYPO_FINAL` | spelling | Correct a reusable Korean final-syllable typo. | Requires a complete token; longer tokens are excluded. |
| `KO_TENSE_AGREEMENT` | basic grammar | Align a report-writing predicate with a completed review in the same sentence. | Requires the completed-review context in the current sentence. |
| `EN_SPELLING_SEPARATE` | spelling | Correct the common English whole-word typo. | ASCII whole-word match only. |
| `EN_SUBJECT_VERB_RESULTS` | basic grammar | Correct agreement for the exact plural subject and adjacent verb. | Exact adjacent plural-subject construction only. |
| `MIXED_EN_SUCCESSFUL` | spelling | Correct the English whole-word typo in English or mixed text. | ASCII whole-word match only. |
| `KO_SPACING_STABLE` | spacing | Join the reusable Korean adverbial construction. | Exact adverbial phrase and lexical end boundary. |
| `KO_SPACING_MODIFY` | spacing | Join the reusable Korean modification predicate. | Exact predicate phrase and lexical end boundary. |
| `EN_SUBJECT_VERB_THIS_ARE` | basic grammar | Correct exact `this are` agreement. | Whole-word demonstrative pair only. |
| `EN_DEMONSTRATIVE_THESE_VESSEL` | basic grammar | Correct exact `these vessel` to `this vessel`. | Singular vessel only; `these vessels` excluded. |
| `EN_DUPLICATE_WORD` | spelling | Drop an adjacent duplicated ASCII word. | Length >= 3; all-caps codes skipped. |
| `KO_TYPO_DONE_DA` | spelling | Correct complete token `됬다`. | Complete token only. |
| `KO_TYPO_DOE_YO` | spelling | Correct complete token `되요`. | Complete token only. |

No rule reads benchmark files, case IDs, source hashes, complete cases, environment variables, current time, random state, filesystem state, locale ordering, subprocesses, OS proofing services, network services, or models.

## Cache foundation

The cache is `MEMORY_ONLY`, instance-owned, and limited to one active source identity, one Instant candidate, and one Deep alternate. It has no disk, settings, telemetry, static-global, or cross-session persistence. Reads require the exact identity.

Explicit invalidation clears identity and candidates for recapture, cancel, dismiss, successful Apply, shutdown, mode change, language change, profile change, terminology revision change, source identity change, and analyzer ID/version change.

## Reconciliation and user edits

The pure state machine implements Captured, InstantAnalyzing, InstantReady, DeepAnalyzing, DeepReady, UserEdited, Applied, Cancelled, and Stale. Instant may become the first reviewable draft. Deep completion remains an alternate and never silently replaces Instant or a user edit. Candidate switching and Apply are explicit domain actions. Switching away from a dirty draft retains a recoverable copy. Stale identity and terminal states reject illegal transitions.

## Privacy and diagnostics

The engine performs no I/O and emits no logs. Its diagnostics are content-free: engine identity, rule IDs, category counts, input sizes, suggestion/suppression counts, validation enum, and false network/persistence flags. Source, replacement, candidate, protected text, terminology values, prompts, clipboard, document content, paths, and account/auth data are never logged.

## Tests and product benchmark

Focused Rust tests cover the descriptor, correction-only enforcement, identity/hash validation, UTF-16 boundaries, surrogate rejection, overlap/duplicate/no-op rejection, protected suppression, descending construction, exact no-change behavior, every retained rule, 39 off-corpus cases, prompt-like data, cache isolation/invalidation, reconciliation, and user-edit protection.

The PowerShell runner compiles the integration target once into an external Cargo target, then starts that same executable in three fresh processes. The ignored Rust benchmark writer calls `InstantSelectionEngine::analyze`, loads only the immutable synthetic corpus, and writes predictions and a deterministic semantic projection under an external task-owned output root. Each predictions file is evaluated by the immutable P3-01 evaluator. Engine timing surrounds construction and analysis only; compilation and process startup are excluded. Peak process RSS and executable bytes are measured content-free by the runner.

Run from the repository root with new external paths:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\run-instant-selection-product-benchmark.ps1 `
  -OutputRoot D:\path\to\new\benchmark-output `
  -CargoTargetRoot D:\path\to\external\cargo-target
```

The runner rejects an existing output root and executes exactly three fixed runs. A genuine run failure ends qualification; it is not retried until green.
