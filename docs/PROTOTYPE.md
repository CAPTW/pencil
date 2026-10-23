# Pencil — build and qualification reference

For installation, start with the [Korean guide](GETTING_STARTED.ko.md) or
[English overview](README.en.md). [Prototype 2 is published](https://github.com/CAPTW/pencil/releases/tag/v0.1.0-prototype.2).
The exact package source and checks are recorded in MANIFEST.json and the release VALIDATION.json; main may contain later source changes; an existing release ZIP is not updated by a source merge.
This is not a production release or a claim of Grammarly-equivalent language quality.

## What works within the tested scope

- Select text, press Ctrl+Shift+G, review/edit the local suggestion, explicitly
  Accept or Copy. Exact target/source/range checks can refuse an unsafe Accept.
- Local Gemma translation and natural, concise and polite rewriting. The baseline
  remains Gemma4 12B IT QAT Q4_0; no smaller-model silent fallback.
- Opted-in Chromium textarea/simple contenteditable recommendations and Accept.
  Loading the extension does not authorize every document or field.
- Optional cloud Deep: selected CLI provider or configured OpenAI-compatible API.
  This is not support for every provider protocol. Consent and external costs are
  separate; local qualification is not live cloud-provider qualification.

Desktop direct Accept is limited to qualified Windows Edit/RichEdit controls.
Modern Notepad, Word, Excel, PowerPoint and Hancom are not generally qualified.
The optional paired Writer adapter has separate bounded evidence; it is not
included in the default package's support claim. No automatic background Apply.

## Build and install from source

Prerequisites: Windows x64, PowerShell 7, Git, Node/npm, Rust MSVC toolchain,
Visual Studio C++ build tools with Windows SDK and release CRT, and WebView2.
The owned build supervisor also requires Python with pywin32.
For the executable ZIP, read START_HERE.md included in the GitHub prerelease.
The commands below instead build a fresh package from source.
Use an empty output directory outside the checkout and keep the checkout clean.

```powershell
git clone --branch codex/grammar-local-trial-ba58b7d https://github.com/CAPTW/pencil.git
cd pencil
npm ci
cargo fetch --locked --manifest-path src-tauri/Cargo.toml
$output = Join-Path $env:TEMP ('pencil-build-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $output | Out-Null
python scripts/mission/owned_build.py --output "$output/supervisor" --timeout 1200 -- pwsh -NoProfile -File scripts/mission/build-receipt.ps1 -SourceRoot . -ReceiptPath "$output/build-receipt.json"
if ($LASTEXITCODE -ne 0) { throw 'Build or owned-process cleanup failed' }
# Stop if the build failed. Packaging verifies the clean source and binary hashes.
pwsh -NoProfile -File scripts/mission/package.ps1 -ReceiptPath "$output/build-receipt.json" -OutputRoot "$output/package"
# Choose a NEW per-user install directory; do not overwrite an existing install.
pwsh -NoProfile -File "$output/package/Install-Grammar.ps1" -InstallRoot "$env:LOCALAPPDATA/PencilPrototype"
```

The supervisor owns only its build process tree. MSVC's `vctip.exe` may remain
after compilation; its exact installed path is declared before this build and
the owned instance is terminated and awaited at completion. The receipt calls
this `BUILD_COMPLETE_HELPERS_REAPED`, not natural shutdown. Other leftover
children still fail the build. No global process-name cleanup or registry
telemetry change is performed. Product tests must use the strict
`owned_trial.py` supervisor, without declaring build helpers.

From the installation directory, explicitly run `Prepare-GrammarLocal.ps1` with
PowerShell 7 to fetch and verify the pinned model/runtime, then `Start-Grammar.ps1`.
The model is roughly 7 GB; allow additional runtime and build disk space. Downloads
and first preparation are separate from response latency. Model/runtime files are
not committed to this repository. Respect their upstream licenses and terms;
public source availability does not change third-party redistribution rights.

The default CUDA 13.4 profile was qualified on an RTX 5070 Ti. CUDA 12.4 is an
explicit packaging alternative, not a guarantee for other GPUs. CPU/low-VRAM
interactive quality and speed are not qualified. Use `Start-Grammar.ps1
-WithoutLocalModel` to avoid model loading; basic rules remain, and external Deep
requires separate configuration/consent. No driver installation or smaller model
is performed automatically.

For Chromium, load the installed `extension` directory in developer mode. Start
with a separate profile and synthetic text, enable the document and then the
field, and review before Accept. See [personal-use instructions](../scripts/package/PERSONAL_USE.md)
for opt-in, shortcuts, installation and `Uninstall-Grammar.ps1 -RemoveUserData`.
Removing user data is optional and deletes the installation's settings/dictionary.

## Latest installed desktop measurement

Product `792032b`, 2026-10-14, one owned Windows/RichEdit session, actual unchanged
Gemma baseline: 14/14 capture, reviewed output and Accept/exact reread checks passed.
Seven used actual hit-tested mouse input; seven used CDP comparison input.

| Observation | Result |
| --- | --- |
| Native mouse DOM click to exact reread | 113–332 ms, 0/7 above 500 ms |
| Short complete responses | 687–1371 ms, 2/10 above 1 second |
| Four-sentence paragraphs | 987–1090 ms, 0/4 above 3 seconds |
| Model readiness | 16.164 seconds |
| Output quality check | 14 matched a previously reviewed small corpus |

These small-sample observations are not hardware-independent promises or a broad
language-quality benchmark. The backend-only Accept duration was not measured.
The review-window offscreen placement defect was fixed. Earlier intermittent
capture/Accept failures remain in the history; their individual causes are not
all established. Two WebView children needed the owned test supervisor's cleanup:
the functional fixture passed, but natural descendant shutdown did not. Final
owned process count was zero. Do not launch overlapping test sessions.

Physical IME, mixed DPI, long-term daily use, arbitrary sites/editors, current
binary reproducibility and broad low-GPU use remain unqualified. Current source
checks in GitHub do not reproduce the local GPU/desktop acceptance. Detailed
historical evidence references are in [HANDOFF](control/HANDOFF.md); referenced
local evidence files are not public artifacts. Later documentation commits are
not new measurements of rebuilt binaries. Mission acceptance remains incomplete.

Prototype 2 has been adopted into main and published as an unsigned Windows x64 ZIP
prerelease. It excludes model weights and downloaded GPU runtime binaries. No new source license is granted
by this preparation; a project-wide license decision remains with the owner.

## Prototype 2 shutdown repair

WebView controllers close before the event loop returns. The application then
waits for its browser process handles, pumping pending STA messages, within one
30-second deadline started before closure. A timeout exits with failure, not
confirmed cleanup. No process-name sweep or external process termination is used.
The owned trial Job remains an independent backstop; forced trial cleanup is FAIL.
Earlier 10-second candidates failed. Separate causal observations of prototype.1
and a candidate took about 22 seconds to drain naturally, motivating a documented
shutdown budget rather than a claim of faster inference or instant shutdown.
The first bounded candidate completed in 12.4 seconds with no forced cleanup;
use the release receipts for the final packaged binary measurements.

The final installed package passed three sequential no-model shutdowns and one
real Gemma readiness/shutdown in a single owned session, with no forced cleanup
and no remaining processes. Observed shutdown was 12.8–18.5 seconds; fresh-profile
readiness was 21.7–31.6 seconds (the last case includes Gemma readiness). These are
local observations, not a speed guarantee. An earlier trial that immediately
closed its CDP connection failed; its cause remains unproven. Final checks kept
CDP attached and required explicit acknowledgement of the production Quit command.
Physical tray-click shutdown and arbitrary debugger disconnect are not qualified.

Historical prototype.1 and 792032b failures above remain valid historical evidence.
This repair does not add general editor support or live cloud-provider acceptance.

## Current source checkpoint — 2026-10-18 (not a new release ZIP)

- User-facing application, tray, extension and Writer display names are Pencil.
  Legacy executable names, installation paths, settings and Native Host identifiers
  stay compatible. The Grammar analysis mode means grammar correction.
- The packaged rHWP integration has bounded actual Gemma evidence for synthetic
  HWP/HWPX translation and rewriting, explicit Accept, full text/format reread,
  Undo, save and reopen. This does not qualify the Hancom desktop applications.
- Proven ordered emphasis mappings can skip the second model inference. Unknown
  mappings retain the existing model path and strict decoder. No weaker model,
  shorter output, persistent document cache or relaxed Apply check was introduced.
- The first same-package trial passed 54 cases / 177 checks. In the 36 paired Korean
  cases the output was identical to the prior checkpoint and recommendation median
  fell from 2.421s to 1.430s. Three requests took 3.50–5.20s: p95 was worse, not a
  performance PASS. All 54 Accepts were 41–186ms. These are shared-load synthetic
  observations, not a controlled hardware A/B or a general 1s claim.
- User data removal now requires explicit, marked trial-contained paths. Normal
  uninstall preserves shared AppData; prior deletion/recovery limitations remain
  recorded in HANDOFF, not silently resolved by this repair.

The source-bound local package is built from dccc246. Latest evidence and repeat
measurements are in the current HANDOFF; existing Prototype 2 release remains unchanged.
