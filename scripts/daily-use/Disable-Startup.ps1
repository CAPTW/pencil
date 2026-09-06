[CmdletBinding()]
param([string]$IntegrationRoot)
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $here 'codex-pencil.exe'
if ($IntegrationRoot) {
  $path = Join-Path $IntegrationRoot 'Startup\Grammar.lnk'
} else {
  $path = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Startup\Grammar.lnk'
}
if (-not (Test-Path -LiteralPath $path)) {
  Write-Output 'REMOVED'
  Write-Output '(none)'
  return
}
$shell = New-Object -ComObject WScript.Shell
$target = $null
try { $target = $shell.CreateShortcut($path).TargetPath } catch { }
if ($target -and ($target -ne $exe)) {
  throw 'Startup shortcut exists but does not target this bundle.'
}
Remove-Item -LiteralPath $path -Force
Write-Output 'REMOVED'
Write-Output $path
