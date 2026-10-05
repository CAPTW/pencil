#Requires -Version 7.0
$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Medium only'}
$receipt=[ordered]@{status='RUNNING'; cleanup=$false; forced=$false; grammarAccept='NOT_RUN'; rendering='SAL_SKIA=raster'}
$office=$null; $python=$null
function Save-Receipt { $receipt|ConvertTo-Json -Depth 4|Set-Content C:\GrammarOutput\launcher.json }
try {
    Save-Receipt
    $extract=Start-Process C:\GrammarLOTools\7z.exe -ArgumentList @('x','C:\GrammarLOTools\engine.paf.exe','App\libreoffice\*','-oC:\GrammarLibreOffice','-y','-bso0','-bsp0') -WindowStyle Hidden -PassThru
    if(-not $extract.WaitForExit(300000)){ $extract.Kill($true); throw 'Extraction timeout' }
    if($extract.ExitCode -ne 0){throw 'Extraction failed'}
    $program='C:\GrammarLibreOffice\App\libreoffice\program'
    # Application-only software rendering for this vGPU-disabled guest; no system setting changes.
    $env:SAL_SKIA='raster'
    $probe='C:\GrammarPayload\libreoffice-selection-probe.py'
    if($env:GRAMMAR_AFFINITY_FIXTURE -eq '1' -or $env:GRAMMAR_WRITER_FIXTURE -eq '1'){
        $component='C:\GrammarPayload\GrammarAffinity.oxt'
        if($env:GRAMMAR_WRITER_FIXTURE -eq '1'){$component='C:\GrammarPayload\GrammarWriter.oxt'}
        $registration=Start-Process "$program\unopkg.com" -ArgumentList @('-env:UserInstallation=file:///C:/GrammarLOProfile','add','--suppress-license',$component) -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\GrammarOutput\registration.log -RedirectStandardError C:\GrammarOutput\registration-error.log
        if(-not $registration.WaitForExit(60000)){$registration.Kill($true);throw 'Fixture registration timeout'}
        if($registration.ExitCode -ne 0){throw 'Fixture registration failed'}
        $probe='C:\GrammarPayload\libreoffice-affinity-probe.py'
        $receipt.scope='fixture component UI affinity; no mutation'
        if($env:GRAMMAR_WRITER_FIXTURE -eq '1'){
            $probe='C:\GrammarPayload\libreoffice-writer-client.py'
            $receipt.scope='experimental target-side Writer transaction; synthetic only'
        }
    }
    $pipe='grammar_fixture_'+[guid]::NewGuid().ToString('N')
    $office=Start-Process "$program\soffice.com" -ArgumentList @('-env:UserInstallation=file:///C:/GrammarLOProfile','--norestore','--nofirststartwizard','--nologo','--nodefault',"--accept=pipe,name=$pipe;urp;StarOffice.ComponentContext") -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\GrammarOutput\office.log -RedirectStandardError C:\GrammarOutput\office-error.log
    $env:PYTHONPATH=$program
    $python=Start-Process "$program\python.exe" -ArgumentList @($probe,$pipe,'direct') -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\GrammarOutput\python.log -RedirectStandardError C:\GrammarOutput\python-error.log
    if(-not $python.WaitForExit(120000)){ $python.Kill($true); $receipt.forced=$true; throw 'Owned Python probe timeout' }
    $receipt.pythonExit=$python.ExitCode
    if(-not $office.WaitForExit(15000)){throw 'Office natural exit timeout'}
    $receipt.officeExit=$office.ExitCode
    if($python.ExitCode -ne 0){throw 'UNO capability probe failed'}
    if($office.ExitCode -ne 0){throw 'Office exit failed'}
    $receipt.status='PASS'
} catch { $receipt.status='FAIL'; $receipt.error=$_.Exception.Message }
finally {
    if($office -and -not $office.HasExited){$office.Kill($true); $office.WaitForExit(10000)|Out-Null; $receipt.forced=$true}
    if($office -and $office.HasExited){$receipt.officeExit=$office.ExitCode}
    $receipt.remainingProcesses=@(Get-Process soffice,soffice.bin -ErrorAction SilentlyContinue).Count
    if($receipt.remainingProcesses -eq 0){
        foreach($owned in @('C:\GrammarLibreOffice','C:\GrammarLOProfile')){
            if(Test-Path -LiteralPath $owned){
                if((Resolve-Path -LiteralPath $owned).Path -ne $owned){throw 'Cleanup path mismatch'}
                Remove-Item -LiteralPath $owned -Recurse -Force
            }
        }
        $receipt.cleanup=(-not(Test-Path C:\GrammarLibreOffice)) -and (-not(Test-Path C:\GrammarLOProfile))
    }
    if(-not $receipt.cleanup -or $receipt.forced){$receipt.status='FAIL'}
    Save-Receipt
}
if($receipt.status -ne 'PASS'){exit 1}
