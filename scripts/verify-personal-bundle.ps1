[CmdletBinding()]
param(
  [Parameter(Mandatory = $true, Position = 0)]
  [ValidateNotNullOrEmpty()]
  [string]$BundlePath,

  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[0-9a-fA-F]{40}$')]
  [string]$ExpectedSourceCommit
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 3.0

$EXPECTED_PROTOCOL_BUNDLE_SHA256 = "c954593823626b5194b7f687c27d542eb87fcfa63e5e11af3ca95f9b6c39c6e8"
$EXPECTED_PACKAGE_LOCK_SHA256 = "c3415432201286856ec3ca854b4931c13e947c131905a6bfc21242a6bfc2e669"
$EXPECTED_CARGO_LOCK_SHA256 = "524f11ebf0d741837761d4dca175d48ac908fcaa14be9ea84800ab344f893cce"
$EXPECTED_PRODUCT_NAME = "Codex Pencil"
$EXPECTED_SOURCE_BRANCH = "main"
$EXPECTED_TARGET_TRIPLE = "x86_64-pc-windows-msvc"
$EXPECTED_ARCHITECTURE = "x64"
$EXPECTED_CODEX_VERSION = "codex-cli 0.144.6"
$EXPECTED_COMMIT_SUBJECT = "build: add reproducible personal bundle workflow"
$IMAGE_FILE_MACHINE_AMD64 = 0x8664

$requiredMetadataEntries = @(
  "README_PERSONAL_USE.md",
  "BUILD_PROVENANCE.json",
  "ARTIFACT_INDEX.json",
  "SHA256SUMS.txt",
  "VERIFY_BUNDLE.ps1"
)

$temporaryExtractionRoot = $null
$failureCode = "UNEXPECTED_VERIFICATION_ERROR"

function Stop-Verification {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Code
  )

  $script:failureCode = $Code
  throw $Code
}

function Get-Sha256 {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Read-JsonObject {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,

    [Parameter(Mandatory = $true)]
    [string]$ErrorCode
  )

  try {
    $value = [System.IO.File]::ReadAllText($Path) | ConvertFrom-Json
  }
  catch {
    Stop-Verification $ErrorCode
  }

  if ($null -eq $value) {
    Stop-Verification $ErrorCode
  }

  return $value
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

function Assert-ScalarEqual {
  param(
    [AllowNull()]
    [object]$Actual,

    [AllowNull()]
    [object]$Expected,

    [Parameter(Mandatory = $true)]
    [string]$ErrorCode
  )

  if ([string]$Actual -cne [string]$Expected) {
    Stop-Verification $ErrorCode
  }
}

function Test-BooleanFalse {
  param(
    [AllowNull()]
    [object]$Value
  )

  return ($Value -is [bool] -and -not $Value)
}

function Get-PeMachine {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $stream = $null
  $reader = $null
  try {
    $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::Read)
    $reader = New-Object System.IO.BinaryReader($stream)
    if ($stream.Length -lt 64) {
      Stop-Verification "PE_FILE_TOO_SMALL"
    }

    if ($reader.ReadUInt16() -ne 0x5A4D) {
      Stop-Verification "PE_DOS_SIGNATURE_MISMATCH"
    }

    $stream.Position = 0x3C
    $peOffset = $reader.ReadInt32()
    if ($peOffset -lt 0 -or ($peOffset + 6) -gt $stream.Length) {
      Stop-Verification "PE_HEADER_OFFSET_INVALID"
    }

    $stream.Position = $peOffset
    if ($reader.ReadUInt32() -ne 0x00004550) {
      Stop-Verification "PE_SIGNATURE_MISMATCH"
    }

    return $reader.ReadUInt16()
  }
  finally {
    if ($null -ne $reader) {
      $reader.Dispose()
    }
    elseif ($null -ne $stream) {
      $stream.Dispose()
    }
  }
}

function Test-AppVersionMatch {
  param(
    [AllowNull()]
    [string]$EmbeddedVersion,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedVersion
  )

  if ([string]::IsNullOrWhiteSpace($EmbeddedVersion)) {
    return $true
  }

  return ($EmbeddedVersion -eq $ExpectedVersion -or $EmbeddedVersion.StartsWith($ExpectedVersion + ".", [System.StringComparison]::Ordinal))
}

function Read-ChecksumManifest {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $records = @{}
  foreach ($line in [System.IO.File]::ReadAllLines($Path)) {
    if ([string]::IsNullOrWhiteSpace($line)) {
      continue
    }

    if ($line -notmatch '^([0-9a-fA-F]{64})  ([^\\/]+)$') {
      Stop-Verification "CHECKSUM_MANIFEST_LINE_INVALID"
    }

    $relativePath = $Matches[2]
    if ($records.ContainsKey($relativePath)) {
      Stop-Verification "CHECKSUM_MANIFEST_DUPLICATE_PATH"
    }

    $records[$relativePath] = $Matches[1].ToLowerInvariant()
  }

  return $records
}

function Get-ArchiveSafety {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,

    [Parameter(Mandatory = $true)]
    [string]$ExtractionRoot
  )

  Add-Type -AssemblyName System.IO.Compression
  Add-Type -AssemblyName System.IO.Compression.FileSystem

  $archive = $null
  $duplicateEntryCount = 0
  $traversalEntryCount = 0
  $symlinkEntryCount = 0
  $directoryEntryCount = 0
  $entryNames = New-Object System.Collections.Generic.List[string]
  $seenNames = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)

  try {
    $archive = [System.IO.Compression.ZipFile]::OpenRead($ArchivePath)
    if ($archive -isnot [System.IO.Compression.ZipArchive]) {
      Stop-Verification "ARCHIVE_TYPE_INVALID"
    }
    foreach ($entry in $archive.Entries) {
      $entryName = $entry.FullName.Replace('\', '/')
      $entryNames.Add($entryName)

      if (-not $seenNames.Add($entryName)) {
        $duplicateEntryCount += 1
      }

      $segments = @($entryName.Split('/'))
      if ([string]::IsNullOrWhiteSpace($entryName) -or
          $entryName.StartsWith("/", [System.StringComparison]::Ordinal) -or
          $entryName -match '^[A-Za-z]:' -or
          $entryName.Contains("\") -or
          @($segments | Where-Object { $_ -eq ".." -or $_ -eq "." -or [string]::IsNullOrWhiteSpace($_) }).Count -gt 0) {
        $traversalEntryCount += 1
      }

      if ([string]::IsNullOrEmpty($entry.Name)) {
        $directoryEntryCount += 1
      }

      $unixFileType = (($entry.ExternalAttributes -shr 16) -band 0xF000)
      if ($unixFileType -eq 0xA000) {
        $symlinkEntryCount += 1
      }
    }

    if ($duplicateEntryCount -ne 0) {
      Stop-Verification "ARCHIVE_DUPLICATE_ENTRY"
    }
    if ($traversalEntryCount -ne 0) {
      Stop-Verification "ARCHIVE_TRAVERSAL_ENTRY"
    }
    if ($symlinkEntryCount -ne 0) {
      Stop-Verification "ARCHIVE_SYMLINK_ENTRY"
    }
    if ($directoryEntryCount -ne 0) {
      Stop-Verification "ARCHIVE_DIRECTORY_ENTRY_UNEXPECTED"
    }

    $extractionPrefix = [System.IO.Path]::GetFullPath($ExtractionRoot).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    foreach ($entry in $archive.Entries) {
      $relativePath = $entry.FullName.Replace('/', [System.IO.Path]::DirectorySeparatorChar)
      $destinationPath = [System.IO.Path]::GetFullPath((Join-Path $ExtractionRoot $relativePath))
      if (-not $destinationPath.StartsWith($extractionPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        Stop-Verification "ARCHIVE_EXTRACTION_PATH_ESCAPE"
      }

      $input = $null
      $output = $null
      try {
        $input = $entry.Open()
        $output = [System.IO.File]::Open($destinationPath, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
        $input.CopyTo($output)
      }
      finally {
        if ($null -ne $output) { $output.Dispose() }
        if ($null -ne $input) { $input.Dispose() }
      }
    }
  }
  catch {
    if ($script:failureCode -eq "UNEXPECTED_VERIFICATION_ERROR") {
      Stop-Verification "ARCHIVE_READ_OR_EXTRACTION_FAILED"
    }
    throw
  }
  finally {
    if ($null -ne $archive) {
      $archive.Dispose()
    }
  }

  return [pscustomobject]@{
    entryCount = $entryNames.Count
    duplicateEntryCount = $duplicateEntryCount
    traversalEntryCount = $traversalEntryCount
    symlinkEntryCount = $symlinkEntryCount
    directoryEntryCount = $directoryEntryCount
  }
}

function Assert-DistributableTextPrivacy {
  param(
    [Parameter(Mandatory = $true)]
    [System.IO.FileInfo[]]$Files
  )

  $absolutePathPattern = '(?i)[A-Za-z]:[\\/](?:Users[\\/][^\\/\r\n]+|dev[\\/]repos[\\/]Grammar)(?:[\\/]|$)'
  $localUsername = [System.Environment]::UserName
  $protectedValuePatterns = @(
    '(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b',
    '(?i)https?://[^\s"''<>]+',
    '\bsk-[A-Za-z0-9]{8,}\b',
    '\b[A-Z0-9]{4}-[A-Z0-9]{4}\b',
    '(?i)"(?:selectedText|replacement|clipboardText|deviceCode|accessToken|refreshToken|accountEmail|codexHome|appData|localAppData)"\s*:'
  )

  $localAbsolutePathCount = 0
  $localUsernameCount = 0
  $forbiddenProtectedValueCount = 0
  foreach ($file in $Files) {
    if ($file.Extension -notin @(".md", ".json", ".txt", ".ps1")) {
      continue
    }

    $text = [System.IO.File]::ReadAllText($file.FullName)
    $localAbsolutePathCount += [regex]::Matches($text, $absolutePathPattern).Count
    if (-not [string]::IsNullOrWhiteSpace($localUsername)) {
      $localUsernameCount += [regex]::Matches($text, [regex]::Escape($localUsername)).Count
    }
    foreach ($pattern in $protectedValuePatterns) {
      $forbiddenProtectedValueCount += [regex]::Matches($text, $pattern).Count
    }
  }

  if ($localAbsolutePathCount -ne 0) {
    Stop-Verification "DISTRIBUTABLE_LOCAL_ABSOLUTE_PATH_FOUND"
  }
  if ($localUsernameCount -ne 0) {
    Stop-Verification "DISTRIBUTABLE_LOCAL_ACCOUNT_NAME_FOUND"
  }
  if ($forbiddenProtectedValueCount -ne 0) {
    Stop-Verification "DISTRIBUTABLE_PROTECTED_VALUE_FOUND"
  }

  return [pscustomobject]@{
    localAbsolutePathCount = $localAbsolutePathCount
    localUsernameCount = $localUsernameCount
    forbiddenProtectedValueCount = $forbiddenProtectedValueCount
  }
}

function Remove-OwnedExtractionRoot {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $tempPrefix = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
  $leaf = Split-Path -Leaf $fullPath
  if (-not $fullPath.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
      -not $leaf.StartsWith("codex-pencil-bundle-verify-", [System.StringComparison]::Ordinal)) {
    Stop-Verification "OWNED_EXTRACTION_ROOT_GUARD_FAILED"
  }

  if (Test-Path -LiteralPath $fullPath) {
    Remove-Item -LiteralPath $fullPath -Recurse -Force
  }
}

try {
  $failureCode = "BUNDLE_PATH_INVALID"
  $resolvedBundle = Resolve-Path -LiteralPath $BundlePath -ErrorAction Stop
  $bundleItem = Get-Item -LiteralPath $resolvedBundle.Path -Force
  $bundleKind = $null
  $bundleRoot = $null
  $archiveSafety = [pscustomobject]@{
    entryCount = 0
    duplicateEntryCount = 0
    traversalEntryCount = 0
    symlinkEntryCount = 0
    directoryEntryCount = 0
  }

  if ($bundleItem.PSIsContainer) {
    $bundleKind = "directory"
    $bundleRoot = $bundleItem.FullName
  }
  elseif ($bundleItem.Extension -ieq ".zip") {
    $bundleKind = "zip"
    $temporaryExtractionRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("codex-pencil-bundle-verify-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $temporaryExtractionRoot -ErrorAction Stop | Out-Null
    $archiveSafety = Get-ArchiveSafety -ArchivePath $bundleItem.FullName -ExtractionRoot $temporaryExtractionRoot
    $bundleRoot = $temporaryExtractionRoot
  }
  else {
    Stop-Verification "BUNDLE_PATH_MUST_BE_ZIP_OR_DIRECTORY"
  }

  $allItems = @(Get-ChildItem -LiteralPath $bundleRoot -Force)
  $reparseItems = @($allItems | Where-Object { ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 })
  $directories = @($allItems | Where-Object { $_.PSIsContainer })
  $files = @($allItems | Where-Object { -not $_.PSIsContainer })
  $nestedItems = @()
  if ($directories.Count -gt 0) {
    $nestedItems = @(Get-ChildItem -LiteralPath $bundleRoot -Recurse -Force)
  }
  $nestedReparseItems = @($nestedItems | Where-Object { ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 })
  $symlinkEntryCount = $reparseItems.Count + $nestedReparseItems.Count + [int]$archiveSafety.symlinkEntryCount

  if ($symlinkEntryCount -ne 0) {
    Stop-Verification "BUNDLE_SYMLINK_OR_REPARSE_POINT_FOUND"
  }
  if ($directories.Count -ne 0) {
    Stop-Verification "BUNDLE_SUBDIRECTORY_UNEXPECTED"
  }

  $executables = @($files | Where-Object { $_.Extension -ieq ".exe" })
  if ($executables.Count -ne 1) {
    Stop-Verification "APPLICATION_EXECUTABLE_COUNT_MISMATCH"
  }

  $expectedNames = @($requiredMetadataEntries + $executables[0].Name)
  $unexpectedFiles = @($files | Where-Object { $_.Name -notin $expectedNames })
  $missingFiles = @($expectedNames | Where-Object { -not (Test-Path -LiteralPath (Join-Path $bundleRoot $_) -PathType Leaf) })
  $unexpectedEntryCount = $unexpectedFiles.Count + $directories.Count
  if ($unexpectedEntryCount -ne 0) {
    Stop-Verification "BUNDLE_UNEXPECTED_ENTRY"
  }
  if ($missingFiles.Count -ne 0 -or $files.Count -ne 6) {
    Stop-Verification "BUNDLE_REQUIRED_ENTRY_MISMATCH"
  }
  if ($bundleKind -eq "zip" -and [int]$archiveSafety.entryCount -ne 6) {
    Stop-Verification "ARCHIVE_ENTRY_COUNT_MISMATCH"
  }

  $provenancePath = Join-Path $bundleRoot "BUILD_PROVENANCE.json"
  $artifactIndexPath = Join-Path $bundleRoot "ARTIFACT_INDEX.json"
  $checksumPath = Join-Path $bundleRoot "SHA256SUMS.txt"
  $provenance = Read-JsonObject -Path $provenancePath -ErrorCode "BUILD_PROVENANCE_JSON_INVALID"
  $artifactIndex = Read-JsonObject -Path $artifactIndexPath -ErrorCode "ARTIFACT_INDEX_JSON_INVALID"

  Assert-ScalarEqual (Get-ObjectProperty $provenance "schemaVersion") 1 "PROVENANCE_SCHEMA_VERSION_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "projectName") "codex-pencil" "PROVENANCE_PROJECT_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "productName") $EXPECTED_PRODUCT_NAME "PROVENANCE_PRODUCT_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "sourceBranch") $EXPECTED_SOURCE_BRANCH "PROVENANCE_BRANCH_MISMATCH"
  Assert-ScalarEqual ([string](Get-ObjectProperty $provenance "sourceCommit")).ToLowerInvariant() $ExpectedSourceCommit.ToLowerInvariant() "PROVENANCE_COMMIT_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "targetTriple") $EXPECTED_TARGET_TRIPLE "PROVENANCE_TARGET_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "architecture") $EXPECTED_ARCHITECTURE "PROVENANCE_ARCHITECTURE_MISMATCH"
  Assert-ScalarEqual ([string](Get-ObjectProperty $provenance "protocolBundleSha256")).ToLowerInvariant() $EXPECTED_PROTOCOL_BUNDLE_SHA256 "PROVENANCE_PROTOCOL_HASH_MISMATCH"
  Assert-ScalarEqual ([string](Get-ObjectProperty $provenance "packageLockSha256")).ToLowerInvariant() $EXPECTED_PACKAGE_LOCK_SHA256 "PROVENANCE_PACKAGE_LOCK_HASH_MISMATCH"
  Assert-ScalarEqual ([string](Get-ObjectProperty $provenance "cargoLockSha256")).ToLowerInvariant() $EXPECTED_CARGO_LOCK_SHA256 "PROVENANCE_CARGO_LOCK_HASH_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "codexVersion") $EXPECTED_CODEX_VERSION "PROVENANCE_CODEX_VERSION_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "commitSubject") $EXPECTED_COMMIT_SUBJECT "PROVENANCE_COMMIT_SUBJECT_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "nsisStatus") "NOT_ATTEMPTED_BY_POLICY" "PROVENANCE_NSIS_STATUS_MISMATCH"

  $versionPatterns = [ordered]@{
    nodeVersion = '^v\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$'
    npmVersion = '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$'
    rustcVersion = '^rustc \d+\.\d+\.\d+(?:[-+][^ ]+)?(?: .+)?$'
    cargoVersion = '^cargo \d+\.\d+\.\d+(?:[-+][^ ]+)?(?: .+)?$'
    tauriVersion = '^tauri-cli \d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$'
  }
  foreach ($field in $versionPatterns.Keys) {
    if ([string](Get-ObjectProperty $provenance $field) -notmatch $versionPatterns[$field]) {
      Stop-Verification ("PROVENANCE_TOOL_VERSION_INVALID_" + $field.ToUpperInvariant())
    }
  }

  $parsedBuildUtc = [System.DateTimeOffset]::MinValue
  if (-not [System.DateTimeOffset]::TryParse([string](Get-ObjectProperty $provenance "buildUtc"), [ref]$parsedBuildUtc)) {
    Stop-Verification "PROVENANCE_BUILD_UTC_INVALID"
  }

  if (-not (Test-BooleanFalse (Get-ObjectProperty $provenance "signingAttempted"))) {
    Stop-Verification "PROVENANCE_SIGNING_POLICY_MISMATCH"
  }
  if (-not (Test-BooleanFalse (Get-ObjectProperty $provenance "updaterEnabled"))) {
    Stop-Verification "PROVENANCE_UPDATER_POLICY_MISMATCH"
  }
  if (-not (Test-BooleanFalse (Get-ObjectProperty $provenance "publicUploadAttempted"))) {
    Stop-Verification "PROVENANCE_PUBLICATION_POLICY_MISMATCH"
  }
  if (-not (Test-BooleanFalse (Get-ObjectProperty $provenance "codexCliVendored"))) {
    Stop-Verification "PROVENANCE_CODEX_VENDORING_POLICY_MISMATCH"
  }
  Assert-ScalarEqual (Get-ObjectProperty $provenance "updaterArtifactCount") 0 "PROVENANCE_UPDATER_ARTIFACT_COUNT_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "localAbsolutePathCount") 0 "PROVENANCE_LOCAL_ABSOLUTE_PATH_COUNT_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "localUsernameCount") 0 "PROVENANCE_LOCAL_ACCOUNT_NAME_COUNT_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "forbiddenProtectedValueCount") 0 "PROVENANCE_PROTECTED_VALUE_COUNT_MISMATCH"

  $appVersion = [string](Get-ObjectProperty $provenance "appVersion")
  if ($appVersion -notmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') {
    Stop-Verification "PROVENANCE_APP_VERSION_INVALID"
  }

  $executableRelativePath = [string](Get-ObjectProperty $provenance "executableRelativePath")
  Assert-ScalarEqual $executableRelativePath $executables[0].Name "PROVENANCE_EXECUTABLE_PATH_MISMATCH"
  $executableSize = [int64]$executables[0].Length
  $executableSha256 = Get-Sha256 $executables[0].FullName
  Assert-ScalarEqual (Get-ObjectProperty $provenance "executableSize") $executableSize "PROVENANCE_EXECUTABLE_SIZE_MISMATCH"
  Assert-ScalarEqual ([string](Get-ObjectProperty $provenance "executableSha256")).ToLowerInvariant() $executableSha256 "PROVENANCE_EXECUTABLE_HASH_MISMATCH"

  $peMachine = Get-PeMachine $executables[0].FullName
  if ($peMachine -ne $IMAGE_FILE_MACHINE_AMD64) {
    Stop-Verification "PE_ARCHITECTURE_MISMATCH"
  }

  $authenticode = Get-AuthenticodeSignature -LiteralPath $executables[0].FullName
  $authenticodeStatus = [string]$authenticode.Status
  Assert-ScalarEqual $authenticodeStatus "NotSigned" "AUTHENTICODE_STATUS_NOT_PRIVATE_UNSIGNED"
  Assert-ScalarEqual (Get-ObjectProperty $provenance "authenticodeStatus") $authenticodeStatus "PROVENANCE_AUTHENTICODE_STATUS_MISMATCH"

  $versionInfo = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($executables[0].FullName)
  $fileVersionPresent = -not [string]::IsNullOrWhiteSpace($versionInfo.FileVersion)
  $productVersionPresent = -not [string]::IsNullOrWhiteSpace($versionInfo.ProductVersion)
  $fileVersionMatches = Test-AppVersionMatch -EmbeddedVersion $versionInfo.FileVersion -ExpectedVersion $appVersion
  $productVersionMatches = Test-AppVersionMatch -EmbeddedVersion $versionInfo.ProductVersion -ExpectedVersion $appVersion
  if (-not $fileVersionMatches -or -not $productVersionMatches) {
    Stop-Verification "PE_VERSION_MISMATCH"
  }

  Assert-ScalarEqual (Get-ObjectProperty $artifactIndex "schemaVersion") 1 "ARTIFACT_INDEX_SCHEMA_VERSION_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $artifactIndex "hashAlgorithm") "SHA-256" "ARTIFACT_INDEX_HASH_ALGORITHM_MISMATCH"
  Assert-ScalarEqual (Get-ObjectProperty $artifactIndex "selfReferencePolicy") "ARTIFACT_INDEX_AND_SHA256SUMS_EXCLUDED" "ARTIFACT_INDEX_SELF_REFERENCE_POLICY_MISMATCH"

  $indexEntries = @(Get-ObjectProperty $artifactIndex "entries")
  $expectedIndexPaths = @($expectedNames | Where-Object { $_ -notin @("ARTIFACT_INDEX.json", "SHA256SUMS.txt") } | Sort-Object)
  if ($indexEntries.Count -ne $expectedIndexPaths.Count) {
    Stop-Verification "ARTIFACT_INDEX_ENTRY_COUNT_MISMATCH"
  }

  $indexPaths = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)
  foreach ($entry in $indexEntries) {
    $relativePath = [string](Get-ObjectProperty $entry "relativePath")
    if (-not $indexPaths.Add($relativePath)) {
      Stop-Verification "ARTIFACT_INDEX_DUPLICATE_PATH"
    }
    if ($relativePath -notin $expectedIndexPaths) {
      Stop-Verification "ARTIFACT_INDEX_UNEXPECTED_PATH"
    }

    $indexedFile = Get-Item -LiteralPath (Join-Path $bundleRoot $relativePath)
    Assert-ScalarEqual (Get-ObjectProperty $entry "size") ([int64]$indexedFile.Length) "ARTIFACT_INDEX_SIZE_MISMATCH"
    Assert-ScalarEqual ([string](Get-ObjectProperty $entry "sha256")).ToLowerInvariant() (Get-Sha256 $indexedFile.FullName) "ARTIFACT_INDEX_HASH_MISMATCH"
  }

  foreach ($expectedIndexPath in $expectedIndexPaths) {
    if (-not $indexPaths.Contains($expectedIndexPath)) {
      Stop-Verification "ARTIFACT_INDEX_REQUIRED_PATH_MISSING"
    }
  }

  $checksumRecords = Read-ChecksumManifest $checksumPath
  $expectedChecksumPaths = @($expectedNames | Where-Object { $_ -ne "SHA256SUMS.txt" } | Sort-Object)
  if ($checksumRecords.Count -ne $expectedChecksumPaths.Count) {
    Stop-Verification "CHECKSUM_MANIFEST_ENTRY_COUNT_MISMATCH"
  }
  foreach ($relativePath in $expectedChecksumPaths) {
    if (-not $checksumRecords.ContainsKey($relativePath)) {
      Stop-Verification "CHECKSUM_MANIFEST_REQUIRED_PATH_MISSING"
    }
    Assert-ScalarEqual $checksumRecords[$relativePath] (Get-Sha256 (Join-Path $bundleRoot $relativePath)) "CHECKSUM_MANIFEST_HASH_MISMATCH"
  }

  $updaterArtifactCount = @($files | Where-Object {
    $_.Name -match '(?i)(^latest\.json$|\.sig$|\.msi$|\.msi\.zip$|setup\.exe$|updater)'
  }).Count
  if ($updaterArtifactCount -ne 0) {
    Stop-Verification "UPDATER_ARTIFACT_FOUND"
  }

  $forbiddenNameCount = @($allItems | Where-Object {
    $_.Name -match '(?i)(^|[._-])(settings|terminology|auth|codex[-_]?home|runtime|log|source|tests?|fixtures?|node_modules|dist|target|temp|cache)([._-]|$)' -or
    $_.Name -in @(".git", "+Chat_Project_Source")
  }).Count
  if ($forbiddenNameCount -ne 0) {
    Stop-Verification "FORBIDDEN_BUNDLE_NAME_FOUND"
  }

  $privacy = Assert-DistributableTextPrivacy -Files $files

  [pscustomobject]@{
    schemaVersion = 1
    verified = $true
    bundleKind = $bundleKind
    portableEntryCount = $files.Count
    applicationExecutableCount = $executables.Count
    executableRelativePath = $executables[0].Name
    executableSize = $executableSize
    executableSha256 = $executableSha256
    sourceCommit = $ExpectedSourceCommit.ToLowerInvariant()
    archiveEntryCount = [int]$archiveSafety.entryCount
    traversalEntryCount = [int]$archiveSafety.traversalEntryCount
    duplicateEntryCount = [int]$archiveSafety.duplicateEntryCount
    symlinkEntryCount = $symlinkEntryCount
    unexpectedEntryCount = $unexpectedEntryCount
    missingEntryCount = $missingFiles.Count
    checksumEntryCount = $checksumRecords.Count
    artifactIndexEntryCount = $indexEntries.Count
    peMachine = "IMAGE_FILE_MACHINE_AMD64"
    architectureVerified = $true
    fileVersionPresent = $fileVersionPresent
    fileVersionMatches = $fileVersionMatches
    productVersionPresent = $productVersionPresent
    productVersionMatches = $productVersionMatches
    authenticodeStatus = $authenticodeStatus
    signingAttempted = $false
    updaterEnabled = $false
    updaterArtifactCount = $updaterArtifactCount
    forbiddenNameCount = $forbiddenNameCount
    localAbsolutePathCount = [int]$privacy.localAbsolutePathCount
    localUsernameCount = [int]$privacy.localUsernameCount
    forbiddenProtectedValueCount = [int]$privacy.forbiddenProtectedValueCount
  } | ConvertTo-Json -Depth 5
  exit 0
}
catch {
  $safeCode = $failureCode
  if ([string]::IsNullOrWhiteSpace($safeCode)) {
    $safeCode = "UNEXPECTED_VERIFICATION_ERROR"
  }
  [Console]::Error.WriteLine("BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED:" + $safeCode)
  exit 1
}
finally {
  if ($null -ne $temporaryExtractionRoot -and (Test-Path -LiteralPath $temporaryExtractionRoot)) {
    try {
      Remove-OwnedExtractionRoot -Path $temporaryExtractionRoot
    }
    catch {
      [Console]::Error.WriteLine("BLOCKED_GRAMMAR_P2_01_ARTIFACT_VERIFICATION_FAILED:OWNED_EXTRACTION_CLEANUP_FAILED")
    }
  }
}
