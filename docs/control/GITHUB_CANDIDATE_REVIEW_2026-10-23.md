# Candidate publication review — 2026-10-23

Owner requested review and non-force publication of the 29 local commits, followed
by Windows CI and correction of failures. Scope is the existing
`codex/grammar-local-trial-ba58b7d` branch in `CAPTW/pencil`; no main merge,
executable release, personal installation or live cloud Provider call this round.

## Reviewed range

`b30ae48a70b1652ed071c8fe3568d19e3eb312f2..4ef212ffa9f1ce2ccc7c69d26d7cec89560843ec`
contains 29 commits and 31 changed paths. Remote ancestry was refreshed before
the fast-forward push. Main remained at the base commit.

The integrator reviewed incremental changed-window accumulation, old-result
rejection, worker admission cancellation, native host/pipe identity binding,
soft supersession versus hard revocation, bounded idle confirmation, quality
refusal reuse, output semantic fences, installed test scope and owned build
helper cleanup. No new publication blocker was identified. This is a
same-integrator review, not independent review or general semantic certification.

All 115 outgoing historical blobs were scanned, including intermediate commits,
for credential patterns, binary/model/archive payloads and suspicious paths.
No matches were found. Corpus additions were reviewed as synthetic evaluation
material. This bounded scan does not prove absence of every possible secret.
External evidence, model/runtime binaries and user profiles were not uploaded.

## Local checks

- Governance including negative self-tests, diff whitespace and TypeScript: PASS.
- Chromium core: 227 assertions plus unpublished-window regression checks PASS;
  worker: 81 PASS; scheduling: 7 PASS.
- Owned trial cleanup regressions: 10 PASS.
- Existing startup cleanup regression: standalone rerun PASS without code or
  timeout changes. Full Rust library rerun: 181 PASS, 27 ignored.
- Both supervised Rust runs explicitly reaped the declared owned MSVC helper;
  process handles signalled and remaining process count zero. This is
  `BUILD_COMPLETE_HELPERS_REAPED`, not natural compiler-child shutdown.

## Remaining acceptance limits

The earlier startup/deadline and cleanup failures remain recorded; passing these
reruns does not establish their root cause or repair. Latest installed-extension
translation/rewrite/Accept remains NOT_RUN after Windows foreground activation
failed before inference. Hosted source tests do not qualify this UI path.

The final prior real-Gemma evaluation remains 31 changed returns, 5 unchanged and
7 quality refusals across 43 requests with one model process. Refusal is not a
successful translation. Lexical person/title/promise checks are conservative,
not general coreference, semantic equivalence or natural-English guarantees.
Warm engine time is not complete recommendation/Accept latency. GPU, physical
IME, ordinary profiles, general editor coverage and live Providers are not newly
qualified by this publication. The final local package remains a retest candidate.

Historical rejected prompt/candidate commits are preserved as history, not
endorsed release points. Mission remains INCOMPLETE; Project Source remains stale.
Evidence: `D:/dev/grammar-evidence/github-sync-20261023` (local only).

## First hosted run and repair

[Run 39963370820](https://github.com/CAPTW/pencil/actions/runs/39963370820)
on `4ef212f` FAILED in launcher integration. Rust targets passed
181/282/31/3/14 tests respectively (ignored live tests excluded); the two actual
broker pre-ready tests also passed. One launcher observer opened a session file
during replacement and raised `WinError 32` instead of continuing its bounded
observation. This is a test observer race; it is distinct from the unresolved
native startup deadline failure described above.

A forced sharing violation reproduced the same failure against the old loop.
The failing test now reuses `wait_phase` and checks the returned failed snapshot,
avoiding a second racy read. Its original ten-second observation limit, app
survival, no-retry-without-cleanup, competing-launch refusal and removal checks
are retained. Added regressions ensure persistent sharing violations cannot
renew the deadline and malformed JSON is never swallowed. No product safety
check, inference budget or timeout was relaxed.

The five deterministic observer regressions pass locally. The supervised local
launcher-suite rerun also exposed Korean Windows error-output decoding failures
(`UnicodeDecodeError` with the fixture's UTF-8 assumption); it is not counted as
a full launcher PASS. The forced pre-fix race remains recorded even though that
local run also reported output-decoding errors. Hosted verification of the fix
is required; its result is separate from the first failed run.

## Repair validation and local fixture follow-up

[Run 39965158557](https://github.com/CAPTW/pencil/actions/runs/39965158557)
on `c2f3245` passed the complete Windows source workflow, including the formerly
failing launcher integration and forced sharing-violation regression. First run
39963370820 remains FAIL; this is a new run after the observer repair.

Further local validation identified two fixture-environment issues. PowerShell
under a hidden Korean Windows process emitted bytes incompatible with the
Python fixture's UTF-8 decoder. Explicit `[Console]::OutputEncoding =
[Text.UTF8Encoding]::new($false)` in the owned PowerShell parent before running
Python resolved decoding; no global console setting or product file was changed.
With that condition, three uninstall checks correctly refused an inherited
supervisor scope different from their own marked temporary scope. Test setup
now binds `GRAMMAR_TRIAL_ROOT` to the newly created fixture scope and restores
the inherited environment after each case. Production cleanup remains unchanged.

That rerun then exposed one timing assumption: the concurrent-launcher test's
three-second app lifetime could expire before uninstall was attempted. It now
uses the existing bounded release marker and confirms the app is still alive
until the test explicitly releases it in `finally`. No cleanup assertion is
removed and the fixture remains bounded.

Final local launcher suite: **37 PASS**, 114.231 seconds. Owned supervisor
completed cleanup with zero remaining processes; only the declared MSVC helper
was explicitly reaped. Earlier local runs remain FAIL (decoding: 2 failures /
15 errors; UTF-8: 3 scope failures; corrected scope: 1 timing failure).
This does not change the unresolved product startup failure or installed Accept
limits above. Final follow-up commit changes test setup/lifetime and records,
not the shipped product. Its Actions result is identified by its exact commit.
