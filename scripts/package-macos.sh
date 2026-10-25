#!/usr/bin/env bash
# Baut "MAZ Launcher.app" (Universal: Apple Silicon + Intel), dazu .dmg und .app.zip.
# Aufruf: scripts/package-macos.sh <arm64-binärdatei> <x86_64-binärdatei> <version>
set -euo pipefail
ARM="$1"; X86="$2"; VERSION="$3"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/dist"; WORK="$(mktemp -d)"
APP="$WORK/MAZ Launcher.app"
mkdir -p "$OUT" "$APP/Contents/MacOS" "$APP/Contents/Resources"

lipo -create -output "$APP/Contents/MacOS/maz-launcher" "$ARM" "$X86"
chmod +x "$APP/Contents/MacOS/maz-launcher"
sed "s/@VERSION@/$VERSION/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"

# Icon (.icns) aus dem PNG erzeugen
SET="$WORK/AppIcon.iconset"; mkdir -p "$SET"
for s in 16 32 128 256 512; do
  sips -z $s $s "$ROOT/assets/icon-512.png" --out "$SET/icon_${s}x${s}.png" >/dev/null
  d=$((s*2)); sips -z $d $d "$ROOT/assets/icon-512.png" --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o "$APP/Contents/Resources/AppIcon.icns"

# Ad-hoc-Signatur (ohne bezahltes Apple-Konto; nötig, damit Apple-Silicon-Macs die App starten)
codesign --force --deep --sign - "$APP"

# .app.zip (für das Update in der App) und .dmg (zum Installieren)
ditto -c -k --keepParent "$APP" "$OUT/MAZ-Launcher-$VERSION-macOS.app.zip"
DMG="$WORK/dmg"; mkdir -p "$DMG"
cp -R "$APP" "$DMG/"
ln -s /Applications "$DMG/Applications"
hdiutil create -volname "MAZ Launcher" -srcfolder "$DMG" -ov -format UDZO "$OUT/MAZ-Launcher-$VERSION-macOS.dmg"
ls -la "$OUT"
