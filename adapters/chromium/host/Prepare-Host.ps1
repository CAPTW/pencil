param(
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][ValidatePattern('^[a-p]{32}$')][string]$ExtensionId,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [switch]$Register,
    # Chrome's per-user native messaging key. Tests pass a task-owned key instead.
    [string]$HostsKey = 'HKCU:\Software\Google\Chrome\NativeMessagingHosts'
)
$ErrorActionPreference = 'Stop'
# A fresh task-owned directory and unique host name prevent overwriting any existing registration.
$source = (Resolve-Path -LiteralPath $Executable).Path
$destination = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $destination) { throw 'OutputDirectory must not exist.' }
$name = 'org.grammar.personal.t' + [Guid]::NewGuid().ToString('N')
$hosts = $HostsKey.TrimEnd('\')
$key = $hosts + '\' + $name
if (Test-Path -LiteralPath $key) { throw 'Registration already exists.' }
New-Item -ItemType Directory -Path $destination | Out-Null
$binary = Join-Path $destination 'grammar-chromium-host.exe'
Copy-Item -LiteralPath $source -Destination $binary
$origin = 'chrome-extension://' + $ExtensionId + '/'
$encoding = [Text.UTF8Encoding]::new($false)
$installation = [Guid]::NewGuid().ToString('N')
[IO.File]::WriteAllText((Join-Path $destination 'grammar-chromium-host.origin.json'), (@{origin=$origin; installation_id=$installation} | ConvertTo-Json), $encoding)
$slots = @(1..4 | ForEach-Object { @{generation=0; token=''; state='FREE'} })
[IO.File]::WriteAllText((Join-Path $destination 'grammar-cleanup.json'), (@{version=1; installation=$installation; slots=$slots} | ConvertTo-Json -Depth 4), $encoding)
[IO.File]::WriteAllBytes((Join-Path $destination 'grammar-cleanup.lock'), [byte[]]@())
$manifestPath = Join-Path $destination 'host.json'
$manifest = @{name=$name; description='Grammar task-owned local Instant host'; path=$binary; type='stdio'; allowed_origins=@($origin)}
[IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json), $encoding)
# The receipt says registered only after the registry key exists and points at host.json.
$receipt = [ordered]@{name=$name; manifest=$manifestPath; executable_sha256=(Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash; registered=$false; registry_key=$key}
$receiptPath = Join-Path $destination 'registration.json'
[IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json), $encoding)
if ($Register) {
    # Call only for a task-owned browser/profile and after test coordinator admission.
    # Chrome creates no NativeMessagingHosts key until some host is registered, so
    # create each missing level. -Force is never used: on an existing registry key
    # it would replace the key and every other host registered under it.
    $current = ($hosts -split '\\')[0]
    foreach ($part in @($hosts -split '\\' | Select-Object -Skip 1)) {
        $current = $current + '\' + $part
        if (-not (Test-Path -LiteralPath $current)) { New-Item -Path $current | Out-Null }
    }
    New-Item -Path $key | Out-Null
    try { Set-Item -LiteralPath $key -Value $manifestPath }
    catch { Remove-Item -LiteralPath $key -ErrorAction SilentlyContinue; throw }
    $receipt.registered = $true
    [IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json), $encoding)
}
$receipt | ConvertTo-Json
