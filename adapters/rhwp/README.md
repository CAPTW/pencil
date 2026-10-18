# rHWP adapter — scoped installed candidate

Current 2026-10-18: installed Native Messaging + actual Gemma is qualified only
for the bounded synthetic body selections recorded in HANDOFF: word-internal
emphasis and partial single/cross-paragraph HWP/HWPX selection, explicit Accept,
Undo and save/reopen. General Hancom/IME/complex-document support is not claimed.
Earlier sections below are preserved chronological investigation records; their
not-installed/not-implemented statements are historical, not current status.

## Historical investigation (2026-10-07)

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


## Trusted editor bootstrap (source fixture only)

bootstrap-overlay.mjs adds a direct lexical initialization call in the pinned
Studio main module. bootstrap.mjs receives the real editor dependencies, without
looking them up in dev globals or accepting page messages. Default admission is
false. Only the server --bootstrap-fixture build grants synthetic HWP/HWPX normal
edit-mode admission; this is not a sensitive-document classifier or deployment
permission. --bootstrap-denied exercises the default refusal before target reads.

Each document generation receives a new disabled session. Generation equality is
checked synchronously before capture/Accept; a 100ms metadata poll clears the UI
and old draft. Hidden-page transitions revoke the session, and pagehide or editor
reinitialization disposes the panel, session and timer. Visibility handling is
implemented but not separately browser-qualified in this checkpoint.

--bootstrap-only checks mounting, edited Accept and exact reread, document-switch
revocation and pagehide disposal for both formats. Input/event/render dev globals
are removed by the fixture; __wasm remains only as the independent reread oracle.
The bootstrap has no injected Instant transport yet. Earlier Instant tests remain
separate; no installed shortcut/widget/package or broad HWP support is claimed.


## Document admission refusal guard

The trusted bootstrap now checks hasLoadedDocument(), documentGeneration,
requiresPasswordForSave, source format, and the separate trusted eligible callback
before capture/Accept and on the metadata-only lifecycle poll. Missing metadata,
exceptions, unloaded/unsupported documents, protection, or host revocation deny.
An observed denial is sticky for that generation: clearing the mutable save-password
flag or restoring the host callback cannot revive the previous grant. A different
loaded generation is evaluated anew; close is terminal. A poll-observed revocation
also clears the UI draft and session (100ms scheduling target, not a hard deadline).

This guard is necessary, not sufficient to identify every protected or sensitive
document. requiresPasswordForSave is mutable save-intent metadata; it does not
prove original load provenance, DRM absence or non-sensitive contents. Unobserved
transient changes are not certified by polling. Default host admission remains
false, synthetic fixture admission is not deployable, and real document admission
and production local transport remain pending. No password contents are accessed.

The protection counterexample sets only synthetic editor metadata, then clicks
Accept in the same JS task before the polling callback. It checks zero subsequent
target reads, unchanged whole-document text, cleared draft and refusal after the
flag is cleared. This is not an encrypted-file or real-password qualification.


## Actual local Gemma writing bridge (2026-10-15, fixture only)

The optional analyzer now supports explicitly enabled translation (ko/en), natural
and polite rewrite intents. The internal panel binds mode to each capture, asks
before discarding a changed draft, and invalidates old authority on mode change.
Reply intent/identity/revision must match. No cloud fallback or retry is added.

`rhwp-api-probe.mjs --gemma-only` (mode is the final positional argument) uses a
manifest-checked installed native host and actual local broker via a Playwright
TEST-ONLY bridge. Owned HWP/HWPX EN-KO/natural/polite6 cases passed12 Accept/text,
character/paragraph/style-property and Undo checks. Only whole homogeneous plain
body paragraphs are qualified. The default bootstrap still denies admission;
trusted deployed transport, source-protection provenance and installed user flow
remain pending. This is not general HWP/Hancom support or saved-file roundtrip proof.
Evidence: rhwp-gemma-20261015/FINAL_RECEIPT.json. Prior Instant/bootstrap fixture
results are separate. `--verify-only` checks all pinned upstream bytes without UI.


## Native transport preparation and styled roundtrip (2026-10-15)

`native-review.mjs` provides a trusted extension application entry using real Chrome
Native Messaging via `native-transport.mjs`. It is not yet bundled/wired into the
installed product. The host registration and verified document-load eligibility
must be supplied by that trusted application; default admission still denies.
Transport unit tests cover context, denial, abort/disconnect, revocation, mismatched
reply, busy/close and deadline. They do not prove native browser deployment.

The separate actual Gemma fixture now checks bold+italic uniform selected text,
mixed-format surrounding paragraphs, explicit Accept, Undo and disk export/reopen
for HWP/HWPX (18 checks). This still uses the earlier TEST-ONLY analysis bridge.
Mixed formatting within the selected paragraph, tables/fields and general complex
HWP documents remain unsupported. Do not remove the current refusal to enable them.


## Installed extension-owned editor candidate (2026-10-16)

The optional `package.ps1 -RhwpSourceRoot <verified-upstream-root>` build now
bundles the pinned Studio into `extension/rhwp/index.html`, linked from the popup.
It uses the installer's Native Messaging registration and the managed local Gemma
broker. There is no page analysis hook, HTTP bridge, cloud fallback or automatic
Accept. The earlier fixture-only statements above describe historical checkpoints.

Before the editor parses document text, `load-protection.mjs` checks bounded
original bytes. Only plain HWP5 with compression as its sole optional header flag,
and bounded HWPX with an empty ODF protection manifest, are admitted. Password,
distribution, DRM, unknown flags/containers and ambiguous ZIP members are refused.
This deliberately rejects some otherwise valid documents. Admission is granted
only after that exact load finishes initializing; switching/failed loads dispose
old review authority. Recent-document persistence and recovery/idle autosave are
disabled in this bundle. Explicit Save downloads a file chosen by the user.

Actual installed Native Messaging with Gemma passed six HWP/HWPX translation,
natural and polite rewrite cases, with independent saved-file parsing for exact
text, character/paragraph/style preservation, Undo and reopen (18 checks).
The selected paragraph is uniform bold+italic; surrounding paragraphs have mixed
formatting. Mixed runs INSIDE the selection, tables, fields, arbitrary complex
files and actual Hancom remain unqualified. This is headless Chromium automation
with a fresh owned profile, not physical keyboard or ordinary profile acceptance.

`test-rhwp-installed.mjs <config.json>` repeats the installed-browser checks under
the existing single-session supervisor. Config supplies evidenceRoot (containing
install and protected synthetic files), playwrightModule, wasmModule, wasmBinary,
browserProfile, chromiumBinary and inputs.hwp/inputs.hwpx. Use synthetic documents
only, a running owned Gemma installation, and a fresh profile. The independent WASM
parser only reads downloaded files; it does not mutate the live editor.

Evidence: external `rhwp-installed-20261016`. The tested package combines verified
native binaries from e6ff394 with the separately hash-bound rHWP addon; it is not
a clean native rebuild from the addon commit. No personal-install update, release
or general support claim follows from this candidate.


## Historical sentence-aligned rich-selection checkpoint (2026-10-16)

The new rich-review path binds complete selected body paragraphs and their original
character-format runs. It sends one full-context, escaped run/paragraph envelope
through installed local Native Messaging; no per-run inference, HTTP bridge or
cloud fallback is introduced. The response must retain every ordered marker.
Review lines retain original styles; blank lines retain paragraph boundaries.
Accept rejects missing boundaries, changed document/selection, expired permission
and revoked admission. It applies all paragraphs in one target-side history
operation and checks saved text/format/outside content, with fenced Undo retained.
Render failures never restore a snapshot over later application edits.

Current admission is intentionally narrower than arbitrary rich text: whole body
paragraphs in one section, at most eight nonempty paragraphs, 24 runs and 3000
selected codepoints, with bounded document inspection. Mixed runs must end at
sentence boundaries. Mid-sentence emphasis, empty selected paragraphs, partial
paragraph ranges, tables/fields and reordered style spans remain unsupported.
This avoids imposing original-language word order on translated inline emphasis.
A missing model marker is a refusal, never a guessed proportional style mapping.

The Native Host now separates the existing 1s grammar request ceiling from an 8s
explicit writing ceiling within the extension's 10s watchdog. Bootstrap and model
inference share that single deadline. The 1s recommendation, 3s paragraph and
500ms Accept performance targets remain unchanged and misses must be reported.
See the dated HANDOFF and external receipts for actual qualification and failures;
this section does not establish general Hancom or whole-document compatibility.


## Inline emphasis and partial body selections (current candidate)

The rich path now captures bounded partial first/last paragraphs, including a
format change inside a word. Only selected text enters the local model request.
Prefix/suffix text and character properties stay local and are checked exactly
alongside all other paragraphs. One synchronous target-side operation preserves
paragraph count, paragraph properties and styles; existing stale/permission,
protected document and uncertain-render guards remain.

Run IDs may move within their original paragraph for translated word order.
Identical full character properties share one style ID within a paragraph. Every original style class must occur at least once. A style can split into multiple
spans of the same ID when translated meaning becomes noncontiguous; at most96
nonempty spans per paragraph. Missing, foreign or cross-paragraph IDs refuse. A validated output order is bound to the capture
before displaying it. Review lines follow that order; a preview shows joined
text with bold/italic and original-text tooltips. Other saved properties retain
the exact original style object even where the preview does not visualize them.
The model can still assign meaning incorrectly: this is not universal semantic
alignment. Review the suggestion before explicit Accept. Empty runs, empty
selected paragraphs, tables/fields and changed paragraph topology remain refused.

History still compares the entire serialized document. HWP Save stamps the view
caret, so the comparison temporarily uses the exact recorded caret metadata and
synchronously restores the current caret, including on export failure. Unsupported
caret representations refuse before writes. No content/format bytes are excluded,
no snapshot is restored by the comparison, and no asynchronous callback runs in
that normalization interval. This does not remove the user-edit Undo fence.

Installed synthetic results and limitations are recorded in the latest HANDOFF.
No general Hancom, all-word linguistic quality or physical IME qualification is
implied. The candidate adds a rebuilt broker with typed style-anchor instructions;
host also adds the typed internal alignment intent; app and model remain those
of the previous composite package.

Rich writing uses two explicit local stages: ordinary translation/rewrite of
selected plain text, then typed `style_align` returning exact non-default substrings of that fixed
result. The most prevalent original style is the paragraph default. Unknown,
missing, overlapping or ambiguous repeated anchors refuse. The adapter assembles
styles over the immutable first result; removing review separators must recover
that result exactly, including whitespace and paragraph breaks. This prevents formatting guidance from
lowering the ordinary writing quality. Both stages share one eight-second abort
budget, one UI slot and the same capture lifecycle. No retry, per-run inference,
cloud request or outside-context text is introduced. The second stage adds latency;
performance targets are unchanged. Ordinary document text cannot select alignment
mode merely by containing an envelope-looking prefix. Ordinary writing retains its literal guard. The typed alignment response is
metadata, so its JSON quotation marks are not treated as document literals;
the adapter retains every character of the already validated fixed result.

Splitting style IDs is an output representation change, not permission to duplicate
meaning; semantic anchoring remains model-dependent and requires review separately
from exact range/format safety. Earlier failed prompt/representation trials remain
recorded; mechanical file equality alone is never language-quality acceptance.
