#Requires -Version 7.0
param([switch]$SavedDocx, [switch]$UniformFormat, [switch]$MixedSurroundings, [switch]$Paragraphs, [switch]$CrossFormat, [switch]$ParagraphSpan, [switch]$UniformTopology,
 [ValidatePattern('^writer-product-package(?:-[a-f0-9]{7})?$')][string]$PackageDirectoryName='writer-product-package')
# Offline, task-owned Sandbox only. Payload contains receipt-bound package/OXT.
$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
$report=[ordered]@{status='RUNNING';classification='INSTALLED_WRITER_WIDGET_OWNED_NEW_PLAIN_DOCUMENT';providerLive='NOT_RUN';cleanup=$false;forced=$false}
$office=$null;$python=$null
function Save-Receipt {$report | ConvertTo-Json -Depth 5 | Set-Content C:\GrammarOutput\launcher.json}
try {
 Save-Receipt
 $report.writerExtensionSha256=(Get-FileHash C:\GrammarPayload\GrammarWriter-product.oxt -Algorithm SHA256).Hash.ToLowerInvariant()
 if($SavedDocx){$report.classification='INSTALLED_WRITER_WIDGET_OWNED_SAVED_PLAIN_DOCX'}
 if($UniformFormat){$report.uniformFormat=$true;$report.savedDocx=[bool]$SavedDocx;$report.classification='INSTALLED_WRITER_WIDGET_OWNED_UNIFORM_FORMAT'}
 if($MixedSurroundings){if(-not $UniformFormat){throw 'Mixed requires UniformFormat'};$report.mixedSurroundings=$true;$report.classification='INSTALLED_WRITER_WIDGET_OWNED_MIXED_SURROUNDINGS'}
 if($Paragraphs){if(-not $MixedSurroundings){throw 'Paragraphs requires MixedSurroundings'};$report.paragraphs=$true;$report.classification='INSTALLED_WRITER_WIDGET_OWNED_PARAGRAPHS'}
 if($CrossFormat){if(-not $Paragraphs){throw 'CrossFormat requires Paragraphs'};$report.crossFormat=$true;$report.classification='INSTALLED_WRITER_WIDGET_OWNED_CROSS_FORMAT'}
 if($ParagraphSpan){if(-not $CrossFormat){throw 'ParagraphSpan requires CrossFormat'};$report.paragraphSpan=$true;$report.classification='INSTALLED_WRITER_WIDGET_OWNED_PARAGRAPH_SPAN'}
 $runtime=Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory | Where-Object{Test-Path (Join-Path $_.FullName 'msedgewebview2.exe')} | Sort-Object Name | Select-Object -Last 1
 if(-not $runtime){throw 'Guest WebView runtime absent'}
 $env:WEBVIEW2_BROWSER_EXECUTABLE_FOLDER=$runtime.FullName
 & (Join-Path "C:\GrammarPayload\$PackageDirectoryName" 'Install-Grammar.ps1') -InstallRoot C:\GrammarTrial -SkipBrowserHost
 $report.sourceCommit=(Get-Content C:\GrammarTrial\MANIFEST.json -Raw|ConvertFrom-Json).sourceCommit
 $extract=Start-Process C:\GrammarLOTools\7z.exe -ArgumentList @('x','C:\GrammarLOTools\engine.paf.exe','App\libreoffice\*','-oC:\GrammarLibreOffice','-y','-bso0','-bsp0') -WindowStyle Hidden -PassThru
 if(-not $extract.WaitForExit(300000)){$extract.Kill($true);$report.forced=$true;throw 'Extraction timeout'}
 if($extract.ExitCode -ne 0){throw 'Extraction failed'}
 $program='C:\GrammarLibreOffice\App\libreoffice\program'
 $build=Get-Content C:\GrammarTrial\BUILD_RECEIPT.json -Raw|ConvertFrom-Json
 if(-not $build.targetApply.writerClientEnabled -or $build.targetApply.writerRuntimePath -cne "$program\soffice.bin" -or (Get-FileHash "$program\soffice.bin" -Algorithm SHA256).Hash -ine $build.targetApply.writerSha256){throw 'Writer build binding mismatch'}
 $env:SAL_SKIA='raster'
 $registration=Start-Process "$program\unopkg.com" -ArgumentList @('-env:UserInstallation=file:///C:/GrammarLOProfile','add','--suppress-license','C:\GrammarPayload\GrammarWriter-product.oxt') -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\GrammarOutput\registration.log -RedirectStandardError C:\GrammarOutput\registration-error.log
 if(-not $registration.WaitForExit(60000)){$registration.Kill($true);$report.forced=$true;throw 'Product extension registration timeout'}
 if($registration.ExitCode -ne 0){throw 'Product extension registration failed'}
 $pipe='grammar_writer_product_'+[guid]::NewGuid().ToString('N')
 $office=Start-Process "$program\soffice.com" -ArgumentList @('-env:UserInstallation=file:///C:/GrammarLOProfile','--norestore','--nofirststartwizard','--nologo','--nodefault',"--accept=pipe,name=$pipe;urp;StarOffice.ComponentContext") -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\GrammarOutput\office.log -RedirectStandardError C:\GrammarOutput\office-error.log
 $env:PYTHONPATH=$program
 $probeArgs=@('C:\GrammarPayload\writer-product-probe.py',$pipe,'--widget')
 if($SavedDocx){$probeArgs+='--saved'}
 if($UniformFormat){$probeArgs+='--formatted'}
 if($MixedSurroundings){$probeArgs+='--mixed'}
 if($Paragraphs){$probeArgs+='--paragraphs'}
 if($CrossFormat){$probeArgs+='--cross-format'}
 if($ParagraphSpan){$probeArgs+='--span'}
 if($UniformTopology){if(-not $ParagraphSpan){throw 'UniformTopology requires ParagraphSpan'};$probeArgs+='--topology';$report.uniformTopology=$true;$report.classification='INSTALLED_WRITER_WIDGET_OWNED_UNIFORM_TOPOLOGY'}
 $python=Start-Process "$program\python.exe" -ArgumentList $probeArgs -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\GrammarOutput\python.log -RedirectStandardError C:\GrammarOutput\python-error.log
 if(-not $python.WaitForExit(120000)){$python.Kill($true);$report.forced=$true;throw 'Widget fixture timeout'}
 $report.pythonExit=$python.ExitCode
 if(-not $office.WaitForExit(15000)){throw 'Office natural exit timeout'}
 $report.officeExit=$office.ExitCode
 if($python.ExitCode -ne 0 -or $office.ExitCode -ne 0){throw 'Product flow failed'}
 $widget=Get-Content C:\GrammarOutput\app-writer-result.json -Raw|ConvertFrom-Json
 if($widget.status -ne 'PASS' -or -not $widget.appNatural){throw 'Widget not qualified'}
 $report.status='PASS'
} catch {$report.status='FAIL';$report.error=$_.Exception.Message}
finally {
 if($office -and -not $office.HasExited){$office.Kill($true);$office.WaitForExit(10000)|Out-Null;$report.forced=$true}
 if(Test-Path C:\GrammarTrial\install-receipt.json){
  try {& C:\GrammarTrial\Uninstall-Grammar.ps1 -InstallRoot C:\GrammarTrial -RemoveUserData | Set-Content C:\GrammarOutput\uninstall.json}
  catch {$report.cleanupError=$_.Exception.Message;$report.status='FAIL'}
 }
 $report.remainingProcesses=@(Get-Process soffice,soffice.bin,codex-pencil -ErrorAction SilentlyContinue).Count
 if($report.remainingProcesses -eq 0){
  foreach($owned in @('C:\GrammarLibreOffice','C:\GrammarLOProfile')){
   if(Test-Path -LiteralPath $owned){
    if((Resolve-Path -LiteralPath $owned).Path -ne $owned){throw 'Cleanup path mismatch'}
    Remove-Item -LiteralPath $owned -Recurse -Force
   }
  }
  $report.cleanup=(-not(Test-Path C:\GrammarLibreOffice)) -and (-not(Test-Path C:\GrammarLOProfile)) -and (-not(Test-Path C:\GrammarTrial))
 }
 if(-not $report.cleanup -or $report.forced){$report.status='FAIL'}
 Save-Receipt
}
if($report.status -ne 'PASS'){exit 1}
