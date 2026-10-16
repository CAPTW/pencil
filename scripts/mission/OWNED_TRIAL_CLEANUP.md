# Owned trial cleanup

New packages include `New-GrammarTrialScope.ps1`. Before a test installation, call
it with a fresh, explicit root (or an already marked root). Install into `root/install`;
settings and WebView data use `root/profile`. It refuses to adopt an existing
unmarked directory and refuses junctions. Under `owned_trial.py`, any nested scope
must remain inside the supervisor's exported `GRAMMAR_TRIAL_ROOT`.

Pass both InstallRoot and TrialRoot to the installed uninstaller, plus an explicit
UserDataDirectory only when deleting synthetic profiles. Omitted data paths never
mean shared AppData. Do not replace this with raw recursive deletion. Preserve
failed evidence; an unsupported old package missing the scope helper must stop.

The CI package driver exports the scope/profile variables to subsequent CI steps.
Its failure registry root is `HKCU:\Software\PencilTrials\<owner-id>`, bound by the
scope marker; ordinary Chrome registrations remain distinct. Sandbox guest scripts
now use `C:\PencilTrialScope\install` and `C:\PencilTrialScope\profile`; their JS and
Writer packaging expectations were updated together. Old archived payloads must be
rebuilt before using these revised guest entry points.

Run `test-installed-cleanup.ps1 -PackageRoot <extracted-package> -TrialRoot <fresh-root>`
inside the supervisor. It installs real package bytes but never launches the app,
model or Provider. It checks unchanged outside sentinels/receipt/registration on
refusal, then removes only the installed test and profile. Additional regression
cases can use `PENCIL_UNINSTALL_SCRIPT` and `PENCIL_SCOPE_SCRIPT` to point the Node
cleanup tests at extracted package scripts rather than repository copies.

These guards constrain the maintained cleanup entry points. They are not OS
sandboxing of arbitrary historical scripts. Never run old external evidence
launchers simply because their prior report passed; migrate their paths and use
current package cleanup scripts first.
