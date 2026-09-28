# Removes one personal Grammar installation created by Install-Grammar.ps1.
# Stops only processes started from this install root, removes only the exact
# native host registration recorded for it, and deletes the install root. The
# receipt and this script are deleted last (after the user data, when asked),
# so an interrupted or failed run can simply be repeated. -RemoveUserData also
# deletes the app's settings, dictionary, WebView data and app-owned Codex
# sign-in for this Windows user; every Grammar copy of this user shares them.
[CmdletBinding()]
param(
  # Default: the installation this script belongs to, else the standard location.
  [string]$InstallRoot,
  [switch]$RemoveUserData,
  # Tests only: task-owned folders in place of the app's user data folders.
  [string[]]$UserDataDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
if (-not $InstallRoot) {
  $InstallRoot = if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'install-receipt.json') -PathType Leaf) { $PSScriptRoot } else { Join-Path $env:LOCALAPPDATA 'GrammarPersonal' }
}
$root = [IO.Path]::GetFullPath($InstallRoot).TrimEnd('\', '/')
$receiptPath = Join-Path $root 'install-receipt.json'
if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) { throw "Not a Grammar installation (install-receipt.json missing): $root" }
$receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
if ($receipt.schema -cne 'grammar-personal-install/v1' -or [IO.Path]::GetFullPath($receipt.installRoot).TrimEnd('\', '/') -ne $root) {
  throw 'Install receipt does not describe this directory.'
}

$prefix = $root + [IO.Path]::DirectorySeparatorChar
function Test-Owned($Process) { $Process.Path -and $Process.Path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) }
$userData = if ($UserDataDirectory) { $UserDataDirectory } else { @(
  (Join-Path $env:APPDATA 'com.local.codexpencil'),
  (Join-Path $env:LOCALAPPDATA 'com.local.codexpencil')
) }
$anotherCopy = 'Another Grammar copy is running and uses the same settings and dictionary. Quit it, then run the uninstaller again, or run it without -RemoveUserData.'
function Test-OtherCopy { [bool]@(Get-Process -Name 'codex-pencil' -ErrorAction SilentlyContinue | Where-Object { -not (Test-Owned $_) }).Count }
# The data is shared with every other Grammar copy of this Windows user.
if ($RemoveUserData -and (Test-OtherCopy)) { throw $anotherCopy }

# WebView2 helper processes can hold user-data files for a moment after the app exits.
function Remove-Tree([string]$Path) {
  for ($attempt = 1; ; $attempt++) {
    try { Remove-Item -LiteralPath $Path -Recurse -Force; return }
    catch { if ($attempt -ge 10) { throw }; Start-Sleep -Seconds 1 }
  }
}
$owned = @(Get-Process -Name 'codex-pencil', 'grammar-chromium-host' -ErrorAction SilentlyContinue | Where-Object { Test-Owned $_ })
foreach ($process in $owned) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
foreach ($process in $owned) { $process.WaitForExit(10000) | Out-Null }

$problems = @()
$notes = @()
# The registration recorded by the installer, or by the host tool if the
# install stopped before the receipt was updated. A key that is already gone
# is fine; a key that now points elsewhere is not ours and stays.
$registration = $receipt.nativeHost
$keep = @('install-receipt.json', 'Uninstall-Grammar.ps1')
$hostReceipt = Join-Path $root 'host/registration.json'
if (-not $registration -and (Test-Path -LiteralPath $hostReceipt -PathType Leaf)) {
  $recorded = Get-Content -LiteralPath $hostReceipt -Raw | ConvertFrom-Json
  $recordedKey = if ($recorded.PSObject.Properties['registry_key']) { $recorded.registry_key } else { 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\' + $recorded.name }
  $registration = [pscustomobject]@{ name = $recorded.name; registryKey = $recordedKey; manifest = $recorded.manifest }
}
$registryKey = $null
# Ours only while it still points at our manifest.
function Test-OurKey { [bool]($registryKey -and (Test-Path -LiteralPath $registryKey) -and (Get-Item -LiteralPath $registryKey).GetValue('') -eq $registration.manifest) }
if ($registration) {
  if ($registration.name -notmatch '^org\.grammar\.personal\.t[a-f0-9]{32}$' -or -not $registration.registryKey.EndsWith('\' + $registration.name)) {
    throw 'The recorded native host registration is not a Grammar registration; nothing was removed.'
  }
  $registryKey = $registration.registryKey
  if (Test-OurKey) {
    try { Remove-Item -LiteralPath $registryKey } catch { $problems += 'native host registration' }
    # Keep the record a repeated run needs to find the key again.
    if (Test-OurKey) { $keep += 'host' }
  } elseif (Test-Path -LiteralPath $registryKey) {
    $notes += 'The native host key now points to another manifest, so it is not this installation''s and was left untouched.'
  }
}

# Everything except the receipt and this script first; those go last.
foreach ($child in @(Get-ChildItem -LiteralPath $root -Force | Where-Object { $_.Name -notin $keep })) {
  try { Remove-Tree $child.FullName } catch { $problems += $child.FullName }
}
# User data before the receipt: if it cannot be removed now, this script is
# still here to finish the job.
if ($RemoveUserData -and -not $problems) {
  if (Test-OtherCopy) {
    $problems += 'user data (another Grammar copy started meanwhile)'
  } else {
    foreach ($directory in $userData) {
      if (Test-Path -LiteralPath $directory) {
        try { Remove-Tree $directory } catch { $problems += $directory }
      }
    }
  }
}
if (-not $problems) {
  try { Remove-Tree $root } catch { $problems += $root }
}

$remaining = [ordered]@{
  installRoot = Test-Path -LiteralPath $root
  nativeHostRegistration = Test-OurKey
  processes = @(Get-Process -Name 'codex-pencil', 'grammar-chromium-host' -ErrorAction SilentlyContinue | Where-Object { Test-Owned $_ }).Count
  userData = @($userData | Where-Object { Test-Path -LiteralPath $_ }).Count
}
$complete = (-not $remaining.installRoot) -and (-not $remaining.nativeHostRegistration) -and
  ($remaining.processes -eq 0) -and ((-not $RemoveUserData) -or $remaining.userData -eq 0)
$result = [ordered]@{
  schema = 'grammar-personal-uninstall/v1'
  sourceCommit = $receipt.sourceCommit
  stoppedProcesses = $owned.Count
  removedUserData = [bool]$RemoveUserData
  remaining = $remaining
  problems = $problems
  notes = $notes
  complete = $complete
  next = if ($complete) { $null } else { 'Quit Grammar, close windows and terminals that use the install folder, then run this uninstaller again.' }
}
$result | ConvertTo-Json -Depth 4
if (-not $complete) { exit 1 }
