param([switch]$Live)
$ErrorActionPreference='Stop'
if($env:USERNAME -ne 'WDAGUtilityAccount' -or $PSScriptRoot -ne 'C:\GrammarQualityPayload'){throw 'Owned Sandbox only'}
$trial='C:\GrammarQuality'
New-Item -ItemType Directory -Force -Path $trial | Out-Null
Set-Content -LiteralPath "$trial/owned-trial.marker" -Value 'owned-synthetic-20261007'
Copy-Item -LiteralPath "$PSScriptRoot/codex.exe" -Destination "$trial/codex.exe" -Force
Copy-Item -LiteralPath "$PSScriptRoot/quality-tests.exe" -Destination "$trial/quality-tests.exe" -Force
Copy-Item -LiteralPath "$PSScriptRoot/vcruntime140.dll" -Destination "$trial/vcruntime140.dll" -Force
$env:CODEX_PENCIL_CODEX_BIN="$trial/codex.exe"
$env:GRAMMAR_QUALITY_TRIAL='owned-synthetic-20261007'
$env:GRAMMAR_QUALITY_LIVE=if($Live){'1'}else{'0'}
$ErrorActionPreference='Continue'
& "$trial/quality-tests.exe" 'codex_client::quality_trial::owned_sandbox_quality_trial' --ignored --exact --nocapture *> 'C:\GrammarQualityOutput/runner.log'
$testExit=$LASTEXITCODE
$ErrorActionPreference='Stop'
@{testExit=$testExit;liveRequested=[bool]$Live;guest=$env:COMPUTERNAME;when=(Get-Date).ToString('o')} | ConvertTo-Json | Set-Content 'C:\GrammarQualityOutput/guest.json'
exit $testExit
