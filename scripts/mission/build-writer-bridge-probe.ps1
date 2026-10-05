#Requires -Version 7.0
param([Parameter(Mandatory)][string]$OutputDirectory,[Parameter(Mandatory)][string]$VerifiedWriterImage)
$ErrorActionPreference='Stop'
$source=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$output=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $output){throw 'Use fresh output'}
$image=Get-Item -LiteralPath $VerifiedWriterImage
# Exact image from the already publisher-hash-verified portable26.2.4.2 archive.
$writerHash=(Get-FileHash -LiteralPath $image.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
if($writerHash -cne '651f5bc9c0a771b65f1e5124079baade982c2b5578fa05d93fe48a665f4593e7'){throw 'Unsupported fixture Writer image'}
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs=& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$dev=Join-Path $vs 'Common7/Tools/VsDevCmd.bat'
foreach($path in @($dev,$source,$output)){if($path -match '["%!\r\n]'){throw 'Unsupported build path'}}
New-Item -ItemType Directory $output | Out-Null
$libs='user32.lib advapi32.lib bcrypt.lib ole32.lib oleaut32.lib uuid.lib'
$exe=Join-Path $output 'writer-bridge-probe.exe'
$cpp=Join-Path $PSScriptRoot 'native-writer-bridge-probe.cpp'
$command='"'+$dev+'" -arch=x64 && cl /nologo /std:c++17 /EHsc /MT /W4 /WX /DUNICODE /D_UNICODE "'+$cpp+'" /Fe:"'+$exe+'" /Fo:"'+$output+'/probe.obj" '+$libs+' /link /WX'
& $env:ComSpec /d /s /c $command *> (Join-Path $output 'probe-build.log')
if($LASTEXITCODE){throw 'Caller compile failed'}
$appHash=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
foreach($variant in @('correct','wrong')){
    $pin=if($variant -eq 'correct'){$writerHash}else{'0'*64}
    $header=Join-Path $output "$variant.hpp"
    [IO.File]::WriteAllLines($header,@(('#define GRAMMAR_TRUSTED_APP_SHA256 "'+$appHash+'"'),
      '#define GRAMMAR_TRUSTED_APP_NAME L"writer-bridge-probe.exe"',
      ('#define GRAMMAR_TRUSTED_WRITER_SHA256 "'+$pin+'"'),
      '#define GRAMMAR_TRUSTED_WRITER_PATH L"C:\\GrammarLibreOffice\\App\\libreoffice\\program\\soffice.bin"'),[Text.UTF8Encoding]::new($false))
    $obj=Join-Path $output $variant
    New-Item -ItemType Directory $obj | Out-Null
    $dll=Join-Path $output $(if($variant -eq 'correct'){'target-apply.dll'}else{'wrong-writer.dll'})
    $command='"'+$dev+'" -arch=x64 && cl /nologo /std:c++17 /EHsc /MT /W4 /WX /DUNICODE /D_UNICODE /FI"'+$header+'" /LD "'+$source+'/src-tauri/native/browser_bridge_api.cpp" "'+$source+'/src-tauri/native/writer_bridge_api.cpp" /Fe:"'+$dll+'" /Fo:"'+$obj+'\\" '+$libs+' /link /WX'
    & $env:ComSpec /d /s /c $command *> (Join-Path $output "$variant-build.log")
    if($LASTEXITCODE){throw "$variant DLL compile failed"}
}
$inputs=@(Get-ChildItem "$source/src-tauri/native" -File | Where-Object Extension -in '.cpp','.hpp')
$inputs+=Get-Item $cpp,$PSCommandPath
[ordered]@{classification='SYNTHETIC_WRITER_BRIDGE_NOT_PRODUCT_INSTALL';sourceCommit=(& git -C $source rev-parse HEAD);
 sourceDirty=(@(& git -C $source status --porcelain).Count -gt 0);writerSha256=$writerHash;brokerSha256=$appHash;
 inputs=@($inputs|Sort-Object FullName|ForEach-Object{@{path=$_.FullName;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}});
 outputs=@(Get-ChildItem $output -File|Where-Object Extension -in '.exe','.dll'|ForEach-Object{@{path=$_.Name;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
}|ConvertTo-Json -Depth 5|Set-Content (Join-Path $output 'build-receipt.json')
