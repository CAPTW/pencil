# Pencil 0.1.0 Prototype 3 — Windows x64

This is an unsigned experimental build. Start with synthetic text. Do not use
confidential text for cloud Provider tests. No administrator installation required.

1. Install PowerShell 7 and Microsoft WebView2 from their official distributions
   if missing. Windows PowerShell 5.1 is insufficient. No developer tools required.
2. Compare the downloaded ZIP's SHA-256 with SHA256SUMS.txt from the release:
   `Get-FileHash .\Pencil-0.1.0-prototype.3-windows-x64.zip -Algorithm SHA256`.
3. Extract the ZIP to a new folder. Open **PowerShell 7** in that folder. After
   verifying the download, Windows may require unblocking the downloaded scripts;
   unblock only these verified files, never disable system execution policy.
4. Run `./Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA/PencilPrototype3"`.
   An existing installation is not overwritten. Read the reported installation path.
5. Change to that installed folder, run `./Prepare-GrammarLocal.ps1` once, then
   `./Start-Grammar.ps1`. Preparation explicitly downloads and hash-checks roughly
   7 GB of Gemma model plus the pinned runtime. Allow ample disk space and time.
   No model download or driver change occurs during installation itself.

The default local profile was tested on RTX 5070 Ti with CUDA 13.4 runtime. Other
GPUs, low VRAM and CPU response speed are NOT qualified. For a PC without suitable
GPU capacity, run `./Start-Grammar.ps1 -WithoutLocalModel` instead: basic local
rules and separately configured consent-bound Deep are available; Gemma quality
and latency do not apply to this mode. There is no automatic small-model fallback.

Select text in a qualified Windows Edit/RichEdit control, press Ctrl+Shift+G,
review/edit the result, then Accept or Copy. New Notepad and arbitrary Word/Excel/
Hancom controls are not qualified. Rejecting an unsupported target is expected.
Chromium users can load the installed `extension` folder in developer mode; first
use a new profile, pin its icon and explicitly enable the document and field.
General sites, Edge and IME are not broadly qualified. See PERSONAL_USE.md.

Exit through the tray before starting again. Never start multiple trial sessions.
Use `./Uninstall-Grammar.ps1` from the installed directory to remove the app;
Normal removal preserves settings/dictionary. `-RemoveUserData` is restricted
to explicitly marked test scopes; it does not delete shared AppData.

This build is not signed and does not guarantee SmartScreen reputation. Do not
disable security software or system policies. If organizational policy blocks
unsigned apps, this prototype cannot be used there. See PROTOTYPE.md for measured
limits and THIRD_PARTY_NOTICES for dependencies. Models/runtime have separate
upstream terms; this ZIP does not contain them.

## Prototype 3 limits — read before installing

This ZIP incorporates incremental request supersession and source-person/title
checks. Latest Windows source CI passed, but the latest installed extension's
actual Gemma translation/rewrite/Accept could not be requalified because browser
activation failed before inference. Earlier scoped UI evidence does not qualify
this new binary. Start with disposable synthetic documents and inspect results.

Earlier startup/cleanup timeout failures remain unresolved despite successful
reruns. Slow disks and competing I/O can delay full model verification and warmup.
One-second recommendations, half-second Accept and three-second paragraphs are
targets, not guarantees. Rejected outputs are not successful translations;
conservative guards can reject valid wording and cannot prove general meaning,
subject/gender binding or natural English. Physical IME, ordinary browser profiles,
Edge, live cloud Providers and arbitrary Office/Hancom editors remain unqualified.
Writer/rHWP integrations have separate evidence and are not bundled in this ZIP.

Install side-by-side into the new path above; do not overwrite another version.
Do not use -RemoveUserData when removing an old version if you want to keep your
settings/dictionary. The extension ID is package-specific; record the ID printed
by the installer and keep the old version closed during the trial. No settings
migration or general-profile deployment was performed for this release.
