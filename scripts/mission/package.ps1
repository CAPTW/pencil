param(
  [Parameter(Mandatory)][string]$ReceiptPath,
  [Parameter(Mandatory)][string]$OutputRoot,
  [string]$VCRuntimeDirectory,
  [string]$RhwpSourceRoot,
  [ValidateSet('cuda12.4','cuda13.4-blackwell')][string]$LocalRuntimeProfile='cuda13.4-blackwell'
)
$ErrorActionPreference='Stop'
$LocalRuntimeProfile=$LocalRuntimeProfile.ToLowerInvariant()
$source=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$receipt=Get-Content -LiteralPath $ReceiptPath -Raw | ConvertFrom-Json
$verified=& (Join-Path $PSScriptRoot 'build-receipt.ps1') -SourceRoot $source -ReceiptPath $ReceiptPath -VerifyOnly -ExecutablePath $receipt.executablePath -HostExecutablePath $receipt.hostExecutablePath
if($verified.schemaVersion -ne 3){throw 'This candidate requires a source-bound Gemma broker and target service build receipt'}
if(Test-Path -LiteralPath $OutputRoot){throw 'OutputRoot must be fresh'}
# Ship release CRT files from the licensed build toolchain, never System32 or a
# debug runtime. No redistributable installer or machine-wide change is needed.
if (-not $VCRuntimeDirectory) {
  $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
  $vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  if ($LASTEXITCODE -ne 0 -or -not $vs) { throw 'Visual Studio release CRT location unavailable; specify -VCRuntimeDirectory' }
  $candidates = @(Get-ChildItem -Path (Join-Path $vs 'VC/Redist/MSVC/*/x64/Microsoft.VC*.CRT') -Directory | Sort-Object { [version]$_.Parent.Parent.Name } -Descending)
  if (-not $candidates.Count) { throw 'Visual Studio release CRT missing' }
  $VCRuntimeDirectory = $candidates[0].FullName
}
$crtRoot = [IO.Path]::GetFullPath($VCRuntimeDirectory)
if ($crtRoot -notmatch '[\\/]VC[\\/]Redist[\\/]MSVC[\\/][0-9.]+[\\/]x64[\\/]Microsoft\.VC[0-9]+\.CRT$') { throw 'Use the x64 release CRT directory from Visual Studio Redist' }
$crtFiles = @(Get-ChildItem -LiteralPath $crtRoot -File -Filter '*.dll')
foreach ($required in @('vcruntime140.dll','vcruntime140_1.dll','msvcp140.dll')) {
  if ($required -cnotin $crtFiles.Name) { throw "Required release CRT missing: $required" }
}
foreach ($file in $crtFiles) {
  $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
  if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation(?:,|$)') { throw "Untrusted release CRT: $($file.Name)" }
}
$destination=[IO.Path]::GetFullPath($OutputRoot)
New-Item -ItemType Directory -Path $destination | Out-Null
$crtDestination = Join-Path $destination 'vc-runtime'
New-Item -ItemType Directory -Path $crtDestination | Out-Null
foreach ($file in $crtFiles) {
  $copied = Join-Path $crtDestination $file.Name
  Copy-Item -LiteralPath $file.FullName -Destination $copied
  $signature = Get-AuthenticodeSignature -LiteralPath $copied
  if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation(?:,|$)') { throw 'Copied CRT signature invalid' }
}
[IO.File]::WriteAllText((Join-Path $destination 'VC_RUNTIME_NOTICE.md'), @"
# Microsoft Visual C++ runtime
Release x64 runtime $($crtFiles[0].VersionInfo.FileVersion), copied from the build toolchain's Visual Studio Redist directory.
Microsoft Corporation. These files remain subject to the Visual Studio distributable-code license terms.
https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files
Only the owned Gemma process PATH is extended; no system installation occurs.
Private deployment requires future security updates through a new verified Grammar package.
"@, [Text.UTF8Encoding]::new($false))
Copy-Item -LiteralPath $verified.executablePath -Destination (Join-Path $destination 'codex-pencil.exe')
Copy-Item -LiteralPath $verified.hostExecutablePath -Destination (Join-Path $destination 'grammar-chromium-host.exe')
Copy-Item -LiteralPath $verified.targetApply.libraryPath -Destination (Join-Path $destination 'target-apply.dll')
Copy-Item -LiteralPath $verified.gemmaBroker.executablePath -Destination (Join-Path $destination 'grammar-gemma-broker.exe')
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
foreach($item in @('Install-Grammar.ps1','New-GrammarTrialScope.ps1','Uninstall-Grammar.ps1','PERSONAL_USE.md','Start-Grammar.ps1','Prepare-GrammarLocal.ps1')){
  Copy-Item -LiteralPath (Join-Path $source ('scripts/package/'+$item)) -Destination (Join-Path $destination $item)
}
$catalogName = if ($LocalRuntimeProfile -ceq 'cuda13.4-blackwell') { 'Gemma-Assets-CUDA13.json' } else { 'Gemma-Assets.json' }
# Package one explicit, source-bound runtime. Do not switch or fall back at launch.
Copy-Item -LiteralPath (Join-Path $source ('scripts/package/'+$catalogName)) -Destination (Join-Path $destination 'Gemma-Assets.json')
Copy-Item -LiteralPath (Join-Path $source 'adapters/chromium/host') -Destination (Join-Path $destination 'host-tools') -Recurse
Copy-Item -LiteralPath (Join-Path $source 'adapters/chromium/README.md') -Destination (Join-Path $destination 'README.md')
Copy-Item -LiteralPath (Join-Path $source 'docs/control/HANDOFF.md') -Destination (Join-Path $destination 'HANDOFF.md')
Copy-Item -LiteralPath $ReceiptPath -Destination (Join-Path $destination 'BUILD_RECEIPT.json')
Copy-Item -LiteralPath (Join-Path $source 'docs/PROTOTYPE.md') -Destination (Join-Path $destination 'PROTOTYPE.md')
Copy-Item -LiteralPath (Join-Path $source 'scripts/package/START_HERE.md') -Destination (Join-Path $destination 'START_HERE.md')
& python (Join-Path $PSScriptRoot 'package-notices.py') (Join-Path $destination 'THIRD_PARTY_NOTICES')
if ($LASTEXITCODE -ne 0) { throw 'Dependency notice collection failed' }
$null=& (Join-Path $PSScriptRoot 'build-receipt.ps1') -SourceRoot $source -ReceiptPath $ReceiptPath -VerifyOnly -ExecutablePath (Join-Path $destination 'codex-pencil.exe') -HostExecutablePath (Join-Path $destination 'grammar-chromium-host.exe') -TargetLibraryPath (Join-Path $destination 'target-apply.dll') -GemmaBrokerPath (Join-Path $destination 'grammar-gemma-broker.exe')
if ($RhwpSourceRoot) {
  & node (Join-Path $PSScriptRoot 'build-rhwp-extension.mjs') $RhwpSourceRoot (Join-Path $destination 'extension')
  if ($LASTEXITCODE -ne 0) { throw 'Pinned rHWP extension build failed' }
}
$files=@(Get-ChildItem -LiteralPath $destination -File -Recurse -Force | Sort-Object FullName | ForEach-Object {
  [ordered]@{path=[IO.Path]::GetRelativePath($destination,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower();bytes=$_.Length}
})
$manifest=[ordered]@{classification='PERSONAL_USE_CANDIDATE_SYNTHETIC_QUALIFIED_SCOPE';sourceCommit=$verified.after.commit;sourceTree=$verified.after.tree;sourcePhysicalDigest=$verified.after.trackedBytesSha256;extensionId=$extensionId;nativeCapture='SCOPE_QUALIFIED_SINGLELINE_AUTOVSCROLL_MULTILINE_EDIT_AND_RICHEDIT_CANDIDATE';nativeApply='TARGET_SIDE_EDIT_RICHEDIT_ACCEPT_AUTOVSCROLL_MULTILINE_INSTALLED_ACCEPTANCE_PENDING';gemmaIntegration='MANAGED_LAUNCHER_DESKTOP_AND_CHROMIUM_LOCAL_WRITING_SCOPED_CANDIDATE';chromiumInstant='TEXTAREA_SIMPLE_CONTENTEDITABLE_SYNTHETIC_QUALIFIED';chromiumDeep='CONSENT_BOUND_SYNTHETIC_PROVIDER_ONLY';antigravityDeep='PRIVACY_UNQUALIFIED';claudeOAuth='UNAVAILABLE_WITH_BARE';providerLive='NOT_RUN';physicalKeyboardIme='NOT_RUN';files=$files}
$manifest.vcRuntime = [ordered]@{deployment='PRIVATE_PROCESS_PATH';version=$crtFiles[0].VersionInfo.FileVersion;files=@($crtFiles | ForEach-Object { 'vc-runtime/' + $_.Name })}
$manifest.localRuntimeProfile = $LocalRuntimeProfile
[IO.File]::WriteAllText((Join-Path $destination 'MANIFEST.json'),($manifest|ConvertTo-Json -Depth 6),[Text.UTF8Encoding]::new($false))
$zip=$destination+'.zip'
if(Test-Path -LiteralPath $zip){throw 'ZIP already exists'}
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory($destination,$zip)
[ordered]@{classification=$manifest.classification;directory=$destination;zip=$zip;zipSha256=(Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLower();sourceCommit=$verified.after.commit;extensionId=$extensionId;localRuntimeProfile=$LocalRuntimeProfile}|ConvertTo-Json
