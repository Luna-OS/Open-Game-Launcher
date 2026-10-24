# Baut die fertige EXE nach dist\MAZ-Launcher.exe
# Optional: -Mini  → zusätzlich eine mit UPX gepackte, noch kleinere EXE
param([switch]$Mini)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
& "$PSScriptRoot/build-css.ps1"
cargo build --release
New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item target/release/MAZ-Launcher.exe dist/MAZ-Launcher.exe -Force
if ($Mini) {
    if (-not (Get-Command upx -ErrorAction SilentlyContinue)) {
        Write-Host "UPX nicht gefunden. Installieren mit: winget install upx.upx"
    } else {
        upx --best --lzma -f -o dist/MAZ-Launcher-mini.exe target/release/MAZ-Launcher.exe
    }
}
Get-ChildItem dist/*.exe | ForEach-Object { "{0,-28} {1,8:N2} MB" -f $_.Name, ($_.Length / 1MB) }
Pop-Location
