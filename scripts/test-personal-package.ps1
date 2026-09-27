# CI driver for the personal package (step 8). Phases run as separate workflow
# steps: build (receipt-bound release build + package), install (fresh root,
# registration wiring and registered-host protocol check), remove (uninstall
# and independent residue check). The built-app and browser tests run between
# install and remove against the installed files. Receipts are content-free.
[CmdletBinding()]
param([Parameter(Mandatory)][ValidateSet('build', 'install', 'remove')][string]$Phase)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$evidence = Join-Path $env:GRAMMAR_EVIDENCE 'personal'
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
$installRoot = Join-Path $env:RUNNER_TEMP 'grammar-personal-install'
$utf8 = [Text.UTF8Encoding]::new($false)
function Write-Receipt([string]$Name, $Value) {
  [IO.File]::WriteAllText((Join-Path $evidence $Name), ($Value | ConvertTo-Json -Depth 8), $utf8)
  "RECEIPT $Name"; $Value | ConvertTo-Json -Depth 8
}
function Add-Env([string]$Name, [string]$Value) { "$Name=$Value" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8 }
function Require-OwnedDesktop {
  # Registry and process changes are allowed only on the opted-in hosted runner.
  foreach ($pair in @(@('GITHUB_ACTIONS', 'true'), @('GITHUB_REPOSITORY', 'CAPTW/pencil'), @('RUNNER_ENVIRONMENT', 'github-hosted'), @('GRAMMAR_OWNED_DESKTOP_TEST', '1'))) {
    if ([Environment]::GetEnvironmentVariable($pair[0]) -cne $pair[1]) { throw "owned desktop opt-in: $($pair[0])" }
  }
  if ($env:GITHUB_REF -notin @('refs/heads/codex/grammar-autonomous-r1', 'refs/heads/claude/eloquent-faraday-hh62qc')) { throw 'owned desktop opt-in: GITHUB_REF' }
}
function Invoke-HostAnalyze([string]$Executable, [string]$Origin) {
  $body = $utf8.GetBytes((@{version = 1; op = 'analyze'; id = 'package'; epoch = 'package-doc'; revision = 1; text = 'seperate' } | ConvertTo-Json -Compress))
  $start = [Diagnostics.ProcessStartInfo]::new($Executable)
  $start.ArgumentList.Add($Origin)
  $start.RedirectStandardInput = $true; $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
  $start.UseShellExecute = $false
  $process = [Diagnostics.Process]::Start($start)
  $process.StandardInput.BaseStream.Write([BitConverter]::GetBytes([uint32]$body.Length), 0, 4)
  $process.StandardInput.BaseStream.Write($body, 0, $body.Length)
  $process.StandardInput.Close()
  $buffer = [IO.MemoryStream]::new()
  $process.StandardOutput.BaseStream.CopyTo($buffer)
  if (-not $process.WaitForExit(10000)) { $process.Kill(); throw 'registered host did not exit' }
  $bytes = $buffer.ToArray()
  if ($process.ExitCode -ne 0 -or $bytes.Length -lt 4) { throw 'registered host rejected the packaged origin' }
  $length = [BitConverter]::ToUInt32($bytes, 0)
  if ($length -ne $bytes.Length - 4) { throw 'registered host framing mismatch' }
  return $utf8.GetString($bytes, 4, $bytes.Length - 4) | ConvertFrom-Json
}

# A failed phase leaves a content-free failure receipt (error, position, source
# status and build log tails) that the job summary prints at the end of the log.
function Write-Failure($Failure) {
  $logs = @(Get-ChildItem -LiteralPath $evidence -Recurse -Include 'build-output.log', 'host-build-output.log' -ErrorAction SilentlyContinue |
    ForEach-Object { [ordered]@{ log = $_.Name; tail = @(Get-Content -LiteralPath $_.FullName -Tail 40) } })
  $status = @(& git -C $repository status --porcelain=v1 --untracked-files=all 2>$null | Select-Object -First 20)
  Write-Receipt "$Phase-failure.json" ([ordered]@{
      phase = $Phase
      error = $Failure.Exception.Message
      position = $Failure.InvocationInfo.PositionMessage
      scriptStack = $Failure.ScriptStackTrace
      sourceStatus = $status
      buildLogTails = $logs
    })
}

try {
switch ($Phase) {
  'build' {
    $receiptPath = Join-Path $evidence 'build-receipt.json'
    & (Join-Path $repository 'scripts/mission/build-receipt.ps1') -SourceRoot $repository -ReceiptPath $receiptPath | Out-Null
    $build = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    Add-Env 'GRAMMAR_APP_EXE' $build.executablePath
    Add-Env 'GRAMMAR_APP_KIND' 'receipt-bound-release-build'
    $short = $build.after.commit.Substring(0, 12)
    $package = & (Join-Path $repository 'scripts/mission/package.ps1') -ReceiptPath $receiptPath -OutputRoot (Join-Path $env:RUNNER_TEMP "grammar-package-$short") | ConvertFrom-Json
    Add-Env 'GRAMMAR_PACKAGE_ZIP' $package.zip
    $manifest = Get-Content -LiteralPath (Join-Path $package.directory 'MANIFEST.json') -Raw | ConvertFrom-Json
    Write-Receipt 'package-build.json' ([ordered]@{
        sourceCommit = $build.after.commit
        sourceTree = $build.after.tree
        sourceTrackedBytesSha256 = $build.after.trackedBytesSha256
        buildCommand = $build.command
        appSha256 = $build.executableSha256
        hostSha256 = $build.hostExecutableSha256
        zipSha256 = $package.zipSha256
        classification = $manifest.classification
        extensionId = $manifest.extensionId
        capabilities = [ordered]@{
          nativeCapture = $manifest.nativeCapture; nativeApply = $manifest.nativeApply
          chromiumInstant = $manifest.chromiumInstant; chromiumDeep = $manifest.chromiumDeep
          providerLive = $manifest.providerLive; physicalKeyboardIme = $manifest.physicalKeyboardIme
        }
        files = $manifest.files
        executablesPublished = $false
      })
  }
  'install' {
    Require-OwnedDesktop
    if (-not $env:GRAMMAR_PACKAGE_ZIP) { throw 'package build did not produce a ZIP' }
    $extracted = Join-Path $env:RUNNER_TEMP ('grammar-package-extracted-' + [Guid]::NewGuid().ToString('N'))
    Expand-Archive -LiteralPath $env:GRAMMAR_PACKAGE_ZIP -DestinationPath $extracted
    $install = & (Join-Path $extracted 'Install-Grammar.ps1') -InstallRoot $installRoot | ConvertFrom-Json
    $manifest = Get-Content -LiteralPath (Join-Path $installRoot 'MANIFEST.json') -Raw | ConvertFrom-Json
    $expected = @{}; foreach ($file in $manifest.files) { $expected[$file.path] = $file.sha256 }
    foreach ($item in @('codex-pencil.exe', 'grammar-chromium-host.exe', 'extension/manifest.json', 'extension/content.js', 'extension/worker.js')) {
      if ((Get-FileHash -LiteralPath (Join-Path $installRoot $item) -Algorithm SHA256).Hash.ToLowerInvariant() -cne $expected[$item]) { throw "installed file differs from package: $item" }
    }
    $registered = (Get-Item -LiteralPath $install.nativeHost.registryKey).GetValue('')
    if ($registered -ne $install.nativeHost.manifest) { throw 'registry does not point at the installed host manifest' }
    $hostManifest = Get-Content -LiteralPath $registered -Raw | ConvertFrom-Json
    $origin = "chrome-extension://$($manifest.extensionId)/"
    if (@($hostManifest.allowed_origins).Count -ne 1 -or $hostManifest.allowed_origins[0] -cne $origin) { throw 'host manifest origin is not the packaged extension' }
    if ((Get-FileHash -LiteralPath $hostManifest.path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $expected['grammar-chromium-host.exe']) { throw 'registered host binary differs from package' }
    $configured = Get-Content -LiteralPath (Join-Path $installRoot 'extension/host-config.js') -Raw
    if (-not $configured.Contains($install.nativeHost.name)) { throw 'installed extension is not configured for the registered host' }
    $response = Invoke-HostAnalyze $hostManifest.path $origin
    if ($response.id -cne 'package' -or @($response.suggestions).Count -lt 1) { throw 'registered host produced no local Instant suggestion' }
    Add-Env 'GRAMMAR_APP_EXE' $install.appExecutable
    Add-Env 'GRAMMAR_APP_KIND' 'installed-personal-package'
    Add-Env 'GRAMMAR_INSTALLED_HOST' $hostManifest.path
    Add-Env 'GRAMMAR_INSTALLED_EXTENSION' $install.extensionDirectory
    Write-Receipt 'package-install.json' ([ordered]@{
        sourceCommit = $install.sourceCommit
        extensionId = $manifest.extensionId
        verifiedAgainstManifest = @('codex-pencil.exe', 'grammar-chromium-host.exe', 'extension/manifest.json', 'extension/content.js', 'extension/worker.js')
        nativeHostRegisteredForPackagedOrigin = $true
        registeredHostLocalInstant = [ordered]@{ suggestions = @($response.suggestions).Count; provider = $null }
        installRootIsFresh = $true
      })
  }
  'remove' {
    Require-OwnedDesktop
    $before = [ordered]@{ installRoot = Test-Path -LiteralPath $installRoot }
    if (-not $before.installRoot) { throw 'nothing installed to remove' }
    $install = Get-Content -LiteralPath (Join-Path $installRoot 'install-receipt.json') -Raw | ConvertFrom-Json
    $result = & (Join-Path $installRoot 'Uninstall-Grammar.ps1') -InstallRoot $installRoot -RemoveUserData | ConvertFrom-Json
    $prefix = [IO.Path]::GetFullPath($installRoot).TrimEnd('\') + '\'
    $independent = [ordered]@{
      installRoot = Test-Path -LiteralPath $installRoot
      registryKey = Test-Path -LiteralPath $install.nativeHost.registryKey
      appData = Test-Path -LiteralPath (Join-Path $env:APPDATA 'com.local.codexpencil')
      localAppData = Test-Path -LiteralPath (Join-Path $env:LOCALAPPDATA 'com.local.codexpencil')
      # Any process still running an installed binary is residue.
      processes = @(Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) }).Count
    }
    Write-Receipt 'package-remove.json' ([ordered]@{ uninstaller = $result; independentCheck = $independent })
    if (-not $result.complete -or $independent.installRoot -or $independent.registryKey -or $independent.appData -or $independent.localAppData -or $independent.processes -ne 0) {
      throw 'uninstall left residue'
    }
  }
}
} catch {
  Write-Failure $_
  throw
}
