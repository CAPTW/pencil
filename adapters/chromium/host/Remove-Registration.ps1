param([Parameter(Mandatory)][string]$PackageDirectory)
$ErrorActionPreference = 'Stop'
$directory = (Resolve-Path -LiteralPath $PackageDirectory).Path
$receipt = Get-Content -LiteralPath (Join-Path $directory 'registration.json') -Raw | ConvertFrom-Json
if ($receipt.name -notmatch '^org\.grammar\.personal\.t[a-f0-9]{32}$') { throw 'Not a task-owned host name.' }
$expected = Join-Path $directory 'host.json'
if ($receipt.manifest -ne $expected) { throw 'Manifest path mismatch.' }
# Older receipts have no registry_key: they always used Chrome's per-user key.
$key = if ($receipt.PSObject.Properties['registry_key']) { $receipt.registry_key } else { 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\' + $receipt.name }
if (-not $key.EndsWith('\' + $receipt.name)) { throw 'Registry key does not name this host.' }
if (Test-Path -LiteralPath $key) {
    if ((Get-Item -LiteralPath $key).GetValue('') -ne $expected) { throw 'Registration changed; leave untouched.' }
    Remove-Item -LiteralPath $key
}
# Keep package/evidence files. No recursive deletion or broad registry cleanup.
