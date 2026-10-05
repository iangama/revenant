[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PackageRoot,

    [Parameter(Mandatory = $true)]
    [string]$RepositoryRoot,

    [Parameter(Mandatory = $true)]
    [string]$EvidenceRoot,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9-]{1,16}$')]
    [string]$RunToken
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$packageRootPath = (Resolve-Path -LiteralPath $PackageRoot).Path
$repositoryRootPath = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$game = Join-Path $packageRootPath 'participant\Revenant.exe'
$verifier = Join-Path $packageRootPath 'operator\Test-RevenantPackage.ps1'
$deletionTool = Join-Path $packageRootPath 'operator\Remove-RevenantReport.ps1'
if (-not (Test-Path -LiteralPath $game -PathType Leaf)) {
    throw "Frozen Windows executable is missing: $game"
}
if (Test-Path -LiteralPath $EvidenceRoot) {
    throw "Windows solo evidence target already exists: $EvidenceRoot"
}
$evidenceRootPath = (New-Item -ItemType Directory -Path $EvidenceRoot).FullName

function Invoke-RevenantProcess {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CaseId,

        [Parameter(Mandatory = $true)]
        [hashtable]$Environment,

        [int]$TimeoutMilliseconds = 60000
    )

    $casePath = Join-Path $evidenceRootPath $CaseId
    $caseRoot = if (Test-Path -LiteralPath $casePath) {
        (Resolve-Path -LiteralPath $casePath).Path
    } else {
        (New-Item -ItemType Directory -Path $casePath).FullName
    }
    $appData = (New-Item -ItemType Directory -Path (Join-Path $caseRoot 'AppData\Roaming') -Force).FullName
    $localAppData = (New-Item -ItemType Directory -Path (Join-Path $caseRoot 'AppData\Local') -Force).FullName
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $game
    $start.Arguments = '--headless'
    $start.WorkingDirectory = Split-Path -Parent $game
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.EnvironmentVariables['APPDATA'] = $appData
    $start.EnvironmentVariables['LOCALAPPDATA'] = $localAppData
    foreach ($entry in $Environment.GetEnumerator()) {
        $start.EnvironmentVariables[[string]$entry.Key] = [string]$entry.Value
    }
    $process = [Diagnostics.Process]::Start($start)
    if (-not $process.WaitForExit($TimeoutMilliseconds)) {
        $process.Kill()
        $process.WaitForExit()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        Set-Content -LiteralPath (Join-Path $caseRoot 'stdout.log') -Value $stdout -Encoding UTF8
        Set-Content -LiteralPath (Join-Path $caseRoot 'stderr.log') -Value $stderr -Encoding UTF8
        throw "$CaseId exceeded $TimeoutMilliseconds ms"
    }
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    Set-Content -LiteralPath (Join-Path $caseRoot 'stdout.log') -Value $stdout -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $caseRoot 'stderr.log') -Value $stderr -Encoding UTF8
    if ($process.ExitCode -ne 0) {
        throw "$CaseId exited with code $($process.ExitCode): $stderr"
    }
    return [pscustomobject]@{
        CaseRoot = $caseRoot
        AppData = $appData
        LocalAppData = $localAppData
        Stdout = $stdout
        Stderr = $stderr
    }
}

function Invoke-DisplayCase {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CaseId,

        [string]$Fixture,

        [Parameter(Mandatory = $true)]
        [string]$ExpectedState
    )

    $casePath = Join-Path $evidenceRootPath $CaseId
    $settingsDirectory = Join-Path $casePath 'AppData\Roaming\Godot\app_userdata\Revenant'
    New-Item -ItemType Directory -Path $settingsDirectory -Force | Out-Null
    if ($Fixture) {
        Copy-Item -LiteralPath (Join-Path $repositoryRootPath "tests\fixtures\$Fixture") `
            -Destination (Join-Path $settingsDirectory 'revenant-settings.cfg')
    }
    $result = Invoke-RevenantProcess -CaseId $CaseId -Environment @{
        REVENANT_VALIDATE_SLICE = '1'
    }
    if ($result.Stdout -notmatch 'M24 display and first-contact validated') {
        throw "$CaseId did not emit the M24 display marker"
    }
    if ($result.Stdout -notmatch [regex]::Escape($ExpectedState)) {
        throw "$CaseId did not apply expected settings: $ExpectedState"
    }
    if ($result.Stdout -notmatch 'M22 audio foundation validated') {
        throw "$CaseId did not validate packaged audio"
    }
    Write-Host "$CaseId passed: $ExpectedState"
}

& $verifier

Invoke-DisplayCase -CaseId '01-clean-full-audible' -Fixture '' `
    -ExpectedState 'guidance=Full muted=false reduced_flash=false'
Invoke-DisplayCase -CaseId '02-compact-muted' -Fixture 'm24-settings-compact-muted.cfg' `
    -ExpectedState 'guidance=Compact muted=true reduced_flash=false'
Invoke-DisplayCase -CaseId '03-off-reduced' -Fixture 'm24-settings-off-reduced.cfg' `
    -ExpectedState 'guidance=Off muted=false reduced_flash=true'
Invoke-DisplayCase -CaseId '04-full-combined' -Fixture 'm24-settings-full-combined.cfg' `
    -ExpectedState 'guidance=Full muted=true reduced_flash=true'

$keyboard = Invoke-RevenantProcess -CaseId '05-keyboard-network-flow' -Environment @{
    REVENANT_GAME_HOST = '127.0.0.1'
    REVENANT_GAME_PORT = '7000'
    REVENANT_GAME_USERNAME = "m24ds-$RunToken-win"
    REVENANT_VALIDATE_KEYBOARD_FLOW = '1'
}
if ($keyboard.Stdout -notmatch 'M24 keyboard-only flow validated') {
    throw 'Packaged Windows keyboard flow did not complete'
}

$abruptRoot = (New-Item -ItemType Directory -Path (Join-Path $evidenceRootPath '06-abrupt-close')).FullName
$abruptAppData = (New-Item -ItemType Directory -Path (Join-Path $abruptRoot 'AppData\Roaming') -Force).FullName
$abruptLocalAppData = (New-Item -ItemType Directory -Path (Join-Path $abruptRoot 'AppData\Local') -Force).FullName
$abruptStart = [Diagnostics.ProcessStartInfo]::new()
$abruptStart.FileName = $game
$abruptStart.Arguments = '--headless'
$abruptStart.WorkingDirectory = Split-Path -Parent $game
$abruptStart.UseShellExecute = $false
$abruptStart.CreateNoWindow = $true
$abruptStart.RedirectStandardOutput = $true
$abruptStart.RedirectStandardError = $true
$abruptStart.EnvironmentVariables['APPDATA'] = $abruptAppData
$abruptStart.EnvironmentVariables['LOCALAPPDATA'] = $abruptLocalAppData
$abruptStart.EnvironmentVariables['REVENANT_PLAYTEST_MODE'] = '1'
$abruptStart.EnvironmentVariables['REVENANT_PLAYTEST_PARTICIPANT'] = 'PT-S009'
$abruptStart.EnvironmentVariables['REVENANT_PLAYTEST_BUILD_ID'] = "m24-solo-$RunToken"
$abruptStart.EnvironmentVariables['REVENANT_PLAYTEST_OBSERVATION_CONSENT'] = '1'
$abruptStart.EnvironmentVariables['REVENANT_PLAYTEST_RETENTION_CONSENT'] = '1'
$abruptProcess = [Diagnostics.Process]::Start($abruptStart)
$reportDirectory = Join-Path $abruptAppData 'Godot\app_userdata\Revenant\playtest'
$report = $null
for ($attempt = 0; $attempt -lt 100; $attempt += 1) {
    $report = Get-ChildItem -LiteralPath $reportDirectory -Filter 'm24-*.json' -File -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($null -ne $report) {
        break
    }
    Start-Sleep -Milliseconds 50
}
if ($null -eq $report) {
    $abruptProcess.Kill()
    $abruptProcess.WaitForExit()
    throw 'Abrupt-close fixture did not create its bounded local report'
}
$abruptProcess.Kill()
$abruptProcess.WaitForExit()
$abruptStdout = $abruptProcess.StandardOutput.ReadToEnd()
$abruptStderr = $abruptProcess.StandardError.ReadToEnd()
Set-Content -LiteralPath (Join-Path $abruptRoot 'stdout.log') -Value $abruptStdout -Encoding UTF8
Set-Content -LiteralPath (Join-Path $abruptRoot 'stderr.log') -Value $abruptStderr -Encoding UTF8
$reportContent = Get-Content -LiteralPath $report.FullName -Raw | ConvertFrom-Json
if ($reportContent.participant_code -ne 'PT-S009' -or $reportContent.terminal_outcome -ne 'running') {
    throw 'Abrupt-close report did not remain an explicitly incomplete synthetic record'
}
if ($report.Length -gt 16384) {
    throw "Abrupt-close report exceeded 16 KiB: $($report.Length)"
}
$reportBytes = $report.Length
$reportId = [IO.Path]::GetFileNameWithoutExtension($report.Name).Substring(4)
$previousAppData = $env:APPDATA
try {
    $env:APPDATA = $abruptAppData
    & $deletionTool -ParticipantCode 'PT-S009' -ReportId $reportId -Confirm:$false
} finally {
    $env:APPDATA = $previousAppData
}
if (Test-Path -LiteralPath $report.FullName) {
    throw 'Selective report deletion did not remove the exact synthetic report'
}

& $verifier

$summary = @(
    "run_token=$RunToken"
    'display_cases=4'
    'keyboard_network_flow=completed'
    'abrupt_terminal_outcome=running'
    "abrupt_report_bytes=$reportBytes"
    'selective_report_deletion=passed'
    'package_internal_hashes=verified_before_and_after'
)
Set-Content -LiteralPath (Join-Path $evidenceRootPath 'summary.txt') -Value $summary -Encoding ASCII
Write-Host "M24 Windows solo matrix passed: $evidenceRootPath"
