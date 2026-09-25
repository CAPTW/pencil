[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$ExecutablePath,
  [Parameter(Mandatory = $true)]
  [string]$OutputRoot,
  [Parameter(Mandatory = $true)]
  [string]$ProductCommit,
  [Parameter(Mandatory = $true)]
  [string]$PackagingCommit,
  [Parameter(Mandatory = $true)]
  [string]$GitTree,
  [Parameter(Mandatory = $true)]
  [string]$GitSubject,
  [Parameter(Mandatory = $true)]
  [string]$BuildReceiptPath
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
if (Test-Path -LiteralPath $OutputRoot) { throw "Output root already exists: $OutputRoot" }
$sourceRoot = Split-Path -Parent $PSScriptRoot
# Verify source, actual executed build receipt and binary before creating any output.
$receipt = & (Join-Path $PSScriptRoot 'mission/build-receipt.ps1') -SourceRoot $sourceRoot -ReceiptPath $BuildReceiptPath -ExecutablePath $ExecutablePath -VerifyOnly
$receiptHash = (Get-FileHash -LiteralPath $BuildReceiptPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($ProductCommit -cne $receipt.after.commit -or $PackagingCommit -cne $receipt.after.commit -or
    $GitTree -cne $receipt.after.tree -or $GitSubject -cne $receipt.after.subject) {
  throw 'Caller metadata does not match measured build source'
}
$bundle = Join-Path $OutputRoot 'portable\Grammar'
New-Item -ItemType Directory -Path $bundle -Force | Out-Null
Copy-Item -LiteralPath $ExecutablePath -Destination (Join-Path $bundle 'codex-pencil.exe')
if ((Get-FileHash -LiteralPath (Join-Path $bundle 'codex-pencil.exe') -Algorithm SHA256).Hash.ToLowerInvariant() -cne $receipt.executableSha256) { throw 'Executable changed during packaging' }
Copy-Item -LiteralPath $BuildReceiptPath -Destination (Join-Path $bundle 'BUILD_RECEIPT.json')
if ((Get-FileHash -LiteralPath (Join-Path $bundle 'BUILD_RECEIPT.json') -Algorithm SHA256).Hash.ToLowerInvariant() -cne $receiptHash) { throw 'Receipt changed during packaging' }
$daily = Join-Path $PSScriptRoot 'daily-use'
foreach ($name in @('Launch-Grammar.cmd','Check-Prerequisites.ps1','Verify-Bundle.ps1','Create-Shortcuts.ps1','Remove-Shortcuts.ps1','Enable-Startup.ps1','Disable-Startup.ps1')) {
  Copy-Item -LiteralPath (Join-Path $daily $name) -Destination (Join-Path $bundle $name)
}
$utf8 = New-Object System.Text.UTF8Encoding $false
function Write-Utf8([string]$Path, [string]$Text) {
  [System.IO.File]::WriteAllText($Path, $Text.TrimStart("`r","`n") + "`n", $utf8)
}
Write-Utf8 (Join-Path $bundle 'README_FOR_DAILY_USE.md') @'
# Grammar daily use

Unsigned personal-use bundle. Extract the ZIP to a stable folder.

1. Run `Verify-Bundle.ps1`
2. Run `Check-Prerequisites.ps1`
3. Launch `Launch-Grammar.cmd` or `codex-pencil.exe`
4. Open from tray or Ctrl+Shift+G
5. Use local Instant without a Provider
6. Select one Provider for Deep
7. Use Test connection before the first Deep request
8. Review the result, then Apply explicitly
9. Use Copy diagnostics after a defect
10. Quit from the tray
11. After the folder is final, optionally run `Create-Shortcuts.ps1`
12. Optionally run `Enable-Startup.ps1`
13. Reverse with `Disable-Startup.ps1` and `Remove-Shortcuts.ps1`

Settings stay in per-user AppData (`%APPDATA%\com.local.codexpencil\`). Moving the bundle does not delete settings. Recreate shortcuts after moving the folder.
'@
Write-Utf8 (Join-Path $bundle 'PROVIDER_SETUP.md') @'
# Provider setup

Providers are external official clients. This bundle does not include Codex, `agy`, or Claude, and does not store credentials.

- Codex: official CLI, ChatGPT-managed sign-in in Settings
- Antigravity: official `agy`; sign in once in a terminal if needed
- Claude: official Claude Code; Settings Sign in
- Missing or signed-out Providers do not disable local Instant
- Last-observed statuses are historical, not guaranteed now
- Use Settings → Refresh and Test connection
- No silent fallback across Providers
'@
Write-Utf8 (Join-Path $bundle 'MANUAL_SMOKE_CHECKLIST.md') @'
1. Verify bundle
2. Check prerequisites
3. Launch
4. Show Grammar
5. Open Settings
6. Confirm all three Provider cards
7. Select a short synthetic sentence
8. Run local Instant
9. Review and explicitly Apply when desired
10. Run Test connection for an authenticated Provider
11. Copy diagnostics after an error
12. Quit from tray
13. Confirm no Grammar or Provider process remains
'@
Write-Utf8 (Join-Path $bundle 'KNOWN_ISSUES.md') @'
- Unsigned personal-use artifact
- Microsoft Edge WebView2 Runtime is required
- Provider clients and authentication are external
- Codex and Claude were last observed signed out
- Antigravity was last observed returning an official structured external ERROR
- Local client versions may change independently
- P3-B is not implemented
- Whole-product formal acceptance is not inferred
'@
$exe = Join-Path $bundle 'codex-pencil.exe'
$exeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLower()
$exeLen = (Get-Item -LiteralPath $exe).Length
$info = [ordered]@{
  schemaVersion = 1
  productName = 'Grammar'
  artifactClassification = 'UNSIGNED_PERSONAL_DAILY_USE_BUNDLE'
  buildUtc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
  gitCommit = $receipt.after.commit
  productCommit = $receipt.after.commit
  gitTree = $receipt.after.tree
  gitSubject = $receipt.after.subject
  executableRelativePath = 'codex-pencil.exe'
  executableBytes = $exeLen
  executableSha256 = $exeHash
  settingsSchemaVersion = 7
  writingContractVersion = 1
  providerKinds = @('codex','antigravity','claude')
  sourceWorktreeClean = $receipt.after.clean
  sourceTrackedBytesSha256 = $receipt.after.trackedBytesSha256
  buildReceiptSha256 = $receiptHash
  buildCommand = $receipt.command
  bundleVersion = '2026-09-06.r8'
  signingState = 'NotSigned'
  updaterState = 'disabled'
}
$infoJson = $info | ConvertTo-Json -Depth 4
Write-Utf8 (Join-Path $bundle 'BUILD_INFO.json') $infoJson
$sumLines = @()
Get-ChildItem -LiteralPath $bundle -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' } | Sort-Object Name | ForEach-Object {
  $h = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower()
  $sumLines += "$h  $($_.Name)"
}
Write-Utf8 (Join-Path $bundle 'SHA256SUMS.txt') (($sumLines -join "`n") + "`n")
$null = & (Join-Path $PSScriptRoot 'mission/build-receipt.ps1') -SourceRoot $sourceRoot -ReceiptPath (Join-Path $bundle 'BUILD_RECEIPT.json') -ExecutablePath $exe -VerifyOnly
& (Join-Path $bundle 'Verify-Bundle.ps1')
if ($LASTEXITCODE -ne 0) { throw 'bundle verifier failed' }
$zip = Join-Path $OutputRoot 'Grammar-portable.zip'
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($bundle, $zip)
Write-Output "BUNDLE=$bundle"
Write-Output "ZIP=$zip"
Write-Output "EXE_SHA=$exeHash"
Write-Output "EXE_BYTES=$exeLen"
