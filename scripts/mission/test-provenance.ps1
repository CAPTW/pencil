[CmdletBinding()]
param([string]$EvidenceRoot = (Join-Path $env:TEMP ('grammar-provenance-' + [Guid]::NewGuid().ToString('N'))))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
if (Test-Path -LiteralPath $EvidenceRoot) { throw 'Use a new task-owned evidence directory' }
$root = Join-Path $EvidenceRoot 'synthetic-source'
New-Item -ItemType Directory -Path (Join-Path $root 'scripts/mission') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot '../build-daily-use-bundle.ps1') -Destination (Join-Path $root 'scripts/build-daily-use-bundle.ps1')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'build-receipt.ps1') -Destination (Join-Path $root 'scripts/mission/build-receipt.ps1')
[IO.File]::WriteAllText((Join-Path $root 'synthetic.txt'), 'synthetic source', [Text.UTF8Encoding]::new($false))
& git -C $root init -q
& git -C $root add .
& git -C $root -c user.name=Synthetic -c user.email=synthetic@example.invalid commit -qm synthetic
if ($LASTEXITCODE -ne 0) { throw 'Synthetic Git setup failed' }
$source = [ordered]@{
  commit = (& git -C $root rev-parse HEAD)
  tree = (& git -C $root rev-parse 'HEAD^{tree}')
  subject = 'synthetic'
  clean = $true
}
$lines = foreach ($path in (& git -C $root ls-files)) { "$path`0$((Get-FileHash -LiteralPath (Join-Path $root $path) -Algorithm SHA256).Hash.ToLowerInvariant())" }
$sha = [Security.Cryptography.SHA256]::Create()
try { $source.trackedBytesSha256 = ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes(($lines -join "`n"))))).Replace('-','').ToLowerInvariant() } finally { $sha.Dispose() }
$exe = Join-Path $EvidenceRoot 'synthetic-not-executable.exe'
[IO.File]::WriteAllText($exe, 'NOT A REAL BUILD', [Text.UTF8Encoding]::new($false))
# Fabricated structures are used ONLY as negative test input. No positive build/acceptance is asserted.
$template = [ordered]@{
  schemaVersion = 1; classification = 'LOCAL_EXECUTED_RELEASE_BUILD'
  command = 'node_modules/.bin/tauri.cmd build --no-bundle --no-sign -- --locked --offline --bin codex-pencil; cargo build --manifest-path src-tauri/Cargo.toml --release --locked --offline --bin grammar-chromium-host'
  exitCode = 0; freshTarget = $true
  startedUtc = '2026-01-01T00:00:00Z'; finishedUtc = '2026-01-01T00:00:01Z'
  before = $source; after = $source
  executableBytes = (Get-Item -LiteralPath $exe).Length
  executableSha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
  hostExitCode = 0
  hostExecutablePath = $exe
  hostExecutableBytes = (Get-Item -LiteralPath $exe).Length
  hostExecutableSha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
}
$passes = [Collections.Generic.List[string]]::new()
function Reject([string]$Name, [scriptblock]$Change, [string]$Expected) {
  $data = $template | ConvertTo-Json -Depth 6 | ConvertFrom-Json
  & $Change $data
  $receipt = Join-Path $EvidenceRoot ($Name + '.json')
  [IO.File]::WriteAllText($receipt, ($data | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
  $output = Join-Path $EvidenceRoot ('output-' + $Name)
  $subject = if ($Name -eq 'false-caller-metadata') { 'false subject' } else { $source.subject }
  $failed = $false
  try {
    & (Join-Path $root 'scripts/build-daily-use-bundle.ps1') -ExecutablePath $exe -OutputRoot $output -ProductCommit $source.commit -PackagingCommit $source.commit -GitTree $source.tree -GitSubject $subject -BuildReceiptPath $receipt
  } catch {
    if ($_.Exception.Message -notlike "*$Expected*") { throw "Unexpected rejection for ${Name}: $($_.Exception.Message)" }
    $failed = $true
  }
  if (-not $failed -or (Test-Path -LiteralPath $output)) { throw "Fail-closed/output-boundary failed: $Name" }
  $passes.Add($Name)
}
Reject 'unexecuted-receipt' { param($r) $r.classification = 'SYNTHETIC_NOT_EXECUTED' } 'not an executed release build'
Reject 'wrong-binary' { param($r) $r.executableSha256 = '0' * 64 } 'Receipt/executable mismatch'
Reject 'wrong-host-binary' { param($r) $r.hostExecutableSha256 = '0' * 64 } 'Receipt/host executable mismatch'
Reject 'source-snapshot-mismatch' { param($r) $r.after.trackedBytesSha256 = '0' * 64 } 'Receipt/source mismatch'
Reject 'false-caller-metadata' { param($r) } 'Caller metadata'
Reject 'false-clean-claim' { param($r) $r.after.clean = $false } 'Receipt/source mismatch'
Reject 'string-clean-claim' { param($r) $r.after.clean = 'true' } 'Receipt/source mismatch'
[IO.File]::WriteAllText((Join-Path $root 'synthetic.txt'), 'changed synthetic source', [Text.UTF8Encoding]::new($false))
Reject 'dirty-source' { param($r) } 'Source must be clean'
$result = [ordered]@{classification='SYNTHETIC_NEGATIVE_TESTS_ONLY'; passed=@($passes); realReleaseBuild='NOT_RUN'; outputCreated=$false}
$json = $result | ConvertTo-Json -Depth 4
[IO.File]::WriteAllText((Join-Path $EvidenceRoot 'result.json'), $json, [Text.UTF8Encoding]::new($false))
$json
