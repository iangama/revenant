[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$packageRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifest = Join-Path $packageRoot 'SHA256SUMS'
if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
    throw "Package checksum manifest is missing: $manifest"
}

$checked = 0
foreach ($line in Get-Content -LiteralPath $manifest) {
    if ($line -notmatch '^([0-9a-f]{64})  \./(.+)$') {
        throw "Malformed checksum line: $line"
    }
    $expected = $Matches[1]
    $relative = $Matches[2].Replace('/', [IO.Path]::DirectorySeparatorChar)
    $path = Join-Path $packageRoot $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Package file is missing: $relative"
    }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        throw "Package checksum mismatch: $relative"
    }
    $checked += 1
}

if ($checked -lt 10) {
    throw "Package manifest is unexpectedly small: $checked files"
}
Write-Host "Package checksums verified: $checked files"

