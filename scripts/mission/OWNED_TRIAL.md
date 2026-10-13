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
