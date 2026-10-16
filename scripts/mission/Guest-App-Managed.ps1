param([switch]$StatusOnly)
$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
$report=[ordered]@{status='FAIL';classification='INSTALLED_UNMODIFIED_APP_AND_BROKER_SYNTHETIC_ASSET_CATALOG';providerLive='NOT_RUN';modelQuality='NOT_RUN'}
$node=$null
try {
 foreach($file in @('target-service-probe.exe','target-editor.exe','managed-target-focus.exe','node.exe','managed-app-e2e.mjs')){if(-not (Test-Path (Join-Path C:\GrammarPayload $file) -PathType Leaf)){throw ('Missing owned fixture: '+$file)}}
 $runtime=Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory | Where-Object{Test-Path (Join-Path $_.FullName 'msedgewebview2.exe')} | Sort-Object Name | Select-Object -Last 1
 if(-not $runtime){throw 'Guest WebView runtime absent'}
 $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=$runtime.FullName
 & C:\GrammarPayload\package-fixture\New-GrammarTrialScope.ps1 -Root C:\PencilTrialScope
 & C:\GrammarPayload\package-fixture\Install-Grammar.ps1 -InstallRoot C:\PencilTrialScope\install
 Copy-Item C:\GrammarPayload\synthetic-assets C:\PencilTrialScope\install\local -Recurse
 $arguments=@('C:\GrammarPayload\managed-app-e2e.mjs');if($StatusOnly){$arguments+='--status-only'}
 $report.scope=if($StatusOnly){'STARTUP_RETRY_CLEANUP_ONLY_CAPTURE_DRAFT_NOT_RUN'}else{'CAPTURE_DRAFT_STARTUP_RETRY_CLEANUP'}
 $node=Start-Process C:\GrammarPayload\node.exe -ArgumentList $arguments -WorkingDirectory C:\GrammarPayload -RedirectStandardOutput C:\GrammarOutput\app.log -RedirectStandardError C:\GrammarOutput\app-error.log -WindowStyle Hidden -PassThru
 if(-not $node.WaitForExit(300000)){$node.Kill($true);$null=$node.WaitForExit(5000);throw 'Managed UI test deadline exceeded'}
 if($node.ExitCode -ne 0){throw 'Managed UI test failed'}
 $result=Get-Content C:\GrammarOutput\app-result.json -Raw|ConvertFrom-Json
 if($result.status -ne 'PASS'){throw 'Missing passing app result'}
 $report.status='PASS'
}catch{$report.error=$_.Exception.Message;$_|Out-String|Set-Content C:\GrammarOutput\guest-error.txt}
finally {
 if($node -and -not $node.HasExited){$node.Kill($true);$null=$node.WaitForExit(5000)}
 if(Test-Path C:\PencilTrialScope\install\install-receipt.json){try{& C:\PencilTrialScope\install\Uninstall-Grammar.ps1 -InstallRoot C:\PencilTrialScope\install -TrialRoot C:\PencilTrialScope -RemoveUserData -UserDataDirectory C:\PencilTrialScope\profile|Set-Content C:\GrammarOutput\uninstall.json}catch{$report.cleanupError=$_.Exception.Message;$report.status='FAIL'}}
 $report.installRootRemaining=Test-Path C:\PencilTrialScope\install
 if($report.installRootRemaining){$report.status='FAIL'}
 $report|ConvertTo-Json -Depth 5|Set-Content C:\GrammarOutput\guest-result.json
}
