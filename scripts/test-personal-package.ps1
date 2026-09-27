# CI driver for the personal package (step 8). Phases run as separate workflow
# steps: build (receipt-bound release build + package), install (fresh root,
# registration wiring and registered-host protocol check), failures (install
# and uninstall failure and recovery paths with task-owned roots, registry keys
# and stand-in processes), remove (uninstall and independent residue check). The built-app and browser tests run between
# install and remove against the installed files (the installed-package
# browser smoke test loads the installed extension unmodified). Receipts are
# content-free.
[CmdletBinding()]
param([Parameter(Mandatory)][ValidateSet('build', 'install', 'failures', 'remove')][string]$Phase)
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
    Add-Env 'GRAMMAR_INSTALL_ROOT' $installRoot
    Write-Receipt 'package-install.json' ([ordered]@{
        sourceCommit = $install.sourceCommit
        extensionId = $manifest.extensionId
        verifiedAgainstManifest = @('codex-pencil.exe', 'grammar-chromium-host.exe', 'extension/manifest.json', 'extension/content.js', 'extension/worker.js')
        nativeHostRegisteredForPackagedOrigin = $true
        registeredHostLocalInstant = [ordered]@{ suggestions = @($response.suggestions).Count; provider = $null }
        installRootIsFresh = $true
      })
  }
  'failures' {
    Require-OwnedDesktop
    if (-not $env:GRAMMAR_PACKAGE_ZIP) { throw 'package build did not produce a ZIP' }
    $work = Join-Path $env:RUNNER_TEMP ('grammar-failures-' + [Guid]::NewGuid().ToString('N'))
    $extracted = Join-Path $work 'package'
    Expand-Archive -LiteralPath $env:GRAMMAR_PACKAGE_ZIP -DestinationPath $extracted
    # Task-owned registry tree standing in for a profile without Chrome's key.
    $testKey = 'HKCU:\Software\GrammarCi' + [Guid]::NewGuid().ToString('N')
    $hosts = "$testKey\Google\Chrome\NativeMessagingHosts"
    $checks = [ordered]@{}
    $stand = @()
    function Invoke-Pwsh([string]$Script, [string[]]$Arguments) {
      $out = & pwsh -NoProfile -File $Script @Arguments 2>&1
      [pscustomobject]@{ exit = $LASTEXITCODE; text = ($out | Out-String) }
    }
    function Start-StandIn([string]$Directory) {
      # A harmless program named codex-pencil.exe stands in for a running copy.
      New-Item -ItemType Directory -Force -Path $Directory | Out-Null
      $exe = Join-Path $Directory 'codex-pencil.exe'
      Copy-Item -LiteralPath (Join-Path $env:SystemRoot 'System32\PING.EXE') -Destination $exe -Force
      $process = Start-Process -FilePath $exe -ArgumentList '-n', '120', '127.0.0.1' -WindowStyle Hidden -PassThru
      $script:stand += $process
      Start-Sleep -Milliseconds 500
      $process
    }
    try {
      # 1. No NativeMessagingHosts key yet: install creates the missing levels
      #    and registers; uninstall removes only its own registration.
      $rootA = Join-Path $work 'install-a'
      $a = & (Join-Path $extracted 'Install-Grammar.ps1') -InstallRoot $rootA -NativeHostsKey $hosts | ConvertFrom-Json
      $checks.missingParentKeysCreatedAndRegistered = [bool]((Test-Path -LiteralPath $a.nativeHost.registryKey) -and
        (Get-Item -LiteralPath $a.nativeHost.registryKey).GetValue('') -eq $a.nativeHost.manifest -and $a.state -eq 'installed')
      $unrelated = "$hosts\org.grammar.personal.t" + [Guid]::NewGuid().ToString('N')
      New-Item -Path $unrelated | Out-Null
      Set-Item -LiteralPath $unrelated -Value 'C:\unrelated\host.json'
      $u = Invoke-Pwsh (Join-Path $rootA 'Uninstall-Grammar.ps1') @()
      $checks.uninstallRemovedOnlyItsRegistration = ($u.exit -eq 0) -and -not (Test-Path -LiteralPath $a.nativeHost.registryKey) -and
        (Test-Path -LiteralPath $unrelated) -and -not (Test-Path -LiteralPath $rootA)

      # 2. Registration fails: the install is rolled back completely.
      $rootB = Join-Path $work 'install-b'
      $b = Invoke-Pwsh (Join-Path $extracted 'Install-Grammar.ps1') @('-InstallRoot', $rootB, '-NativeHostsKey', 'HKZZ:\GrammarNoSuchDrive\NativeMessagingHosts')
      $checks.registrationFailureRolledBack = ($b.exit -ne 0) -and ($b.text -match 'Nothing was left installed') -and -not (Test-Path -LiteralPath $rootB)

      # 3. Existing root, changed file and unlisted file are refused before anything is created.
      $rootC = Join-Path $work 'install-c'
      New-Item -ItemType Directory -Path $rootC | Out-Null
      $c = Invoke-Pwsh (Join-Path $extracted 'Install-Grammar.ps1') @('-InstallRoot', $rootC, '-SkipBrowserHost')
      $checks.existingRootRefused = ($c.exit -ne 0) -and ($c.text -match 'To upgrade') -and (@(Get-ChildItem -LiteralPath $rootC -Force).Count -eq 0)
      Remove-Item -LiteralPath $rootC
      $tampered = Join-Path $work 'tampered'
      Copy-Item -LiteralPath $extracted -Destination $tampered -Recurse
      Add-Content -LiteralPath (Join-Path $tampered 'extension/content.js') -Value '// changed'
      $t = Invoke-Pwsh (Join-Path $tampered 'Install-Grammar.ps1') @('-InstallRoot', $rootC, '-SkipBrowserHost')
      $checks.changedFileRefused = ($t.exit -ne 0) -and ($t.text -match 'Packaged file changed') -and -not (Test-Path -LiteralPath $rootC)
      $extra = Join-Path $work 'extra'
      Copy-Item -LiteralPath $extracted -Destination $extra -Recurse
      Set-Content -LiteralPath (Join-Path $extra 'extension/extra.js') -Value 'void 0;'
      $x = Invoke-Pwsh (Join-Path $extra 'Install-Grammar.ps1') @('-InstallRoot', $rootC, '-SkipBrowserHost')
      $checks.unlistedFileRefused = ($x.exit -ne 0) -and ($x.text -match 'does not list') -and -not (Test-Path -LiteralPath $rootC)

      # 4. A locked file: uninstall stops incomplete, keeps its receipt and
      #    script, and a second run finishes once the lock is gone.
      $rootD = Join-Path $work 'install-d'
      $null = & (Join-Path $extracted 'Install-Grammar.ps1') -InstallRoot $rootD -SkipBrowserHost
      $lock = [IO.File]::Open((Join-Path $rootD 'README.md'), 'Open', 'Read', 'None')
      try {
        $first = Invoke-Pwsh (Join-Path $rootD 'Uninstall-Grammar.ps1') @()
      } finally { $lock.Dispose() }
      $checks.lockedUninstallIncompleteAndRepeatable = ($first.exit -ne 0) -and ($first.text -match 'run this uninstaller again') -and
        (Test-Path -LiteralPath (Join-Path $rootD 'install-receipt.json')) -and (Test-Path -LiteralPath (Join-Path $rootD 'Uninstall-Grammar.ps1'))
      $second = Invoke-Pwsh (Join-Path $rootD 'Uninstall-Grammar.ps1') @()
      $checks.repeatedUninstallCompleted = ($second.exit -eq 0) -and -not (Test-Path -LiteralPath $rootD)

      # 5. A running copy of this install is stopped; -RemoveUserData is refused
      #    while another copy runs, and then nothing is removed.
      $rootE = Join-Path $work 'install-e'
      $null = & (Join-Path $extracted 'Install-Grammar.ps1') -InstallRoot $rootE -SkipBrowserHost
      $own = Start-StandIn $rootE
      $other = Start-StandIn (Join-Path $work 'other-copy')
      $refused = Invoke-Pwsh (Join-Path $rootE 'Uninstall-Grammar.ps1') @('-RemoveUserData')
      $checks.removeUserDataRefusedWhileAnotherCopyRuns = ($refused.exit -ne 0) -and ($refused.text -match 'Another Grammar copy is running') -and
        (Test-Path -LiteralPath $rootE) -and -not $own.HasExited
      Stop-Process -Id $other.Id -Force
      $stopped = Invoke-Pwsh (Join-Path $rootE 'Uninstall-Grammar.ps1') @()
      $own.WaitForExit(10000) | Out-Null
      $checks.runningOwnCopyStoppedAndRemoved = ($stopped.exit -eq 0) -and $own.HasExited -and -not (Test-Path -LiteralPath $rootE)
    } finally {
      foreach ($process in $stand) { if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue } }
      if (Test-Path -LiteralPath $testKey) { Remove-Item -LiteralPath $testKey -Recurse }
      foreach ($root in @('install-a', 'install-b', 'install-c', 'install-d', 'install-e')) {
        $path = Join-Path $work $root
        if (Test-Path -LiteralPath (Join-Path $path 'Uninstall-Grammar.ps1')) { $null = Invoke-Pwsh (Join-Path $path 'Uninstall-Grammar.ps1') @() }
      }
    }
    $checks.taskOwnedRegistryTreeRemoved = -not (Test-Path -LiteralPath $testKey)
    Write-Receipt 'package-failures.json' $checks
    $failed = @($checks.Keys | Where-Object { -not $checks[$_] })
    if ($failed.Count) { throw "install/uninstall failure paths: $($failed -join ', ')" }
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
      # Any process still running one of the installed binaries is residue.
      processes = @(Get-Process -Name 'codex-pencil', 'grammar-chromium-host' -ErrorAction SilentlyContinue |
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
