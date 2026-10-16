$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
Start-Transcript C:\GrammarOutput\medium-run.log
$report=[ordered]@{status='FAIL';classification='INSTALLED_TARGET_APPLY_SANDBOX_MEDIUM';providerLive='NOT_RUN'}
$node=$null
try {
 $runtime=Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory | Where-Object{Test-Path (Join-Path $_.FullName 'msedgewebview2.exe')} | Sort-Object Name | Select-Object -Last 1
 if(-not $runtime){throw 'Guest WebView runtime absent'}
 $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=$runtime.FullName
 & C:\GrammarPayload\package-target-apply\New-GrammarTrialScope.ps1 -Root C:\PencilTrialScope
 & C:\GrammarPayload\package-target-apply\Install-Grammar.ps1 -InstallRoot C:\PencilTrialScope\install
 $report.sourceCommit=(Get-Content C:\PencilTrialScope\install\MANIFEST.json -Raw|ConvertFrom-Json).sourceCommit
 $report.appSha256=(Get-FileHash C:\PencilTrialScope\install\codex-pencil.exe -Algorithm SHA256).Hash
 $report.targetSha256=(Get-FileHash C:\PencilTrialScope\install\target-apply.dll -Algorithm SHA256).Hash
 $receipt=Get-Content C:\PencilTrialScope\install\BUILD_RECEIPT.json -Raw|ConvertFrom-Json
 if($receipt.targetApply.brokerSha256 -ine $report.appSha256 -or $receipt.targetApply.librarySha256 -ine $report.targetSha256){throw 'Installed target binding mismatch'}
 $node=Start-Process C:\GrammarPayload\node.exe -ArgumentList C:\GrammarPayload\target-app-e2e.mjs -WorkingDirectory C:\GrammarPayload -RedirectStandardOutput C:\GrammarOutput\app.log -RedirectStandardError C:\GrammarOutput\app-error.log -WindowStyle Hidden -PassThru
 if(-not $node.WaitForExit(360000)){$node.Kill($true);$null=$node.WaitForExit(5000);throw 'App test deadline exceeded'}
 $report.exitCode=$node.ExitCode
 if($node.ExitCode -ne 0){throw 'App target test failed'}
 $result=Get-Content C:\GrammarOutput\app-target-result.json -Raw|ConvertFrom-Json
 if($result.status -ne 'PASS' -or $result.checks.Count -lt 8 -or -not $result.appNatural -or -not $result.editorNatural){throw 'Incomplete target flow'}
 $report.status='PASS'
}catch{$report.error=$_.Exception.Message;$_|Out-String|Set-Content C:\GrammarOutput\medium-error.txt}
finally {
 if($node -and -not $node.HasExited){$node.Kill($true);$null=$node.WaitForExit(5000)}
 if(Test-Path C:\PencilTrialScope\install\install-receipt.json){try{& C:\PencilTrialScope\install\Uninstall-Grammar.ps1 -InstallRoot C:\PencilTrialScope\install -TrialRoot C:\PencilTrialScope -RemoveUserData -UserDataDirectory C:\PencilTrialScope\profile|Set-Content C:\GrammarOutput\uninstall.json}catch{$report.cleanupError=$_.Exception.Message;$report.status='FAIL'}}
 $report.installRootRemaining=Test-Path C:\PencilTrialScope\install
 if($report.installRootRemaining){$report.status='FAIL'}
 $report|ConvertTo-Json -Depth 5|Set-Content C:\GrammarOutput\medium-result.json
 Stop-Transcript
}
