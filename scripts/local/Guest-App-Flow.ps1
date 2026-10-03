$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Unprivileged trial required'}
Start-Transcript C:\GrammarOutput\medium-run.log
$report=[ordered]@{status='FAIL';classification='LOCAL_SANDBOX_MEDIUM_INTEGRITY';provider='SYNTHETIC_EXECUTABLE_ONLY';liveProvider='NOT_RUN';groups=(& whoami.exe /groups /fo csv)}
$node=$null
try {
 $env:PATH=($env:PATH.Split(';') | Where-Object { $_ -ine 'C:\GrammarPayload\pwsh' }) -join ';'
 $runtime=Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory | Where-Object{Test-Path (Join-Path $_.FullName 'msedgewebview2.exe')} | Sort-Object Name | Select-Object -Last 1
 if(-not $runtime){throw 'Guest runtime absent'}
 $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=$runtime.FullName
 $report.runtime=$runtime.FullName
 & C:\GrammarPayload\package-flow\Install-Grammar.ps1 -InstallRoot C:\GrammarTrial
 New-Item -ItemType Directory C:\GrammarAppFlow\runtime -ErrorAction Stop|Out-Null
 Copy-Item C:\GrammarPayload\test-runtime\child.exe C:\GrammarAppFlow\runtime\child.exe
 $env:GRAMMAR_OWNED_SANDBOX_TEST='1'
 $env:GRAMMAR_PLAYWRIGHT='C:\GrammarPayload\node_modules\playwright'
 $env:GRAMMAR_EVIDENCE='C:\GrammarAppFlow'
 $env:GRAMMAR_APP_EXE='C:\GrammarTrial\codex-pencil.exe'
 $env:GRAMMAR_APP_KIND='INSTALLED_UNMODIFIED_APP_MEDIUM_EXPLICIT_GUEST_WEBVIEW_RUNTIME'
 $node=Start-Process C:\GrammarPayload\node.exe -ArgumentList 'C:\GrammarPayload\scripts\test-app-flow-e2e.mjs' -WorkingDirectory C:\GrammarPayload -RedirectStandardOutput C:\GrammarOutput\app.log -RedirectStandardError C:\GrammarOutput\app-error.log -WindowStyle Hidden -PassThru
 if(-not $node.WaitForExit(600000)){$node.Kill($true);$null=$node.WaitForExit(5000);throw 'Harness deadline exceeded'}
 $report.exitCode=$node.ExitCode
 if(Test-Path C:\GrammarAppFlow\app-flow\result.json){Copy-Item C:\GrammarAppFlow\app-flow\result.json C:\GrammarOutput\app-result.json}
 if($node.ExitCode -ne 0){throw 'App harness failed; see preserved logs'}
 $result=Get-Content C:\GrammarOutput\app-result.json -Raw|ConvertFrom-Json
 if($result.status -ne 'PASS' -or $result.checks.Count -lt 19){throw 'Incomplete app result'}
 $report.status='PASS'
}catch{$report.error=$_.Exception.Message;$_|Out-String|Set-Content C:\GrammarOutput\medium-error.txt}
finally {
 if($node -and -not $node.HasExited){$node.Kill($true);$null=$node.WaitForExit(5000)}
 if(Test-Path C:\GrammarTrial\install-receipt.json){try{& C:\GrammarTrial\Uninstall-Grammar.ps1 -InstallRoot C:\GrammarTrial -RemoveUserData|Set-Content C:\GrammarOutput\uninstall.json}catch{$report.cleanupError=$_.Exception.Message}}
 $report|ConvertTo-Json -Depth 5|Set-Content C:\GrammarOutput\medium-result.json
 Stop-Transcript
}
