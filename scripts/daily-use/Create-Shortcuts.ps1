[CmdletBinding()]
param(
  [switch]$Desktop,
  [string]$IntegrationRoot
)
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $here 'codex-pencil.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw 'codex-pencil.exe is missing next to this script.' }
$shell = New-Object -ComObject WScript.Shell
function New-GrammarShortcut([string]$Path) {
  $dir = Split-Path -Parent $Path
  if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
  $lnk = $shell.CreateShortcut($Path)
  $lnk.TargetPath = $exe
  $lnk.WorkingDirectory = $here
  $lnk.WindowStyle = 1
  $lnk.Description = 'Grammar'
  $lnk.Save()
  return $Path
}
$created = @()
if ($IntegrationRoot) {
  $created += New-GrammarShortcut (Join-Path $IntegrationRoot 'StartMenu\Grammar.lnk')
  if ($Desktop) { $created += New-GrammarShortcut (Join-Path $IntegrationRoot 'Desktop\Grammar.lnk') }
} else {
  $start = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Grammar.lnk'
  $created += New-GrammarShortcut $start
  if ($Desktop) {
    $desk = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Grammar.lnk'
    $created += New-GrammarShortcut $desk
  }
}
Write-Output 'CREATED'
$created | ForEach-Object { Write-Output $_ }
