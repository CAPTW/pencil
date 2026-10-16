#Requires -Version 7.0
# Personal installation of a verified Grammar package (current user only).
# Verifies every packaged file against MANIFEST.json before copying anything,
# installs into a fresh directory, and registers the Chromium native host for
# the package's fixed extension ID. No account, Provider or network is used.
# A failed install removes what it created. If that cleanup is incomplete, the
# install root keeps its receipt and Uninstall-Grammar.ps1, which finish it.
[CmdletBinding()]
param(
  [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'GrammarPersonal'),
  [switch]$SkipBrowserHost,
  # Tests only: a task-owned registry key instead of Chrome's per-user key.
  [string]$NativeHostsKey = 'HKCU:\Software\Google\Chrome\NativeMessagingHosts'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
$utf8 = [Text.UTF8Encoding]::new($false)
$package = $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $package 'MANIFEST.json') -Raw | ConvertFrom-Json
$listed = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($file in $manifest.files) {
  $path = Join-Path $package $file.path
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Packaged file missing: $($file.path)" }
  if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $file.sha256) {
    throw "Packaged file changed: $($file.path)"
  }
  [void]$listed.Add($file.path)
}
# Folders are copied whole, so a file MANIFEST.json does not list would be
# installed unchecked. Extract the ZIP into an empty folder.
foreach ($item in @(Get-ChildItem -LiteralPath $package -File -Recurse -Force)) {
  $relative = [IO.Path]::GetRelativePath($package, $item.FullName).Replace('\', '/')
  if ($relative -cne 'MANIFEST.json' -and -not $listed.Contains($relative)) {
    throw "Package contains a file MANIFEST.json does not list: $relative. Extract the ZIP into an empty folder."
  }
}
if ($manifest.extensionId -notmatch '^[a-p]{32}$') { throw 'Package has no fixed extension ID.' }

$root = [IO.Path]::GetFullPath($InstallRoot).TrimEnd('\', '/')
if (Test-Path -LiteralPath $root) {
  throw "InstallRoot already exists: $root. To upgrade, run its Uninstall-Grammar.ps1 first (settings and dictionary stay; shared AppData removal is disabled), or choose a new directory."
}
# Ask the packaged app's own loader before creating files or registry entries.
# No GUI, user settings or Provider initialization occurs in this mode.
$probe = [Diagnostics.ProcessStartInfo]::new((Join-Path $package 'codex-pencil.exe'))
$probe.UseShellExecute = $false
$probe.CreateNoWindow = $true
$probe.ArgumentList.Add('--check-webview2')
$process = [Diagnostics.Process]::Start($probe)
try {
  if (-not $process.WaitForExit(30000)) {
    $process.Kill($true)
    if (-not $process.WaitForExit(5000)) { throw 'WebView2 prerequisite probe did not terminate; nothing installed.' }
    throw 'WebView2 prerequisite probe timed out; nothing installed.'
  }
  if ($process.ExitCode -ne 0) {
    throw "WebView2 runtime is unavailable (probe exit $($process.ExitCode)); nothing installed. Install Microsoft WebView2 Evergreen Runtime from https://developer.microsoft.com/microsoft-edge/webview2/ and retry."
  }
} finally { $process.Dispose() }
$receiptPath = Join-Path $root 'install-receipt.json'
$receipt = [ordered]@{
  schema = 'grammar-personal-install/v1'
  state = 'incomplete'
  sourceCommit = $manifest.sourceCommit
  classification = $manifest.classification
  installRoot = $root
  installedUtc = [DateTime]::UtcNow.ToString('o')
  appExecutable = Join-Path $root 'codex-pencil.exe'
  extensionDirectory = Join-Path $root 'extension'
  extensionId = $manifest.extensionId
  nativeHost = $null
}
function Write-InstallReceipt { [IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 4), $utf8) }

New-Item -ItemType Directory -Path $root | Out-Null
try {
  # The uninstaller and the receipt come first, so an interrupted install can
  # always be removed with the uninstaller inside the install root.
  Copy-Item -LiteralPath (Join-Path $package 'Uninstall-Grammar.ps1') -Destination (Join-Path $root 'Uninstall-Grammar.ps1')
  Write-InstallReceipt
  foreach ($item in @('codex-pencil.exe', 'grammar-chromium-host.exe', 'MANIFEST.json', 'BUILD_RECEIPT.json', 'PERSONAL_USE.md', 'README.md', 'HANDOFF.md')) {
    Copy-Item -LiteralPath (Join-Path $package $item) -Destination (Join-Path $root $item)
  }
  foreach ($localFile in @('Start-Grammar.ps1','Prepare-GrammarLocal.ps1','Gemma-Assets.json','New-GrammarTrialScope.ps1')) {
    if ($listed.Contains($localFile)) { Copy-Item -LiteralPath (Join-Path $package $localFile) -Destination (Join-Path $root $localFile) }
  }
  if ($listed.Contains('grammar-gemma-broker.exe')) {
    Copy-Item -LiteralPath (Join-Path $package 'grammar-gemma-broker.exe') -Destination (Join-Path $root 'grammar-gemma-broker.exe')
  }
  if ($listed.Contains('target-apply.dll')) {
    Copy-Item -LiteralPath (Join-Path $package 'target-apply.dll') -Destination (Join-Path $root 'target-apply.dll')
  }
  if ($manifest.PSObject.Properties['vcRuntime']) {
    $crtRoot = Join-Path $root 'vc-runtime'
    New-Item -ItemType Directory -Path $crtRoot | Out-Null
    foreach ($relative in @($manifest.vcRuntime.files)) {
      if ($relative -cnotmatch '^vc-runtime/[A-Za-z0-9_]+\.dll$' -or -not $listed.Contains($relative)) { throw 'Invalid CRT manifest entry' }
      $installed = Join-Path $root $relative
      Copy-Item -LiteralPath (Join-Path $package $relative) -Destination $installed
      $entry = @($manifest.files | Where-Object path -CEQ $relative)
      if ($entry.Count -ne 1 -or (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry[0].sha256) { throw 'Installed CRT hash mismatch' }
    }
    Copy-Item -LiteralPath (Join-Path $package 'VC_RUNTIME_NOTICE.md') -Destination (Join-Path $root 'VC_RUNTIME_NOTICE.md')
  }
  Copy-Item -LiteralPath (Join-Path $package 'extension') -Destination (Join-Path $root 'extension') -Recurse
  Copy-Item -LiteralPath (Join-Path $package 'host-tools') -Destination (Join-Path $root 'host-tools') -Recurse

  if (-not $SkipBrowserHost) {
    $registration = & (Join-Path $root 'host-tools/Prepare-Host.ps1') `
      -Executable (Join-Path $root 'grammar-chromium-host.exe') `
      -ExtensionId $manifest.extensionId -OutputDirectory (Join-Path $root 'host') -UseInstalledRuntime -Register -HostsKey $NativeHostsKey | ConvertFrom-Json
    # Record the registration before anything else can fail.
    $receipt.nativeHost = [ordered]@{
      name = $registration.name
      registryKey = $registration.registry_key
      manifest = $registration.manifest
      executableSha256 = $registration.executable_sha256.ToLowerInvariant()
    }
    Write-InstallReceipt
    $config = Join-Path $root 'extension/host-config.js'
    [IO.File]::WriteAllText($config, "export const HOST = $(ConvertTo-Json $registration.name);`n", $utf8)
  }
  $receipt.state = 'installed'
  Write-InstallReceipt
} catch {
  $failure = $_.Exception.Message
  $left = @()
  $hostReceiptPath = Join-Path $root 'host/registration.json'
  if (Test-Path -LiteralPath $hostReceiptPath) {
    # Prepare-Host may have registered even if a later step failed.
    try { & (Join-Path $root 'host-tools/Remove-Registration.ps1') -PackageDirectory (Join-Path $root 'host') }
    catch { $left += 'the native host registration' }
  }
  if (-not $left) {
    # The receipt and the uninstaller go last, so a partial rollback can
    # still be finished with the uninstaller the message names.
    $last = @('install-receipt.json', 'Uninstall-Grammar.ps1')
    foreach ($child in @(Get-ChildItem -LiteralPath $root -Force | Where-Object { $_.Name -notin $last })) {
      try { Remove-Item -LiteralPath $child.FullName -Recurse -Force } catch { $left += $child.FullName }
    }
    if (-not $left) {
      try { Remove-Item -LiteralPath $root -Recurse -Force } catch { $left += $root }
    }
  }
  if ($left) {
    throw "Install failed: $failure Cleanup was incomplete ($($left -join ', ')). Run `"$root\Uninstall-Grammar.ps1`" to remove it."
  }
  throw "Install failed: $failure Nothing was left installed; fix the cause and run the installer again."
}
$receipt | ConvertTo-Json -Depth 4
