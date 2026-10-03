$ErrorActionPreference='Stop'
if($PSScriptRoot -ne 'C:\GrammarPayload' -or $env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Owned guest only'}
# Existing Sandbox login context; no token elevation, security policy changes,
# credentials or host desktop access. The app still uses MediumLaunch.
$watch=[Diagnostics.Stopwatch]::StartNew()
while(-not(Test-Path C:\GrammarOutput\tray-request.json) -and $watch.Elapsed.TotalMinutes -lt 10){Start-Sleep -Milliseconds 200}
if(-not(Test-Path C:\GrammarOutput\tray-request.json)){exit 2}
try {
 $request=Get-Content C:\GrammarOutput\tray-request.json -Raw|ConvertFrom-Json
 $result=& C:\GrammarPayload\Guest-Tray-Quit.ps1 -AppPid ([int]$request.appPid) | ConvertFrom-Json
 $result|Add-Member -NotePropertyName helperIntegrity -NotePropertyValue 'EXISTING_GUEST_LOGIN'
}catch{$result=@{status='GESTURE_FAILED';error=$_.Exception.Message}}
$result|ConvertTo-Json -Depth 4|Set-Content C:\GrammarOutput\tray-gesture.tmp -Encoding utf8
Move-Item C:\GrammarOutput\tray-gesture.tmp C:\GrammarOutput\tray-gesture.json
