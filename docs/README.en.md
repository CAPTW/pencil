# Pencil

Local writing assistance for Windows: **select → review → explicitly Accept or Copy**.

[Download Prototype 2](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2) · [한국어 안내](../README.md) · [Build and qualification details](PROTOTYPE.md)

Pencil offers local Gemma translation and natural, concise and polite rewriting,
plus inline suggestions in explicitly enabled Chromium fields. Cloud CLI providers
and OpenAI-compatible Chat Completions are optional, consent-bound Deep paths.
This is an unsigned Windows x64 prerelease, not universal editor support.

## Install and start

1. Prepare **PowerShell 7** and **WebView2** using their Microsoft distributions.
2. Download `Pencil-0.1.0-prototype.2-windows-x64.zip` and `SHA256SUMS.txt` from
   release Assets. “Source code (zip)” is not the executable package.
3. Compare the ZIP's `Get-FileHash -Algorithm SHA256` with the checksums file.
4. Extract into a fresh folder; open PowerShell 7 there.

```powershell
.\Install-Grammar.ps1 -InstallRoot "$env:LOCALAPPDATA\PencilPrototype1"
Set-Location "$env:LOCALAPPDATA\PencilPrototype1"
.\Prepare-GrammarLocal.ps1
.\Start-Grammar.ps1
```

Prepare downloads and verifies the roughly 7 GB Gemma4 12B IT QAT Q4_0 model and
runtime once. Subsequent launches need only Start. Keep the launcher shell open
while using the app and exit through its tray menu. Do not overlap sessions.

The default CUDA 13.4 profile was tested on RTX 5070 Ti. Other GPUs and CPU speed
are unqualified. To skip model download/loading, omit Prepare and launch with
`Start-Grammar.ps1 -WithoutLocalModel`. Basic rules and separately configured Deep
remain; this mode does not provide equivalent local Gemma translation/rewriting.

For desktop review, select text in a qualified Edit/RichEdit control and press
Ctrl+Shift+G. New Notepad, Office and Hancom are not generally qualified. For
Chromium, load the installed `extension` folder in developer mode, pin the icon,
then enable the document and the supported textarea/simple contenteditable field.
Review before Accept. Browser Undo is not guaranteed. Sensitive fields are refused.

## Understand the limits

- Earlier product `792032b`: 14/14 scoped real-Gemma synthetic RichEdit checks;
  native mouse Accept 113–332 ms; short responses 687–1371 ms (2/10 over 1 second);
  four-sentence paragraphs 987–1090 ms; startup 16.164 seconds.
- Prototype 1 (`49fb641`) required forced cleanup of five descendants; that
  historical failure remains recorded. Prototype 2 explicitly closes WebViews,
  releases the UI runtime and waits for browser exit within a 30-second deadline.
  A closed window does not mean background cleanup is finished. See the release
  `VALIDATION.json` for exact packaged-binary checks and timings.
- Physical IME, mixed DPI, arbitrary sites, another physical PC and long-term use
  remain open. This shutdown repair is not new language-quality qualification.
- CLI consent persists; Auto rewrite can send captured text after consent. Turn it
  off for manual control. API Deep uses an explicit action and a session-only key.
- The ZIP is unsigned. Do not disable system security to run it. Third-party/model
  terms still apply; no project-wide open-source license has been assigned.

Read START_HERE.md in the ZIP. Uninstall with its installed
`Uninstall-Grammar.ps1`; `-RemoveUserData` also deletes shared per-user settings,
dictionary and app-owned sign-in data, potentially affecting other installations.
Preserve needed data/models first. Report only synthetic, non-sensitive examples
through the [bug form](https://github.com/CAPTW/pencil/issues/new?template=bug_report.md).
