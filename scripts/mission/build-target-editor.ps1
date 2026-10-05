#Requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$OutputPath)
$ErrorActionPreference='Stop'
$output=[IO.Path]::GetFullPath($OutputPath)
if(Test-Path -LiteralPath $output){throw 'Use a fresh fixture output path'}
$compiler=Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$source=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'target-editor.cs'))
if(-not (Test-Path -LiteralPath $compiler)){throw 'x64 .NET Framework compiler unavailable'}
& $compiler /nologo /target:exe /platform:x64 /warn:4 /warnaserror+ ('/out:'+$output) $source
if($LASTEXITCODE -ne 0){throw 'Owned fixture controller compile failed'}
[ordered]@{sourceSha256=(Get-FileHash $source).Hash; binarySha256=(Get-FileHash $output).Hash; classification='OWNED_SANDBOX_FIXTURE_ONLY'}
