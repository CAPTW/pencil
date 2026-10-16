# Create/reuse only an explicitly owned test scope. Never adopts a nonempty directory.
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Root,[string]$RegistryId)
$ErrorActionPreference='Stop'
Set-StrictMode -Version 3.0
if($Root -notmatch '^[A-Za-z]:[\\/]' -or $Root.Substring(2).Contains(':') -or $Root -match '(^|[\\/])\.\.?([\\/]|$)'){throw 'INVALID_TRIAL_ROOT'}
$full=[IO.Path]::GetFullPath($Root).TrimEnd('\','/')
if($env:GRAMMAR_TRIAL_ROOT){
 $parent=[IO.Path]::GetFullPath($env:GRAMMAR_TRIAL_ROOT).TrimEnd('\','/')
 if($full -ine $parent -and -not $full.StartsWith($parent+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'SCOPE_OUTSIDE_SUPERVISOR'}
}
foreach($blocked in @($env:USERPROFILE,$env:APPDATA,$env:LOCALAPPDATA,[IO.Path]::GetPathRoot($full))){if($blocked -and ($full -ieq $blocked.TrimEnd('\') -or $blocked.StartsWith($full+'\',[StringComparison]::OrdinalIgnoreCase))){throw 'TRIAL_ROOT_TOO_BROAD'}}
$p=$full
while($p){if((Test-Path -LiteralPath $p) -and ((Get-Item -LiteralPath $p -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)){throw 'TRIAL_ROOT_REPARSE'};$p=[IO.Path]::GetDirectoryName($p)}
$marker=Join-Path $full '.pencil-trial-owner.json'
if(Test-Path -LiteralPath $full){
 if(-not(Test-Path -LiteralPath $marker -PathType Leaf)){throw 'EXISTING_ROOT_NOT_OWNED'}
 if((Get-Item -LiteralPath $marker -Force).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'TRIAL_MARKER_REPARSE'}
 $owner=Get-Content -LiteralPath $marker -Raw|ConvertFrom-Json
 if($owner.schema -cne 'pencil-owned-trial/v1' -or $owner.root -ine $full -or $owner.id -notmatch '^[a-f0-9]{32}$'){throw 'TRIAL_OWNER_INVALID'}
 if($RegistryId -and $owner.id -cne $RegistryId){throw 'TRIAL_REGISTRY_ID_MISMATCH'}
}else{
 if($RegistryId -and $RegistryId -notmatch '^[a-f0-9]{32}$'){throw 'TRIAL_REGISTRY_ID_INVALID'}
 $id=if($RegistryId){$RegistryId}else{[guid]::NewGuid().ToString('N')}
 New-Item -ItemType Directory -Path $full|Out-Null
 $owner=[ordered]@{schema='pencil-owned-trial/v1';root=$full;id=$id;nativeHostsKey=('HKCU:\Software\PencilTrials\'+$id+'\NativeMessagingHosts')}
 [IO.File]::WriteAllText($marker,($owner|ConvertTo-Json),[Text.UTF8Encoding]::new($false))
}
$profile=Join-Path $full 'profile'
if((Test-Path -LiteralPath $profile) -and ((Get-Item -LiteralPath $profile -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)){throw 'TRIAL_PROFILE_REPARSE'}
New-Item -ItemType Directory -Path $profile -Force|Out-Null
$env:GRAMMAR_TRIAL_ROOT=$full
$env:GRAMMAR_HOST_TRIAL='owned-scope'
$env:GRAMMAR_PROFILE_DIRECTORY=$profile
$env:WEBVIEW2_USER_DATA_FOLDER=Join-Path $profile 'webview'
