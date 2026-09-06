# Codex Pencil personal bundle workflow

This workflow produces the private-use Windows x64 portable ZIP for P2-01. The authorized runtime ancestor is P1-03 commit `73af3d93231d06e8370d1da11ff6b2bb76ff3a1d`. The build script accepts only a clean `main` commit descended from that ancestor whose baseline diff is limited to the approved packaging workflow and documentation paths.

## Artifact policy

- The portable ZIP is mandatory.
- The bundle is unsigned; `NotSigned` is the expected private-use Authenticode status.
- There is no updater, updater artifact, release channel, timestamp, public upload, or bundled Codex CLI.
- `codex-cli 0.144.6` remains an exact external runtime prerequisite.
- MSI is not produced.
- Optional NSIS is not part of the mandatory workflow. P2-01 records `NOT_ATTEMPTED_BY_POLICY`; producing an installer requires a separately chosen local-only optional run and may not alter the verified portable files.
- Existing commit-bound output roots are never overwritten.

The five metadata files plus exactly one application executable form the six portable entries:

1. `codex-pencil.exe`
2. `README_PERSONAL_USE.md`
3. `BUILD_PROVENANCE.json`
4. `ARTIFACT_INDEX.json`
5. `SHA256SUMS.txt`
6. `VERIFY_BUNDLE.ps1`

`ARTIFACT_INDEX.json` hashes the four non-self-referential payload files. `SHA256SUMS.txt` hashes every portable file except itself, including `ARTIFACT_INDEX.json`. The output-root index records the ZIP hash separately, avoiding self-referential archive provenance.

## Read-only preflight ledger

The P2-01 preflight observed these content-free values before mutation:

| Item | Observed value |
| --- | --- |
| Product / version | Codex Pencil / 0.1.0 |
| Identifier | `com.local.codexpencil` |
| Main binary | `codex-pencil.exe` |
| Target / architecture | `x86_64-pc-windows-msvc` / x64 |
| Accepted Tauri bundle setting | active, targets `all`; mandatory workflow overrides bundling with `--no-bundle` |
| Frontend command / configured dist | `npm run build:frontend` / `../dist`; workflow supplies an external build directory |
| Icons | three configured paths, all present |
| Updater configuration/dependencies | absent / 0 |
| Windows signing configuration | absent |
| Current-user certificates / usable code-signing identities | 7 / 0 |
| Node / npm | v24.18.1 / 11.11.0 |
| rustc / cargo | 1.97.1 / 1.97.1 |
| Local Tauri CLI | 2.11.3 |
| Codex CLI | 0.144.6 |
| Cached NSIS/WiX tool discovery | present; no bundler command invoked during preflight |

Retained authority hashes:

- Protocol bundle: `c954593823626b5194b7f687c27d542eb87fcfa63e5e11af3ca95f9b6c39c6e8`
- `package-lock.json`: `c3415432201286856ec3ca854b4931c13e947c131905a6bfc21242a6bfc2e669`
- `src-tauri/Cargo.lock`: `524f11ebf0d741837761d4dca175d48ac908fcaa14be9ea84800ab344f893cce`

## Prerequisites

- Windows 10 or 11 x64 with Microsoft Edge WebView2 Runtime.
- Existing locked `node_modules`; do not run install or update as part of packaging.
- Existing Rust MSVC target and cached Cargo dependencies.
- Local Tauri CLI supplied by the locked repository dependencies.
- Exact external `codex-cli 0.144.6` resolution.
- A clean `main` worktree at the expected packaging commit.

The mandatory path sets Cargo offline mode, uses `--locked`, builds the frontend and Cargo target into unique owned temporary directories, and invokes Tauri with `--no-bundle` and `--no-sign`. It does not write source `dist` or `src-tauri/target`.

## Build

Run from the repository root after creating the packaging workflow commit:

```powershell
$packagingHead = (git rev-parse HEAD).Trim()
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-personal-bundle.ps1 `
  -ExpectedSourceCommit $packagingHead `
  -OutputBase 'D:\dev\artifacts\Codex-Pencil\P2-01'
```

The final output root is commit-bound:

```text
D:\dev\artifacts\Codex-Pencil\P2-01\<PACKAGING_HEAD>\
  portable\
    Codex-Pencil-0.1.0-x64\
      codex-pencil.exe
      README_PERSONAL_USE.md
      BUILD_PROVENANCE.json
      ARTIFACT_INDEX.json
      SHA256SUMS.txt
      VERIFY_BUNDLE.ps1
  Codex-Pencil-0.1.0-x64-portable.zip
  OUTPUT_ARTIFACT_INDEX.json
  OUTPUT_SHA256SUMS.txt
  verification\
```

## Verify

Verify the ZIP with the tracked verifier:

```powershell
$packagingHead = (git rev-parse HEAD).Trim()
$root = Join-Path 'D:\dev\artifacts\Codex-Pencil\P2-01' $packagingHead
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-personal-bundle.ps1 `
  -BundlePath (Join-Path $root 'Codex-Pencil-0.1.0-x64-portable.zip') `
  -ExpectedSourceCommit $packagingHead
```

After extraction, run the included verifier from the portable directory:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\VERIFY_BUNDLE.ps1 `
  -BundlePath . `
  -ExpectedSourceCommit <PACKAGING_HEAD>
```

The verifier rejects traversal, duplicate archive names, symlinks/reparse points, unexpected files, hash/provenance drift, non-x64 PE files, signing, updater artifacts, protected-value patterns, the local username, and local absolute paths.

## Troubleshooting

The workflow fails closed with content-free codes:

- `BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH`: branch, commit, cleanliness, ancestry, tool pin, or retained hash differs.
- `BLOCKED_GRAMMAR_P2_01_OUTPUT_ROOT_ALREADY_EXISTS`: the commit-bound child already exists; do not delete or overwrite it in this workflow.
- `BLOCKED_GRAMMAR_P2_01_UNEXPECTED_RELEASE_CONFIGURATION`: signing or updater behavior appeared in accepted source.
- `BLOCKED_GRAMMAR_P2_01_RUNTIME_SCOPE_DRIFT`: baseline-to-commit changes are outside the packaging-only allowlist.
- `BLOCKED_GRAMMAR_P2_01_BUILD_FAILED`: an offline frontend or Tauri no-bundle build failed.
- `BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED`: archive, manifest, PE, Authenticode, updater, or privacy verification failed.

On a failed build, the script reports its uniquely owned temporary root for evidence and does not reuse it. On complete success, that owned temporary root is removed.

## R8 daily-use portable bundle

Current Owner handoff is the R8 portable folder and ZIP, not the P2-01 formal packaging chain above.

From a clean worktree:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-daily-use-bundle.ps1 `
  -ExecutablePath 'path\to\codex-pencil.exe' `
  -OutputRoot 'D:\dev\artifacts\Codex-Pencil\GRAMMAR-PERSONAL-DAILY-USE-R8\<UTC>' `
  -ProductCommit '<product-commit>' `
  -PackagingCommit '<packaging-commit>' `
  -GitTree '<tree>' `
  -GitSubject '<subject>'
```

The portable root is `portable\Grammar\`. Helpers are relative to that folder. Settings remain in per-user AppData. Shortcuts and startup are optional, explicit, and reversible. Provider clients stay external.
