#Requires -Version 7.0
param([Parameter(Mandatory)][string]$EvidenceRoot)
$ErrorActionPreference='Stop'
$root=[IO.Path]::GetFullPath($EvidenceRoot)
if(Test-Path -LiteralPath $root){throw 'Fresh task-owned evidence required'}
New-Item -ItemType Directory -Path $root | Out-Null
$prepare=Join-Path $PSScriptRoot '../adapters/chromium/host/Prepare-Host.ps1'
$checks=@()
function Require($Condition,[string]$Name){if(-not $Condition){throw $Name};$script:checks+=$Name}
# No registry, process, clipboard or user profile access. Files are inert.
foreach($name in @('grammar-chromium-host.exe','codex-pencil.exe','target-apply.dll')){
 [IO.File]::WriteAllText((Join-Path $root $name),'synthetic layout only')
}
$source=Join-Path $root 'grammar-chromium-host.exe'
$hostDirectory=Join-Path $root 'host'
try {
 $receipt=& $prepare -Executable $source -ExtensionId ('a'*32) -OutputDirectory $hostDirectory -UseInstalledRuntime | ConvertFrom-Json
 $manifest=Get-Content -LiteralPath $receipt.manifest -Raw|ConvertFrom-Json
 Require ($manifest.path -ceq $source) 'registered image is the installed root image'
 Require (-not (Test-Path (Join-Path $hostDirectory 'grammar-chromium-host.exe'))) 'no disconnected host copy'
 Require ((Split-Path $manifest.path) -ceq (Split-Path (Join-Path $root 'target-apply.dll'))) 'host DLL and trusted desktop share runtime root'
 foreach($name in @('grammar-chromium-host.origin.json','grammar-cleanup.json','grammar-cleanup.lock')){
  Require (Test-Path (Join-Path $root $name)) ('runtime sidecar '+$name)
 }
 Require (-not $receipt.registered) 'no registry writes'
 $original=(Get-FileHash (Join-Path $root 'grammar-chromium-host.origin.json')).Hash
 $rejected=$false
 try {& $prepare -Executable $source -ExtensionId ('b'*32) -OutputDirectory (Join-Path $root 'wrong') -UseInstalledRuntime | Out-Null}catch{$rejected=$true}
 Require $rejected 'noncanonical layout refused'
 Require ((Get-FileHash (Join-Path $root 'grammar-chromium-host.origin.json')).Hash -ceq $original) 'existing sidecar unchanged'
 $conflict=Join-Path $root 'conflict'
 New-Item -ItemType Directory $conflict | Out-Null
 foreach($name in @('grammar-chromium-host.exe','codex-pencil.exe','target-apply.dll','grammar-chromium-host.origin.json')){
  [IO.File]::WriteAllText((Join-Path $conflict $name),'keep existing bytes')
 }
 $rejected=$false
 try {& $prepare -Executable (Join-Path $conflict 'grammar-chromium-host.exe') -ExtensionId ('b'*32) -OutputDirectory (Join-Path $conflict 'host') -UseInstalledRuntime | Out-Null}catch{$rejected=$true}
 Require ($rejected -and -not (Test-Path (Join-Path $conflict 'host'))) 'sidecar conflict refused before metadata creation'
 Require ([IO.File]::ReadAllText((Join-Path $conflict 'grammar-chromium-host.origin.json')) -ceq 'keep existing bytes') 'sidecar conflict leaves bytes intact'
 $legacy=Join-Path $root 'standalone'
 $old=& $prepare -Executable $source -ExtensionId ('c'*32) -OutputDirectory $legacy | ConvertFrom-Json
 $oldManifest=Get-Content $old.manifest -Raw|ConvertFrom-Json
 Require ($oldManifest.path -ceq (Join-Path $legacy 'grammar-chromium-host.exe')) 'standalone task registration keeps isolated copy'
 [ordered]@{status='PASS';checks=$checks;registry='NOT_USED'}|ConvertTo-Json -Depth 4
}catch{[ordered]@{status='FAIL';checks=$checks;error=$_.Exception.Message}|ConvertTo-Json -Depth 4;exit 1}
