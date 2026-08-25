$ErrorActionPreference = "Stop"

function Write-Section {
  param([string]$Title)
  Write-Host ""
  Write-Host "== $Title =="
}

function Test-CommandVersion {
  param(
    [string]$Name,
    [bool]$RequiredForFullBuild = $false
  )

  $command = Get-Command $Name -ErrorAction SilentlyContinue
  if ($null -eq $command) {
    if ($RequiredForFullBuild) {
      Write-Host "${Name}: missing (required for full Tauri build)"
    } else {
      Write-Host "${Name}: missing"
    }
    return $false
  }

  Write-Host "${Name} path: $($command.Source)"
  try {
    $version = & $Name --version 2>$null
    if ($LASTEXITCODE -eq 0 -and $version) {
      Write-Host "${Name} version: $version"
    } else {
      Write-Host "${Name} version: unavailable"
    }
  } catch {
    Write-Host "${Name} version: unavailable ($($_.Exception.Message))"
  }

  return $true
}

function Invoke-Step {
  param(
    [Parameter(Mandatory = $true)][string]$Label,
    [Parameter(Mandatory = $true)][scriptblock]$Action
  )

  & $Action
  # $ErrorActionPreference = "Stop" does NOT catch native-command (npm/cargo) failures,
  # so the exit code must be checked explicitly or a failed build would report success.
  if ($LASTEXITCODE -ne 0) {
    throw "${Label} failed with exit code ${LASTEXITCODE}."
  }
}

function Enable-RustupPathIfNeeded {
  $cargo = Get-Command "cargo" -ErrorAction SilentlyContinue
  $rustc = Get-Command "rustc" -ErrorAction SilentlyContinue
  if ($null -ne $cargo -and $null -ne $rustc) {
    return
  }

  $rustupBin = Join-Path $env:USERPROFILE ".cargo\bin"
  if ((Test-Path (Join-Path $rustupBin "cargo.exe")) -and (Test-Path (Join-Path $rustupBin "rustc.exe"))) {
    $env:PATH = "$rustupBin;$env:PATH"
    Write-Host "Added Rustup bin to this verification session PATH: $rustupBin"
  }
}

if (-not (Test-Path "package.json")) {
  throw "Run this script from the repository root."
}

Enable-RustupPathIfNeeded

Write-Section "Toolchain"
$nodeOk = Test-CommandVersion "node"
$npmOk = Test-CommandVersion "npm"
$codexOk = Test-CommandVersion "codex"
$rustcOk = Test-CommandVersion "rustc" $true
$cargoOk = Test-CommandVersion "cargo" $true

if (-not $nodeOk -or -not $npmOk) {
  throw "Node.js and npm are required for verification."
}

if (-not $codexOk) {
  Write-Host "Codex CLI is required at runtime for login and rewrites. Install with: npm install -g @openai/codex"
}

Write-Section "Frontend Checks"
Invoke-Step "npm run test:window-chrome" { npm run test:window-chrome }
Invoke-Step "npm run typecheck" { npm run typecheck }
Invoke-Step "npm run build:frontend" { npm run build:frontend }

Write-Section "Tauri Build"
if ($cargoOk -and $rustcOk) {
  Invoke-Step "npm run build" { npm run build }
} else {
  Write-Host "Skipping npm run build because Rust/Cargo is not installed or not on PATH."
}
