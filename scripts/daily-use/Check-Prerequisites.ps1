[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
function Get-CommandVersion([string]$Name) {
  $cmd = Get-Command $Name -ErrorAction SilentlyContinue
  if (-not $cmd) { return [pscustomobject]@{ available = $false; version = $null; path = $null } }
  $path = $cmd.Source
  $ver = $null
  try {
    $out = & $path --version 2>&1 | Select-Object -First 1
    if ($out) { $ver = ([string]$out).Trim() }
  } catch { }
  [pscustomobject]@{ available = $true; version = $ver; path = Split-Path -Leaf $path }
}
function Test-WebView2 {
  $keys = @(
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
  )
  foreach ($key in $keys) {
    if (Test-Path -LiteralPath $key) {
      $pv = (Get-ItemProperty -LiteralPath $key -ErrorAction SilentlyContinue).pv
      if ($pv) { return [pscustomobject]@{ available = $true; version = [string]$pv } }
    }
  }
  return [pscustomobject]@{ available = $false; version = $null }
}
$arch = $env:PROCESSOR_ARCHITECTURE
$webview = Test-WebView2
$codex = Get-CommandVersion 'codex'
$agy = Get-CommandVersion 'agy'
$claude = Get-CommandVersion 'claude'
[pscustomobject]@{
  architecture = $arch
  architectureSupported = ($arch -eq 'AMD64')
  webView2 = $webview
  webView2Required = $true
  providers = [pscustomobject]@{
    codex = $codex
    antigravity = $agy
    claude = $claude
  }
  note = 'Provider clients are optional for local Instant. Sign-in is checked in the app. This script does not inspect credentials or start writing requests.'
} | ConvertTo-Json -Depth 5
if (-not $webview.available) {
  Write-Host 'WebView2 Runtime was not found. Install Microsoft Edge WebView2 Runtime, then relaunch.'
  exit 2
}
if ($arch -ne 'AMD64') {
  Write-Host 'This bundle targets 64-bit Windows.'
  exit 3
}
exit 0
