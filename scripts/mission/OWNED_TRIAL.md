# Owned Windows diagnostic sessions

Use `owned_trial.py --output <new external evidence directory> --timeout <seconds> -- <command>`
for synthetic desktop trials. It requires the developer environment's Python and
pywin32; these are not product/runtime dependencies. The tests also use psutil.

The supervisor acquires a same-session mutex, starts its child suspended without a
console, assigns a non-breakaway Windows Job, then resumes it. Concurrent trials
are refused. The Job kills descendants if the supervisor dies. Normal completion,
timeout, and abandoned descendants are distinguished in `supervisor-result.json`.
Forced cleanup never upgrades a failed trial to PASS. Job zero and process-handle
completion are checked before releasing the mutex. Cleanup has a separate bounded
10-second budget, not a renewed trial execution deadline.

Do not put MSIX/unpackaged process-tree transitions inside the supervised tree.
Run the supervisor after the existing explicit transition. Do not run two viewers
or a second desktop trial while a trial is active. An old launcher that does not
use this mutex is not made safe by it: retain the app/model/editor preflight.

Job cleanup closes processes; it does not restore application configuration files.
After interrupted trials, inspect and preserve stale task-owned endpoint/session
files before explicitly preparing a new diagnostic copy. Never delete or overwrite
personal configuration to make a test start. Never treat a modified diagnostic
copy as an unmodified installed-package qualification.

Run `python scripts/mission/test_owned_trial.py` before the interactive trial.
The tests cover normal no-console exit, timeout descendants, abandoned children,
cross-process mutex exclusion, and supervisor termination. They launch only their
own short synthetic Python processes.

For installed review/Accept testing, compile `owned-review-click.cs` with the
existing x64 C# compiler outside the source tree. Arguments are app PID, target
PID, their exact verified executable paths, then the DOM button center x/y,
viewport width/height and devicePixelRatio. The caller must wait for the Accept
button to be enabled and stable, verify package hashes and use only its synthetic
profile. The helper verifies process/window ownership, physical viewport size,
unobscured hit target and actual cursor position before sending mouse input.
An offscreen or unrelated target is a refusal, never a reason to force input.

Do not require a separate helper to force the review foreground before a click:
its foreground privileges differ from the app and the user. Measure from the
trusted DOM click through exact editor reread; also retain helper preparation
and click-to-UI-confirmation durations. Playwright request-to-return is a distinct
measurement and must not be relabelled retroactively as native Apply time.
Tauri 2.11.3's internal invoke is non-writable; observe DOM status with a
MutationObserver instead of replacing IPC internals. This bounds user-observed
completion, not isolated backend time. Preserve missed clicks and failed traces.
