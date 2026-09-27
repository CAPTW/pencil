[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$SourceRoot,
  [Parameter(Mandatory)][string]$ReceiptPath,
  [switch]$VerifyOnly,
  [string]$ExecutablePath,
  [string]$HostExecutablePath
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
$source = (Resolve-Path -LiteralPath $SourceRoot).Path
$receiptFile = [IO.Path]::GetFullPath($ReceiptPath)
function GitValue([string[]]$Arguments) {
  $value = & git -C $source @Arguments
  if ($LASTEXITCODE -ne 0) { throw 'Git measurement failed' }
  return ($value -join "`n")
}
if ([IO.Path]::GetFullPath((GitValue @('rev-parse','--show-toplevel'))) -ine $source) { throw 'SourceRoot must be the repository root' }
function Measure-Source {
  if (GitValue @('status','--porcelain=v1','--untracked-files=all')) { throw 'Source must be clean, including untracked files' }
  # git status hides edits under assume-unchanged (lowercase tag) and
  # skip-worktree (S), so such a tree could differ from the commit it names.
  if (@(& git -C $source ls-files -v | Where-Object { $_ -cmatch '^(?:[a-z]|S) ' }).Count) { throw 'Source has assume-unchanged or skip-worktree entries' }
  $headBefore = GitValue @('rev-parse','HEAD')
  $lines = foreach ($relative in (& git -C $source -c core.quotepath=false ls-files)) {
    if ($LASTEXITCODE -ne 0) { throw 'Source enumeration failed' }
    $path = Join-Path $source $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw 'Tracked source file missing' }
    if ((Get-Item -Force -LiteralPath $path).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse source files unsupported' }
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    "$relative`0$hash"
  }
  $bytes = [Text.Encoding]::UTF8.GetBytes(($lines -join "`n"))
  $sha = [Security.Cryptography.SHA256]::Create()
  try { $digest = ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-','').ToLowerInvariant() } finally { $sha.Dispose() }
  if ((GitValue @('status','--porcelain=v1','--untracked-files=all')) -or (GitValue @('rev-parse','HEAD')) -cne $headBefore) { throw 'Source changed during measurement' }
  return [ordered]@{
    commit = $headBefore
    tree = (GitValue @('rev-parse','HEAD^{tree}'))
    subject = (GitValue @('show','-s','--format=%s','HEAD'))
    trackedBytesSha256 = $digest
    clean = $true
  }
}
function Same-Source($a, $b) {
  return $a.commit -ceq $b.commit -and $a.tree -ceq $b.tree -and
    $a.subject -ceq $b.subject -and $a.trackedBytesSha256 -ceq $b.trackedBytesSha256 -and
    $a.clean -is [bool] -and $b.clean -is [bool] -and $a.clean -and $b.clean
}
$command = 'node_modules/.bin/tauri.cmd build --no-bundle --no-sign -- --locked --offline --bin codex-pencil; cargo build --manifest-path src-tauri/Cargo.toml --release --locked --offline --bin grammar-chromium-host'
if ($VerifyOnly) {
  if (-not $ExecutablePath) { throw 'ExecutablePath required for receipt verification' }
  $receipt = Get-Content -LiteralPath $receiptFile -Raw | ConvertFrom-Json
  if ($receipt.schemaVersion -ne 1 -or $receipt.classification -cne 'LOCAL_EXECUTED_RELEASE_BUILD' -or
      $receipt.command -cne $command -or $receipt.exitCode -ne 0 -or $receipt.freshTarget -isnot [bool] -or -not $receipt.freshTarget) {
    throw 'Receipt is not an executed release build'
  }
  $measured = Measure-Source
  if (-not (Same-Source $receipt.before $receipt.after) -or -not (Same-Source $measured $receipt.after)) { throw 'Receipt/source mismatch' }
  $exe = Get-Item -LiteralPath $ExecutablePath
  if ($exe.Length -ne $receipt.executableBytes -or (Get-FileHash -LiteralPath $exe.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $receipt.executableSha256) { throw 'Receipt/executable mismatch' }
  if ($receipt.PSObject.Properties.Name -contains 'hostExecutablePath') {
    if ($receipt.hostExitCode -ne 0) { throw 'Host build failed in receipt' }
    $hostPath = if ($HostExecutablePath) { $HostExecutablePath } else { $receipt.hostExecutablePath }
    $hostBinary = Get-Item -LiteralPath $hostPath
    if ($hostBinary.Length -ne $receipt.hostExecutableBytes -or (Get-FileHash -LiteralPath $hostBinary.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $receipt.hostExecutableSha256) { throw 'Receipt/host executable mismatch' }
  }
  if ([DateTime]::Parse($receipt.finishedUtc).ToUniversalTime() -lt [DateTime]::Parse($receipt.startedUtc).ToUniversalTime()) { throw 'Receipt timestamps invalid' }
  return $receipt
}
if (Test-Path -LiteralPath $receiptFile) { throw 'Receipt already exists' }
if ($receiptFile.StartsWith($source.TrimEnd('\','/') + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Receipt must be outside source worktree' }
$before = Measure-Source
$evidence = Split-Path -Parent $receiptFile
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
$target = Join-Path $evidence ('release-target-' + [Guid]::NewGuid().ToString('N'))
if (Test-Path -LiteralPath $target) { throw 'Fresh target already exists' }
New-Item -ItemType Directory -Path $target | Out-Null
$started = [DateTime]::UtcNow.ToString('o')
$oldTarget = $env:CARGO_TARGET_DIR
try {
  $env:CARGO_TARGET_DIR = $target
  Push-Location -LiteralPath $source
  try {
    & (Join-Path $source 'node_modules/.bin/tauri.cmd') build --no-bundle --no-sign -- --locked --offline --bin codex-pencil 2>&1 | Tee-Object -FilePath (Join-Path $target 'build-output.log')
    $buildExit = $LASTEXITCODE
    if ($buildExit -ne 0) { throw "Desktop build failed with exit $buildExit; no receipt emitted" }
    & cargo build --manifest-path src-tauri/Cargo.toml --release --locked --offline --bin grammar-chromium-host 2>&1 | Tee-Object -FilePath (Join-Path $target 'host-build-output.log')
    $hostExit = $LASTEXITCODE
    if ($hostExit -ne 0) { throw "Host build failed with exit $hostExit; no receipt emitted" }
  } finally { Pop-Location }
} finally { $env:CARGO_TARGET_DIR = $oldTarget }
if ($buildExit -ne 0) { throw "Build failed with exit $buildExit; no receipt emitted" }
$after = Measure-Source
if (-not (Same-Source $before $after)) { throw 'Source changed during build; no receipt emitted' }
$exe = Get-Item -LiteralPath (Join-Path $target 'release/codex-pencil.exe')
$hostBinary = Get-Item -LiteralPath (Join-Path $target 'release/grammar-chromium-host.exe')
$receipt = [ordered]@{
  schemaVersion = 1
  classification = 'LOCAL_EXECUTED_RELEASE_BUILD'
  command = $command
  exitCode = $buildExit
  freshTarget = $true
  startedUtc = $started
  finishedUtc = [DateTime]::UtcNow.ToString('o')
  before = $before
  after = $after
  executablePath = $exe.FullName
  executableBytes = $exe.Length
  executableSha256 = (Get-FileHash -LiteralPath $exe.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  hostExitCode = $hostExit
  hostExecutablePath = $hostBinary.FullName
  hostExecutableBytes = $hostBinary.Length
  hostExecutableSha256 = (Get-FileHash -LiteralPath $hostBinary.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
}
[IO.File]::WriteAllText($receiptFile, ($receipt | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
$receipt
