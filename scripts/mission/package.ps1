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
# Whole folders are copied, so they must hold exactly the files of the commit:
# no ignored, untracked or hidden extras (build-receipt already refused edits).
foreach ($folder in @('adapters/chromium/extension', 'adapters/chromium/host')) {
  $tracked = @(& git -C $source -c core.quotepath=false ls-files -- $folder)
  if ($LASTEXITCODE -ne 0) { throw 'Source enumeration failed' }
  $present = @(Get-ChildItem -LiteralPath (Join-Path $source $folder) -File -Recurse -Force | ForEach-Object { [IO.Path]::GetRelativePath($source, $_.FullName).Replace('\','/') })
  $extra = @($present | Where-Object { $_ -cnotin $tracked })
  if ($extra.Count) { throw "Packaged source folder has files outside the commit: $($extra -join ', ')" }
}
Copy-Item -LiteralPath (Join-Path $source 'adapters/chromium/extension') -Destination (Join-Path $destination 'extension') -Recurse
# Fix the packaged extension's ID with a per-package public key (the private key
# is never needed to load an unpacked extension and is discarded), so the
# installer can register the native host for that exact origin.
$rsa=[Security.Cryptography.RSA]::Create(2048)
try { $der=$rsa.ExportSubjectPublicKeyInfo() } finally { $rsa.Dispose() }
$sha=[Security.Cryptography.SHA256]::Create()
try { $idHex=([BitConverter]::ToString($sha.ComputeHash($der))).Replace('-','').ToLowerInvariant().Substring(0,32) } finally { $sha.Dispose() }
$extensionId=-join ($idHex.ToCharArray() | ForEach-Object { [char](97 + [Convert]::ToInt32([string]$_,16)) })
$extensionManifestPath=Join-Path $destination 'extension/manifest.json'
$extensionManifest=Get-Content -LiteralPath $extensionManifestPath -Raw | ConvertFrom-Json
$extensionManifest | Add-Member -NotePropertyName key -NotePropertyValue ([Convert]::ToBase64String($der))
[IO.File]::WriteAllText($extensionManifestPath,($extensionManifest | ConvertTo-Json -Depth 6),[Text.UTF8Encoding]::new($false))
foreach($item in @('Install-Grammar.ps1','Uninstall-Grammar.ps1','PERSONAL_USE.md')){
  Copy-Item -LiteralPath (Join-Path $source ('scripts/package/'+$item)) -Destination (Join-Path $destination $item)
}
Copy-Item -LiteralPath (Join-Path $source 'adapters/chromium/host') -Destination (Join-Path $destination 'host-tools') -Recurse
Copy-Item -LiteralPath (Join-Path $source 'adapters/chromium/README.md') -Destination (Join-Path $destination 'README.md')
Copy-Item -LiteralPath (Join-Path $source 'docs/control/HANDOFF.md') -Destination (Join-Path $destination 'HANDOFF.md')
Copy-Item -LiteralPath $ReceiptPath -Destination (Join-Path $destination 'BUILD_RECEIPT.json')
$null=& (Join-Path $PSScriptRoot 'build-receipt.ps1') -SourceRoot $source -ReceiptPath $ReceiptPath -VerifyOnly -ExecutablePath (Join-Path $destination 'codex-pencil.exe') -HostExecutablePath (Join-Path $destination 'grammar-chromium-host.exe')
$files=@(Get-ChildItem -LiteralPath $destination -File -Recurse -Force | Sort-Object FullName | ForEach-Object {
  [ordered]@{path=[IO.Path]::GetRelativePath($destination,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower();bytes=$_.Length}
})
$manifest=[ordered]@{classification='PERSONAL_USE_CANDIDATE_SYNTHETIC_QUALIFIED_SCOPE';sourceCommit=$verified.after.commit;sourceTree=$verified.after.tree;sourcePhysicalDigest=$verified.after.trackedBytesSha256;extensionId=$extensionId;nativeCapture='STANDARD_EDIT_ONLY_SYNTHETIC_QUALIFIED';nativeApply='NONE_DESKTOP_COPY_BUTTON_ONLY_NATIVE_EDITORS_NEVER_MUTATED';chromiumInstant='TEXTAREA_SIMPLE_CONTENTEDITABLE_SYNTHETIC_QUALIFIED';chromiumDeep='CONSENT_BOUND_SYNTHETIC_PROVIDER_ONLY';antigravityDeep='PRIVACY_UNQUALIFIED';claudeOAuth='UNAVAILABLE_WITH_BARE';providerLive='NOT_RUN';physicalKeyboardIme='NOT_RUN';files=$files}
[IO.File]::WriteAllText((Join-Path $destination 'MANIFEST.json'),($manifest|ConvertTo-Json -Depth 6),[Text.UTF8Encoding]::new($false))
$zip=$destination+'.zip'
if(Test-Path -LiteralPath $zip){throw 'ZIP already exists'}
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory($destination,$zip)
[ordered]@{classification=$manifest.classification;directory=$destination;zip=$zip;zipSha256=(Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLower();sourceCommit=$verified.after.commit;extensionId=$extensionId}|ConvertTo-Json
