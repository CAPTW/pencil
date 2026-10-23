# Prototype 3 post-release validation — 2026-10-23

Published ZIP/source remain unchanged: `5d43b2b42f50a663644eefae12e6205eb80033a6`,
SHA-256 `4c42f69b9c26c06b59463ac2d200392c19adfbf8f004c5db4423014d9ca95fb8`.
This report adds evidence; it does not replace the original release receipts.

## Installed real Gemma, bounded Chromium scope

Original packaged launcher, app, broker, native host and installed extension were
used in a fresh medium-integrity Windows Sandbox with networking/clipboard disabled.
The pinned model/runtime came from the read-only D-drive asset mapping. Only the
owned guest profile/assets/installation and exact registration were removed.
No extension permission, activation or response injection was used. Real keyboard
toolbar activation succeeded twice; Playwright drove synthetic document controls.

Four existing 3–4-sentence cases (EN→KO, KO→EN, English/Korean natural rewrite),
each in textarea and simple contenteditable: **8 exact Accept rereads passed**.
Four repeated-edit/stale-Accept checks passed; changed-source Accept preserved the
new text. Cache/Accept made no inference request. Disable/navigation/reload,
unchanged installed extension hashes and memory-only storage checks passed.

Explicit Review locally click → visible suggestion next frame: 1089.3–1372ms,
p50 1138.3ms, p95/p99 1372ms. Accept → exact reread: 58.1–63.1ms,
p50 59.7ms, p95/p99 63.1ms. Paragraph3s and Accept500ms misses: 0/8.
All eight exceed 1s; this is not proactive committed-input latency qualification.
Model copy: 98.682s; launcher-to-ready: 12.310s, separately excluded from warm timing.

Same-integrator unblinded output review found the bounded names, quantities,
negation, conditions and Korean endings preserved. English remains stiff
(“is scheduled to”, “It is necessary”). These are reused regression cases,
not unseen evaluation or general semantic/fluency qualification.

Natural launcher exit0; browser/host/model/app residuals0, uninstall complete,
owned profile/install/registration removed, Sandbox inventory0. First preparation
attempt failed before product launch because host-machine.txt was omitted;
its failure and separate cleanup are preserved. Second serial attempt passed.

Desktop capture/Accept on this ZIP, physical IME, normal profiles, Edge, live
Providers and broader editor/hardware qualification remain NOT_RUN. Mission
INCOMPLETE; Project Source remains stale. A tray-icon removal warning was logged
despite launcher exit0 and complete cleanup; not erased or called a crash.

## CI failure and bounded repair

Release source main CI39969950355 and record candidate CI39970645697 passed.
Record main CI39970685154 failed: active-launcher uninstall refusal exceeded the
existing10s test deadline. The exact internal cause of that hosted delay is unknown.
Investigation found tree traversal and Add-Type compilation preceded lock refusal.
The repair checks an existing lock read-only before those operations in owned
cleanup, with no file creation. Full containment/link/tree checks and the final
exclusive mutation lock remain. The test deadline is unchanged.

A deterministic production-script regression holds a real exclusive Windows
handle and makes tree enumeration fail: old code fails at enumeration; repaired
code must refuse at the lock without changing the receipt or executable.
Local first full run had17 UTF-8 decoding errors from inherited console encoding;
preserved separately from product behavior and the hosted timeout.
Final local and hosted results are recorded in the handoff/evidence receipt.

Evidence: `D:/dev/grammar-evidence/prototype3-gemma-20261023/` (r2/guest,
quality-review.json, ci-red, ci-green, ci-green-utf8 and hosted CI logs).
The source uninstaller repair is not retroactively present in the published ZIP.

Local repair: launcher38/38 and owned cleanup boundary17/17 PASS; both supervisors report COMPLETE with0 residual processes. GitHub repair CI is a separate pending result at this commit.
