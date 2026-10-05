#Requires -Version 7.0
[CmdletBinding()]
param([Parameter(Mandatory)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
$source=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$output=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $output){throw 'Use a fresh evidence directory'}
if($output.StartsWith($source.TrimEnd('\','/')+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Evidence must be outside source'}
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs=& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if($LASTEXITCODE -ne 0 -or -not $vs){throw 'MSVC unavailable'}
$dev=Join-Path $vs 'Common7/Tools/VsDevCmd.bat'
foreach($path in @($source,$output,$dev)){if($path -match '["%!\r\n]'){throw 'Unsupported shell metacharacter in path'}}
New-Item -ItemType Directory $output | Out-Null
$libs='user32.lib imm32.lib ole32.lib oleaut32.lib uuid.lib advapi32.lib bcrypt.lib comctl32.lib'
$define=' '
$names=@('apply-channel')
$sourceCommit=& git -C $source rev-parse HEAD
function Get-ProbeInputs {
    $files=@(Get-ChildItem -LiteralPath (Join-Path $source 'src-tauri/native') -File | Where-Object Extension -in '.hpp','.cpp')
    $files+=@($names | ForEach-Object {Get-Item (Join-Path $PSScriptRoot "native-$_-probe.cpp")})
    $files+=Get-Item $PSCommandPath
    @($files | Sort-Object FullName | ForEach-Object {[ordered]@{path=$_.FullName.Substring($source.Length+1);sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
}
$before=@(Get-ProbeInputs)
$before|ConvertTo-Json -Depth 4|Set-Content (Join-Path $output 'source-before.json')
foreach($name in $names){
    $obj=Join-Path $output "$name-obj"
    New-Item -ItemType Directory $obj | Out-Null
    $cpp=Join-Path $PSScriptRoot "native-$name-probe.cpp"
    $exe=Join-Path $output "$name-probe.exe"
    $command='"'+$dev+'" -arch=x64 && cl /nologo /std:c++17 /EHsc /MT /W4 /WX /DUNICODE /D_UNICODE "'+$cpp+'" /Fe:"'+$exe+'" /Fo:"'+$obj+'\probe.obj" '+$libs+' /link /WX'
    $command=$command.Replace(' /DUNICODE ',' /DUNICODE '+$define)
    & $env:ComSpec /d /s /c $command *> (Join-Path $output "$name-build.log")
    if($LASTEXITCODE -ne 0){throw "$name compile failed"}
}
$after=@(Get-ProbeInputs)
if(($before|ConvertTo-Json -Compress) -cne ($after|ConvertTo-Json -Compress) -or $sourceCommit -cne (& git -C $source rev-parse HEAD)){throw 'Source changed during build; evidence is not qualified'}
[ordered]@{
    classification='SOURCE_HASH_BOUND_NATIVE_PROBES_NOT_INSTALLED_ACCEPTANCE'
    sourceCommit=$sourceCommit
    sourceDirty=(@(& git -C $source status --porcelain).Count -gt 0)
    diagnostics=$false
    inputs=$before
    outputs=@(Get-ChildItem -LiteralPath $output -File | Where-Object Extension -in '.exe','.dll' | ForEach-Object {@{path=$_.Name;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
}|ConvertTo-Json -Depth 5|Set-Content (Join-Path $output 'build-receipt.json')
