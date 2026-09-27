# Removes one personal Grammar installation created by Install-Grammar.ps1.
# Stops only processes started from this install root, removes only the exact
# native host registration recorded in install-receipt.json, and deletes the
# install root. -RemoveUserData also deletes the app's settings, dictionary,
# WebView and app-owned Codex home for this Windows user.
[CmdletBinding()]
param(
  [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'GrammarPersonal'),
  [switch]$RemoveUserData
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
$root = [IO.Path]::GetFullPath($InstallRoot)
$receiptPath = Join-Path $root 'install-receipt.json'
if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) { throw 'Not a Grammar installation (install-receipt.json missing).' }
$receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
if ($receipt.schema -cne 'grammar-personal-install/v1' -or [IO.Path]::GetFullPath($receipt.installRoot) -ne $root) {
  throw 'Install receipt does not describe this directory.'
}

$prefix = $root.TrimEnd('\') + '\'
# WebView2 helper processes can hold user-data files for a moment after the app exits.
function Remove-Tree([string]$Path) {
  for ($attempt = 1; ; $attempt++) {
    try { Remove-Item -LiteralPath $Path -Recurse -Force; return }
    catch { if ($attempt -ge 10) { throw }; Start-Sleep -Seconds 1 }
  }
}
$owned = @(Get-Process -Name 'codex-pencil', 'grammar-chromium-host' -ErrorAction SilentlyContinue |
  Where-Object { $_.Path -and $_.Path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) })
foreach ($process in $owned) { Stop-Process -Id $process.Id -Force }
foreach ($process in $owned) { $process.WaitForExit(10000) | Out-Null }

if ($receipt.nativeHost) {
  & (Join-Path $root 'host-tools/Remove-Registration.ps1') -PackageDirectory (Join-Path $root 'host')
}
Remove-Tree $root

$userData = @(
  (Join-Path $env:APPDATA 'com.local.codexpencil'),
  (Join-Path $env:LOCALAPPDATA 'com.local.codexpencil')
)
if ($RemoveUserData) {
  foreach ($directory in $userData) {
    if (Test-Path -LiteralPath $directory) { Remove-Tree $directory }
  }
}

$remaining = [ordered]@{
  installRoot = Test-Path -LiteralPath $root
  nativeHostRegistration = [bool]($receipt.nativeHost -and (Test-Path -LiteralPath $receipt.nativeHost.registryKey))
  processes = @(Get-Process -Name 'codex-pencil', 'grammar-chromium-host' -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and $_.Path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) }).Count
  userData = @($userData | Where-Object { Test-Path -LiteralPath $_ }).Count
}
$result = [ordered]@{
  schema = 'grammar-personal-uninstall/v1'
  sourceCommit = $receipt.sourceCommit
  stoppedProcesses = $owned.Count
  removedUserData = [bool]$RemoveUserData
  remaining = $remaining
  complete = (-not $remaining.installRoot) -and (-not $remaining.nativeHostRegistration) -and
    ($remaining.processes -eq 0) -and ((-not $RemoveUserData) -or $remaining.userData -eq 0)
}
$result | ConvertTo-Json -Depth 4
if (-not $result.complete) { exit 1 }
