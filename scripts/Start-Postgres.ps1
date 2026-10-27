param([int]$Port = 55432)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$tools = Join-Path $root '.tools'
$bin = Join-Path $tools 'pgsql\bin'
$cluster = Join-Path $tools 'pg-data'
$config = Join-Path $tools 'postgresql.json'
if (!(Test-Path -LiteralPath (Join-Path $bin 'initdb.exe'))) { throw 'Install PostgreSQL binaries in .tools/pgsql or use your own server and PRM_DATABASE_URL.' }
function Protect-File([string]$Path) {
    & icacls.exe $Path /inheritance:r /grant:r "$($env:USERDOMAIN)\$($env:USERNAME):(F)" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Unable to protect database credentials.' }
}
if (!(Test-Path -LiteralPath $config)) {
    if (Test-Path -LiteralPath $cluster) { throw 'Existing cluster without configuration; restore .tools/postgresql.json before continuing.' }
    $adminPassword = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
    $appPassword = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
    $passwordFile = Join-Path $tools 'pg-init-password'
    [IO.File]::WriteAllText($passwordFile, $adminPassword)
    Protect-File $passwordFile
    try {
        & (Join-Path $bin 'initdb.exe') -D $cluster -U postgres --encoding=UTF8 --locale=C --auth=scram-sha-256 --pwfile=$passwordFile
        if ($LASTEXITCODE -ne 0) { throw 'PostgreSQL initialization failed.' }
    } finally { Remove-Item -LiteralPath $passwordFile -Force }
    & icacls.exe $cluster /inheritance:r /grant:r "$($env:USERDOMAIN)\$($env:USERNAME):(OI)(CI)(F)" | Out-Null
    @{ port=$Port; admin_password=$adminPassword; password=$appPassword; url="postgresql://prm:$appPassword@127.0.0.1:$Port/prm"; provisioned=$false } | ConvertTo-Json | Set-Content -LiteralPath $config -Encoding utf8
    Protect-File $config
}
$settings = Get-Content -LiteralPath $config -Raw | ConvertFrom-Json
$Port = $settings.port
& (Join-Path $bin 'pg_ctl.exe') status -D $cluster *> $null
if ($LASTEXITCODE -ne 0) {
    $arguments = @('start','-D', ('"' + $cluster + '"'), '-l', ('"' + (Join-Path $tools 'postgresql.log') + '"'), '-o', ('"-h 127.0.0.1 -p ' + $Port + '"'), '-w')
    $process = Start-Process -FilePath (Join-Path $bin 'pg_ctl.exe') -ArgumentList $arguments -WindowStyle Hidden -PassThru
    if (!$process.WaitForExit(30000)) { throw 'PostgreSQL startup timed out.' }
    if ($process.ExitCode -ne 0) { throw 'PostgreSQL startup failed. See .tools/postgresql.log.' }
}
if (!$settings.provisioned) {
    $oldPassword = $env:PGPASSWORD
    $env:PGPASSWORD = $settings.admin_password
    try {
        "CREATE ROLE prm LOGIN PASSWORD '$($settings.password)' NOSUPERUSER NOCREATEDB NOCREATEROLE; CREATE DATABASE prm OWNER prm;" | & (Join-Path $bin 'psql.exe') -X -h 127.0.0.1 -p $Port -U postgres -d postgres -v ON_ERROR_STOP=1
        if ($LASTEXITCODE -ne 0) { throw 'Database provisioning failed.' }
        $settings.provisioned = $true
        $settings | ConvertTo-Json | Set-Content -LiteralPath $config -Encoding utf8
    } finally { $env:PGPASSWORD = $oldPassword }
}
$env:PRM_DATABASE_URL = $settings.url
Write-Output "PostgreSQL ready on 127.0.0.1:$Port (database prm, application role prm)."
