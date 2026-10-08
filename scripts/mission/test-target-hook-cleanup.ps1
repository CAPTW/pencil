#Requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$OutputRoot)
$ErrorActionPreference='Stop'
$source=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$output=[IO.Path]::GetFullPath($OutputRoot)
if(Test-Path -LiteralPath $output){throw 'OutputRoot must be fresh'}
if($output.StartsWith($source.TrimEnd('\','/')+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Evidence must be outside source'}
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs=& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if($LASTEXITCODE -ne 0 -or -not $vs){throw 'MSVC toolchain unavailable'}
$dev=Join-Path $vs 'Common7/Tools/VsDevCmd.bat'
foreach($path in @($source,$output,$dev)){if($path -match '["%!\r\n]'){throw 'Unsupported build path'}}
New-Item -ItemType Directory -Path $output | Out-Null
foreach($name in @('native-target-hook-cleanup','native-target-hook-cleanup-policy')) {
  $test=Join-Path $source ('scripts/mission/'+$name+'.cpp')
  $exe=Join-Path $output ($name+'.exe')
  $command='"'+$dev+'" -arch=x64 && cl /nologo /std:c++17 /EHsc /MT /W4 /WX /DUNICODE /D_UNICODE "'+$test+'" /Fe:"'+$exe+'" /Fo:"'+(Join-Path $output ($name+'.obj'))+'" bcrypt.lib advapi32.lib user32.lib'
  & $env:ComSpec /d /s /c $command *> (Join-Path $output ($name+'-build.log'))
  if($LASTEXITCODE -ne 0){throw 'Native regression compile failed'}
  # Hook tests own their threads; policy test injects Unhook failure and owns its
  # child process. Neither uses foreign windows, input, profiles or clipboard.
  & $exe *> (Join-Path $output ($name+'-test.log'))
  if($LASTEXITCODE -ne 0){throw 'Native hook cleanup regression failed'}
  Get-Content -LiteralPath (Join-Path $output ($name+'-test.log'))
}
