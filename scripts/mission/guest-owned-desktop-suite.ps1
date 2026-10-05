param([ValidateSet('all','standard-edit','standard-scope','standard-dispatcher','rich-service','standard-service','review-service','multiline-edit','multiline-dispatcher','multiline-service')][string]$Probe='all')
$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $env:COMPUTERNAME -eq (Get-Content C:\GrammarPayload\host-machine.txt -Raw).Trim()){throw 'Guest only'}
if(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Restricted Medium only'}
$rows=@()
foreach($name in @('standard-edit','standard-scope','standard-dispatcher','rich-service','standard-service','review-service','multiline-edit','multiline-dispatcher','multiline-service')){
    if($Probe -ne 'all' -and $Probe -ne $name){continue}
    $exe=if($name -like '*service'){'target-service'}else{$name}
    if($name -in @('multiline-edit','multiline-dispatcher')){$exe=$name.Replace('multiline-','standard-')}
    $options=@{FilePath="C:\GrammarPayload\$exe-probe.exe";WindowStyle='Hidden';PassThru=$true;RedirectStandardOutput="C:\GrammarOutput\native\$name.log";RedirectStandardError="C:\GrammarOutput\native\$name-error.log"}
    if($name -eq 'standard-service'){$options.ArgumentList='--standard'}
    if($name -eq 'review-service'){$options.ArgumentList='--standard-review'}
    if($name -in @('multiline-edit','multiline-dispatcher')){$options.ArgumentList='--autovscroll'}
    if($name -eq 'multiline-service'){$options.ArgumentList='--multiline'}
    $p=Start-Process @options
    if(-not $p.WaitForExit(180000)){$p.Kill($true);throw "Owned $name exceeded deadline"}
    $rows+=@{name=$name;exitCode=$p.ExitCode;exited=$p.HasExited}
    [ordered]@{classification='CANDIDATE_NATIVE_ONLY_NOT_INSTALLED';providerLive='NOT_RUN';checks=$rows}|ConvertTo-Json -Depth 4|Set-Content C:\GrammarOutput\native\result.json
}

