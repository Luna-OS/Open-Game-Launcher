# Erzeugt assets/maz.css aus tailwind.css (scannt alle .rs-Dateien nach Klassen).
# Nutzt die eigenständige Tailwind-EXE – kein Node.js nötig.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$tools = Join-Path $root "tools"
$tw = Join-Path $tools "tailwindcss.exe"
if (-not (Test-Path $tw)) {
    New-Item -ItemType Directory -Force $tools | Out-Null
    Write-Host "Lade Tailwind CSS (einmalig) ..."
    Invoke-WebRequest -Uri "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-windows-x64.exe" -OutFile $tw
}
Push-Location $root
& $tw -i tailwind.css -o assets/maz.css --minify
Pop-Location
Write-Host "Fertig: assets/maz.css"
