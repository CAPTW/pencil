#Requires -Version 7.0
$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
$rows=@()
foreach($name in @('apply-channel','browser-bridge')) {
    $p=Start-Process "C:\GrammarPayload\$name-probe.exe" -WindowStyle Hidden -PassThru -RedirectStandardOutput "C:\GrammarOutput\$name.log" -RedirectStandardError "C:\GrammarOutput\$name-error.log"
    $natural=$p.WaitForExit(180000)
    if(-not $natural){$p.Kill($true);throw "Owned $name timeout; not qualified"}
    $rows+=@{name=$name;naturalExit=$natural;exitCode=$p.ExitCode;sha256=(Get-FileHash "C:\GrammarPayload\$name-probe.exe").Hash.ToLowerInvariant()}
    $rows|ConvertTo-Json -Depth 4|Set-Content C:\GrammarOutput\transport-result.json
}
& C:\GrammarPayload\Guest-Deadline-OwnedDesktop.ps1

