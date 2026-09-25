param(
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][ValidatePattern('^[a-p]{32}$')][string]$ExtensionId,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [switch]$Register
)
$ErrorActionPreference = 'Stop'
# A fresh task-owned directory and unique host name prevent overwriting any existing registration.
$source = (Resolve-Path -LiteralPath $Executable).Path
$destination = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $destination) { throw 'OutputDirectory must not exist.' }
$name = 'org.grammar.personal.t' + [Guid]::NewGuid().ToString('N')
$key = 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\' + $name
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
$receipt = @{name=$name; manifest=$manifestPath; executable_sha256=(Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash; registered=[bool]$Register}
[IO.File]::WriteAllText((Join-Path $destination 'registration.json'), ($receipt | ConvertTo-Json), $encoding)
if ($Register) {
    # Call only for a task-owned browser/profile and after test coordinator admission.
    New-Item -Path $key | Out-Null
    Set-Item -LiteralPath $key -Value $manifestPath
}
$receipt | ConvertTo-Json
