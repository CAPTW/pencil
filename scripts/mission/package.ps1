param(
  [Parameter(Mandatory)][string]$ReceiptPath,
  [Parameter(Mandatory)][string]$OutputRoot
)
$ErrorActionPreference='Stop'
$source=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$receipt=Get-Content -LiteralPath $ReceiptPath -Raw | ConvertFrom-Json
$verified=& (Join-Path $PSScriptRoot 'build-receipt.ps1') -SourceRoot $source -ReceiptPath $ReceiptPath -VerifyOnly -ExecutablePath $receipt.executablePath -HostExecutablePath $receipt.hostExecutablePath
if(Test-Path -LiteralPath $OutputRoot){throw 'OutputRoot must be fresh'}
$destination=[IO.Path]::GetFullPath($OutputRoot)
New-Item -ItemType Directory -Path $destination | Out-Null
Copy-Item -LiteralPath $verified.executablePath -Destination (Join-Path $destination 'codex-pencil.exe')
Copy-Item -LiteralPath $verified.hostExecutablePath -Destination (Join-Path $destination 'grammar-chromium-host.exe')
Copy-Item -LiteralPath (Join-Path $source 'adapters/chromium/extension') -Destination (Join-Path $destination 'extension') -Recurse
Copy-Item -LiteralPath (Join-Path $source 'adapters/chromium/host') -Destination (Join-Path $destination 'host-tools') -Recurse
Copy-Item -LiteralPath (Join-Path $source 'adapters/chromium/README.md') -Destination (Join-Path $destination 'README.md')
Copy-Item -LiteralPath (Join-Path $source 'docs/control/HANDOFF.md') -Destination (Join-Path $destination 'HANDOFF.md')
Copy-Item -LiteralPath $ReceiptPath -Destination (Join-Path $destination 'BUILD_RECEIPT.json')
$null=& (Join-Path $PSScriptRoot 'build-receipt.ps1') -SourceRoot $source -ReceiptPath $ReceiptPath -VerifyOnly -ExecutablePath (Join-Path $destination 'codex-pencil.exe') -HostExecutablePath (Join-Path $destination 'grammar-chromium-host.exe')
$files=@(Get-ChildItem -LiteralPath $destination -File -Recurse | Sort-Object FullName | ForEach-Object {
  [ordered]@{path=[IO.Path]::GetRelativePath($destination,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower();bytes=$_.Length}
})
$manifest=[ordered]@{classification='UNQUALIFIED_MISSION_CANDIDATE';sourceCommit=$verified.after.commit;sourceTree=$verified.after.tree;sourcePhysicalDigest=$verified.after.trackedBytesSha256;nativeCapture='WITHHELD';nativeApply='COPY_ONLY';chromiumDeep='CONSENT_BOUND_CANDIDATE_UNQUALIFIED';antigravityDeep='PRIVACY_UNQUALIFIED';claudeOAuth='UNAVAILABLE_WITH_BARE';providerLive='NOT_RUN';files=$files}
[IO.File]::WriteAllText((Join-Path $destination 'MANIFEST.json'),($manifest|ConvertTo-Json -Depth 6),[Text.UTF8Encoding]::new($false))
$zip=$destination+'.zip'
if(Test-Path -LiteralPath $zip){throw 'ZIP already exists'}
[IO.Compression.ZipFile]::CreateFromDirectory($destination,$zip)
[ordered]@{classification=$manifest.classification;directory=$destination;zip=$zip;zipSha256=(Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLower();sourceCommit=$verified.after.commit}|ConvertTo-Json
