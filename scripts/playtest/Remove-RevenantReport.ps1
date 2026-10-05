[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'High')]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^PT-[A-Z0-9]{4}$')]
    [string]$ParticipantCode,

    [ValidatePattern('^[0-9a-f]{32}$')]
    [string]$ReportId
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$reportDirectory = Join-Path $env:APPDATA 'Godot\app_userdata\Revenant\playtest'
if (-not (Test-Path -LiteralPath $reportDirectory -PathType Container)) {
    Write-Host 'No Revenant playtest report directory exists.'
    exit 0
}

$matchedFiles = @()
foreach ($file in Get-ChildItem -LiteralPath $reportDirectory -File) {
    if ($file.Name -notmatch '^m24-([0-9a-f]{32})\.json(\.tmp)?$') {
        continue
    }
    try {
        $report = Get-Content -LiteralPath $file.FullName -Raw | ConvertFrom-Json
    }
    catch {
        Write-Warning "Skipped unreadable report: $($file.Name)"
        continue
    }
    if ($report.participant_code -ne $ParticipantCode) {
        continue
    }
    if ($ReportId -and $report.report_id -ne $ReportId) {
        continue
    }
    $matchedFiles += $file
}

foreach ($file in $matchedFiles) {
    if ($PSCmdlet.ShouldProcess($file.FullName, 'Permanently delete local playtest report')) {
        Remove-Item -LiteralPath $file.FullName -Force
        Write-Host "Deleted report: $($file.Name)"
    }
}

if ($matchedFiles.Count -eq 0) {
    Write-Host 'No matching report was found.'
}
