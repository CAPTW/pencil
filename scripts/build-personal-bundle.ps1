[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[0-9a-fA-F]{40}$')]
  [string]$ExpectedSourceCommit,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$OutputBase
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 3.0

$AUTHORIZED_P1_03_BASELINE = "73af3d93231d06e8370d1da11ff6b2bb76ff3a1d"
$EXPECTED_PROTOCOL_BUNDLE_SHA256 = "c954593823626b5194b7f687c27d542eb87fcfa63e5e11af3ca95f9b6c39c6e8"
$EXPECTED_PACKAGE_LOCK_SHA256 = "c3415432201286856ec3ca854b4931c13e947c131905a6bfc21242a6bfc2e669"
$EXPECTED_CARGO_LOCK_SHA256 = "524f11ebf0d741837761d4dca175d48ac908fcaa14be9ea84800ab344f893cce"
$EXPECTED_CODEX_VERSION = "codex-cli 0.144.6"
$EXPECTED_TARGET_TRIPLE = "x86_64-pc-windows-msvc"
$EXPECTED_ARCHITECTURE = "x64"
$EXPECTED_PRODUCT_NAME = "Codex Pencil"
$EXPECTED_BINARY_NAME = "codex-pencil.exe"
$EXPECTED_SOURCE_BRANCH = "main"

$Policy = [ordered]@{
  SigningAttempted = $false
  UpdaterEnabled = $false
  PublicUploadAttempted = $false
  CodexCliVendored = $false
}

$requiredPortableEntries = @(
  "codex-pencil.exe",
  "README_PERSONAL_USE.md",
  "BUILD_PROVENANCE.json",
  "ARTIFACT_INDEX.json",
  "SHA256SUMS.txt",
  "VERIFY_BUNDLE.ps1"
)

$approvedPackagingPaths = @(
  "README.md",
  "docs/PERSONAL_BUNDLE.md",
  "package.json",
  "scripts/build-personal-bundle.ps1",
  "scripts/test-personal-bundle-contract.ps1",
  "scripts/verify-personal-bundle.ps1",
  "src-tauri/tauri.conf.json"
)

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$temporaryBuildRoot = $null
$buildSucceeded = $false
$savedEnvironment = @{}

function Stop-Build {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Token
  )

  throw $Token
}

function Invoke-GitText {
  param(
    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [string[]]$Arguments,

    [string]$FailureToken = "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  )

  $output = @(& git @Arguments 2>$null)
  if ($LASTEXITCODE -ne 0) {
    Stop-Build $FailureToken
  }
  return ($output -join "`n").Trim()
}

function Invoke-CheckedCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,

    [Parameter(Mandatory = $true)]
    [AllowEmptyCollection()]
    [string[]]$Arguments,

    [Parameter(Mandatory = $true)]
    [string]$FailureToken
  )

  & $FilePath @Arguments
  if ($LASTEXITCODE -ne 0) {
    Stop-Build $FailureToken
  }
}

function Get-ObjectProperty {
  param(
    [Parameter(Mandatory = $true)]
    [object]$Object,

    [Parameter(Mandatory = $true)]
    [string]$Name
  )

  $property = $Object.PSObject.Properties[$Name]
  if ($null -eq $property) {
    return $null
  }
  return $property.Value
}

function Get-Sha256 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Write-Utf8NoBom {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,

    [Parameter(Mandatory = $true)]
    [AllowEmptyString()]
    [string]$Text
  )

  $encoding = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($Path, $Text, $encoding)
}

function Write-JsonFile {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,

    [Parameter(Mandatory = $true)]
    [object]$Value
  )

  Write-Utf8NoBom -Path $Path -Text (($Value | ConvertTo-Json -Depth 10) + "`n")
}

function Assert-ExternalOutputBase {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,

    [Parameter(Mandatory = $true)]
    [string]$RepositoryAbsolutePath
  )

  if (-not [System.IO.Path]::IsPathRooted($Path)) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
  }

  $fullOutput = [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
  $fullRepository = [System.IO.Path]::GetFullPath($RepositoryAbsolutePath).TrimEnd('\', '/')
  $repositoryPrefix = $fullRepository + [System.IO.Path]::DirectorySeparatorChar
  if ($fullOutput -eq $fullRepository -or $fullOutput.StartsWith($repositoryPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
  }

  return $fullOutput
}

function Assert-CleanMainSource {
  $branch = Invoke-GitText -Arguments @("branch", "--show-current")
  if ($branch -cne $EXPECTED_SOURCE_BRANCH) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }

  $status = Invoke-GitText -Arguments @("status", "--porcelain", "--untracked-files=all")
  if (-not [string]::IsNullOrWhiteSpace($status)) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }
}

function Assert-ApprovedPackagingDiff {
  $changedPathsText = Invoke-GitText -Arguments @("diff", "--name-only", ($AUTHORIZED_P1_03_BASELINE + "..HEAD"))
  $changedPaths = @($changedPathsText -split "`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
  $unexpectedPaths = @($changedPaths | Where-Object { $_ -notin $approvedPackagingPaths })
  if ($unexpectedPaths.Count -ne 0) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_RUNTIME_SCOPE_DRIFT"
  }
}

function Get-ProtocolBundleHash {
  $bundleRoot = Join-Path $repositoryRoot "src-tauri\protocol\codex-cli-0.144.6"
  $records = @(Get-ChildItem -LiteralPath $bundleRoot -Recurse -File -Filter "*.json" | ForEach-Object {
    [pscustomobject]@{
      relativePath = [System.IO.Path]::GetFullPath($_.FullName).Substring([System.IO.Path]::GetFullPath($bundleRoot).TrimEnd('\').Length + 1).Replace('\', '/')
      sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()
    }
  } | Sort-Object relativePath)

  if ($records.Count -ne 267) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }

  $manifest = (($records | ForEach-Object { $_.sha256 + "  " + $_.relativePath }) -join "`n") + "`n"
  $bytes = (New-Object System.Text.UTF8Encoding($false)).GetBytes($manifest)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    return -join ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString("x2") })
  }
  finally {
    $sha.Dispose()
  }
}

function Assert-RetainedHashes {
  $protocolHash = Get-ProtocolBundleHash
  $packageLockHash = Get-Sha256 (Join-Path $repositoryRoot "package-lock.json")
  $cargoLockHash = Get-Sha256 (Join-Path $repositoryRoot "src-tauri\Cargo.lock")
  if ($protocolHash -cne $EXPECTED_PROTOCOL_BUNDLE_SHA256 -or
      $packageLockHash -cne $EXPECTED_PACKAGE_LOCK_SHA256 -or
      $cargoLockHash -cne $EXPECTED_CARGO_LOCK_SHA256) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }

  return [pscustomobject]@{
    protocolBundleSha256 = $protocolHash
    packageLockSha256 = $packageLockHash
    cargoLockSha256 = $cargoLockHash
  }
}

function Assert-ReleaseConfiguration {
  param(
    [Parameter(Mandatory = $true)]
    [object]$TauriConfig,

    [Parameter(Mandatory = $true)]
    [object]$PackageConfig,

    [Parameter(Mandatory = $true)]
    [string]$CargoManifestText
  )

  $plugins = Get-ObjectProperty $TauriConfig "plugins"
  $updaterConfig = if ($null -ne $plugins) { Get-ObjectProperty $plugins "updater" } else { $null }
  $bundle = Get-ObjectProperty $TauriConfig "bundle"
  $createUpdaterArtifacts = if ($null -ne $bundle) { Get-ObjectProperty $bundle "createUpdaterArtifacts" } else { $null }
  $windows = if ($null -ne $bundle) { Get-ObjectProperty $bundle "windows" } else { $null }

  $signingConfigured = $false
  if ($null -ne $windows) {
    foreach ($propertyName in @("certificateThumbprint", "timestampUrl", "digestAlgorithm", "signCommand")) {
      if ($null -ne (Get-ObjectProperty $windows $propertyName)) {
        $signingConfigured = $true
      }
    }
  }

  $dependencyNames = @()
  foreach ($containerName in @("dependencies", "devDependencies")) {
    $container = Get-ObjectProperty $PackageConfig $containerName
    if ($null -ne $container) {
      $dependencyNames += @($container.PSObject.Properties.Name)
    }
  }

  if ($null -ne $updaterConfig -or
      $null -ne $createUpdaterArtifacts -or
      $signingConfigured -or
      @($dependencyNames | Where-Object { $_ -match '(?i)updater' }).Count -ne 0 -or
      $CargoManifestText -match '(?im)^\s*[^#\r\n]*updater[^\r\n]*$') {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_UNEXPECTED_RELEASE_CONFIGURATION"
  }
}

function Get-FirstLineVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,

    [string[]]$Arguments = @()
  )

  $output = @(& $FilePath @Arguments 2>&1)
  if ($LASTEXITCODE -ne 0 -or $output.Count -eq 0) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
  }
  return ([string]$output[0]).Trim()
}

function Get-ToolVersions {
  $tauriPath = Join-Path $repositoryRoot "node_modules\.bin\tauri.cmd"
  if (-not (Test-Path -LiteralPath $tauriPath -PathType Leaf)) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
  }

  $codexCommands = @(Get-Command "codex.cmd" -All -ErrorAction SilentlyContinue)
  if ($codexCommands.Count -eq 0) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
  }

  $versions = [ordered]@{
    nodeVersion = Get-FirstLineVersion -FilePath "node" -Arguments @("--version")
    npmVersion = Get-FirstLineVersion -FilePath "npm" -Arguments @("--version")
    rustcVersion = Get-FirstLineVersion -FilePath "rustc" -Arguments @("--version")
    cargoVersion = Get-FirstLineVersion -FilePath "cargo" -Arguments @("--version")
    tauriVersion = Get-FirstLineVersion -FilePath $tauriPath -Arguments @("--version")
    codexVersion = Get-FirstLineVersion -FilePath $codexCommands[0].Source -Arguments @("--version")
  }

  if ($versions.codexVersion -cne $EXPECTED_CODEX_VERSION) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }

  return [pscustomobject]$versions
}

function Get-FileRecord {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Root,

    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $item = Get-Item -LiteralPath $Path
  $rootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
  $pathFull = [System.IO.Path]::GetFullPath($item.FullName)
  $relativePath = $pathFull.Substring($rootFull.Length + 1).Replace('\', '/')
  return [pscustomobject][ordered]@{
    relativePath = $relativePath
    size = [int64]$item.Length
    sha256 = Get-Sha256 $item.FullName
  }
}

function Assert-DistributablePrivacy {
  param(
    [Parameter(Mandatory = $true)]
    [string]$PortableRoot,

    [Parameter(Mandatory = $true)]
    [string]$RepositoryAbsolutePath
  )

  $textFiles = @(Get-ChildItem -LiteralPath $PortableRoot -File | Where-Object { $_.Extension -in @(".md", ".json", ".txt", ".ps1") })
  $localUsername = [System.Environment]::UserName
  $forbiddenLiteralValues = @(
    $RepositoryAbsolutePath,
    $env:USERPROFILE
  ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
  if (-not [string]::IsNullOrWhiteSpace($localUsername) -and -not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
    $forbiddenLiteralValues += $env:USERPROFILE.TrimEnd('\', '/')
  }
  $protectedPatterns = @(
    '(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b',
    '(?i)https?://[^\s"''<>]+',
    '\bsk-[A-Za-z0-9]{8,}\b',
    '\b[A-Z0-9]{4}-[A-Z0-9]{4}\b',
    '(?i)"(?:selectedText|replacement|clipboardText|deviceCode|accessToken|refreshToken|accountEmail|codexHome|appData|localAppData)"\s*:'
  )

  $localAbsolutePathCount = 0
  $localUsernameCount = 0
  $forbiddenProtectedValueCount = 0
  foreach ($file in $textFiles) {
    $text = [System.IO.File]::ReadAllText($file.FullName)
    if (-not [string]::IsNullOrWhiteSpace($localUsername)) {
      $localUsernameCount += [regex]::Matches($text, [regex]::Escape($localUsername)).Count
    }
    foreach ($literal in $forbiddenLiteralValues) {
      $localAbsolutePathCount += [regex]::Matches($text, [regex]::Escape($literal), [System.Text.RegularExpressions.RegexOptions]::IgnoreCase).Count
    }
    foreach ($pattern in $protectedPatterns) {
      $forbiddenProtectedValueCount += [regex]::Matches($text, $pattern).Count
    }
  }

  if ($localAbsolutePathCount -ne 0 -or $localUsernameCount -ne 0 -or $forbiddenProtectedValueCount -ne 0) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED"
  }

  return [pscustomobject]@{
    localAbsolutePathCount = $localAbsolutePathCount
    localUsernameCount = $localUsernameCount
    forbiddenProtectedValueCount = $forbiddenProtectedValueCount
  }
}

function New-StableZip {
  param(
    [Parameter(Mandatory = $true)]
    [string]$PortableRoot,

    [Parameter(Mandatory = $true)]
    [string]$ZipPath,

    [Parameter(Mandatory = $true)]
    [datetime]$Timestamp
  )

  Add-Type -AssemblyName System.IO.Compression
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $archive = [System.IO.Compression.ZipFile]::Open($ZipPath, [System.IO.Compression.ZipArchiveMode]::Create)
  try {
    foreach ($file in @(Get-ChildItem -LiteralPath $PortableRoot -File | Sort-Object Name)) {
      $entry = $archive.CreateEntry($file.Name, [System.IO.Compression.CompressionLevel]::Optimal)
      $entry.LastWriteTime = [System.DateTimeOffset]::new($Timestamp.ToUniversalTime())
      $input = $null
      $output = $null
      try {
        $input = [System.IO.File]::OpenRead($file.FullName)
        $output = $entry.Open()
        $input.CopyTo($output)
      }
      finally {
        if ($null -ne $output) { $output.Dispose() }
        if ($null -ne $input) { $input.Dispose() }
      }
    }
  }
  finally {
    $archive.Dispose()
  }
}

function Invoke-PortableVerifier {
  param(
    [Parameter(Mandatory = $true)]
    [string]$VerifierPath,

    [Parameter(Mandatory = $true)]
    [string]$PathToVerify,

    [Parameter(Mandatory = $true)]
    [string]$SourceCommit
  )

  $output = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $VerifierPath -BundlePath $PathToVerify -ExpectedSourceCommit $SourceCommit 2>&1)
  if ($LASTEXITCODE -ne 0) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED"
  }
  return ($output -join "`n")
}

function Assert-OwnedBuildRoot {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $ownedBase = [System.IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA "Temp")).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
  $leaf = Split-Path -Leaf $fullPath
  if (-not $fullPath.StartsWith($ownedBase, [System.StringComparison]::OrdinalIgnoreCase) -or
      -not $leaf.StartsWith("codex-pencil-p2-01-", [System.StringComparison]::Ordinal)) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
  }
}

function Remove-OwnedBuildRoot {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  Assert-OwnedBuildRoot $Path
  if (Test-Path -LiteralPath $Path) {
    Remove-Item -LiteralPath $Path -Recurse -Force
  }
}

try {
  Set-Location -LiteralPath $repositoryRoot
  $repositoryAbsolutePath = [System.IO.Path]::GetFullPath($repositoryRoot)
  $resolvedOutputBase = Assert-ExternalOutputBase -Path $OutputBase -RepositoryAbsolutePath $repositoryAbsolutePath
  Assert-CleanMainSource

  $head = Invoke-GitText -Arguments @("rev-parse", "HEAD")
  if ($head -cne $ExpectedSourceCommit.ToLowerInvariant()) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }
  & git merge-base --is-ancestor $AUTHORIZED_P1_03_BASELINE $head
  if ($LASTEXITCODE -ne 0) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }
  Assert-ApprovedPackagingDiff

  $finalOutputRoot = Join-Path $resolvedOutputBase $head
  if (Test-Path -LiteralPath $finalOutputRoot) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_OUTPUT_ROOT_ALREADY_EXISTS"
  }

  $retainedHashes = Assert-RetainedHashes
  $packageConfig = [System.IO.File]::ReadAllText((Join-Path $repositoryRoot "package.json")) | ConvertFrom-Json
  $tauriConfig = [System.IO.File]::ReadAllText((Join-Path $repositoryRoot "src-tauri\tauri.conf.json")) | ConvertFrom-Json
  $cargoManifestText = [System.IO.File]::ReadAllText((Join-Path $repositoryRoot "src-tauri\Cargo.toml"))
  Assert-ReleaseConfiguration -TauriConfig $tauriConfig -PackageConfig $packageConfig -CargoManifestText $cargoManifestText

  $productName = [string](Get-ObjectProperty $tauriConfig "productName")
  $appVersion = [string](Get-ObjectProperty $tauriConfig "version")
  $packageVersion = [string](Get-ObjectProperty $packageConfig "version")
  if ($productName -cne $EXPECTED_PRODUCT_NAME -or $appVersion -cne $packageVersion) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }
  $cargoVersionPattern = '(?m)^version\s*=\s*"{0}"\s*$' -f [regex]::Escape($appVersion)
  if ($cargoManifestText -notmatch '(?m)^name\s*=\s*"codex-pencil"\s*$' -or
      $cargoManifestText -notmatch $cargoVersionPattern) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }

  $toolVersions = Get-ToolVersions
  $commitSubject = Invoke-GitText -Arguments @("show", "-s", "--format=%s", "HEAD")
  $buildUtc = (Get-Date).ToUniversalTime()
  $packageStem = ($productName -replace '\s+', '-') + "-" + $appVersion + "-" + $EXPECTED_ARCHITECTURE
  $zipName = $packageStem + "-portable.zip"

  $temporaryBuildRoot = Join-Path (Join-Path $env:LOCALAPPDATA "Temp") ("codex-pencil-p2-01-" + [guid]::NewGuid().ToString("N"))
  Assert-OwnedBuildRoot $temporaryBuildRoot
  New-Item -ItemType Directory -Path $temporaryBuildRoot | Out-Null
  $frontendRoot = Join-Path $temporaryBuildRoot "frontend"
  $cargoTargetRoot = Join-Path $temporaryBuildRoot "cargo-target"
  $stagingOutputRoot = Join-Path $temporaryBuildRoot "output"
  New-Item -ItemType Directory -Path $frontendRoot | Out-Null
  New-Item -ItemType Directory -Path $cargoTargetRoot | Out-Null
  New-Item -ItemType Directory -Path $stagingOutputRoot | Out-Null

  $tscPath = Join-Path $repositoryRoot "node_modules\.bin\tsc.cmd"
  $vitePath = Join-Path $repositoryRoot "node_modules\.bin\vite.cmd"
  $tauriPath = Join-Path $repositoryRoot "node_modules\.bin\tauri.cmd"
  foreach ($toolPath in @($tscPath, $vitePath, $tauriPath)) {
    if (-not (Test-Path -LiteralPath $toolPath -PathType Leaf)) {
      Stop-Build "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
    }
  }

  Invoke-CheckedCommand -FilePath $tscPath -Arguments @() -FailureToken "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
  Invoke-CheckedCommand -FilePath $vitePath -Arguments @("build", "--outDir", $frontendRoot, "--emptyOutDir") -FailureToken "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"

  $tauriOverlayPath = Join-Path $temporaryBuildRoot "tauri.personal.conf.json"
  $tauriOverlay = [ordered]@{
    build = [ordered]@{
      beforeBuildCommand = ""
      beforeBundleCommand = ""
      frontendDist = $frontendRoot
    }
    bundle = [ordered]@{
      active = $false
    }
  }
  Write-JsonFile -Path $tauriOverlayPath -Value $tauriOverlay

  foreach ($environmentName in @("CARGO_TARGET_DIR", "CARGO_NET_OFFLINE", "CI")) {
    $savedEnvironment[$environmentName] = [System.Environment]::GetEnvironmentVariable($environmentName, "Process")
  }
  $env:CARGO_TARGET_DIR = $cargoTargetRoot
  $env:CARGO_NET_OFFLINE = "true"
  $env:CI = "true"

  Invoke-CheckedCommand -FilePath $tauriPath -Arguments @(
    "build",
    "--ci",
    "--no-bundle",
    "--no-sign",
    "--target", $EXPECTED_TARGET_TRIPLE,
    "--config", $tauriOverlayPath,
    "--",
    "--locked"
  ) -FailureToken "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"

  $builtExecutable = Join-Path $cargoTargetRoot ($EXPECTED_TARGET_TRIPLE + "\release\" + $EXPECTED_BINARY_NAME)
  if (-not (Test-Path -LiteralPath $builtExecutable -PathType Leaf)) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED"
  }

  $portableRoot = Join-Path (Join-Path $stagingOutputRoot "portable") $packageStem
  $verificationRoot = Join-Path $stagingOutputRoot "verification"
  New-Item -ItemType Directory -Path $portableRoot -Force | Out-Null
  New-Item -ItemType Directory -Path $verificationRoot -Force | Out-Null
  Copy-Item -LiteralPath $builtExecutable -Destination (Join-Path $portableRoot $EXPECTED_BINARY_NAME)
  Copy-Item -LiteralPath (Join-Path $PSScriptRoot "verify-personal-bundle.ps1") -Destination (Join-Path $portableRoot "VERIFY_BUNDLE.ps1")

  $readme = @"
# Codex Pencil private portable build

This is a private, unsigned personal build of Codex Pencil $appVersion for Windows x64. Its Authenticode status is expected to be NotSigned. There is no public support channel.

Codex CLI is an external prerequisite and is not included. Install and retain exactly codex-cli 0.144.6; Codex Pencil resolves the supported external command at runtime. Node, authentication state, and Codex home data are not bundled.

On first use, ChatGPT-managed device login may be required. Before the first writing request, the app displays its cloud-processing disclosure: only the selected text and matched approved terminology are sent to Codex/ChatGPT for processing.

## Use

1. Extract the ZIP to a normal writable directory.
2. Run $EXPECTED_BINARY_NAME.
3. Use the tray menu or Ctrl+Shift+G to show the widget. Settings are available from the tray and widget.
4. Quit through the tray menu before replacing the bundle.

There is no auto-update. Update by quitting, verifying a newer private bundle, and replacing the extracted directory. Do not merge old application files into a new bundle.

Preferences and terminology are stored locally in the app's Windows data boundary; selected text, generated replacements, clipboard content, and writing history are not stored there. The app processes a user selection only. It does not capture screenshots, perform OCR, or implement keylogging.

Clipboard capture and restore are text-focused. Password fields and other protected controls may refuse copying. An unelevated Codex Pencil process cannot safely inject input into an elevated editor; use matching integrity levels or paste manually when the app reports copy-only fallback.

Verify an extracted bundle from its directory:

    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\VERIFY_BUNDLE.ps1 -BundlePath . -ExpectedSourceCommit $head

Source commit: $head

Package version: $appVersion
"@
  Write-Utf8NoBom -Path (Join-Path $portableRoot "README_PERSONAL_USE.md") -Text ($readme.Trim() + "`n")

  $portableExecutable = Get-Item -LiteralPath (Join-Path $portableRoot $EXPECTED_BINARY_NAME)
  $executableSha256 = Get-Sha256 $portableExecutable.FullName
  $authenticodeStatus = [string](Get-AuthenticodeSignature -LiteralPath $portableExecutable.FullName).Status
  if ($authenticodeStatus -cne "NotSigned") {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED"
  }

  $provenance = [ordered]@{
    schemaVersion = 1
    projectName = "codex-pencil"
    productName = $productName
    appVersion = $appVersion
    sourceBranch = $EXPECTED_SOURCE_BRANCH
    sourceCommit = $head
    commitSubject = $commitSubject
    targetTriple = $EXPECTED_TARGET_TRIPLE
    architecture = $EXPECTED_ARCHITECTURE
    buildUtc = $buildUtc.ToString("o")
    nodeVersion = $toolVersions.nodeVersion
    npmVersion = $toolVersions.npmVersion
    rustcVersion = $toolVersions.rustcVersion
    cargoVersion = $toolVersions.cargoVersion
    tauriVersion = $toolVersions.tauriVersion
    codexVersion = $toolVersions.codexVersion
    protocolBundleSha256 = $retainedHashes.protocolBundleSha256
    packageLockSha256 = $retainedHashes.packageLockSha256
    cargoLockSha256 = $retainedHashes.cargoLockSha256
    executableRelativePath = $EXPECTED_BINARY_NAME
    executableSize = [int64]$portableExecutable.Length
    executableSha256 = $executableSha256
    authenticodeStatus = $authenticodeStatus
    signingAttempted = $Policy.SigningAttempted
    updaterEnabled = $Policy.UpdaterEnabled
    updaterArtifactCount = 0
    publicUploadAttempted = $Policy.PublicUploadAttempted
    codexCliVendored = $Policy.CodexCliVendored
    nsisStatus = "NOT_ATTEMPTED_BY_POLICY"
    localAbsolutePathCount = 0
    localUsernameCount = 0
    forbiddenProtectedValueCount = 0
  }
  Write-JsonFile -Path (Join-Path $portableRoot "BUILD_PROVENANCE.json") -Value $provenance

  $artifactIndexPaths = @(
    $EXPECTED_BINARY_NAME,
    "README_PERSONAL_USE.md",
    "BUILD_PROVENANCE.json",
    "VERIFY_BUNDLE.ps1"
  ) | Sort-Object
  $artifactIndexEntries = @($artifactIndexPaths | ForEach-Object { Get-FileRecord -Root $portableRoot -Path (Join-Path $portableRoot $_) })
  $artifactIndex = [ordered]@{
    schemaVersion = 1
    hashAlgorithm = "SHA-256"
    selfReferencePolicy = "ARTIFACT_INDEX_AND_SHA256SUMS_EXCLUDED"
    entries = $artifactIndexEntries
  }
  Write-JsonFile -Path (Join-Path $portableRoot "ARTIFACT_INDEX.json") -Value $artifactIndex

  $checksumPaths = @($requiredPortableEntries | Where-Object { $_ -ne "SHA256SUMS.txt" } | Sort-Object)
  $checksumLines = @($checksumPaths | ForEach-Object { (Get-Sha256 (Join-Path $portableRoot $_)) + "  " + $_ })
  Write-Utf8NoBom -Path (Join-Path $portableRoot "SHA256SUMS.txt") -Text (($checksumLines -join "`n") + "`n")

  $privacy = Assert-DistributablePrivacy -PortableRoot $portableRoot -RepositoryAbsolutePath $repositoryAbsolutePath
  if ($privacy.localAbsolutePathCount -ne 0 -or $privacy.localUsernameCount -ne 0 -or $privacy.forbiddenProtectedValueCount -ne 0) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED"
  }

  $directoryReport = Invoke-PortableVerifier -VerifierPath (Join-Path $portableRoot "VERIFY_BUNDLE.ps1") -PathToVerify $portableRoot -SourceCommit $head
  Write-Utf8NoBom -Path (Join-Path $verificationRoot "portable-directory.json") -Text ($directoryReport.Trim() + "`n")

  $zipPath = Join-Path $stagingOutputRoot $zipName
  New-StableZip -PortableRoot $portableRoot -ZipPath $zipPath -Timestamp $buildUtc
  $zipReport = Invoke-PortableVerifier -VerifierPath (Join-Path $portableRoot "VERIFY_BUNDLE.ps1") -PathToVerify $zipPath -SourceCommit $head
  Write-Utf8NoBom -Path (Join-Path $verificationRoot "portable-zip.json") -Text ($zipReport.Trim() + "`n")

  $outputArtifactFiles = @(
    (Get-Item -LiteralPath $zipPath)
    (Get-ChildItem -LiteralPath $portableRoot -File)
    (Get-ChildItem -LiteralPath $verificationRoot -File)
  )
  $outputArtifactEntries = @($outputArtifactFiles | ForEach-Object { Get-FileRecord -Root $stagingOutputRoot -Path $_.FullName } | Sort-Object relativePath)
  $outputIndex = [ordered]@{
    schemaVersion = 1
    sourceCommit = $head
    hashAlgorithm = "SHA-256"
    selfReferencePolicy = "OUTPUT_INDEX_AND_OUTPUT_SHA256SUMS_EXCLUDED"
    portableZipRelativePath = $zipName
    portableZipSize = [int64](Get-Item -LiteralPath $zipPath).Length
    portableZipSha256 = Get-Sha256 $zipPath
    artifacts = $outputArtifactEntries
  }
  $outputIndexPath = Join-Path $stagingOutputRoot "OUTPUT_ARTIFACT_INDEX.json"
  Write-JsonFile -Path $outputIndexPath -Value $outputIndex

  $outputChecksumFiles = @($outputArtifactFiles + (Get-Item -LiteralPath $outputIndexPath))
  $outputChecksumLines = @($outputChecksumFiles | ForEach-Object {
    $record = Get-FileRecord -Root $stagingOutputRoot -Path $_.FullName
    $record.sha256 + "  " + $record.relativePath
  } | Sort-Object)
  Write-Utf8NoBom -Path (Join-Path $stagingOutputRoot "OUTPUT_SHA256SUMS.txt") -Text (($outputChecksumLines -join "`n") + "`n")

  Assert-CleanMainSource
  $postBuildHashes = Assert-RetainedHashes
  if ($postBuildHashes.protocolBundleSha256 -cne $retainedHashes.protocolBundleSha256 -or
      $postBuildHashes.packageLockSha256 -cne $retainedHashes.packageLockSha256 -or
      $postBuildHashes.cargoLockSha256 -cne $retainedHashes.cargoLockSha256) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_BASELINE_MISMATCH"
  }

  if (-not (Test-Path -LiteralPath $resolvedOutputBase)) {
    New-Item -ItemType Directory -Path $resolvedOutputBase -Force | Out-Null
  }
  if (Test-Path -LiteralPath $finalOutputRoot) {
    Stop-Build "BLOCKED_GRAMMAR_P2_01_OUTPUT_ROOT_ALREADY_EXISTS"
  }
  New-Item -ItemType Directory -Path $finalOutputRoot | Out-Null
  foreach ($item in @(Get-ChildItem -LiteralPath $stagingOutputRoot -Force)) {
    Copy-Item -LiteralPath $item.FullName -Destination $finalOutputRoot -Recurse
  }

  $finalPortableRoot = Join-Path (Join-Path $finalOutputRoot "portable") $packageStem
  $finalZipPath = Join-Path $finalOutputRoot $zipName
  $null = Invoke-PortableVerifier -VerifierPath (Join-Path $finalPortableRoot "VERIFY_BUNDLE.ps1") -PathToVerify $finalPortableRoot -SourceCommit $head
  $null = Invoke-PortableVerifier -VerifierPath (Join-Path $finalPortableRoot "VERIFY_BUNDLE.ps1") -PathToVerify $finalZipPath -SourceCommit $head

  foreach ($line in [System.IO.File]::ReadAllLines((Join-Path $finalOutputRoot "OUTPUT_SHA256SUMS.txt"))) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    if ($line -notmatch '^([0-9a-fA-F]{64})  (.+)$') {
      Stop-Build "BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED"
    }
    $relativePath = $Matches[2].Replace('/', [System.IO.Path]::DirectorySeparatorChar)
    $filePath = [System.IO.Path]::GetFullPath((Join-Path $finalOutputRoot $relativePath))
    $finalPrefix = [System.IO.Path]::GetFullPath($finalOutputRoot).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    if (-not $filePath.StartsWith($finalPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
        -not (Test-Path -LiteralPath $filePath -PathType Leaf) -or
        (Get-Sha256 $filePath) -cne $Matches[1].ToLowerInvariant()) {
      Stop-Build "BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED"
    }
  }

  $buildSucceeded = $true
  [pscustomobject]@{
    status = "PASS"
    sourceCommit = $head
    outputRoot = $finalOutputRoot
    portableDirectory = ("portable/" + $packageStem)
    portableZip = $zipName
    portableZipSize = [int64](Get-Item -LiteralPath $finalZipPath).Length
    portableZipSha256 = Get-Sha256 $finalZipPath
    executableRelativePath = ("portable/" + $packageStem + "/" + $EXPECTED_BINARY_NAME)
    executableSize = [int64](Get-Item -LiteralPath (Join-Path $finalPortableRoot $EXPECTED_BINARY_NAME)).Length
    executableSha256 = Get-Sha256 (Join-Path $finalPortableRoot $EXPECTED_BINARY_NAME)
    authenticodeStatus = "NotSigned"
    signingAttempted = $false
    updaterEnabled = $false
    publicUploadAttempted = $false
    codexCliVendored = $false
    nsisStatus = "NOT_ATTEMPTED_BY_POLICY"
  } | ConvertTo-Json -Depth 5
  exit 0
}
catch {
  $message = [string]$_.Exception.Message
  $tokenMatch = [regex]::Match($message, 'BLOCKED_GRAMMAR_P2_01_[A-Z0-9_]+')
  $token = if ($tokenMatch.Success) { $tokenMatch.Value } else { "BLOCKED_GRAMMAR_P2_01_BUILD_FAILED" }
  [Console]::Error.WriteLine($token)
  if ($null -ne $temporaryBuildRoot -and (Test-Path -LiteralPath $temporaryBuildRoot)) {
    [Console]::Error.WriteLine("P2_01_OWNED_TEMP_ROOT_PRESERVED=" + $temporaryBuildRoot)
  }
  exit 1
}
finally {
  foreach ($environmentName in @("CARGO_TARGET_DIR", "CARGO_NET_OFFLINE", "CI")) {
    if ($savedEnvironment.ContainsKey($environmentName)) {
      [System.Environment]::SetEnvironmentVariable($environmentName, $savedEnvironment[$environmentName], "Process")
    }
  }
  Set-Location -LiteralPath $repositoryRoot
  if ($buildSucceeded -and $null -ne $temporaryBuildRoot -and (Test-Path -LiteralPath $temporaryBuildRoot)) {
    Remove-OwnedBuildRoot $temporaryBuildRoot
  }
}
