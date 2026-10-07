# rHWP investigation — not an enabled Grammar adapter

Owner suggested rHWP as the synthetic HWP/HWPX application. Pin: upstream
edwardkim/rhwp 0.8.7, commit in upstream.json, MIT. No production dependency or
native admission was added. This is not actual Hancom qualification.

## Executed scope (2026-10-07)

Evidence: D:/dev/grammar-evidence/grammar-rhwp-20261007/run-final/receipt.json.
16 assertions pass in sandboxed headless Chromium using a fresh owned profile:
both formats create/export/open, whole plain paragraph apply/revert, exact outside
text preservation, stale revision refusal, four hash mismatch refusals, edited
export/reopen. Korean and astral Unicode are included. Three paragraphs are used:
the first contains section controls and correctly refuses agent editing.

Two selection probes expose an integration gap: after selecting paragraph 2,
an earlier hash-bound command still changes paragraph 1. This is valid upstream
explicit-target behavior, but fails Grammar's current-selection contract.
Do not connect the existing command directly to desktop Accept. A target-side
selection/authority fence is needed; an async preflight alone is insufficient.

API calls perform mutations through the real upstream controller. Readonly dev
hooks construct evidence and reread results; these hooks are not a product capture
implementation. Public selection context supplies hashes, not selected plaintext.
Render-failure snapshot rollback, interleaved edits, journal lifetime, opt-in and
revocation still need review/testing before any adapter activation.

Autosave settings are disabled before load; service workers and external requests
are blocked. Observed drafts=0, recent metadata=4, page errors=0. Upstream recent
store records filename/format/time, not document bytes for this load path. The
entire task profile is removed after each run. No user documents, screenshots,
clipboard, account, Provider or normal profile are involved.

## Re-run the preserved fixture

Use the external evidence root above containing upstream/ and upstream-receipt.json.
The receipt binds 864 source files to pinned Git blobs/SHA256. The core npm archive
is SHA512 checked against upstream.json; npm installation used the pinned lockfile
with --ignore-scripts --no-audit --no-fund. Keep source/dependencies outside Grammar.

Run `node scripts/mission/rhwp-probe-server.mjs <evidence-root>` in one terminal.
Run `node scripts/mission/rhwp-api-probe.mjs <evidence-root> <absolute-playwright-package>
<absolute-chrome-exe> <new-output-directory>` in another. The probe rechecks source
hashes/archive integrity before running. Server is loopback port 17707, HMR off;
an explicit route serves the pinned WASM bytes because the initial Vite static
route returned HTML. Stop the owned server afterward. Browser sandbox stays on.

Preserved failed runs: r1 hot reload destroyed execution context; r2/r3 WASM route
returned HTML; r4 first paragraph control refusal (fixture target assumption).
r5 passed 8 checks; r6 and final passed 16. These are API fixtures, not installed
Grammar, physical input/IME, arbitrary formatting, or general HWP/HWPX acceptance.

## Target-side gate continuation

`accept-gate.mjs` is a target-process component, still not installed/activated.
Trusted host opt-in binds a cloned reviewed command to a single-use token with a
30s lifetime. Exact document epoch/revision/hash and noncollapsed editable selection
are checked at capture, Accept, before history entry, and synchronously inside the
mutation callback. A separate asynchronous preflight is not the mutation authority.
One gate handles one Accept attempt; terminal disposal releases its controller and
in-memory journal reference. Disable clears pending authority. Concurrent Accept
is refused. There is no page-message transport, desktop shortcut, selected-text
capture UI or provider integration yet.

`input-overlay.mjs` applies narrowly matched source changes to the pinned upstream
InputHandler/CommandHistory in the test server. Upstream files remain untouched;
the executed source is explicitly a Grammar overlay, not unmodified upstream.
The gate refuses an executor without the overlay capability. Non-Grammar operation
types retain upstream behavior. For Grammar render failure, never restore the
whole-document snapshot: discard only the exact failed history entry, notify dirty
state, report an uncertain failure, close the gate. No replay or automatic rollback.
Synchronous mutation failures retain upstream behavior and need further fault tests.

Preserved `gate-r2` counterexample: a synthetic application mutation in another
paragraph during a throwing render callback was lost in both formats. Its aggregate
API fixture status is not a safety pass; `userEditPreserved=false` is the failure.
`gate-r3` preserves the edit; final additionally checks failed history removal and
terminal gate cleanup. This is injected application activity, not physical typing.
Review was by the same integrator; no independent reviewer. Product activation,
trusted bridge packaging, selected-only capture, normal Undo/Redo and sensitive
document admission still require qualification.

The gate-final run failed an assumed history count (0 versus1); it is preserved.
The corrected oracle compares exact history command identities immediately before
and after the render failure, requiring no new failed entry and all prior entries
retained. Final target is gate-qualified/receipt.json. Earlier rejection effects on
normal editor Undo still need qualification; no Undo-support claim is made.

## Bound Undo/Redo continuation

The Grammar overlay now associates each successful `snapshot:grammar:apply`
history command with before/after document SHA256 values (WeakMap, memory only).
Undo requires the exact after image; Redo requires the exact before image. The
check precedes snapshot restoration and stack movement. A mismatch retains the
entry and refuses, rather than falling through to an older snapshot. Non-Grammar
history behavior is unchanged. Failure computing the postimage releases the
unrecorded snapshot without restoring document contents.

`history-red` reproduces unrelated paragraph edits lost through both Undo and Redo.
`history-green` stops on the new typed refusal because the fixture initially did
not catch the expected error; this is retained. Final fixture accepts that exact
error only in the negative cases and still compares complete document text and
history entry identity. Normal roundtrips must complete without any error.
The fixture adds three extra Undo/Redo cycles per format. This remains an internal
real-editor API test, not keyboard/IME or installed Grammar qualification. Capture,
UI routing, hostile synchronous callbacks and rich formatting remain pending.

## Selected-text review component

`review-session.mjs` adds a trusted in-editor capture/review lifetime. It is off by
default and requires a host-supplied eligibility decision before reading text or
exporting document state. Only an exact whole plain paragraph (1–4000 codepoints)
is currently supported; partial, empty and non-body selections are refused without
expanding the selection. Upstream evidence checks reject controls/fields and mixed
formatting. Document/neighbor hashes are computed inside the editor; the review
payload contains only `{id, source}`. This is selected-only output, not a claim that
the editor's internal hash calculation reads only the selected text.

`review-panel.mjs` is a small internal browser review UI: explicit Enable, Capture,
editable draft, Accept and Disable. It initializes the draft with source text;
it does not yet run Grammar Instant or Deep. Accept edits only replacement text in
the original bound grant; it does not renew the original 30s capture lifetime or
replace target/document binding. Recapture consumes the old ID; Disable clears the
draft/authority. Stale asynchronous UI completion cannot replace a newer status.

This is not the installed desktop widget or an enabled public adapter. The fixture
obtains real editor dependencies through dev hooks; production dependency injection,
document-sensitive admission and trusted deployment remain unqualified. No page
message listener, remote transport, persistence, inference or Provider is added.

Run the same probe with a final `--review-only` argument for UI tests in a fresh
owned browser profile. HWP/HWPX checks use actual Playwright button clicks and draft
input, then reread all document text. They cover disabled/ineligible zero text reads,
selected-only payload, explicit edited Accept, moved selection, Disable clearing,
recapture stale ID, partial selection, revoked admission and changed source. This
is automated browser input, not physical keyboard/IME or installed qualification.

## Local Instant continuation

The panel accepts an optional trusted local analyzer. instant-draft.mjs reuses the
Chromium DocumentSession/SelectionReview UTF-16 validation and draft assembly, not
a second spelling engine. Source remains editable during analysis. Publication
requires the same capture, unchanged user-edit revision and a fresh target-side
binding/authority check. Disable, recapture, Accept and close cancel publication.
Only one request per analyzer may be in flight; no retry, Provider or fallback.
Abort stops publication; physical cancellation depends on the injected transport.

The --instant-only fixture invokes the real Grammar native host local engine via
framed analyze requests. Its owned executable copy/origin sidecar is unregistered.
The Playwright bridge is test-only, not production transport. Each process has a
10s deadline and256KiB output cap; receipt records binary SHA256/calls/exit codes.
Delayed replies hold actual analysis results to test edit/Disable/selection races.
instant-host-build-proof.json binds the offline locked debug build to tracked Rust
bytes. No live Provider, Deep or remote upload. This extends the earlier source-only
draft scope when an analyzer is supplied; default mounting still works without it.
Production bootstrap/transport, desktop shortcut/widget and installed package remain
pending. The build emits existing unused-library warnings; none were suppressed.
