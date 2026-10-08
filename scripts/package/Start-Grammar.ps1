#Requires -Version 7.0
# Owns one installed broker. The application and browser never supply executable hashes.
[CmdletBinding()]
param(
  [string]$RuntimeDirectory = (Join-Path $PSScriptRoot 'local/runtime'),
  [string]$ModelPath = (Join-Path $PSScriptRoot 'local/gemma-4-12b-it-qat-q4_0.gguf')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
$root = $PSScriptRoot
$pins = [Collections.Generic.List[IO.FileStream]]::new()
$lock = $null; $broker = $null; $app = $null; $published = $null; $original = $null
$read = $null; $drain = $null; $cleanupComplete = $false
$configPath = Join-Path $root 'grammar-chromium-host.origin.json'
$utf8 = [Text.UTF8Encoding]::new($false)
function Pin-File([string]$Name) {
  $entries = @($manifest.files | Where-Object { $_.path -ceq $Name })
  if ($entries.Count -ne 1) { throw "Missing or ambiguous package entry: $Name" }
  $stream = [IO.File]::Open((Join-Path $root $Name), 'Open', 'Read', 'Read')
  $pins.Add($stream)
  $sha = [Security.Cryptography.SHA256]::Create()
  try { $hash = ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant() }
  finally { $sha.Dispose() }
  if ($stream.Length -ne $entries[0].bytes -or $hash -cne $entries[0].sha256) { throw "Package file changed: $Name" }
  return $hash
}
function Write-Config([string]$Text) {
  $temp = $configPath + '.' + [Guid]::NewGuid().ToString('N') + '.tmp'
  try {
    [IO.File]::WriteAllText($temp, $Text, $utf8)
    [IO.File]::Move($temp, $configPath, $true)
  } finally { if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp } }
}
try {
  # The retained exclusive handle prevents two launchers publishing different brokers.
  $lock = [IO.File]::Open((Join-Path $root 'gemma-session.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
  $manifest = Get-Content -LiteralPath (Join-Path $root 'MANIFEST.json') -Raw | ConvertFrom-Json
  $installation = Get-Content -LiteralPath (Join-Path $root 'install-receipt.json') -Raw | ConvertFrom-Json
  if ($installation.schema -cne 'grammar-personal-install/v1' -or $installation.state -cne 'installed' -or [IO.Path]::GetFullPath($installation.installRoot).TrimEnd('\','/') -ine $root.TrimEnd('\','/') -or $installation.sourceCommit -cne $manifest.sourceCommit) { throw 'Installation is incomplete or being removed' }
  $null = Pin-File 'codex-pencil.exe'
  $hostHash = Pin-File 'grammar-chromium-host.exe'
  $brokerHash = Pin-File 'grammar-gemma-broker.exe'
  $null = Pin-File 'Gemma-Assets.json'
  $assets = Get-Content -LiteralPath (Join-Path $root 'Gemma-Assets.json') -Raw | ConvertFrom-Json
  if ($assets.schema -cne 'grammar-local-assets/v1') { throw 'Unsupported local assets catalog' }
  $runtime = Join-Path ([IO.Path]::GetFullPath($RuntimeDirectory)) 'llama-server.exe'
  if (-not (Test-Path -LiteralPath $runtime -PathType Leaf) -or -not (Test-Path -LiteralPath $ModelPath -PathType Leaf)) {
    throw 'Local Gemma assets are missing. Prepare the pinned model and runtime before starting.'
  }
  $original = [IO.File]::ReadAllText($configPath)
  $config = $original | ConvertFrom-Json
  if ($config.PSObject.Properties['gemma']) { throw 'An existing Gemma endpoint is present; refusing to replace it.' }
  if ($config.origin -cne ('chrome-extension://' + $manifest.extensionId + '/') -or -not $config.installation_id) { throw 'Installed host origin mismatch' }
  $start = [Diagnostics.ProcessStartInfo]::new((Join-Path $root 'grammar-gemma-broker.exe'))
  $start.UseShellExecute = $false; $start.CreateNoWindow = $true
  $start.RedirectStandardInput = $true; $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
  foreach ($arg in @($runtime, $assets.runtimeSha256, [IO.Path]::GetFullPath($ModelPath), $assets.model.sha256, (Join-Path $root 'grammar-chromium-host.exe'), $hostHash, $assets.runtimeBundleSha256)) { $start.ArgumentList.Add($arg) }
  Write-Host 'Starting local Gemma. The first model load can take longer than a recommendation.'
  $broker = [Diagnostics.Process]::Start($start)
  # Drain without retaining diagnostics or allowing the child pipe to fill.
  $drain = $broker.StandardError.BaseStream.CopyToAsync([IO.Stream]::Null)
  $read = $broker.StandardOutput.ReadLineAsync()
  $clock = [Diagnostics.Stopwatch]::StartNew()
  while (-not $read.IsCompleted -and $clock.Elapsed.TotalSeconds -lt 190) { Start-Sleep -Milliseconds 50 }
  if (-not $read.IsCompleted) { throw 'Gemma startup timed out' }
  $line = $read.GetAwaiter().GetResult(); $read = $null
  if (-not $line -or $line.Length -gt 1024) { throw 'Gemma did not become ready' }
  $ready = $line | ConvertFrom-Json
  if ($ready.version -ne 1 -or $ready.pid -ne $broker.Id -or $ready.name -notmatch '^\\\\\.\\pipe\\Grammar\.Gemma\.Broker\.[0-9a-f-]{36}$' -or $broker.HasExited) { throw 'Invalid Gemma readiness' }
  $config | Add-Member -NotePropertyName gemma -NotePropertyValue @{pipe=$ready.name;broker_sha256=$brokerHash}
  $candidateConfig = $config | ConvertTo-Json -Depth 6 -Compress
  # Publish only if the installer sidecar has not changed during model startup.
  if ([IO.File]::ReadAllText($configPath) -cne $original) { throw 'Host configuration changed during startup' }
  Write-Config $candidateConfig
  $published = $candidateConfig
  $read = $broker.StandardOutput.ReadLineAsync()
  $appStart = [Diagnostics.ProcessStartInfo]::new((Join-Path $root 'codex-pencil.exe'))
  $appStart.UseShellExecute = $false; $appStart.WorkingDirectory = $root
  $app = [Diagnostics.Process]::Start($appStart)
  Write-Host 'Local Gemma is ready for enabled Chromium fields. Desktop model routing is not yet enabled.'
  while (-not $app.WaitForExit(200)) {
    if ($broker.HasExited) { throw 'Local Gemma stopped. No automatic restart or cloud fallback was attempted.' }
  }
} finally {
  # Restore only the bytes this invocation published; preserve outside changes.
  $configConflict = $false
  if ($null -ne $published) {
    try {
      if ([IO.File]::ReadAllText($configPath) -ceq $published) { Write-Config $original }
      elseif ([IO.File]::ReadAllText($configPath) -cne $original) { $configConflict = $true }
    } catch { $configConflict = $true }
  }
  try {
    if ($null -ne $broker) {
      $broker.StandardInput.Close()
      if (-not $broker.WaitForExit(12000)) {
        $broker.Kill($true)
        $null = $broker.WaitForExit(5000)
        throw 'Gemma cleanup incomplete; owned process tree termination requested'
      }
      if ($null -ne $read -and $read.Wait(1000)) {
        $cleanupLine = $read.GetAwaiter().GetResult()
        $cleanupComplete = $cleanupLine -ceq '{"cleanup":"complete"}' -and $broker.ExitCode -eq 0
      }
      if ($null -ne $published -and -not $cleanupComplete) { throw 'Gemma cleanup was not confirmed' }
      if ($null -ne $drain -and -not $drain.Wait(1000)) { throw 'Gemma diagnostic reader did not finish' }
    }
  } finally {
    if ($null -ne $broker) { $broker.Dispose() }
    if ($null -ne $app) { $app.Dispose() }
    foreach ($pin in $pins) { $pin.Dispose() }
    if ($null -ne $lock) { $lock.Dispose() }
  }
  if ($configConflict) { throw 'Host configuration changed externally; preserved for inspection' }
}
Write-Host 'Gemma cleanup complete.'
