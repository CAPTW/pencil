[CmdletBinding()]
param([string]$IntegrationRoot)
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $here 'codex-pencil.exe'
$candidates = @()
if ($IntegrationRoot) {
  $candidates += Join-Path $IntegrationRoot 'StartMenu\Grammar.lnk'
  $candidates += Join-Path $IntegrationRoot 'Desktop\Grammar.lnk'
} else {
  $candidates += Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Grammar.lnk'
  $candidates += Join-Path ([Environment]::GetFolderPath('Desktop')) 'Grammar.lnk'
}
$removed = @()
$shell = New-Object -ComObject WScript.Shell
foreach ($path in $candidates) {
  if (-not (Test-Path -LiteralPath $path)) { continue }
  $target = $null
  try { $target = $shell.CreateShortcut($path).TargetPath } catch { }
  if ($target -and ($target -ne $exe)) { continue }
  Remove-Item -LiteralPath $path -Force
  $removed += $path
}
Write-Output 'REMOVED'
if ($removed.Count -eq 0) { Write-Output '(none)' } else { $removed | ForEach-Object { Write-Output $_ } }
