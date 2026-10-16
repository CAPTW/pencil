# Installed-package cleanup verification only: never starts the app or model.
param([Parameter(Mandatory)][string]$PackageRoot,[Parameter(Mandatory)][string]$TrialRoot)
$ErrorActionPreference='Stop'
Set-StrictMode -Version 3.0
& (Join-Path $PackageRoot 'New-GrammarTrialScope.ps1') -Root $TrialRoot
$owner=Get-Content (Join-Path $TrialRoot '.pencil-trial-owner.json') -Raw|ConvertFrom-Json
$install=Join-Path $TrialRoot 'install'
$profile=Join-Path $TrialRoot 'profile'
# Sibling sentinel is synthetic and created only under the supervising evidence root.
$outside=Join-Path (Split-Path -Parent $TrialRoot) 'outside-sentinel'
if(Test-Path -LiteralPath $outside){throw 'Sentinel must be fresh'}
New-Item -ItemType Directory -Path $outside|Out-Null
Set-Content (Join-Path $outside 'keep.txt') 'synthetic sentinel' -NoNewline
$sentinelHash=(Get-FileHash (Join-Path $outside 'keep.txt')).Hash
$receipt=& (Join-Path $PackageRoot 'Install-Grammar.ps1') -InstallRoot $install -NativeHostsKey $owner.nativeHostsKey|ConvertFrom-Json
$before=(Get-FileHash (Join-Path $install 'install-receipt.json')).Hash
$remove=Join-Path $install 'Uninstall-Grammar.ps1'
$checks=[Collections.Generic.List[string]]::new()
foreach($name in @('Uninstall-Grammar.ps1','New-GrammarTrialScope.ps1')){
 if((Get-FileHash (Join-Path $install $name)).Hash -ne (Get-FileHash (Join-Path $PackageRoot $name)).Hash){throw 'Installed cleanup hash mismatch'}
};$checks.Add('installed cleanup files match packaged bytes')
function Refused([string[]]$Arguments){
 $out=& pwsh -NoProfile -File $remove -InstallRoot $install @Arguments 2>&1
 if($LASTEXITCODE -eq 0){throw 'Unsafe cleanup unexpectedly succeeded'}
 if((Get-FileHash (Join-Path $install 'install-receipt.json')).Hash -ne $before){throw 'Refused cleanup changed receipt'}
 if((Get-FileHash (Join-Path $outside 'keep.txt')).Hash -ne $sentinelHash){throw 'Outside sentinel changed'}
 if(-not(Test-Path -LiteralPath $receipt.nativeHost.registryKey)){throw 'Refused cleanup removed registry'}
}
Refused @('-RemoveUserData');$checks.Add('missing data path refused unchanged')
Refused @('-RemoveUserData','-UserDataDirectory',$outside);$checks.Add('outside sibling refused unchanged')
Refused @('-RemoveUserData','-UserDataDirectory',($TrialRoot+'\..\outside-sentinel'));$checks.Add('traversal refused unchanged')
$link=Join-Path $TrialRoot 'linked-profile'
New-Item -ItemType Junction -Path $link -Target $outside|Out-Null
Refused @('-RemoveUserData','-UserDataDirectory',$link);$checks.Add('junction refused unchanged')
# Remove only the link entry, not its target.
[IO.Directory]::Delete($link)
$removed=& $remove -InstallRoot $install -TrialRoot $TrialRoot -RemoveUserData -UserDataDirectory $profile|ConvertFrom-Json
if(-not $removed.complete -or (Test-Path -LiteralPath $install) -or (Test-Path -LiteralPath $profile) -or (Test-Path -LiteralPath $receipt.nativeHost.registryKey)){throw 'Owned cleanup incomplete'}
if((Get-FileHash (Join-Path $outside 'keep.txt')).Hash -ne $sentinelHash){throw 'Outside sentinel changed'}
$checks.Add('owned installation profile and host registration removed; outside preserved')
[ordered]@{status='PASS';checks=$checks;userDataTouched=$false;appStarted=$false;registryRemoved=$true;outsideSentinelUnchanged=$true}|ConvertTo-Json -Depth 4
