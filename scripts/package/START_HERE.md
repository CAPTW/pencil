# Pencil 0.1.0 Prototype 1 — Windows x64

This is an unsigned experimental build. Start with synthetic text. Do not use
confidential text for cloud Provider tests. No administrator installation required.

1. Install PowerShell 7 and Microsoft WebView2 from their official distributions
   if missing. Windows PowerShell 5.1 is insufficient. No developer tools required.
2. Compare the downloaded ZIP's SHA-256 with SHA256SUMS.txt from the release:
   `Get-FileHash .\Pencil-0.1.0-prototype.1-windows-x64.zip -Algorithm SHA256`.
3. Extract the ZIP to a new folder. Open **PowerShell 7** in that folder. After
   verifying the download, Windows may require unblocking the downloaded scripts;
   unblock only these verified files, never disable system execution policy.
4. Run `./Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA/PencilPrototype1"`.
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
add `-RemoveUserData` only if you also want to delete its settings/dictionary.

This build is not signed and does not guarantee SmartScreen reputation. Do not
disable security software or system policies. If organizational policy blocks
unsigned apps, this prototype cannot be used there. See PROTOTYPE.md for measured
limits and THIRD_PARTY_NOTICES for dependencies. Models/runtime have separate
upstream terms; this ZIP does not contain them.
