[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 3.0

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$buildScriptPath = Join-Path $PSScriptRoot "build-personal-bundle.ps1"
$verifyScriptPath = Join-Path $PSScriptRoot "verify-personal-bundle.ps1"
$documentationPath = Join-Path $repositoryRoot "docs\PERSONAL_BUNDLE.md"

$failures = New-Object System.Collections.Generic.List[string]
$assertionCount = 0

function Assert-Contract {
  param(
    [Parameter(Mandatory = $true)]
    [bool]$Condition,

    [Parameter(Mandatory = $true)]
    [string]$Code
  )

  $script:assertionCount += 1
  if (-not $Condition) {
    $script:failures.Add($Code)
  }
}

function Read-TrackedText {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    return ""
  }

  return [System.IO.File]::ReadAllText($Path)
}

$buildSource = Read-TrackedText -Path $buildScriptPath
$verifySource = Read-TrackedText -Path $verifyScriptPath
$documentationSource = Read-TrackedText -Path $documentationPath

Assert-Contract (Test-Path -LiteralPath $buildScriptPath -PathType Leaf) "BUILD_SCRIPT_MISSING"
Assert-Contract (Test-Path -LiteralPath $verifyScriptPath -PathType Leaf) "VERIFY_SCRIPT_MISSING"
Assert-Contract (Test-Path -LiteralPath $documentationPath -PathType Leaf) "DOCUMENTATION_MISSING"

foreach ($parameterName in @("ExpectedSourceCommit", "OutputBase")) {
  Assert-Contract ($buildSource -match ("(?m)\$" + [regex]::Escape($parameterName) + "\b")) ("BUILD_PARAMETER_MISSING_" + $parameterName.ToUpperInvariant())
}

foreach ($guard in @(
  "AUTHORIZED_P1_03_BASELINE",
  "Assert-ExternalOutputBase",
  "Assert-CleanMainSource",
  "Assert-ApprovedPackagingDiff",
  "BLOCKED_GRAMMAR_P2_01_OUTPUT_ROOT_ALREADY_EXISTS",
  "CARGO_NET_OFFLINE",
  "--locked",
  "--no-bundle",
  "--no-sign"
)) {
  Assert-Contract ($buildSource.Contains($guard)) ("BUILD_GUARD_MISSING_" + $guard.ToUpperInvariant().Replace("-", "_").Replace(" ", "_"))
}

$requiredEntries = @(
  "codex-pencil.exe",
  "README_PERSONAL_USE.md",
  "BUILD_PROVENANCE.json",
  "ARTIFACT_INDEX.json",
  "SHA256SUMS.txt",
  "VERIFY_BUNDLE.ps1"
)
foreach ($entry in $requiredEntries) {
  Assert-Contract ($buildSource.Contains($entry)) ("PORTABLE_ENTRY_MISSING_" + $entry.ToUpperInvariant().Replace(".", "_"))
}

foreach ($policyLiteral in @(
  'SigningAttempted = $false',
  'UpdaterEnabled = $false',
  'PublicUploadAttempted = $false',
  'CodexCliVendored = $false'
)) {
  Assert-Contract ($buildSource.Contains($policyLiteral)) ("FALSE_POLICY_MISSING_" + $policyLiteral.Split(" ")[0].ToUpperInvariant())
}

foreach ($field in @(
  "schemaVersion",
  "projectName",
  "productName",
  "appVersion",
  "sourceCommit",
  "sourceBranch",
  "commitSubject",
  "targetTriple",
  "architecture",
  "buildUtc",
  "nodeVersion",
  "npmVersion",
  "rustcVersion",
  "cargoVersion",
  "tauriVersion",
  "codexVersion",
  "protocolBundleSha256",
  "packageLockSha256",
  "cargoLockSha256",
  "executableRelativePath",
  "executableSize",
  "executableSha256",
  "authenticodeStatus",
  "signingAttempted",
  "updaterEnabled",
  "updaterArtifactCount",
  "publicUploadAttempted",
  "codexCliVendored",
  "localAbsolutePathCount",
  "localUsernameCount",
  "forbiddenProtectedValueCount",
  "nsisStatus"
)) {
  Assert-Contract ($buildSource -match ("(?i)\b" + [regex]::Escape($field) + "\b")) ("PROVENANCE_FIELD_MISSING_" + $field.ToUpperInvariant())
}

foreach ($privacyGuard in @(
  "Assert-DistributablePrivacy",
  "UserName",
  "USERPROFILE",
  "repositoryAbsolutePath",
  "localUsernameCount",
  "forbiddenProtectedValueCount"
)) {
  Assert-Contract ($buildSource.Contains($privacyGuard)) ("PRIVACY_GUARD_MISSING_" + $privacyGuard.ToUpperInvariant())
}

foreach ($verificationGuard in @(
  "ZipArchive",
  "duplicateEntryCount",
  "traversalEntryCount",
  "symlinkEntryCount",
  "unexpectedEntryCount",
  "Get-AuthenticodeSignature",
  "IMAGE_FILE_MACHINE_AMD64",
  "updaterArtifactCount",
  "ARTIFACT_INDEX.json",
  "SHA256SUMS.txt",
  "BUILD_PROVENANCE.json"
)) {
  Assert-Contract ($verifySource.Contains($verificationGuard)) ("VERIFY_GUARD_MISSING_" + $verificationGuard.ToUpperInvariant().Replace("-", "_"))
}

foreach ($documentationContract in @(
  "portable ZIP",
  "unsigned",
  "no updater",
  "codex-cli 0.144.6",
  "build-personal-bundle.ps1",
  "verify-personal-bundle.ps1",
  "commit-bound"
)) {
  Assert-Contract ($documentationSource -match [regex]::Escape($documentationContract)) ("DOCUMENTATION_CONTRACT_MISSING_" + $documentationContract.ToUpperInvariant().Replace(" ", "_"))
}

if ($failures.Count -gt 0) {
  [pscustomobject]@{
    status = "RED"
    assertionCount = $assertionCount
    failureCount = $failures.Count
    failures = @($failures)
  } | ConvertTo-Json -Depth 4
  exit 1
}

[pscustomobject]@{
  status = "GREEN"
  assertionCount = $assertionCount
  failureCount = 0
} | ConvertTo-Json -Depth 3
