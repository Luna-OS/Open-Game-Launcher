#!/usr/bin/env sh
# Erzeugt assets/maz.css aus tailwind.css (Linux/macOS).
set -e
cd "$(dirname "$0")/.."
mkdir -p tools
if [ ! -x tools/tailwindcss ]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) f=tailwindcss-linux-x64 ;;
    Linux-aarch64) f=tailwindcss-linux-arm64 ;;
    Darwin-arm64) f=tailwindcss-macos-arm64 ;;
    *) f=tailwindcss-macos-x64 ;;
  esac
  curl -sSL -o tools/tailwindcss "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/$f"
  chmod +x tools/tailwindcss
fi
tools/tailwindcss -i tailwind.css -o assets/maz.css --minify
echo "Fertig: assets/maz.css"
