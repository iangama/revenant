[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'Test-RevenantArchive.ps1')
$game = Join-Path $PSScriptRoot '..\client\windows\Revenant.exe'
$variables = @{
    REVENANT_GAME_HOST = '127.0.0.1'
    REVENANT_GAME_PORT = if ($env:REVENANT_GAME_PORT) { $env:REVENANT_GAME_PORT } else { '7000' }
    REVENANT_PLAYTEST_MODE = '0'
}
$previous = @{}
try {
    foreach ($name in $variables.Keys) {
        $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
        [Environment]::SetEnvironmentVariable($name, $variables[$name], 'Process')
    }
    $process = Start-Process -FilePath $game -WorkingDirectory (Split-Path $game) -PassThru -Wait
    exit $process.ExitCode
}
finally {
    foreach ($name in $variables.Keys) {
        [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process')
    }
}
