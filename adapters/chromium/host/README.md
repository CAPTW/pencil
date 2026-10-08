# Local Instant and consent-bound Deep native host

Run the preparation/removal scripts with PowerShell 7 (`pwsh -NoProfile -File ...`); do not launch Windows PowerShell with an inherited incompatible module path.

Build with `cargo build --manifest-path src-tauri/Cargo.toml --locked --offline --bin grammar-chromium-host`.
Run binary unit tests with the equivalent `cargo test ... --bin grammar-chromium-host`.
Run `python adapters/chromium/host/test_host.py <built-executable>` for the synthetic executable protocol fixture.
For a coordinator-supplied synthetic fixture executable, `python adapters/chromium/host/test_host_deep.py <host-executable> <fixture-executable> <new-evidence-directory>` tests real host/runtime success, consent rejection, cancellation and EOF. It overrides all Provider executable paths with that fixture and uses a minimal task-owned environment; never supply a real Provider executable as the fixture.

The host accepts Chromium's origin argument only when it exactly matches the single extension origin in `grammar-chromium-host.origin.json` beside its executable. Messages have a four-byte little-endian length and UTF-8 JSON. Local requests are `{version:1, op:"analyze", id, epoch, revision, text}`; id/epoch are nonempty and at most 128 UTF-8 bytes, and revision is a positive JavaScript-safe integer. Frame limit is 128 KiB, text limit 8192 UTF-16 code units, response limit 256 KiB. Suggestion start/end are UTF-16 offsets. Unknown JSON fields, malformed/truncated/oversized input and invalid origins close the channel without document-bearing errors.

Explicit Deep requests use `op:"deep"` and add `provider:"codex"|"antigravity"|"claude"` and `consent:true`. Missing consent/provider or empty text is rejected before executable discovery. The shared production headless runtime dispatches only that Provider; no fallback or replay. No terminology is supplied by this host. Successful responses echo id/epoch/revision and include `replacement`, `source_sha256`, `provider`, and `cleanup_complete:true`. Errors contain only stable codes. A cleanup failure yields `cleanup_complete:false` and must never be treated as a completed cancellation.

Deep additionally requires a cleanup ticket from `cleanup-reserve` with a canonical lowercase UUID `cleanup_token`. Control messages retain `version:1,id,epoch,revision,text:""`, without consent/provider. The ticket is `{installation,slot,generation,token}`; `cleanup-find` finds an unacknowledged token after a lost reserve reply. `cleanup-query` and `cleanup-ack` take `cleanup_ticket` and echo it. Four installation-owned slots transition FREE → RESERVED → PENDING → COMPLETE → FREE under an exclusive Windows file lock. Deep atomically consumes RESERVED before execution; query atomically retires RESERVED so a delayed Deep cannot start. Only checked teardown produces COMPLETE for PENDING. ACK is idempotent for the exact freed generation until reuse. Generation never resets. A hard-crash PENDING, missing/corrupt ledger, mismatched ticket or failed persistence remains unavailable; PID absence or elapsed time is never cleanup proof.

`Prepare-Host.ps1` creates the fixed ledger and lock in a fresh origin-bound installation with an installation identifier. Origin-only older installations remain Instant-only. The host does not recreate missing ledgers, reclaim unknown slots or probe Providers on query. Records contain only opaque installation/token/generation/state, never document identifiers, source hashes, text or Provider output. The ledger uses a fixed temporary file, flushed data and atomic replacement; failed writes retain conservative state and bounded files. Filesystem kernel waits do not have a hard timeout. Non-Windows receipt operations are unavailable. No claim is made against malicious modification by the same user account.

Replacement uses documented [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw) replacement and write-through flags. Power-loss durability has not been qualified; this implementation does not claim recovery after storage rollback or power failure.

A cancellation is `{version:1,op:"cancel",id,epoch,revision,text:""}` matching the active Deep request exactly. The host sets its cancellation token, retains and awaits the request future, then replies with the original id only after checked teardown. There is no immediate cancellation acknowledgment. Only one Deep request is active; concurrent analysis/Deep receives `deep_busy`. Instant remains local and never discovers or invokes Providers.

The actor has a bounded input channel of two frames and output channel of one response. One process-lifetime standard input reader thread and one standard output writer thread service the pipes; response acknowledgment has a three-second deadline. EOF, malformed input, or a stalled/failed output cancels and awaits the active Provider and checked runtime shutdown before the host process exits. Completed pipe threads are joined; a remaining OS pipe wait is closed by process exit after Provider teardown. This is not cancellation by dropping the rewrite future. Synchronous filesystem operations still have the runtime's documented bounded-attempt rather than hard kernel-timeout limitation.

The host never persists document text. Provider history controls are separate: Claude is invoked with `--no-session-persistence`, `--bare`, `--disallowedTools '*'`, and `--disable-slash-commands`; bare mode does not support the existing OAuth login flow. Headless Antigravity returns `provider_privacy_unqualified` before admission until history suppression is qualified, with no fallback. Live Provider behavior remains NOT_RUN. Tests using the synthetic service prove actor cancellation/EOF/output ownership and consent gates, not external-account behavior. `test_host.py` runs only real-binary Instant, missing-consent rejection, framing and origin cases; it does not invoke a Provider, read credentials, or establish external-account acceptance. The Deep fixture checks Claude privacy flag booleans without saving raw arguments or source text. Live Provider calls require separate Owner authorization.

`Prepare-Host.ps1 -Executable <exe> -ExtensionId <32 letters a-p> -OutputDirectory <new-task-owned-dir>` prepares a unique host manifest, copied executable, origin sidecar and hash receipt. It does not register by default. The optional `-Register` creates only a unique HKCU Chrome native host entry; use only with coordinator-admitted task-owned browser testing. It never overwrites an existing key/package. Use the returned host name in extension configuration. `Remove-Registration.ps1 -PackageDirectory <dir>` removes only that exact unchanged registration and preserves files. Normal user-profile installation remains outside this fixture workflow.
# Candidate Gemma local route

The owned launcher may add `gemma` to `grammar-chromium-host.origin.json`:
`{"pipe":"\\\\.\\pipe\\Grammar.Gemma.Broker.<uuid>","broker_sha256":"<64 hex digits>"}`.
The broker executable must be named `grammar-gemma-broker.exe` beside this host.
The host verifies and retains its image pin; OS process-image checks bind the live
pipe peer before any document text is sent. Browser frames cannot select a broker
or executable. Missing `gemma` preserves the Instant rules path; an explicitly
configured but unavailable broker fails local analysis without rules fallback.
A failed local image pin does not disable consent/cleanup handling for external Deep.

This route is currently qualified only by the task-owned modified-extension trial
in `scripts/mission/gemma-browser-trial.mjs`. It copies the extension, changes its
key/host configuration, adds permission for its loopback fixture, and registers a
unique temporary native host. It is not installed-package or toolbar qualification.
The personal installer does not enable this option. Never point a normal profile
at the test registration. The launcher owns broker readiness and stdin lifetime;
EOF ends admission and awaits model cleanup. No automatic restart or fallback.

The trusted broker launcher requires seven arguments, in order:
`runtime runtime-sha model model-sha host host-sha runtime-bundle-sha`.
The last digest must come from the reviewed runtime manifest, not from blindly
hashing whatever is installed at launch. It covers the adjacent runtime EXE and
DLL files: sort filenames, concatenate `filename + NUL + lowercase-file-SHA256 + LF`,
then SHA256 that UTF-8 byte sequence. The broker retains deny-write/delete handles
for verified files until service cleanup. Missing/changed/extra adjacent DLLs at
verification fail admission; six-argument callers fail closed. Verification shares
the existing 180s startup deadline and revocation signal, with 128-file/8GiB limits.
This identifies the supplied adjacent bundle, not Windows system libraries or a
sandbox against a malicious same-account process adding files after verification.
Current owned launch scripts retain the reviewed CUDA12 bundle; CUDA13 remains an
unpromoted comparator. Personal installer integration remains pending.
