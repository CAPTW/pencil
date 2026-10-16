$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
$lock=[IO.File]::Open('C:\GrammarOutput\rhwp.lock',[IO.FileMode]::CreateNew,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
$report=[ordered]@{status='FAIL';scope='ACTUAL_GEMMA_RHWP_SANDBOX';providerLive='NOT_RUN'}
$node=$null
try{
 if(@(Get-Process codex-pencil,grammar-gemma-broker,llama-server,chrome,node -ErrorAction SilentlyContinue).Count){throw 'Previous trial process exists'}
 $runtime=Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory | Where-Object{Test-Path (Join-Path $_.FullName 'msedgewebview2.exe')}|Sort-Object Name|Select-Object -Last 1
 if(-not $runtime){throw 'Guest WebView absent'}
 $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=$runtime.FullName
 & C:\GrammarPackage\New-GrammarTrialScope.ps1 -Root C:\PencilTrialScope
 & C:\GrammarPackage\Install-Grammar.ps1 -InstallRoot C:\PencilTrialScope\install | Set-Content C:\GrammarOutput\install.json
 # Stage explicit read-only assets into the guest installation before measuring
 # product startup. All original manifest hashes are still checked by the broker.
 $stageClock=[Diagnostics.Stopwatch]::StartNew()
 $local='C:\PencilTrialScope\install\local'
 if(Test-Path -LiteralPath $local){throw 'Fresh guest asset directory required'}
 New-Item -ItemType Directory -Path $local|Out-Null
 [ordered]@{phase='copying';complete=$false}|ConvertTo-Json|Set-Content C:\GrammarOutput\asset-stage.json
 Copy-Item -LiteralPath C:\GrammarAssets\runtime -Destination $local -Recurse
 $catalog=Get-Content C:\PencilTrialScope\install\Gemma-Assets.json -Raw|ConvertFrom-Json
 Copy-Item -LiteralPath (Join-Path C:\GrammarAssets $catalog.model.name) -Destination $local
 $modelHash=(Get-FileHash -LiteralPath (Join-Path $local $catalog.model.name) -Algorithm SHA256).Hash.ToLowerInvariant()
 if($modelHash -cne $catalog.model.sha256){throw 'Guest staged model hash mismatch'}
 [ordered]@{phase='staged';complete=$true;elapsedMs=$stageClock.ElapsedMilliseconds;modelSha256=$modelHash;runtimeValidation='RECHECKED_BY_PRODUCT_BROKER'}|ConvertTo-Json|Set-Content C:\GrammarOutput\asset-stage.json
 Copy-Item C:\GrammarPayload\protected-synthetic.hwp,C:\GrammarPayload\protected-synthetic.hwpx C:\PencilTrialScope
 $node=Start-Process C:\GrammarPayload\node.exe -ArgumentList C:\GrammarPayload\rhwp-gemma-sandbox.mjs -WorkingDirectory C:\GrammarPayload -WindowStyle Hidden -RedirectStandardOutput C:\GrammarOutput\app.log -RedirectStandardError C:\GrammarOutput\app-error.log -PassThru
 if(-not $node.WaitForExit(720000)){throw 'Whole GUI trial deadline exceeded'}
 if($node.ExitCode -ne 0){throw 'Actual Gemma rHWP flow failed'}
 $report.status='PASS'
}catch{$report.error=$_.Exception.ToString()}
finally{
 if($node -and -not $node.HasExited){$node.Kill($true);$null=$node.WaitForExit(5000);$report.forcedHarness=$true;$report.status='FAIL'}
 foreach($pattern in @('*.hwp','*.hwpx','installed-receipt.json')){Get-ChildItem -Path (Join-Path C:\PencilTrialScope $pattern) -File -ErrorAction SilentlyContinue | Copy-Item -Destination C:\GrammarOutput}
 if(Test-Path C:\PencilTrialScope\install\install-receipt.json){try{
  $removed=& C:\PencilTrialScope\install\Uninstall-Grammar.ps1 -InstallRoot C:\PencilTrialScope\install -TrialRoot C:\PencilTrialScope -RemoveUserData -UserDataDirectory @('C:\PencilTrialScope\profile','C:\PencilTrialScope\browser-profile')|ConvertFrom-Json
  $removed|ConvertTo-Json -Depth 5|Set-Content C:\GrammarOutput\uninstall.json
  if(-not $removed.complete){throw 'Owned cleanup incomplete'}
 }catch{$report.cleanupError=$_.Exception.ToString();$report.status='FAIL'}}
 $report.remainingProcesses=@(Get-Process codex-pencil,grammar-gemma-broker,grammar-chromium-host,llama-server,chrome,node,msedgewebview2 -ErrorAction SilentlyContinue|Select-Object Id,ProcessName)
 $report.installRemaining=Test-Path C:\PencilTrialScope\install
 $report.profileRemaining=(Test-Path C:\PencilTrialScope\profile) -or (Test-Path C:\PencilTrialScope\browser-profile)
 if($report.remainingProcesses.Count -or $report.installRemaining -or $report.profileRemaining){$report.status='FAIL'}
 $lock.Dispose();$report|ConvertTo-Json -Depth 6|Set-Content C:\GrammarOutput\guest-result.json
}
