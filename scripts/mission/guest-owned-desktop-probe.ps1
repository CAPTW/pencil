$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
New-Item -ItemType Directory C:\GrammarOutput\native -ErrorAction Stop | Out-Null
$p=Start-Process C:\GrammarPayload\owned-desktop-probe.exe -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\GrammarOutput\launcher.log -RedirectStandardError C:\GrammarOutput\launcher-error.log
$natural=$p.WaitForExit(330000)
if(-not $natural){$p.Kill();$null=$p.WaitForExit(5000)}
[ordered]@{classification='OWNED_DESKTOP_LAUNCHER_NOT_PRODUCT_ACCEPTANCE';naturalExit=$natural;exited=$p.HasExited;exitCode=$p.ExitCode;probeSha256=(Get-FileHash C:\GrammarPayload\owned-desktop-probe.exe).Hash.ToLowerInvariant()}|ConvertTo-Json|Set-Content C:\GrammarOutput\launcher-result.json
