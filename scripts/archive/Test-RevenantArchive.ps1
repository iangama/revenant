[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$seen = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $root 'SHA256SUMS')) {
    if ($line -notmatch '^([0-9a-f]{64})  ([^\\]+)$') { throw 'Invalid checksum manifest' }
    $expected = $Matches[1]
    $relative = $Matches[2]
    if ($relative.StartsWith('/') -or $relative.Contains(':') -or $relative.Split('/') -contains '..' -or $seen.ContainsKey($relative)) {
        throw 'Invalid or duplicate payload path'
    }
    $path = Join-Path $root $relative
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "Checksum mismatch: $relative" }
    $seen[$relative] = $true
}
$files = @(Get-ChildItem -LiteralPath $root -File -Recurse | Where-Object { $_.FullName -ne (Join-Path $root 'SHA256SUMS') })
if ($seen.Count -eq 0 -or $files.Count -ne $seen.Count) { throw 'Empty manifest or extra files' }
Write-Host "Archive verified: $($seen.Count) payloads"
