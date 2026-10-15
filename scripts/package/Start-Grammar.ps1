#Requires -Version 7.0
# Owns one installed broker. The application and browser never supply executable hashes.
[CmdletBinding()]
param(
  [string]$RuntimeDirectory = (Join-Path $PSScriptRoot 'local/runtime'),
  [string]$ModelPath = (Join-Path $PSScriptRoot 'local/gemma-4-12b-it-qat-q4_0.gguf'),
  # Managed desktop grammar is the default; explicit false preserves browser-only mode.
  [switch]$EnableDesktopGemma = $true,
  # Explicit portable mode: no model load/download and no automatic cloud request.
  [switch]$WithoutLocalModel,
  [string]$ProfileDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
$root = $PSScriptRoot
$pins = [Collections.Generic.List[IO.FileStream]]::new()
$lock = $null; $broker = $null; $app = $null; $published = $null; $original = $null
$read = $null; $drain = $null; $cleanupComplete = $false; $runtimeNotStarted = $false
$configConflict = $false; $brokerFailed = $false; $missingAssets = $false
$session = $null; $sessionPath = $null; $sessionBytes = $null; $retryTicket = $null
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
    # ReplaceFile preserves atomic publication while Rust readers retain their
    # FILE_SHARE_DELETE handles. Move(overwrite) can fail on that same overlap.
    [IO.File]::Replace($temp, $configPath, [NullString]::Value)
  } finally { if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp } }
}
function Publish-Session([string]$Phase, $PipeName = $null) {
  if ($null -eq $sessionPath) { return }
  $text = @{version=1;session=$session;phase=$Phase;pipe=$PipeName;retryTicket=$retryTicket} | ConvertTo-Json -Compress
  $temp = $sessionPath + '.tmp'
  try {
    [IO.File]::WriteAllText($temp,$text,$utf8)
    if ([IO.File]::Exists($sessionPath)) {
      [IO.File]::Replace($temp,$sessionPath,[NullString]::Value)
    } else {
      [IO.File]::Move($temp,$sessionPath)
    }
    $script:sessionBytes = $text
  } finally { if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp } }
}
function Start-Desktop {
  $appStart = [Diagnostics.ProcessStartInfo]::new((Join-Path $root 'codex-pencil.exe'))
  $appStart.UseShellExecute = $false; $appStart.WorkingDirectory = $root
  if ($ProfileDirectory) { $appStart.Environment['GRAMMAR_PROFILE_DIRECTORY'] = $ProfileDirectory }
  foreach ($name in @('GRAMMAR_GEMMA_PIPE','GRAMMAR_GEMMA_BROKER_SHA256','GRAMMAR_GEMMA_SESSION','GRAMMAR_GEMMA_LAUNCHER_PID')) { $null = $appStart.Environment.Remove($name) }
  if ($EnableDesktopGemma -and -not $WithoutLocalModel) {
    $appStart.Environment['GRAMMAR_GEMMA_SESSION'] = $session
    $appStart.Environment['GRAMMAR_GEMMA_LAUNCHER_PID'] = [string]$PID
    $appStart.Environment['GRAMMAR_GEMMA_BROKER_SHA256'] = $brokerHash
  }
  return [Diagnostics.Process]::Start($appStart)
}
function Restore-Config {
  if ($null -eq $published) { return $false }
  try {
    $current = [IO.File]::ReadAllText($configPath)
    if ($current -ceq $published) { Write-Config $original; return $false }
    return $current -cne $original
  } catch { return $true }
}
function Start-Broker {
  $script:retryTicket = $null
  $script:cleanupComplete = $false
  $script:runtimeNotStarted = $false
  $script:brokerFailed = $false
  $script:missingAssets = $false
  Publish-Session 'starting'
  # Repeat this admission check on every explicit attempt. Missing files allocate
  # no process, so the managed app can stay open and offer a new retry ticket.
  if (-not (Test-Path -LiteralPath $runtime -PathType Leaf) -or -not (Test-Path -LiteralPath $ModelPath -PathType Leaf)) {
    $script:missingAssets = $true
    throw 'Local Gemma assets are missing. Run Prepare-GrammarLocal.ps1 explicitly, then Retry local model.'
  }
  Write-Host 'Starting local Gemma. The first model load can take longer than a recommendation.'
  $script:broker = [Diagnostics.Process]::Start($start)
  # Drain without retaining diagnostics or allowing the child pipe to fill.
  $script:drain = $broker.StandardError.BaseStream.CopyToAsync([IO.Stream]::Null)
  $script:read = $broker.StandardOutput.ReadLineAsync()
  $clock = [Diagnostics.Stopwatch]::StartNew()
  while (-not $read.IsCompleted -and $clock.Elapsed.TotalSeconds -lt 190) {
    if ($null -ne $app -and $app.HasExited) { throw 'App closed while Gemma was preparing' }
    Start-Sleep -Milliseconds 50
  }
  if ($null -ne $app -and $app.HasExited) { throw 'App closed while Gemma was preparing' }
  if (-not $read.IsCompleted) { throw 'Gemma startup timed out' }
  $line = $read.GetAwaiter().GetResult(); $script:read = $null
  if ($line -ceq '{"cleanup":"complete"}') {
    $script:cleanupComplete = $true
    throw 'Gemma startup failed; cleanup acknowledged'
  }
  if ($line -ceq '{"startup":"not_started"}') {
    $script:runtimeNotStarted = $true
    throw 'Gemma artifact validation failed; no runtime was started'
  }
  if (-not $line -or $line.Length -gt 1024) { throw 'Gemma did not become ready' }
  $ready = $line | ConvertFrom-Json
  if ($ready.version -ne 1 -or $ready.pid -ne $broker.Id -or $ready.name -notmatch '^\\\\\.\\pipe\\Grammar\.Gemma\.Broker\.[0-9a-f-]{36}$' -or $broker.HasExited) { throw 'Invalid Gemma readiness' }
  $config = $original | ConvertFrom-Json
  $config | Add-Member -NotePropertyName gemma -NotePropertyValue @{pipe=$ready.name;broker_sha256=$brokerHash}
  $candidateConfig = $config | ConvertTo-Json -Depth 6 -Compress
  # Publish only if the installer sidecar has not changed during model startup.
  if ([IO.File]::ReadAllText($configPath) -cne $original) { throw 'Host configuration changed during startup' }
  Write-Config $candidateConfig
  $script:published = $candidateConfig
  $script:read = $broker.StandardOutput.ReadLineAsync()
  Publish-Session 'ready' $ready.name
  if ($null -eq $app) { $script:app = Start-Desktop }
  Write-Host 'Local Gemma is ready for enabled Chromium fields. Desktop grammar uses the managed local connection unless explicitly disabled.'
 }
function Confirm-Cleanup {
  if ($null -ne $read -and $read.Wait(1000)) {
    $script:cleanupComplete = $read.GetAwaiter().GetResult() -ceq '{"cleanup":"complete"}'
    $script:read = $null
  }
  if ($null -ne $drain -and -not $drain.Wait(1000)) { return $false }
  return $cleanupComplete
}

function Start-Attempt {
  try { Start-Broker } catch {
    if (-not $EnableDesktopGemma -or $null -eq $app -or $app.HasExited) { throw }
    $script:brokerFailed = $true
    $script:retryTicket = $null
    # A marker alone is insufficient: reap the broker and drain its diagnostics.
    if ($null -ne $broker) {
      if (-not $broker.HasExited) { $broker.StandardInput.Close() }
      if (-not $broker.WaitForExit(12000)) {
        $broker.Kill($true); $null=$broker.WaitForExit(5000)
        throw 'Gemma cleanup incomplete; owned process tree termination requested'
      }
      $script:configConflict = Restore-Config
      $confirmedCleanup = Confirm-Cleanup
      # Trusted pinned broker, reaped above, with completed diagnostic drain.
      # No-runtime receipt is not a claim that an engine was stopped successfully.
      if (($confirmedCleanup -or ($runtimeNotStarted -and $drain.IsCompleted)) -and -not $configConflict) {
        $script:retryTicket = [Guid]::NewGuid().ToString('N')
      }
    }
    if ($missingAssets -and $null -eq $broker) {
      # No process was created; this is not a claim of runtime cleanup completion.
      $script:retryTicket = [Guid]::NewGuid().ToString('N')
      Write-Warning 'Local model files are missing. Complete Prepare-GrammarLocal.ps1, then choose Retry local model. No automatic download or request is started.'
    }
    Publish-Session 'failed'
    Write-Warning 'Gemma preparation failed. Your draft remains open. Retry is available after confirmed cleanup or when no runtime was started; no request is replayed.'
  }
}

try {
  if ($ProfileDirectory) {
    if (-not [IO.Path]::IsPathFullyQualified($ProfileDirectory) -or -not (Test-Path -LiteralPath $ProfileDirectory -PathType Container)) { throw 'ProfileDirectory must be an existing absolute directory.' }
    $ProfileDirectory = [IO.Path]::GetFullPath($ProfileDirectory)
  }
  # The retained exclusive handle prevents two launchers publishing different brokers.
  $lock = [IO.File]::Open((Join-Path $root 'gemma-session.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
  $manifest = Get-Content -LiteralPath (Join-Path $root 'MANIFEST.json') -Raw | ConvertFrom-Json
  $installation = Get-Content -LiteralPath (Join-Path $root 'install-receipt.json') -Raw | ConvertFrom-Json
  if ($installation.schema -cne 'grammar-personal-install/v1' -or $installation.state -cne 'installed' -or [IO.Path]::GetFullPath($installation.installRoot).TrimEnd('\','/') -ine $root.TrimEnd('\','/') -or $installation.sourceCommit -cne $manifest.sourceCommit) { throw 'Installation is incomplete or being removed' }
  $appHash = Pin-File 'codex-pencil.exe'
  $hostHash = Pin-File 'grammar-chromium-host.exe'
  $brokerHash = Pin-File 'grammar-gemma-broker.exe'
  $crtPath = $null
  if ($manifest.PSObject.Properties['vcRuntime']) {
    if ($manifest.vcRuntime.deployment -cne 'PRIVATE_PROCESS_PATH') { throw 'Unsupported CRT deployment' }
    $crtEntries = @($manifest.vcRuntime.files)
    if ($crtEntries.Count -lt 3 -or @($crtEntries | Select-Object -Unique).Count -ne $crtEntries.Count) { throw 'Missing or duplicate CRT entries' }
    foreach ($required in @('vcruntime140.dll','vcruntime140_1.dll','msvcp140.dll')) {
      if (('vc-runtime/' + $required) -cnotin $crtEntries) { throw 'Required private CRT missing' }
    }
    foreach ($entry in $crtEntries) {
      if ($entry -cnotmatch '^vc-runtime/[A-Za-z0-9_]+\.dll$') { throw 'Invalid private CRT path' }
      $null = Pin-File $entry
    }
    $crtPath = Join-Path $root 'vc-runtime'
    $present = @(Get-ChildItem -LiteralPath $crtPath -Force | ForEach-Object { 'vc-runtime/' + $_.Name })
    if ($present.Count -ne $crtEntries.Count -or @($present | Where-Object { $_ -cnotin $crtEntries }).Count) { throw 'Unexpected private CRT file' }
  }
  $null = Pin-File 'Gemma-Assets.json'
  $assets = Get-Content -LiteralPath (Join-Path $root 'Gemma-Assets.json') -Raw | ConvertFrom-Json
  if ($assets.schema -cne 'grammar-local-assets/v1') { throw 'Unsupported local assets catalog' }
  $runtime = Join-Path ([IO.Path]::GetFullPath($RuntimeDirectory)) 'llama-server.exe'
  $original = [IO.File]::ReadAllText($configPath)
  $config = $original | ConvertFrom-Json
  if ($config.PSObject.Properties['gemma']) { throw 'An existing Gemma endpoint is present; refusing to replace it.' }
  if ($config.origin -cne ('chrome-extension://' + $manifest.extensionId + '/') -or -not $config.installation_id) { throw 'Installed host origin mismatch' }
  if ($WithoutLocalModel) {
    Write-Host 'Local model disabled explicitly. Basic local rules remain available. External Deep requires your provider setup and consent; no automatic fallback.'
    $app = Start-Desktop
    $app.WaitForExit()
    if ($app.ExitCode -ne 0) { throw 'Grammar exited unsuccessfully.' }
    return
  }
  $start = [Diagnostics.ProcessStartInfo]::new((Join-Path $root 'grammar-gemma-broker.exe'))
  $start.UseShellExecute = $false; $start.CreateNoWindow = $true
  # Only the owned broker and its model child inherit this path. Never modify
  # machine/user PATH or the caller environment. Pins stay held until shutdown.
  if ($null -ne $crtPath) { $start.Environment['PATH'] = $crtPath + [IO.Path]::PathSeparator + $start.Environment['PATH'] }
  $start.RedirectStandardInput = $true; $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
  foreach ($arg in @($runtime, $assets.runtimeSha256, [IO.Path]::GetFullPath($ModelPath), $assets.model.sha256, (Join-Path $root 'grammar-chromium-host.exe'), $hostHash, $assets.runtimeBundleSha256, (Join-Path $root 'codex-pencil.exe'), $appHash)) { $start.ArgumentList.Add($arg) }
  if ($EnableDesktopGemma) {
    $session = [Guid]::NewGuid().ToString('N')
    $sessionPath = Join-Path $root ("gemma-session-" + $session + '.json')
    Publish-Session 'starting'
    $app = Start-Desktop
  }
  Start-Attempt
  while (-not $app.WaitForExit(200)) {
    if (-not $brokerFailed -and $broker.HasExited) {
      $brokerFailed = $true
      $configConflict = Restore-Config
      if ((Confirm-Cleanup) -and -not $configConflict -and $EnableDesktopGemma) {
        $retryTicket = [Guid]::NewGuid().ToString('N')
      }
      Publish-Session 'failed'
      Write-Warning 'Local Gemma stopped. Your draft remains open. In-app retry is available only after confirmed cleanup. No request is replayed. Close Grammar from the tray and relaunch if retry is unavailable.'
      # Keep image pins and installation ownership until the user closes the app.
      # Exiting here would permit uninstall or a duplicate launcher over its draft.
    }
    if ($null -ne $retryTicket) {
      $requestPath = $sessionPath + '.retry'
      if ((Test-Path -LiteralPath $requestPath -PathType Leaf) -and (Get-Item -LiteralPath $requestPath).Length -eq 32 -and [IO.File]::ReadAllText($requestPath) -ceq $retryTicket) {
        # Consume a single explicit generation-bound request before starting anything.
        Remove-Item -LiteralPath $requestPath
        $retryTicket = $null
        Publish-Session 'starting'
        if ($app.HasExited) { break }
        if ([IO.File]::ReadAllText($configPath) -cne $original) { throw 'Host configuration changed before retry' }
        if ($null -ne $broker) { $broker.Dispose() }; $broker = $null; $read = $null; $drain = $null
        Start-Attempt
      }
    }
  }
  if ($null -ne $app -and $app.HasExited -and $app.ExitCode -ne 0) {
    throw 'Grammar exited unsuccessfully; application cleanup was not confirmed.'
  }
} catch {
  $retryTicket = $null
  try { Publish-Session 'failed' } catch { Write-Warning 'Local Gemma status could not be updated.' }
  # A failed startup must revoke the broker promptly, while retaining the app draft.
  try {
    if ($null -ne $broker -and -not $broker.HasExited) {
      $broker.StandardInput.Close()
      if (-not $broker.WaitForExit(12000)) { $broker.Kill($true); $null=$broker.WaitForExit(5000) }
    }
  } finally {
    if ($null -ne $app) {
      Write-Warning 'Local Gemma unavailable. Close Grammar from the tray, then relaunch to retry.'
      $app.WaitForExit()
    }
  }
  throw
} finally {
  # Restore only the bytes this invocation published; preserve outside changes.
  $restoreConflict = Restore-Config
  $configConflict = $configConflict -or $restoreConflict
  try {
    if ($brokerFailed -and $null -eq $broker) { throw 'Local Gemma remains unavailable; no runtime was started.' }
    if ($null -ne $broker) {
      if (-not $broker.HasExited) { $broker.StandardInput.Close() }
      if (-not $broker.WaitForExit(12000)) {
        $broker.Kill($true)
        $null = $broker.WaitForExit(5000)
        throw 'Gemma cleanup incomplete; owned process tree termination requested'
      }
      $null = Confirm-Cleanup
      if ($null -ne $published -and -not $cleanupComplete -and -not $runtimeNotStarted) { throw 'Gemma cleanup was not confirmed' }
      if ($null -ne $drain -and -not $drain.Wait(1000)) { throw 'Gemma diagnostic reader did not finish' }
      if ($brokerFailed -and $runtimeNotStarted) { throw 'Local Gemma artifact validation failed; no runtime was started. Retry explicitly.' }
      if ($brokerFailed -or ($null -ne $published -and $broker.ExitCode -ne 0)) { if ($cleanupComplete) { throw 'Local Gemma stopped; cleanup confirmed. Retry explicitly.' }
        throw 'Local Gemma operation failed; cleanup was not confirmed.' }
    }
  } finally {
    if ($null -ne $broker) { $broker.Dispose() }
    if ($null -ne $app) { $app.Dispose() }
    if ($null -ne $sessionPath -and $null -ne $retryTicket) {
      $requestPath = $sessionPath + '.retry'
      if ((Test-Path -LiteralPath $requestPath -PathType Leaf) -and (Get-Item -LiteralPath $requestPath).Length -eq 32 -and [IO.File]::ReadAllText($requestPath) -ceq $retryTicket) { Remove-Item -LiteralPath $requestPath }
    }
    if ($null -ne $sessionPath -and (Test-Path -LiteralPath $sessionPath)) {
      if ([IO.File]::ReadAllText($sessionPath) -ceq $sessionBytes) { Remove-Item -LiteralPath $sessionPath }
    }
    foreach ($pin in $pins) { $pin.Dispose() }
    if ($null -ne $lock) { $lock.Dispose() }
  }
  if ($configConflict) { throw 'Host configuration changed externally; preserved for inspection' }
}
Write-Host 'Gemma cleanup complete.'
