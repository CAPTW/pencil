# Installed target-side Apply qualification

Run only inside the existing owned Windows Sandbox, at restricted Medium
integrity, with networking/clipboard redirection disabled. Never use a host
desktop, real document or user browser profile. These are synthetic checks;
physical keyboard/IME, arbitrary editors and live Providers are not qualified.

Build a clean app/package with `build-receipt.ps1` and `package.ps1` (receipt
schema 2), including the DLL bound to that app hash. Stage the package as
`C:\GrammarPayload\package-target-apply` without changing any installed file.
Build `native-target-service-probe.cpp` as the owned RichEdit target fixture and
`build-target-editor.ps1 -OutputPath <fresh outside-repository target-editor.exe>`
as its controller. Stage both executables, `target-app-e2e.mjs`, pinned local
Node/Playwright dependencies, `host-machine.txt`, and `Guest-App-TargetApply.ps1`
alongside the existing restricted `MediumLaunch.exe` helper. Payload is read-only
in the guest; evidence maps to `C:\GrammarOutput`. Stop every guest before
modifying its mapped payload. The compiled controller cannot run outside WDAG
or elevated; it owns and checks the target process on every command.

The guest driver installs to a fresh `C:\GrammarTrial`, verifies app/DLL receipt
binding, disables auto Deep without granting cloud consent, and exercises actual
hotkey capture, local Instant, edited draft and the visible Apply button. It
re-reads target text after normal Apply, replay, source/range changes, password
revocation, Dismiss and separate explicit Copy. App and editor must exit normally;
uninstall must remove its root. Missing checks, rejection of normal Apply or
forced cleanup fail the run. Keep failed runs and exact source/binary hashes.

The controller uses synthetic input only on its owned target; product code does
not regain the removed Ctrl+V/clipboard-restore implementation. This scope is
RichEdit50W with explicit non-sensitive input scope, not standard Edit or general
Windows editors. Re-run against a newly built package after product changes;
passing standalone native probes never substitutes for installed-app evidence.
