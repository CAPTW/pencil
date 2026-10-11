#Requires -Version 7.0
# Explicit first-time setup. No account, driver install or model execution.
[CmdletBinding()]
param([string]$AssetCache, [string]$TimingReceiptPath)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0
$root = [IO.Path]::GetFullPath($PSScriptRoot)
$catalogPath = Join-Path $root 'Gemma-Assets.json'
$manifest = Get-Content -LiteralPath (Join-Path $root 'MANIFEST.json') -Raw | ConvertFrom-Json
$entry = @($manifest.files | Where-Object path -CEQ 'Gemma-Assets.json')
if ($entry.Count -ne 1 -or (Get-FileHash -LiteralPath $catalogPath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $entry[0].sha256) { throw 'Local assets catalog changed' }
$catalog = Get-Content -LiteralPath $catalogPath -Raw | ConvertFrom-Json
if ($catalog.schema -cne 'grammar-local-assets/v1') { throw 'Unsupported local assets catalog' }
$final = Join-Path $root 'local'
$stage = Join-Path $root 'local.pending'
if ((Test-Path -LiteralPath $final) -or (Test-Path -LiteralPath $stage)) { throw 'Local assets already exist, or a previous setup needs cleanup. Existing files were preserved.' }
$http = [Net.Http.HttpClient]::new()
$http.Timeout = [Threading.Timeout]::InfiniteTimeSpan
$cancel = [Threading.CancellationTokenSource]::new([TimeSpan]::FromHours(2))
$complete = $false
$timing = [Diagnostics.Stopwatch]::StartNew()
$phases = [Collections.Generic.List[object]]::new()
function Record-Phase([string]$Phase, [long]$Started) {
  $phases.Add(@{phase=$Phase;elapsedMs=$timing.ElapsedMilliseconds;durationMs=($timing.ElapsedMilliseconds-$Started)})
  if ($TimingReceiptPath) {
    # Explicit diagnostic destination only; no document, credential or source paths.
    try { [IO.File]::WriteAllText([IO.Path]::GetFullPath($TimingReceiptPath), (@{schema='grammar-asset-timing/v1';complete=$complete;phases=@($phases.ToArray())}|ConvertTo-Json -Depth 5)) } catch { Write-Warning "Asset timing receipt unavailable" }
  }
}
function Expand-QualifiedRuntime([string]$Archive, [string]$Destination) {
  $zip=[IO.Compression.ZipFile]::OpenRead($Archive)
  try {
    $names=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    [long]$total=0
    foreach ($entry in $zip.Entries) {
      # Pinned runtime archives are flat. Validate even entries not extracted.
      if ($entry.FullName -cnotmatch '^[A-Za-z0-9_.-]+$' -or $entry.FullName -in @('.','..') -or -not $names.Add($entry.FullName)) { throw 'Invalid or duplicate runtime archive entry' }
      $total += $entry.Length
      if ($total -gt 8GB) { throw 'Runtime archive exceeds bound' }
      if (Test-Path -LiteralPath (Join-Path $Destination $entry.FullName)) { throw 'Runtime archive collision' }
    }
    foreach ($entry in $zip.Entries) {
      $cancel.Token.ThrowIfCancellationRequested()
      # Avoid creating 20 unused executables only to immediately delete them.
      # All DLLs and license files remain, preserving the qualified bundle digest.
      if ($entry.FullName.EndsWith('.exe',[StringComparison]::OrdinalIgnoreCase) -and $entry.FullName -cne 'llama-server.exe') { continue }
      [IO.Compression.ZipFileExtensions]::ExtractToFile($entry,(Join-Path $Destination $entry.FullName),$false)
    }
  } finally { $zip.Dispose() }
}
try {
  New-Item -ItemType Directory -Path $stage | Out-Null
  $runtime = Join-Path $stage 'runtime'
  New-Item -ItemType Directory -Path $runtime | Out-Null
  foreach ($asset in @($catalog.runtimeArchives) + @($catalog.model)) {
    $phaseStart=$timing.ElapsedMilliseconds
    if ($asset.name -notmatch '^[A-Za-z0-9_.-]+$' -or $asset.sha256 -cnotmatch '^[0-9a-f]{64}$' -or $asset.size -le 0) { throw 'Invalid asset descriptor' }
    $destination = Join-Path $stage $asset.name
    if ($AssetCache) {
      # Explicit cache mode is offline; never silently downloads on a cache miss.
      $cached = Join-Path ([IO.Path]::GetFullPath($AssetCache)) $asset.name
      Write-Host "Copying cached $($asset.name)."
      Copy-Item -LiteralPath $cached -Destination $destination
    } else {
      $uri = [Uri]$asset.url
      if ($uri.Scheme -cne 'https' -or $uri.Host -notin @('github.com','huggingface.co') -or $uri.UserInfo) { throw 'Asset URL is outside the trusted catalog origins' }
      Write-Host "Downloading $($asset.name) ($($asset.size) bytes)."
      $response = $http.GetAsync($uri, [Net.Http.HttpCompletionOption]::ResponseHeadersRead, $cancel.Token).GetAwaiter().GetResult()
      try {
        $null = $response.EnsureSuccessStatusCode()
        $input = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
        $output = [IO.File]::Open($destination, 'CreateNew', 'Write', 'None')
        try {
          $buffer = [byte[]]::new(1048576); [long]$total = 0
          while (($count = $input.ReadAsync($buffer,0,$buffer.Length,$cancel.Token).GetAwaiter().GetResult()) -gt 0) {
            $total += $count
            if ($total -gt $asset.size) { throw 'Download exceeds the pinned asset size' }
            $output.Write($buffer,0,$count)
          }
        } finally { $output.Dispose(); $input.Dispose() }
      } finally { $response.Dispose() }
    }
    Write-Host "Verifying $($asset.name)."
    if ((Get-Item -LiteralPath $destination).Length -ne $asset.size -or (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant() -cne $asset.sha256) { throw "Asset integrity failed: $($asset.name)" }
    $cancel.Token.ThrowIfCancellationRequested()
    Record-Phase $(if($asset.name.EndsWith('.zip')){'runtime_copy_verify'}else{'model_copy_verify'}) $phaseStart
    if ($asset.name.EndsWith('.zip',[StringComparison]::Ordinal)) {
      $phaseStart=$timing.ElapsedMilliseconds
      Expand-QualifiedRuntime $destination $runtime
      Remove-Item -LiteralPath $destination
      Record-Phase 'runtime_extract' $phaseStart
    }
  }
  $phaseStart=$timing.ElapsedMilliseconds
  $names = [string[]]@(Get-ChildItem -LiteralPath $runtime -File | Where-Object { $_.Extension -iin @('.exe','.dll') } | ForEach-Object Name)
  [Array]::Sort($names,[StringComparer]::Ordinal)
  $lines = foreach ($name in $names) { $hash=(Get-FileHash -LiteralPath (Join-Path $runtime $name) -Algorithm SHA256).Hash.ToLowerInvariant(); "$name`0$hash`n" }
  $sha = [Security.Cryptography.SHA256]::Create()
  try { $bundle = ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes(($lines -join ''))))).Replace('-','').ToLowerInvariant() }
  finally { $sha.Dispose() }
  if ($bundle -cne $catalog.runtimeBundleSha256 -or (Get-FileHash -LiteralPath (Join-Path $runtime 'llama-server.exe') -Algorithm SHA256).Hash.ToLowerInvariant() -cne $catalog.runtimeSha256) { throw 'Extracted runtime bundle mismatch' }
  Record-Phase 'runtime_hash' $phaseStart
  $cancel.Token.ThrowIfCancellationRequested()
  [IO.File]::WriteAllText((Join-Path $stage 'assets-receipt.json'), (@{schema='grammar-local-assets-installed/v1';modelSha256=$catalog.model.sha256;runtimeBundleSha256=$bundle;sourceCommit=$manifest.sourceCommit;inference='NOT_RUN'} | ConvertTo-Json))
  [IO.Directory]::Move($stage,$final)
  $complete = $true
  Record-Phase 'complete' $timing.ElapsedMilliseconds
} finally {
  $http.Dispose(); $cancel.Dispose()
  if (-not $complete -and (Test-Path -LiteralPath $stage)) {
    $resolved = [IO.Path]::GetFullPath($stage)
    if ($resolved -cne (Join-Path $root 'local.pending')) { throw 'Refusing unexpected cleanup path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}
Write-Host 'Pinned local assets are ready. Run Start-Grammar.ps1 to start the application and Chromium Gemma service.'
