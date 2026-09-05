# P3-03 Instant Selection runtime wiring

Insertion seam:

`AFTER_SELECTION_CAPTURE_TOKEN_ESTABLISHMENT_BEFORE_DEEP_RESULT_PROMOTION`

Canonical source-byte authority remains `MAIN_HEAD_GIT_BLOBS`. Physical CRLF checkout is accepted only as exact EOL-only representation.

Runtime adapter: `src-tauri/src/p3_03_runtime.rs`

- Correction/Grammar mode only.
- Source is resolved from backend CaptureSession, never from the frontend.
- Instant runs off the UI thread via `spawn_blocking`.
- Deep request count remains exactly one per capture.
- Memory-only session cache with eleven invalidation reasons.
- Protected spans are projected from already approved matched protected terminology.
- Automatic Apply remains zero.

Frontend reducer: `src/instantSelectionRuntime.ts`

- First valid candidate may initialize the draft.
- Late Instant/Deep results never silently overwrite a current or dirty draft.
- Dirty candidate switching requires explicit confirmation.
