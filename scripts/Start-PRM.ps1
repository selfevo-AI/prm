param([int]$Port = 4317, [switch]$Build, [string]$DataDirectory = '', [switch]$ModelWorker)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
if (!$env:PRM_DATABASE_URL) { & (Join-Path $PSScriptRoot 'Start-Postgres.ps1') }

$cargoDirectory = Join-Path $env:USERPROFILE '.cargo\bin'
$gccDirectory = Join-Path $projectRoot '.tools\w64devkit\bin'
$env:Path = "$cargoDirectory;$gccDirectory;$env:Path"
if ($Build -or !(Test-Path -LiteralPath (Join-Path $projectRoot 'target\debug\prm.exe')) -or !(Test-Path -LiteralPath (Join-Path $projectRoot 'dist\index.html'))) {
    if (!(Test-Path -LiteralPath (Join-Path $projectRoot 'node_modules'))) {
        & npm.cmd ci
        if ($LASTEXITCODE -ne 0) { throw 'Frontend dependency installation failed.' }
    }
    & npm.cmd run build
    if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed.' }
    & cargo.exe build --locked --bin prm
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed.' }
}
while (Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue) { $Port += 1 }
if ($Port -gt 65535) { throw 'No available port.' }
$env:PRM_PORT = [string]$Port
if ($DataDirectory) { $env:PRM_DATA_DIR = $DataDirectory } else { $env:PRM_DATA_DIR = Join-Path $projectRoot 'data' }
New-Item -ItemType Directory -Force -Path $env:PRM_DATA_DIR | Out-Null
# The model worker and the server share an executor token; generate one for this run if none is set.
if ($ModelWorker -and !$env:PRM_AGENT_TOKEN) {
    $env:PRM_AGENT_TOKEN = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
}
$process = Start-Process -FilePath (Join-Path $projectRoot 'target\debug\prm.exe') -WorkingDirectory $projectRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $env:PRM_DATA_DIR 'server.log') -RedirectStandardError (Join-Path $env:PRM_DATA_DIR 'server-error.log')
Start-Sleep -Milliseconds 700
if ($process.HasExited) { throw 'PRM did not start. See server-error.log in the data directory.' }
Write-Output "PRM: http://127.0.0.1:$Port (PID $($process.Id))"
if ($ModelWorker) {
    $env:PRM_URL = "http://127.0.0.1:$Port"
    $worker = Start-Process -FilePath 'node' -ArgumentList @((Join-Path $projectRoot 'scripts\prm-model-worker.mjs'), '--interval', '5') -WorkingDirectory $projectRoot -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $env:PRM_DATA_DIR 'model-worker.log') -RedirectStandardError (Join-Path $env:PRM_DATA_DIR 'model-worker-error.log')
    Write-Output "Model worker: PID $($worker.Id) (log: model-worker.log in the data directory)"
}
