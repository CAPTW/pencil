$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
$report=[ordered]@{status='FAIL';scope='WORD_DESKTOP_ROUTE_REAL_TSF_PIPE_INERT_OM';office='NOT_RUN';hostClipboard='NOT_USED'}
$probe=$null
try {
    $report.probeSha256=(Get-FileHash C:\GrammarPayload\word-route-probe.exe -Algorithm SHA256).Hash.ToLowerInvariant()
    $probe=Start-Process C:\GrammarPayload\word-route-probe.exe -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\GrammarOutput\probe.log -RedirectStandardError C:\GrammarOutput\probe-error.log
    if(-not $probe.WaitForExit(90000)){$probe.Kill($true);$null=$probe.WaitForExit(5000);throw 'Owned probe deadline'}
    $report.exitCode=$probe.ExitCode
    if($probe.ExitCode -ne 0){throw 'Probe failed'}
    $report.status='PASS'
}catch{$report.error=$_.Exception.Message}
finally {
    if($probe -and -not $probe.HasExited){$probe.Kill($true);$null=$probe.WaitForExit(5000);$report.status='FAIL'}
    $report.processExited=($probe -and $probe.HasExited)
    $report|ConvertTo-Json|Set-Content C:\GrammarOutput\result.json
}
