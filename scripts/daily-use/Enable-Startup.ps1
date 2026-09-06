[CmdletBinding()]
param([string]$IntegrationRoot)
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $here 'codex-pencil.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw 'codex-pencil.exe is missing next to this script.' }
$shell = New-Object -ComObject WScript.Shell
if ($IntegrationRoot) {
  $path = Join-Path $IntegrationRoot 'Startup\Grammar.lnk'
} else {
  $path = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Startup\Grammar.lnk'
}
$dir = Split-Path -Parent $path
if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
$lnk = $shell.CreateShortcut($path)
$lnk.TargetPath = $exe
$lnk.WorkingDirectory = $here
$lnk.WindowStyle = 7
$lnk.Description = 'Grammar startup'
$lnk.Save()
Write-Output 'CREATED'
Write-Output $path
