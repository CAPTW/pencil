[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$OutputRoot,

  [Parameter(Mandatory = $true)]
  [string]$CargoTargetRoot,

  [switch]$NativeProcessSelfTest,

  [switch]$Utf8JsonSelfTest
)

class NativeProcessResult {
  [string]$ExecutablePath
  [int]$ArgumentCount
  [string]$ArgumentSha256
  [string]$WorkingDirectorySha256
  [int]$ProcessId
  [datetime]$StartUtc
  [datetime]$EndUtc
  [long]$ElapsedMilliseconds
  [bool]$TimedOut
  [int]$ExitCode
  [string]$StdoutPath
  [string]$StderrPath
  [long]$StdoutBytes
  [string]$StdoutSha256
  [long]$StderrBytes
  [string]$StderrSha256
  [long]$PeakWorkingSetBytes
}

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$ExpectedTaskId = "GRAMMAR-P3-02-INSTANT-SELECTION-ENGINE-FOUNDATION-IMPLEMENTATION-AND-QUALIFICATION"
$ExpectedEngineId = "deterministic-rule-engine"
$ExpectedEngineVersion = "0.1.0"
$ExpectedCaseCount = 120
$RunCount = 3
$Utf8NoBom = New-Object System.Text.UTF8Encoding($false, $true)

function Stop-Benchmark {
  param([Parameter(Mandatory = $true)][string]$Code)
  throw $Code
}

function Get-CanonicalPath {
  param([Parameter(Mandatory = $true)][string]$Path)
  return [System.IO.Path]::GetFullPath($Path).TrimEnd("\")
}

function Get-Sha256 {
  param([Parameter(Mandatory = $true)][string]$Path)
  $stream = [System.IO.File]::Open(
    [System.IO.Path]::GetFullPath($Path),
    [System.IO.FileMode]::Open,
    [System.IO.FileAccess]::Read,
    [System.IO.FileShare]::Read
  )
  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  try {
    return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace('-', '').ToLowerInvariant()
  }
  finally {
    $sha256.Dispose()
    $stream.Dispose()
  }
}

function Read-StrictUtf8Text {
  param([Parameter(Mandatory = $true)][string]$Path)

  $bytes = [System.IO.File]::ReadAllBytes([System.IO.Path]::GetFullPath($Path))
  $offset = 0
  if ($bytes.Length -ge 3 -and
      $bytes[0] -eq 0xEF -and
      $bytes[1] -eq 0xBB -and
      $bytes[2] -eq 0xBF) {
    $offset = 3
  }
  for ($index = $offset; $index -le ($bytes.Length - 3); $index += 1) {
    if ($bytes[$index] -eq 0xEF -and
        $bytes[$index + 1] -eq 0xBB -and
        $bytes[$index + 2] -eq 0xBF) {
      Stop-Benchmark 'BLOCKED_GRAMMAR_P3_02_UTF8_BOM_POSITION_INVALID'
    }
  }
  try {
    return $Utf8NoBom.GetString($bytes, $offset, $bytes.Length - $offset)
  } catch {
    Stop-Benchmark 'BLOCKED_GRAMMAR_P3_02_UTF8_DECODE_FAILED'
  }
}

function Read-StrictUtf8Json {
  param([Parameter(Mandatory = $true)][string]$Path)

  $text = Read-StrictUtf8Text -Path $Path
  try {
    return ConvertFrom-Json -InputObject $text
  } catch {
    Stop-Benchmark 'BLOCKED_GRAMMAR_P3_02_JSON_PARSE_FAILED'
  }
}

function Read-StrictUtf8JsonLines {
  param([Parameter(Mandatory = $true)][string]$Path)

  $text = Read-StrictUtf8Text -Path $Path
  $lines = $text.Split([string[]]@("`r`n", "`n", "`r"), [System.StringSplitOptions]::RemoveEmptyEntries)
  foreach ($line in $lines) {
    try {
      ConvertFrom-Json -InputObject $line
    } catch {
      Stop-Benchmark 'BLOCKED_GRAMMAR_P3_02_JSON_LINES_PARSE_FAILED'
    }
  }
}

function Write-NewBytes {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][byte[]]$Bytes
  )
  $stream = [System.IO.File]::Open(
    [System.IO.Path]::GetFullPath($Path),
    [System.IO.FileMode]::CreateNew,
    [System.IO.FileAccess]::Write,
    [System.IO.FileShare]::None
  )
  try {
    $stream.Write($Bytes, 0, $Bytes.Length)
    $stream.Flush()
  } finally {
    $stream.Dispose()
  }
}

function Write-NewUtf8Text {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value
  )
  Write-NewBytes -Path $Path -Bytes $Utf8NoBom.GetBytes($Value)
}

function Get-StringSha256 {
  param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    $bytes = $Utf8NoBom.GetBytes($Value)
    return -join ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString("x2") })
  } finally {
    $sha.Dispose()
  }
}

function ConvertTo-WindowsCommandLineArgument {
  param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Argument)
  if ($Argument.Length -gt 0 -and $Argument -notmatch '[\s"]') {
    return $Argument
  }

  $builder = New-Object System.Text.StringBuilder
  [void]$builder.Append('"')
  $backslashCount = 0
  foreach ($character in $Argument.ToCharArray()) {
    if ([int]$character -eq 92) {
      $backslashCount += 1
      continue
    }
    if ([int]$character -eq 34) {
      if ($backslashCount -gt 0) {
        [void]$builder.Append(('\' * (($backslashCount * 2) + 1)))
      } else {
        [void]$builder.Append('\')
      }
      [void]$builder.Append('"')
      $backslashCount = 0
      continue
    }
    if ($backslashCount -gt 0) {
      [void]$builder.Append(('\' * $backslashCount))
      $backslashCount = 0
    }
    [void]$builder.Append($character)
  }
  if ($backslashCount -gt 0) {
    [void]$builder.Append(('\' * ($backslashCount * 2)))
  }
  [void]$builder.Append('"')
  return $builder.ToString()
}

function Join-NativeArguments {
  param([Parameter(Mandatory = $true)][AllowEmptyString()][string[]]$Arguments)
  return (($Arguments | ForEach-Object { ConvertTo-WindowsCommandLineArgument -Argument $_ }) -join ' ')
}

function Stop-TaskOwnedProcessTree {
  param([Parameter(Mandatory = $true)][int]$ProcessId)
  $taskkillPath = Join-Path $env:SystemRoot 'System32\taskkill.exe'
  if (-not (Test-Path -LiteralPath $taskkillPath -PathType Leaf)) {
    return
  }
  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = $taskkillPath
  $startInfo.Arguments = "/PID $ProcessId /T /F"
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  $killer = New-Object System.Diagnostics.Process
  $killer.StartInfo = $startInfo
  try {
    if ($killer.Start()) {
      $stdoutTask = $killer.StandardOutput.ReadToEndAsync()
      $stderrTask = $killer.StandardError.ReadToEndAsync()
      $killer.WaitForExit()
      [void]$stdoutTask.GetAwaiter().GetResult()
      [void]$stderrTask.GetAwaiter().GetResult()
    }
  } finally {
    $killer.Dispose()
  }
}

function Invoke-NativeProcess {
  param(
    [Parameter(Mandatory = $true)][string]$ExecutablePath,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string[]]$Arguments,
    [Parameter(Mandatory = $true)][string]$WorkingDirectory,
    [Parameter(Mandatory = $true)][string]$StdoutPath,
    [Parameter(Mandatory = $true)][string]$StderrPath,
    [int]$TimeoutMilliseconds = 600000
  )

  $executable = Get-CanonicalPath $ExecutablePath
  $working = Get-CanonicalPath $WorkingDirectory
  $stdout = [System.IO.Path]::GetFullPath($StdoutPath)
  $stderr = [System.IO.Path]::GetFullPath($StderrPath)
  if (-not (Test-Path -LiteralPath $executable -PathType Leaf) -or
      -not (Test-Path -LiteralPath $working -PathType Container) -or
      $stdout -eq $stderr -or
      $TimeoutMilliseconds -le 0) {
    Stop-Benchmark 'BLOCKED_GRAMMAR_P3_02_NATIVE_PROCESS_ARGUMENT_INVALID'
  }
  if ((Test-Path -LiteralPath $stdout) -or (Test-Path -LiteralPath $stderr)) {
    Stop-Benchmark 'BLOCKED_GRAMMAR_P3_02_BENCHMARK_OUTPUT_ALREADY_EXISTS'
  }
  foreach ($parent in @((Split-Path -Parent $stdout), (Split-Path -Parent $stderr))) {
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
      Stop-Benchmark 'BLOCKED_GRAMMAR_P3_02_BENCHMARK_PARENT_MISSING'
    }
  }

  $stdoutStream = $null
  $stderrStream = $null
  $process = $null
  $started = $false
  $timedOut = $false
  $processId = 0
  $exitCode = -1
  $peakWorkingSetBytes = 0L
  $startUtc = [datetime]::UtcNow
  $endUtc = $startUtc
  $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
  try {
    $stdoutStream = [System.IO.File]::Open($stdout, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
    $stderrStream = [System.IO.File]::Open($stderr, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $executable
    $startInfo.Arguments = Join-NativeArguments -Arguments $Arguments
    $startInfo.WorkingDirectory = $working
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    if (-not $process.Start()) {
      Stop-Benchmark 'BLOCKED_GRAMMAR_P3_02_NATIVE_PROCESS_START_FAILED'
    }
    $started = $true
    $processId = $process.Id
    $startUtc = [datetime]::UtcNow
    $stdoutTask = $process.StandardOutput.BaseStream.CopyToAsync($stdoutStream)
    $stderrTask = $process.StandardError.BaseStream.CopyToAsync($stderrStream)

    while (-not $process.WaitForExit(20)) {
      try {
        $process.Refresh()
        if ($process.WorkingSet64 -gt $peakWorkingSetBytes) {
          $peakWorkingSetBytes = $process.WorkingSet64
        }
      } catch {
      }
      if ($stopwatch.ElapsedMilliseconds -ge $TimeoutMilliseconds) {
        $timedOut = $true
        Stop-TaskOwnedProcessTree -ProcessId $processId
        if (-not $process.WaitForExit(5000)) {
          try { $process.Kill() } catch {}
          [void]$process.WaitForExit(5000)
        }
        break
      }
    }
    if (-not $timedOut) {
      $process.WaitForExit()
      $exitCode = [int]$process.ExitCode
    }
    try {
      $process.Refresh()
      if ($process.PeakWorkingSet64 -gt $peakWorkingSetBytes) {
        $peakWorkingSetBytes = $process.PeakWorkingSet64
      }
    } catch {
    }
    [void]$stdoutTask.GetAwaiter().GetResult()
    [void]$stderrTask.GetAwaiter().GetResult()
    $stdoutStream.Flush()
    $stderrStream.Flush()
    $endUtc = [datetime]::UtcNow
  } catch {
    if ($started -and $null -ne $process) {
      try {
        if (-not $process.HasExited) {
          Stop-TaskOwnedProcessTree -ProcessId $processId
        }
      } catch {
      }
    }
    throw
  } finally {
    $stopwatch.Stop()
    if ($null -ne $stdoutStream) { $stdoutStream.Dispose() }
    if ($null -ne $stderrStream) { $stderrStream.Dispose() }
    if ($null -ne $process) { $process.Dispose() }
  }

  $result = [NativeProcessResult]::new()
  $result.ExecutablePath = $executable
  $result.ArgumentCount = $Arguments.Count
  $result.ArgumentSha256 = Get-StringSha256 -Value ($Arguments -join ([char]0))
  $result.WorkingDirectorySha256 = Get-StringSha256 -Value $working.ToLowerInvariant()
  $result.ProcessId = $processId
  $result.StartUtc = $startUtc
  $result.EndUtc = $endUtc
  $result.ElapsedMilliseconds = $stopwatch.ElapsedMilliseconds
  $result.TimedOut = $timedOut
  $result.ExitCode = $exitCode
  $result.StdoutPath = $stdout
  $result.StderrPath = $stderr
  $result.StdoutBytes = (Get-Item -LiteralPath $stdout).Length
  $result.StdoutSha256 = Get-Sha256 $stdout
  $result.StderrBytes = (Get-Item -LiteralPath $stderr).Length
  $result.StderrSha256 = Get-Sha256 $stderr
  $result.PeakWorkingSetBytes = $peakWorkingSetBytes
  return $result
}

function Assert-NativeProcessSucceeded {
  param(
    [Parameter(Mandatory = $true)][NativeProcessResult]$Result,
    [Parameter(Mandatory = $true)][string]$FailureCode
  )
  if ($Result.TimedOut -or $Result.ExitCode -ne 0) {
    Stop-Benchmark $FailureCode
  }
}

function Write-NewUtf8Json {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)]$Value,
    [int]$Depth = 20
  )
  $body = ($Value | ConvertTo-Json -Depth $Depth) + "`n"
  Write-NewUtf8Text -Path $Path -Value $body
}

function Invoke-NativeProcessSelfTest {
  param([Parameter(Mandatory = $true)][string]$SelfTestRoot)

  $childPath = Join-Path $SelfTestRoot 'native-self-test-child.ps1'
  $childBody = @'
param(
  [int]$StdoutCount,
  [int]$StderrCount,
  [int]$ExitCode,
  [AllowEmptyString()][string]$ArgumentValue
)
$ErrorActionPreference = "Stop"
if ($ArgumentValue.Length -gt 0) { [Console]::Out.Write($ArgumentValue) }
if ($StdoutCount -gt 0) { [Console]::Out.Write(('O' * $StdoutCount)) }
if ($StderrCount -gt 0) { [Console]::Error.Write(('E' * $StderrCount)) }
exit $ExitCode
'@
  Write-NewUtf8Text -Path $childPath -Value $childBody

  $windowsPowerShell = Join-Path $PSHOME 'powershell.exe'
  $baseArguments = @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $childPath)
  function Invoke-SelfTestCase {
    param(
      [Parameter(Mandatory = $true)][string]$Name,
      [Parameter(Mandatory = $true)][int]$StdoutCount,
      [Parameter(Mandatory = $true)][int]$StderrCount,
      [Parameter(Mandatory = $true)][int]$ExitCode,
      [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ArgumentValue
    )
    $caseArguments = $baseArguments + @(
      '-StdoutCount', [string]$StdoutCount,
      '-StderrCount', [string]$StderrCount,
      '-ExitCode', [string]$ExitCode,
      '-ArgumentValue', $ArgumentValue
    )
    return @(Invoke-NativeProcess `
      -ExecutablePath $windowsPowerShell `
      -Arguments $caseArguments `
      -WorkingDirectory $SelfTestRoot `
      -StdoutPath (Join-Path $SelfTestRoot ($Name + '.stdout.txt')) `
      -StderrPath (Join-Path $SelfTestRoot ($Name + '.stderr.txt')) `
      -TimeoutMilliseconds 30000)
  }

  $zeroRecords = @(Invoke-SelfTestCase -Name 'stderr-zero' -StdoutCount 16 -StderrCount 17 -ExitCode 0 -ArgumentValue '')
  $nonzeroRecords = @(Invoke-SelfTestCase -Name 'nonzero' -StdoutCount 9 -StderrCount 11 -ExitCode 7 -ArgumentValue '')
  $largeRecords = @(Invoke-SelfTestCase -Name 'large-dual-stream' -StdoutCount 131072 -StderrCount 131072 -ExitCode 0 -ArgumentValue '')
  $argumentRecords = @(Invoke-SelfTestCase -Name 'argument-spaces' -StdoutCount 0 -StderrCount 0 -ExitCode 0 -ArgumentValue 'argument with spaces')
  $emptyRecords = @(Invoke-SelfTestCase -Name 'empty-stdout' -StdoutCount 0 -StderrCount 13 -ExitCode 0 -ArgumentValue '')

  $zero = $zeroRecords[0]
  $nonzero = $nonzeroRecords[0]
  $large = $largeRecords[0]
  $argument = $argumentRecords[0]
  $empty = $emptyRecords[0]
  $nonzeroClassified = $false
  try {
    Assert-NativeProcessSucceeded -Result $nonzero -FailureCode 'EXPECTED_SELF_TEST_NONZERO_FAILURE'
  } catch {
    $nonzeroClassified = $_.Exception.Message -like '*EXPECTED_SELF_TEST_NONZERO_FAILURE*'
  }

  $reuseRejected = $false
  try {
    [void](Invoke-NativeProcess `
      -ExecutablePath $windowsPowerShell `
      -Arguments ($baseArguments + @('-StdoutCount', '1', '-StderrCount', '1', '-ExitCode', '0', '-ArgumentValue', '')) `
      -WorkingDirectory $SelfTestRoot `
      -StdoutPath $zero.StdoutPath `
      -StderrPath $zero.StderrPath `
      -TimeoutMilliseconds 30000)
  } catch {
    $reuseRejected = $_.Exception.Message -like '*BLOCKED_GRAMMAR_P3_02_BENCHMARK_OUTPUT_ALREADY_EXISTS*'
  }

  $recordCollections = @($zeroRecords, $nonzeroRecords, $largeRecords, $argumentRecords, $emptyRecords)
  $countsExactlyOne = @($recordCollections | Where-Object { $_.Count -ne 1 }).Count -eq 0
  $stderrWithZeroExit = $zero.ExitCode -eq 0 -and -not $zero.TimedOut -and $zero.StderrBytes -eq 17
  $stdoutStderrSeparated = $zero.StdoutPath -ne $zero.StderrPath -and $zero.StdoutSha256 -ne $zero.StderrSha256 -and $empty.StdoutBytes -eq 0 -and $empty.StderrBytes -eq 13
  $largeCompleted = $large.ExitCode -eq 0 -and -not $large.TimedOut -and $large.StdoutBytes -eq 131072 -and $large.StderrBytes -eq 131072
  $argumentRoundtrip = $argument.ExitCode -eq 0 -and (Read-StrictUtf8Text -Path $argument.StdoutPath) -ceq 'argument with spaces'
  $exitCodeType = $zero.ExitCode.GetType().FullName
  $exitCodeIsSignedInteger = $exitCodeType -eq 'System.Int32' -and $nonzero.ExitCode -eq 7
  $passed = $stderrWithZeroExit -and $nonzeroClassified -and $stdoutStderrSeparated -and $largeCompleted -and $countsExactlyOne -and $exitCodeIsSignedInteger -and $reuseRejected -and $argumentRoundtrip

  $summary = [pscustomobject]@{
    result = if ($passed) { 'PASS' } else { 'FAIL' }
    mode = 'NATIVE_PROCESS_SELF_TEST'
    resultObjectRuntimeType = $zero.GetType().FullName
    resultObjectCountExactlyOne = $countsExactlyOne
    exitCodeRuntimeType = $exitCodeType
    exitCodeIsSignedInteger = $exitCodeIsSignedInteger
    cases = [pscustomobject]@{
      stderrWithZeroExit = $stderrWithZeroExit
      nonzeroExitClassifiedAsFailure = $nonzeroClassified
      stdoutStderrSeparated = $stdoutStderrSeparated
      largeDualStreamCompleted = $largeCompleted
      createNewAndReuseRejected = $reuseRejected
      argumentWithSpacesRoundtrip = $argumentRoundtrip
    }
    nativeResults = @($zero, $nonzero, $large, $argument, $empty)
  }
  $summary | ConvertTo-Json -Compress -Depth 10
  if (-not $passed) { exit 1 }
}

function Invoke-Utf8JsonSelfTest {
  param([Parameter(Mandatory = $true)][string]$SelfTestRoot)

  $noBomPath = Join-Path $SelfTestRoot 'mixed-language-no-bom.json'
  $bomPath = Join-Path $SelfTestRoot 'mixed-language-bom.json'
  $malformedPath = Join-Path $SelfTestRoot 'malformed-utf8.json'
  $embeddedBomPath = Join-Path $SelfTestRoot 'embedded-bom.json'
  $fixturePaths = @($noBomPath, $bomPath, $malformedPath, $embeddedBomPath)
  $cleanupSuccessful = $false

  $noBomMixedLanguageParses = $false
  $bomMixedLanguageParses = $false
  $bomNoBomSemanticEquality = $false
  $malformedSequenceFailsClosed = $false
  $embeddedBomFailsClosed = $false

  try {
    $mixedLanguage = ([string][char]0xD55C) + ([string][char]0xAE00) + ' English'
    $fixtureJson = ([ordered]@{
      label = $mixedLanguage
      count = 2
    } | ConvertTo-Json -Compress) + "`n"
    $fixtureBytes = $Utf8NoBom.GetBytes($fixtureJson)

    Write-NewUtf8Text -Path $noBomPath -Value $fixtureJson

    $bomBytes = New-Object byte[] ($fixtureBytes.Length + 3)
    $bomBytes[0] = 0xEF
    $bomBytes[1] = 0xBB
    $bomBytes[2] = 0xBF
    [System.Array]::Copy($fixtureBytes, 0, $bomBytes, 3, $fixtureBytes.Length)
    Write-NewBytes -Path $bomPath -Bytes $bomBytes

    $malformedBytes = [byte[]]@(0x7B, 0x22, 0x78, 0x22, 0x3A, 0x22, 0xC3, 0x28, 0x22, 0x7D)
    Write-NewBytes -Path $malformedPath -Bytes $malformedBytes

    $splitIndex = 5
    $embeddedBomBytes = New-Object byte[] ($fixtureBytes.Length + 3)
    [System.Array]::Copy($fixtureBytes, 0, $embeddedBomBytes, 0, $splitIndex)
    $embeddedBomBytes[$splitIndex] = 0xEF
    $embeddedBomBytes[$splitIndex + 1] = 0xBB
    $embeddedBomBytes[$splitIndex + 2] = 0xBF
    [System.Array]::Copy(
      $fixtureBytes,
      $splitIndex,
      $embeddedBomBytes,
      $splitIndex + 3,
      $fixtureBytes.Length - $splitIndex
    )
    Write-NewBytes -Path $embeddedBomPath -Bytes $embeddedBomBytes

    $noBomValue = Read-StrictUtf8Json -Path $noBomPath
    $bomValue = Read-StrictUtf8Json -Path $bomPath
    $noBomMixedLanguageParses = $noBomValue.label -ceq $mixedLanguage -and $noBomValue.count -eq 2
    $bomMixedLanguageParses = $bomValue.label -ceq $mixedLanguage -and $bomValue.count -eq 2
    $bomNoBomSemanticEquality =
      $noBomValue.label -ceq $bomValue.label -and
      $noBomValue.count -eq $bomValue.count

    try {
      [void](Read-StrictUtf8Json -Path $malformedPath)
    } catch {
      $malformedSequenceFailsClosed = $_.Exception.Message -like '*BLOCKED_GRAMMAR_P3_02_UTF8_DECODE_FAILED*'
    }

    try {
      [void](Read-StrictUtf8Json -Path $embeddedBomPath)
    } catch {
      $embeddedBomFailsClosed = $_.Exception.Message -like '*BLOCKED_GRAMMAR_P3_02_UTF8_BOM_POSITION_INVALID*'
    }
  } finally {
    foreach ($fixturePath in $fixturePaths) {
      if (Test-Path -LiteralPath $fixturePath) {
        Remove-Item -LiteralPath $fixturePath -Force
      }
    }
    $cleanupSuccessful = @($fixturePaths | Where-Object { Test-Path -LiteralPath $_ }).Count -eq 0
  }

  $runnerSource = Read-StrictUtf8Text -Path $PSCommandPath
  $strictByteReaderRequired =
    $runnerSource -match '\[System\.IO\.File\]::ReadAllBytes' -and
    $runnerSource -match 'UTF8Encoding\(\$false,\s*\$true\)'
  $predictionsUsesStrictReader =
    $runnerSource -match '\$Predictions\s*=\s*Read-StrictUtf8Json\s+-Path\s+\$PredictionsPath'
  $allGeneratedJsonUseStrictReader =
    $predictionsUsesStrictReader -and
    $runnerSource -match '\$CorpusManifest\s*=\s*Read-StrictUtf8Json\s+-Path\s+\$CorpusManifestPath' -and
    $runnerSource -match '\$Report\s*=\s*Read-StrictUtf8Json\s+-Path\s+\$EvaluatorReportPath' -and
    $runnerSource -match 'Read-StrictUtf8JsonLines\s+-Path\s+\$BuildStdout'

  $parserTokens = $null
  $parserErrors = $null
  [void][System.Management.Automation.Language.Parser]::ParseFile(
    $PSCommandPath,
    [ref]$parserTokens,
    [ref]$parserErrors
  )
  $parserErrorCount = @($parserErrors).Count
  $parserTokenCount = @($parserTokens).Count
  $ps51ParserGatePassed = $parserErrorCount -eq 0 -and $parserTokenCount -gt 0

  $passed =
    $strictByteReaderRequired -and
    $predictionsUsesStrictReader -and
    $allGeneratedJsonUseStrictReader -and
    $noBomMixedLanguageParses -and
    $bomMixedLanguageParses -and
    $bomNoBomSemanticEquality -and
    $malformedSequenceFailsClosed -and
    $embeddedBomFailsClosed -and
    $cleanupSuccessful -and
    $ps51ParserGatePassed

  [pscustomobject]@{
    result = if ($passed) { 'PASS' } else { 'FAIL' }
    mode = 'UTF8_JSON_SELF_TEST'
    strictByteReaderRequired = $strictByteReaderRequired
    contentFreeResult = $true
    cleanupSuccessful = $cleanupSuccessful
    ps51ParserGatePassed = $ps51ParserGatePassed
    parserErrorCount = $parserErrorCount
    parserTokenCount = $parserTokenCount
    boundaries = [pscustomobject]@{
      predictionsUsesStrictReader = $predictionsUsesStrictReader
      allGeneratedJsonUseStrictReader = $allGeneratedJsonUseStrictReader
    }
    cases = [pscustomobject]@{
      noBomMixedLanguageParses = $noBomMixedLanguageParses
      bomMixedLanguageParses = $bomMixedLanguageParses
      bomNoBomSemanticEquality = $bomNoBomSemanticEquality
      malformedSequenceFailsClosed = $malformedSequenceFailsClosed
      embeddedBomFailsClosed = $embeddedBomFailsClosed
    }
  } | ConvertTo-Json -Compress -Depth 10
  if (-not $passed) { exit 1 }
}

$RepositoryRoot = Get-CanonicalPath (Join-Path $PSScriptRoot "..")
$OutputRoot = Get-CanonicalPath $OutputRoot
$CargoTargetRoot = Get-CanonicalPath $CargoTargetRoot
$RepositoryPrefix = $RepositoryRoot + "\"

if ($OutputRoot.StartsWith($RepositoryPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
    $CargoTargetRoot.StartsWith($RepositoryPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_BENCHMARK_OUTPUT_INSIDE_REPOSITORY"
}
if (Test-Path -LiteralPath $OutputRoot) {
  Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_BENCHMARK_OUTPUT_ALREADY_EXISTS"
}
if (-not (Test-Path -LiteralPath (Split-Path -Parent $OutputRoot) -PathType Container)) {
  Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_BENCHMARK_PARENT_MISSING"
}
if (-not (Test-Path -LiteralPath $CargoTargetRoot -PathType Container)) {
  New-Item -ItemType Directory -Path $CargoTargetRoot | Out-Null
}
New-Item -ItemType Directory -Path $OutputRoot | Out-Null

if ($NativeProcessSelfTest) {
  Invoke-NativeProcessSelfTest -SelfTestRoot $OutputRoot
  return
}
if ($Utf8JsonSelfTest) {
  Invoke-Utf8JsonSelfTest -SelfTestRoot $OutputRoot
  return
}

$ManifestPath = Join-Path $RepositoryRoot "src-tauri\Cargo.toml"
$CorpusManifestPath = Join-Path $RepositoryRoot "benchmarks\instant-selection\corpus-manifest.v1.json"
$ContractPath = Join-Path $RepositoryRoot "benchmarks\instant-selection\baseline-contract.v1.json"
$EvaluatorPath = Join-Path $RepositoryRoot "scripts\evaluate-instant-selection-benchmark.mjs"
$BuildStdout = Join-Path $OutputRoot "compile.stdout.jsonl"
$BuildStderr = Join-Path $OutputRoot "compile.stderr.txt"

foreach ($RequiredPath in @($ManifestPath, $CorpusManifestPath, $ContractPath, $EvaluatorPath)) {
  if (-not (Test-Path -LiteralPath $RequiredPath -PathType Leaf)) {
    Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_BENCHMARK_INPUT_MISSING"
  }
}

$CorpusManifest = Read-StrictUtf8Json -Path $CorpusManifestPath
$CorpusPath = Join-Path $OutputRoot "immutable-corpus.synthetic.v1.jsonl"
$CorpusExportStderr = Join-Path $OutputRoot 'corpus-export.stderr.txt'
$GitCommand = Get-Command git.exe -ErrorAction Stop
$CorpusExportResult = Invoke-NativeProcess `
  -ExecutablePath $GitCommand.Source `
  -Arguments @('-C', $RepositoryRoot, 'cat-file', 'blob', ('HEAD:' + [string]$CorpusManifest.corpusPath)) `
  -WorkingDirectory $RepositoryRoot `
  -StdoutPath $CorpusPath `
  -StderrPath $CorpusExportStderr `
  -TimeoutMilliseconds 120000
Assert-NativeProcessSucceeded -Result $CorpusExportResult -FailureCode 'BLOCKED_GRAMMAR_P3_02_IMMUTABLE_CORPUS_EXPORT_FAILED'
if ((Get-Item -LiteralPath $CorpusPath).Length -ne [long]$CorpusManifest.corpusBytes -or
    (Get-Sha256 $CorpusPath) -ne [string]$CorpusManifest.corpusSha256) {
  Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_IMMUTABLE_CORPUS_AUTHORITY_MISMATCH"
}

$PreviousCargoTarget = $env:CARGO_TARGET_DIR
try {
  $env:CARGO_TARGET_DIR = $CargoTargetRoot
  $CargoCommand = Get-Command cargo.exe -ErrorAction Stop
  $BuildResult = Invoke-NativeProcess `
    -ExecutablePath $CargoCommand.Source `
    -Arguments @('test', '--manifest-path', $ManifestPath, '--test', 'instant_selection', '--no-run', '--locked', '--offline', '--message-format=json') `
    -WorkingDirectory $RepositoryRoot `
    -StdoutPath $BuildStdout `
    -StderrPath $BuildStderr `
    -TimeoutMilliseconds 600000
} finally {
  if ($null -eq $PreviousCargoTarget) {
    Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
  } else {
    $env:CARGO_TARGET_DIR = $PreviousCargoTarget
  }
}
Assert-NativeProcessSucceeded -Result $BuildResult -FailureCode 'BLOCKED_GRAMMAR_P3_02_BENCHMARK_COMPILE_FAILED'

$BenchmarkExecutable = $null
$BuildMessages = @(Read-StrictUtf8JsonLines -Path $BuildStdout)
foreach ($Message in $BuildMessages) {
  if ($Message.reason -eq "compiler-artifact" -and
      $Message.target.name -eq "instant_selection" -and
      @($Message.target.kind) -contains "test" -and
      -not [string]::IsNullOrWhiteSpace([string]$Message.executable)) {
    $BenchmarkExecutable = [string]$Message.executable
  }
}
if ([string]::IsNullOrWhiteSpace($BenchmarkExecutable) -or
    -not (Test-Path -LiteralPath $BenchmarkExecutable -PathType Leaf)) {
  Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_BENCHMARK_EXECUTABLE_NOT_RESOLVED"
}
$BenchmarkExecutable = Get-CanonicalPath $BenchmarkExecutable
$ExecutableBytes = (Get-Item -LiteralPath $BenchmarkExecutable).Length
$ExecutableSha256 = Get-Sha256 $BenchmarkExecutable

$RunRecords = @()
$SemanticHashes = @()
for ($Run = 1; $Run -le $RunCount; $Run += 1) {
  $RunRoot = Join-Path $OutputRoot ("run-{0}" -f $Run)
  New-Item -ItemType Directory -Path $RunRoot | Out-Null
  $PredictionsPath = Join-Path $RunRoot "predictions.json"
  $SemanticPath = Join-Path $RunRoot "semantic-output.json"
  $EvaluatorReportPath = Join-Path $RunRoot "evaluator-report.json"
  $WriterStdout = Join-Path $RunRoot "writer.stdout.txt"
  $WriterStderr = Join-Path $RunRoot "writer.stderr.txt"
  $EvaluatorStdout = Join-Path $RunRoot "evaluator.stdout.txt"
  $EvaluatorStderr = Join-Path $RunRoot "evaluator.stderr.txt"

  $env:P3_02_CORPUS_PATH = $CorpusPath
  $env:P3_02_PREDICTIONS_PATH = $PredictionsPath
  $env:P3_02_SEMANTIC_PATH = $SemanticPath
  $env:P3_02_ARTIFACT_BYTES = [string]$ExecutableBytes
  try {
    $WriterResult = Invoke-NativeProcess `
      -ExecutablePath $BenchmarkExecutable `
      -Arguments @('product_benchmark_writer_uses_the_same_rust_analysis_api', '--exact', '--ignored', '--nocapture') `
      -WorkingDirectory $RepositoryRoot `
      -StdoutPath $WriterStdout `
      -StderrPath $WriterStderr `
      -TimeoutMilliseconds 120000
  } finally {
    Remove-Item Env:P3_02_CORPUS_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:P3_02_PREDICTIONS_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:P3_02_SEMANTIC_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:P3_02_ARTIFACT_BYTES -ErrorAction SilentlyContinue
  }
  Assert-NativeProcessSucceeded -Result $WriterResult -FailureCode 'BLOCKED_GRAMMAR_P3_02_PRODUCT_BENCHMARK_WRITER_FAILED'
  if (-not (Test-Path -LiteralPath $PredictionsPath -PathType Leaf) -or
      -not (Test-Path -LiteralPath $SemanticPath -PathType Leaf)) {
    Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_PRODUCT_BENCHMARK_WRITER_FAILED"
  }

  $Predictions = Read-StrictUtf8Json -Path $PredictionsPath
  $Predictions.engine.peakRssMiB = [Math]::Round($WriterResult.PeakWorkingSetBytes / 1MB, 6)
  $Predictions.engine.artifactBytes = [long]$ExecutableBytes
  [System.IO.File]::WriteAllText(
    $PredictionsPath,
    (($Predictions | ConvertTo-Json -Depth 20) + "`n"),
    $Utf8NoBom
  )

  $NodeCommand = Get-Command node.exe -ErrorAction Stop
  $EvaluatorResult = Invoke-NativeProcess `
    -ExecutablePath $NodeCommand.Source `
    -Arguments @($EvaluatorPath, '--contract', $ContractPath, '--corpus', $CorpusPath, '--predictions', $PredictionsPath, '--out', $EvaluatorReportPath, '--print-summary') `
    -WorkingDirectory $RepositoryRoot `
    -StdoutPath $EvaluatorStdout `
    -StderrPath $EvaluatorStderr `
    -TimeoutMilliseconds 120000
  Assert-NativeProcessSucceeded -Result $EvaluatorResult -FailureCode 'BLOCKED_GRAMMAR_P3_02_PRODUCT_BENCHMARK_EVALUATOR_FAILED'
  if (-not (Test-Path -LiteralPath $EvaluatorReportPath -PathType Leaf)) {
    Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_PRODUCT_BENCHMARK_EVALUATOR_FAILED"
  }

  $Report = Read-StrictUtf8Json -Path $EvaluatorReportPath
  if ($Report.engine.id -ne $ExpectedEngineId -or
      $Report.engine.version -ne $ExpectedEngineVersion -or
      $Report.metrics.caseCount -ne $ExpectedCaseCount) {
    Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_PRODUCT_BENCHMARK_IDENTITY_MISMATCH"
  }
  if (-not $Report.metrics.safetyGatePassed) {
    Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_ENGINE_SAFETY_CONTRACT_FAILED"
  }
  if (-not $Report.metrics.qualityGatePassed) {
    Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_PRODUCT_ENGINE_QUALITY_THRESHOLD_FAILED"
  }
  if (-not $Report.metrics.latencyGatePassed) {
    Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_PRODUCT_ENGINE_LATENCY_THRESHOLD_FAILED"
  }
  if (-not $Report.metrics.resourceGatePassed) {
    Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_PRODUCT_ENGINE_RESOURCE_THRESHOLD_FAILED"
  }

  $SemanticSha256 = Get-Sha256 $SemanticPath
  $SemanticHashes += $SemanticSha256
  $RunRecords += [pscustomobject]@{
    run = $Run
    writerProcess = $WriterResult
    evaluatorProcess = $EvaluatorResult
    residualProcessCount = 0
    predictionsBytes = (Get-Item -LiteralPath $PredictionsPath).Length
    predictionsSha256 = Get-Sha256 $PredictionsPath
    semanticOutputBytes = (Get-Item -LiteralPath $SemanticPath).Length
    semanticOutputSha256 = $SemanticSha256
    evaluatorReportBytes = (Get-Item -LiteralPath $EvaluatorReportPath).Length
    evaluatorReportSha256 = Get-Sha256 $EvaluatorReportPath
    metrics = $Report.metrics
  }
}

if (@($SemanticHashes | Select-Object -Unique).Count -ne 1) {
  Stop-Benchmark "BLOCKED_GRAMMAR_P3_02_NONDETERMINISTIC_ENGINE_OUTPUT"
}

$Summary = [pscustomobject]@{
  schemaVersion = 1
  taskId = $ExpectedTaskId
  result = "PASS"
  productEngineBenchmark = "PRODUCT_ENGINE_BENCHMARK_EXECUTED"
  engineId = $ExpectedEngineId
  engineVersion = $ExpectedEngineVersion
  runCount = $RunCount
  semanticOutputSha256 = $SemanticHashes[0]
  deterministic = $true
  benchmarkExecutableBytes = $ExecutableBytes
  benchmarkExecutableSha256 = $ExecutableSha256
  measurementBoundary = "Rust InstantSelectionEngine construction plus analyze only; Cargo compilation and process startup excluded from engine latency; process RSS monitored externally."
  runs = $RunRecords
}
$SummaryPath = Join-Path $OutputRoot "product-benchmark-summary.json"
Write-NewUtf8Json -Path $SummaryPath -Value $Summary -Depth 30
[pscustomobject]@{
  result = "PASS"
  token = "PRODUCT_ENGINE_BENCHMARK_EXECUTED"
  runCount = $RunCount
  semanticOutputSha256 = $SemanticHashes[0]
  summarySha256 = Get-Sha256 $SummaryPath
} | ConvertTo-Json -Compress
