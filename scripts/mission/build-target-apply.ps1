#Requires -Version 7.0
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$ExecutablePath,
  [Parameter(Mandatory)][string]$OutputDirectory
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version 3.0
$source=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$exe=Get-Item -LiteralPath $ExecutablePath
if($exe.Name -cne 'codex-pencil.exe'){throw 'Build must bind the installed codex-pencil.exe filename'}
$output=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $output){throw 'Target service output must be fresh'}
if($output.StartsWith($source.TrimEnd('\','/')+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Target service build must be outside source'}
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs=(& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
if($LASTEXITCODE -ne 0 -or -not $vs){throw 'MSVC C++ toolchain unavailable'}
$dev=Join-Path $vs 'Common7/Tools/VsDevCmd.bat'
foreach($path in @($source,$output,$dev,$exe.FullName)){
  if($path -match '["%!\r\n]'){throw 'Unsupported shell metacharacter in build path'}
}
New-Item -ItemType Directory -Path $output | Out-Null
New-Item -ItemType Directory -Path (Join-Path $output 'obj') | Out-Null
$hash=(Get-FileHash -LiteralPath $exe.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$header=Join-Path $output 'trusted-build.hpp'
[IO.File]::WriteAllLines($header,@(('#define GRAMMAR_TRUSTED_APP_SHA256 "'+$hash+'"'),'#define GRAMMAR_TRUSTED_APP_NAME L"codex-pencil.exe"'),[Text.UTF8Encoding]::new($false))
$dll=Join-Path $output 'target-apply.dll'
$cpp=Join-Path $source 'src-tauri/native/target_apply.cpp'
$api=Join-Path $source 'src-tauri/native/target_apply_api.cpp'
$browser=Join-Path $source 'src-tauri/native/browser_bridge_api.cpp'
$obj=(Join-Path $output 'obj').TrimEnd('\','/')+'\\' # Escape the trailing slash before the CRT closing quote.
$command='"'+$dev+'" -arch=x64 && cl /nologo /std:c++17 /EHsc /MT /W4 /WX /DUNICODE /D_UNICODE /FI"'+$header+'" /LD "'+$cpp+'" "'+$api+'" "'+$browser+'" /Fe:"'+$dll+'" /Fo:"'+$obj+'" bcrypt.lib advapi32.lib ole32.lib oleaut32.lib uuid.lib user32.lib comctl32.lib imm32.lib /link /WX'
& $env:ComSpec /d /s /c $command *> (Join-Path $output 'build.log')
if($LASTEXITCODE -ne 0){throw 'Target service compile/link failed; inspect build.log'}
if((Get-FileHash -LiteralPath $exe.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $hash){throw 'Broker changed while building target service'}
$binary=Get-Item -LiteralPath $dll
[ordered]@{
  classification='TARGET_SERVICE_BOUND_TO_BUILT_BROKER_NOT_ACCEPTANCE'
  brokerSha256=$hash; brokerFileName=$exe.Name; libraryPath=$binary.FullName
  libraryBytes=$binary.Length; librarySha256=(Get-FileHash -LiteralPath $dll -Algorithm SHA256).Hash.ToLowerInvariant()
  compilerPolicy='MSVC_X64_CPP17_STATIC_CRT_W4_WX'; exitCode=0
}
