# Personal installation of a verified Grammar package (current user only).
# Verifies every packaged file against MANIFEST.json before copying anything,
# installs into a fresh directory, and registers the Chromium native host for
# the package's fixed extension ID. No account, Provider or network is used.
[CmdletBinding()]
param(
  [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA 'GrammarPersonal'),
  [switch]$SkipBrowserHost
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
$package = $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $package 'MANIFEST.json') -Raw | ConvertFrom-Json
foreach ($file in $manifest.files) {
  $path = Join-Path $package $file.path
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Packaged file missing: $($file.path)" }
  if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $file.sha256) {
    throw "Packaged file changed: $($file.path)"
  }
}
if ($manifest.extensionId -notmatch '^[a-p]{32}$') { throw 'Package has no fixed extension ID.' }

$root = [IO.Path]::GetFullPath($InstallRoot)
if (Test-Path -LiteralPath $root) { throw 'InstallRoot already exists. Uninstall first or choose a new directory.' }
New-Item -ItemType Directory -Path $root | Out-Null
foreach ($item in @('codex-pencil.exe', 'grammar-chromium-host.exe', 'MANIFEST.json', 'BUILD_RECEIPT.json', 'PERSONAL_USE.md', 'README.md', 'HANDOFF.md', 'Uninstall-Grammar.ps1')) {
  Copy-Item -LiteralPath (Join-Path $package $item) -Destination (Join-Path $root $item)
}
Copy-Item -LiteralPath (Join-Path $package 'extension') -Destination (Join-Path $root 'extension') -Recurse
Copy-Item -LiteralPath (Join-Path $package 'host-tools') -Destination (Join-Path $root 'host-tools') -Recurse

$registration = $null
if (-not $SkipBrowserHost) {
  $hostDirectory = Join-Path $root 'host'
  $registration = & (Join-Path $root 'host-tools/Prepare-Host.ps1') `
    -Executable (Join-Path $root 'grammar-chromium-host.exe') `
    -ExtensionId $manifest.extensionId -OutputDirectory $hostDirectory -Register | ConvertFrom-Json
  $config = Join-Path $root 'extension/host-config.js'
  [IO.File]::WriteAllText($config, "export const HOST = $(ConvertTo-Json $registration.name);`n", [Text.UTF8Encoding]::new($false))
}

$receipt = [ordered]@{
  schema = 'grammar-personal-install/v1'
  sourceCommit = $manifest.sourceCommit
  classification = $manifest.classification
  installRoot = $root
  installedUtc = [DateTime]::UtcNow.ToString('o')
  appExecutable = Join-Path $root 'codex-pencil.exe'
  extensionDirectory = Join-Path $root 'extension'
  extensionId = $manifest.extensionId
  nativeHost = if ($registration) {
    [ordered]@{
      name = $registration.name
      registryKey = 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\' + $registration.name
      manifest = $registration.manifest
      executableSha256 = $registration.executable_sha256.ToLowerInvariant()
    }
  } else { $null }
}
[IO.File]::WriteAllText((Join-Path $root 'install-receipt.json'), ($receipt | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
$receipt | ConvertTo-Json -Depth 4
