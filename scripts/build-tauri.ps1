$ErrorActionPreference = "Stop"

function Enable-RustupPathIfNeeded {
  $cargo = Get-Command "cargo" -ErrorAction SilentlyContinue
  $rustc = Get-Command "rustc" -ErrorAction SilentlyContinue
  if ($null -ne $cargo -and $null -ne $rustc) {
    return
  }

  $rustupBin = Join-Path $env:USERPROFILE ".cargo\bin"
  if ((Test-Path (Join-Path $rustupBin "cargo.exe")) -and (Test-Path (Join-Path $rustupBin "rustc.exe"))) {
    $env:PATH = "$rustupBin;$env:PATH"
    Write-Host "Added Rustup bin to this build session PATH: $rustupBin"
  }
}

if (-not (Test-Path "package.json")) {
  throw "Run this script from the repository root."
}

Enable-RustupPathIfNeeded

$tauri = Join-Path (Get-Location) "node_modules\.bin\tauri.cmd"
if (-not (Test-Path $tauri)) {
  throw "Tauri CLI was not found. Run npm install first."
}

& $tauri build
exit $LASTEXITCODE
