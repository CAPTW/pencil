$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
$report=[ordered]@{status='FAIL';scope='OFFLINE_INSTALLED_GUI_AND_CLEANUP_BOUNDARY';providerLive='NOT_RUN'}
$node=$null
try {
 & C:\GrammarPayload\test-installed-cleanup.ps1 -PackageRoot C:\GrammarPayload\package -TrialRoot C:\CleanupNegativeScope | Set-Content C:\GrammarOutput\negative-result.json
 $env:GRAMMAR_TRIAL_ROOT=''
 & C:\GrammarPayload\package\New-GrammarTrialScope.ps1 -Root C:\PencilTrialScope
 $runtime=Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory | Where-Object{Test-Path (Join-Path $_.FullName 'msedgewebview2.exe')} | Sort-Object Name | Select-Object -Last 1
 if(-not $runtime){throw 'Guest WebView unavailable'}
 $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=$runtime.FullName
 & C:\GrammarPayload\package\Install-Grammar.ps1 -InstallRoot C:\PencilTrialScope\install | Set-Content C:\GrammarOutput\install.json
 $node=Start-Process C:\GrammarPayload\node.exe -ArgumentList C:\GrammarPayload\cleanup-gui.mjs -WorkingDirectory C:\GrammarPayload -WindowStyle Hidden -RedirectStandardOutput C:\GrammarOutput\gui.log -RedirectStandardError C:\GrammarOutput\gui-error.log -PassThru
 if(-not $node.WaitForExit(240000)){throw 'GUI deadline exceeded'}
 if($node.ExitCode -ne 0){throw 'GUI test failed'}
 $report.status='PASS'
}catch{$report.error=$_.Exception.ToString()}
finally{
 if($node -and -not $node.HasExited){$node.Kill($true);$null=$node.WaitForExit(5000);$report.status='FAIL'}
 if(Test-Path C:\PencilTrialScope\install\install-receipt.json){try{
  & C:\PencilTrialScope\install\Uninstall-Grammar.ps1 -InstallRoot C:\PencilTrialScope\install -TrialRoot C:\PencilTrialScope -RemoveUserData -UserDataDirectory C:\PencilTrialScope\profile | Set-Content C:\GrammarOutput\uninstall.json
 }catch{$report.cleanupError=$_.Exception.ToString();$report.status='FAIL'}}
 $report.installRemaining=Test-Path C:\PencilTrialScope\install
 $report.profileRemaining=Test-Path C:\PencilTrialScope\profile
 $report.productProcesses=@(Get-Process codex-pencil,grammar-gemma-broker,grammar-chromium-host -ErrorAction SilentlyContinue).Count
 if($report.installRemaining -or $report.profileRemaining -or $report.productProcesses){$report.status='FAIL'}
 $report|ConvertTo-Json -Depth 5|Set-Content C:\GrammarOutput\guest-result.json
}
