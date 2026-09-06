[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$failures = New-Object System.Collections.Generic.List[string]
function Check([bool]$ok, [string]$id) { if (-not $ok) { $failures.Add($id) } }
$daily = Join-Path $root 'scripts\daily-use'
foreach ($name in @('Launch-Grammar.cmd','Check-Prerequisites.ps1','Verify-Bundle.ps1','Create-Shortcuts.ps1','Remove-Shortcuts.ps1','Enable-Startup.ps1','Disable-Startup.ps1')) {
  Check (Test-Path -LiteralPath (Join-Path $daily $name)) "MISSING_$name"
}
$builder = Join-Path $root 'scripts\build-daily-use-bundle.ps1'
Check (Test-Path -LiteralPath $builder) 'BUILDER_MISSING'
$launch = [System.IO.File]::ReadAllText((Join-Path $daily 'Launch-Grammar.cmd'))
Check ($launch.Contains('%~dp0')) 'LAUNCH_RELATIVE'
Check (-not $launch.Contains('D:\dev\repos\Grammar')) 'LAUNCH_REPO_PATH'
$prereq = [System.IO.File]::ReadAllText((Join-Path $daily 'Check-Prerequisites.ps1'))
Check ($prereq.Contains('WebView2')) 'PREREQ_WEBVIEW'
Check ($prereq.Contains('agy')) 'PREREQ_AGY'
Check (-not $prereq.Contains('rewrite')) 'PREREQ_NO_WRITE'
$create = [System.IO.File]::ReadAllText((Join-Path $daily 'Create-Shortcuts.ps1'))
Check ($create.Contains('IntegrationRoot')) 'SHORTCUT_TEST_ROOT'
$enable = [System.IO.File]::ReadAllText((Join-Path $daily 'Enable-Startup.ps1'))
Check ($enable.Contains('Startup')) 'STARTUP_FOLDER'
$docs = [System.IO.File]::ReadAllText((Join-Path $root 'docs\PERSONAL_BUNDLE.md'))
Check ($docs.Contains('build-daily-use-bundle.ps1')) 'DOCS_R8'
$temp = Join-Path $env:TEMP ("grammar-r8-shortcut-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null
Copy-Item -LiteralPath (Join-Path $daily 'Create-Shortcuts.ps1') -Destination $temp
Copy-Item -LiteralPath (Join-Path $daily 'Remove-Shortcuts.ps1') -Destination $temp
Copy-Item -LiteralPath (Join-Path $daily 'Enable-Startup.ps1') -Destination $temp
Copy-Item -LiteralPath (Join-Path $daily 'Disable-Startup.ps1') -Destination $temp
Set-Content -LiteralPath (Join-Path $temp 'codex-pencil.exe') -Value 'placeholder'
$integ = Join-Path $temp 'integ'
& (Join-Path $temp 'Create-Shortcuts.ps1') -Desktop -IntegrationRoot $integ | Out-Null
Check (Test-Path -LiteralPath (Join-Path $integ 'StartMenu\Grammar.lnk')) 'SHORTCUT_CREATED'
& (Join-Path $temp 'Enable-Startup.ps1') -IntegrationRoot $integ | Out-Null
Check (Test-Path -LiteralPath (Join-Path $integ 'Startup\Grammar.lnk')) 'STARTUP_CREATED'
& (Join-Path $temp 'Disable-Startup.ps1') -IntegrationRoot $integ | Out-Null
Check (-not (Test-Path -LiteralPath (Join-Path $integ 'Startup\Grammar.lnk'))) 'STARTUP_REMOVED'
& (Join-Path $temp 'Remove-Shortcuts.ps1') -IntegrationRoot $integ | Out-Null
Check (-not (Test-Path -LiteralPath (Join-Path $integ 'StartMenu\Grammar.lnk'))) 'SHORTCUT_REMOVED'
Remove-Item -LiteralPath $temp -Recurse -Force
if ($failures.Count -gt 0) {
  Write-Output 'RED'
  $failures | ForEach-Object { Write-Output $_ }
  exit 1
}
Write-Output 'GREEN'
exit 0
