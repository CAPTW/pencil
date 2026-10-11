#Requires -Version 7.0
# Synthetic executable probes exercise the production installer, not a copy of it.
param([Parameter(Mandatory)][string]$EvidenceRoot)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath($EvidenceRoot)
if (Test-Path -LiteralPath $root) { throw 'EvidenceRoot must be fresh' }
New-Item -ItemType Directory -Path $root | Out-Null
$csc = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$results = @()
foreach ($case in @('missing', 'crash', 'timeout', 'available')) {
  $package = Join-Path $root $case
  New-Item -ItemType Directory -Path $package | Out-Null
  $body = switch ($case) {
    missing { 'return 78;' }
    crash { 'return 7;' }
    timeout { 'System.Threading.Thread.Sleep(60000); return 0;' }
    available { 'return 0;' }
  }
  $source = Join-Path $root ($case + '.cs')
  [IO.File]::WriteAllText($source, 'class Probe { static int Main(string[] args) { if(args.Length != 1 || args[0] != "--check-webview2") return 99; ' + $body + ' } }')
  & $csc /nologo /target:winexe "/out:$package\codex-pencil.exe" $source
  if ($LASTEXITCODE -ne 0) { throw 'Synthetic probe compile failed' }
  Copy-Item (Join-Path $PSScriptRoot 'package/Install-Grammar.ps1') $package
  foreach ($name in @('grammar-chromium-host.exe','Uninstall-Grammar.ps1','BUILD_RECEIPT.json','PERSONAL_USE.md','README.md','HANDOFF.md')) {
    [IO.File]::WriteAllText((Join-Path $package $name), 'synthetic fixture only')
  }
  foreach ($folder in @('extension','host-tools')) { New-Item -ItemType Directory (Join-Path $package $folder) | Out-Null }
  $crtNames = @('vc-runtime/vcruntime140.dll','vc-runtime/vcruntime140_1.dll','vc-runtime/msvcp140.dll')
  if ($case -eq 'available') {
    New-Item -ItemType Directory (Join-Path $package 'vc-runtime') | Out-Null
    foreach ($name in $crtNames) { [IO.File]::WriteAllText((Join-Path $package $name), 'synthetic CRT copy fixture; never loaded') }
    [IO.File]::WriteAllText((Join-Path $package 'VC_RUNTIME_NOTICE.md'), 'synthetic notice')
  }
  $files = @(Get-ChildItem $package -File -Recurse | ForEach-Object { @{path=[IO.Path]::GetRelativePath($package,$_.FullName).Replace('\','/');sha256=(Get-FileHash $_.FullName).Hash.ToLowerInvariant()} })
  $manifest = @{extensionId=('a'*32);sourceCommit='SYNTHETIC';classification='SYNTHETIC';files=$files}
  if ($case -eq 'available') { $manifest.vcRuntime=@{deployment='PRIVATE_PROCESS_PATH';files=$crtNames} }
  $manifest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $package 'MANIFEST.json')
  $destination = Join-Path $root ('installed-' + $case)
  $watch = [Diagnostics.Stopwatch]::StartNew()
  $failure = $null
  try { & (Join-Path $package 'Install-Grammar.ps1') -InstallRoot $destination -SkipBrowserHost | Out-Null }
  catch { $failure = $_.Exception.Message }
  if ($case -eq 'available') {
    if ($failure) { throw $failure }
    $receipt = Get-Content (Join-Path $destination 'install-receipt.json') -Raw | ConvertFrom-Json
    if ($receipt.state -ne 'installed' -or $receipt.nativeHost) { throw 'Available probe installation failed' }
    foreach ($name in $crtNames + @('VC_RUNTIME_NOTICE.md')) {
      if ((Get-FileHash -LiteralPath (Join-Path $destination $name)).Hash -cne (Get-FileHash -LiteralPath (Join-Path $package $name)).Hash) { throw 'Private CRT was not installed byte-for-byte' }
    }
  } else {
    $expected = if ($case -eq 'timeout') { 'timed out' } else { 'probe exit ' + $(if ($case -eq 'missing') {78} else {7}) }
    if (-not $failure -or $failure -notlike "*$expected*" -or (Test-Path -LiteralPath $destination)) { throw "Incorrect fail-closed outcome for $case" }
  }
  $results += @{case=$case;status='PASS';elapsedMs=$watch.ElapsedMilliseconds;installCreated=(Test-Path -LiteralPath $destination)}
}
@{classification='SYNTHETIC_PROBES_PRODUCTION_INSTALLER';results=$results;realAppDiscovery='NOT_RUN'} | ConvertTo-Json -Depth 5 | Tee-Object -FilePath (Join-Path $root 'result.json')
