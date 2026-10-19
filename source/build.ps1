$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    $taskTarget = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $PSScriptRoot 'target' }
    Copy-Item -LiteralPath (Join-Path $taskTarget 'release\Shike.exe') -Destination (Join-Path $PSScriptRoot '..\Shike.exe') -Force
    Write-Host 'Built ../Shike.exe'
} finally {
    Pop-Location
}
