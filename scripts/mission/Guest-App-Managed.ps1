$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
$report=[ordered]@{status='FAIL';classification='INSTALLED_UNMODIFIED_APP_AND_BROKER_SYNTHETIC_ASSET_CATALOG';providerLive='NOT_RUN';modelQuality='NOT_RUN'}
$node=$null
try {
 $runtime=Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory | Where-Object{Test-Path (Join-Path $_.FullName 'msedgewebview2.exe')} | Sort-Object Name | Select-Object -Last 1
 if(-not $runtime){throw 'Guest WebView runtime absent'}
 $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=$runtime.FullName
 & C:\GrammarPayload\package-fixture\Install-Grammar.ps1 -InstallRoot C:\GrammarTrial -SkipBrowserHost
 Copy-Item C:\GrammarPayload\synthetic-assets C:\GrammarTrial\local -Recurse
 $node=Start-Process C:\GrammarPayload\node.exe -ArgumentList C:\GrammarPayload\managed-app-e2e.mjs -WorkingDirectory C:\GrammarPayload -RedirectStandardOutput C:\GrammarOutput\app.log -RedirectStandardError C:\GrammarOutput\app-error.log -WindowStyle Hidden -PassThru
 if(-not $node.WaitForExit(300000)){$node.Kill($true);$null=$node.WaitForExit(5000);throw 'Managed UI test deadline exceeded'}
 if($node.ExitCode -ne 0){throw 'Managed UI test failed'}
 $result=Get-Content C:\GrammarOutput\app-result.json -Raw|ConvertFrom-Json
 if($result.status -ne 'PASS'){throw 'Missing passing app result'}
 $report.status='PASS'
}catch{$report.error=$_.Exception.Message;$_|Out-String|Set-Content C:\GrammarOutput\guest-error.txt}
finally {
 if($node -and -not $node.HasExited){$node.Kill($true);$null=$node.WaitForExit(5000)}
 if(Test-Path C:\GrammarTrial\install-receipt.json){try{& C:\GrammarTrial\Uninstall-Grammar.ps1 -InstallRoot C:\GrammarTrial -RemoveUserData|Set-Content C:\GrammarOutput\uninstall.json}catch{$report.cleanupError=$_.Exception.Message;$report.status='FAIL'}}
 $report.installRootRemaining=Test-Path C:\GrammarTrial
 if($report.installRootRemaining){$report.status='FAIL'}
 $report|ConvertTo-Json -Depth 5|Set-Content C:\GrammarOutput\guest-result.json
}
