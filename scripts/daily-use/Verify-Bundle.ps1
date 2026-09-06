[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$required = @(
  'codex-pencil.exe',
  'Launch-Grammar.cmd',
  'Check-Prerequisites.ps1',
  'Verify-Bundle.ps1',
  'Create-Shortcuts.ps1',
  'Remove-Shortcuts.ps1',
  'Enable-Startup.ps1',
  'Disable-Startup.ps1',
  'README_FOR_DAILY_USE.md',
  'PROVIDER_SETUP.md',
  'MANUAL_SMOKE_CHECKLIST.md',
  'KNOWN_ISSUES.md',
  'BUILD_INFO.json',
  'SHA256SUMS.txt'
)
$failures = New-Object System.Collections.Generic.List[string]
foreach ($name in $required) {
  if (-not (Test-Path -LiteralPath (Join-Path $here $name))) { $failures.Add("MISSING:$name") }
}
$names = (Get-ChildItem -LiteralPath $here -File).Name
if (($names | Group-Object | Where-Object { $_.Count -gt 1 })) { $failures.Add('DUPLICATE_FILE') }
$infoPath = Join-Path $here 'BUILD_INFO.json'
$sumsPath = Join-Path $here 'SHA256SUMS.txt'
if ((Test-Path -LiteralPath $infoPath) -and (Test-Path -LiteralPath $sumsPath)) {
  $info = Get-Content -LiteralPath $infoPath -Raw | ConvertFrom-Json
  $exe = Join-Path $here 'codex-pencil.exe'
  $exeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLower()
  $exeLen = (Get-Item -LiteralPath $exe).Length
  if ([int64]$info.executableBytes -ne $exeLen) { $failures.Add('EXE_BYTES_MISMATCH') }
  if ([string]$info.executableSha256 -ne $exeHash) { $failures.Add('EXE_HASH_MISMATCH') }
  Get-Content -LiteralPath $sumsPath | Where-Object { $_.Trim() } | ForEach-Object {
    $parts = $_ -split '\s+', 2
    if ($parts.Count -ne 2) { $failures.Add('SUMS_LINE'); return }
    $rel = $parts[1].Trim()
    if ($rel.StartsWith('payload/')) { $rel = $rel.Substring(8) }
    $file = Join-Path $here $rel
    if (-not (Test-Path -LiteralPath $file)) { $failures.Add("SUMS_MISSING:$rel"); return }
    $actual = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $parts[0].ToLower()) { $failures.Add("SUMS_MISMATCH:$rel") }
  }
  $repoNeedle = 'D:\dev\repos' + '\Grammar'
  $oldNeedle = 'GRAMMAR-PROVIDER' + '-RUNTIME-R6'
  $textFiles = Get-ChildItem -LiteralPath $here -File | Where-Object { $_.Extension -match '\.(ps1|cmd|md|json|txt)$' -and $_.Name -ne 'Verify-Bundle.ps1' }
  foreach ($file in $textFiles) {
    $text = [System.IO.File]::ReadAllText($file.FullName)
    if ($text.Contains($repoNeedle)) { $failures.Add("REPO_PATH:$($file.Name)") }
    if ($text.Contains($oldNeedle)) { $failures.Add("OLD_ARTIFACT:$($file.Name)") }
    if ($text -match '(?i)(sk-[A-Za-z0-9]{10,}|api_key\s*=\s*\S+)') { $failures.Add("SECRET_SHAPE:$($file.Name)") }
  }
  foreach ($bad in @('.git','node_modules','target','Cargo.lock','package-lock.json')) {
    if (Test-Path -LiteralPath (Join-Path $here $bad)) { $failures.Add("PROHIBITED:$bad") }
  }
}
if ($failures.Count -gt 0) {
  Write-Output 'FAIL'
  $failures | ForEach-Object { Write-Output $_ }
  exit 1
}
Write-Output 'PASS'
exit 0
