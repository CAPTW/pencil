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
