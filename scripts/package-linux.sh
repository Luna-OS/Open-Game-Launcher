#!/usr/bin/env bash
# Packt die fertige Linux-Datei in alle Formate:
# AppImage, .deb, .rpm, .pkg.tar.zst (Pacman), Flatpak, Snap.
# Aufruf: scripts/package-linux.sh <binärdatei> <version> <x86_64|aarch64> [formate...]
set -euo pipefail

BIN="$1"; VERSION="$2"; ARCH="$3"; shift 3
FORMATS="${*:-appimage deb rpm pacman flatpak snap}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/dist"
WORK="$(mktemp -d)"
ID="io.github.mikeadamrgb.MAZLauncher"
PKG="maz-launcher"
DESC="All your Steam, Epic, Roblox and own games in one launcher"
URL="https://github.com/MikeAdamRGB/MAZ-Lauchner"
case "$ARCH" in
  x86_64) DEB_ARCH=amd64; SNAP_ARCH=amd64 ;;
  aarch64) DEB_ARCH=arm64; SNAP_ARCH=arm64 ;;
  *) echo "Unbekannte Architektur $ARCH"; exit 1 ;;
esac
mkdir -p "$OUT"

# Gemeinsamer Dateibaum (/usr/…)
stage_usr() {
  local d="$1"
  install -Dm755 "$BIN" "$d/usr/bin/$PKG"
  install -Dm644 "$ROOT/packaging/linux/$ID.desktop" "$d/usr/share/applications/$ID.desktop"
  install -Dm644 "$ROOT/packaging/linux/$ID.metainfo.xml" "$d/usr/share/metainfo/$ID.metainfo.xml"
  install -Dm644 "$ROOT/assets/icon-256.png" "$d/usr/share/icons/hicolor/256x256/apps/$ID.png"
  install -Dm644 "$ROOT/assets/icon-512.png" "$d/usr/share/icons/hicolor/512x512/apps/$ID.png"
  install -Dm644 "$ROOT/LICENSE" "$d/usr/share/licenses/$PKG/LICENSE"
}

build_deb() {
  local d="$WORK/deb"
  stage_usr "$d"
  mkdir -p "$d/DEBIAN"
  local size; size=$(du -sk "$d/usr" | cut -f1)
  cat > "$d/DEBIAN/control" <<CTL
Package: $PKG
Version: $VERSION
Architecture: $DEB_ARCH
Maintainer: MikeAdamRGB <m.zweers10@users.noreply.github.com>
Installed-Size: $size
Depends: libwebkit2gtk-4.1-0, libgtk-3-0, libc6 (>= 2.35)
Section: games
Priority: optional
Homepage: $URL
Description: $DESC
 MAZ Launcher shows your Steam, Epic (via Heroic), Roblox and own games
 in one library. Everything runs locally.
CTL
  dpkg-deb --root-owner-group --build "$d" "$OUT/${PKG}_${VERSION}_${DEB_ARCH}.deb"
}

build_rpm() {
  local top="$WORK/rpm"
  mkdir -p "$top"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,BUILDROOT}
  stage_usr "$top/stage"
  cat > "$top/SPECS/$PKG.spec" <<SPEC
Name: $PKG
Version: $VERSION
Release: 1
Summary: $DESC
License: MIT
URL: $URL
BuildArch: $ARCH
AutoReqProv: no
Requires: libwebkit2gtk-4.1.so.0()(64bit)
Requires: libgtk-3.so.0()(64bit)

%description
MAZ Launcher shows your Steam, Epic (via Heroic), Roblox and own games in one library.

%install
mkdir -p %{buildroot}
cp -a $top/stage/usr %{buildroot}/

%files
/usr/bin/$PKG
/usr/share/applications/$ID.desktop
/usr/share/metainfo/$ID.metainfo.xml
/usr/share/icons/hicolor/256x256/apps/$ID.png
/usr/share/icons/hicolor/512x512/apps/$ID.png
/usr/share/licenses/$PKG/LICENSE
SPEC
  rpmbuild --define "_topdir $top" --define "_build_id_links none" --target "$ARCH" -bb "$top/SPECS/$PKG.spec"
  cp "$top/RPMS/$ARCH/$PKG-$VERSION-1.$ARCH.rpm" "$OUT/"
}

build_pacman() {
  local d="$WORK/pacman"
  stage_usr "$d"
  local size; size=$(du -sb "$d/usr" | cut -f1)
  cat > "$d/.PKGINFO" <<PKGI
pkgname = $PKG
pkgbase = $PKG
pkgver = $VERSION-1
pkgdesc = $DESC
url = $URL
builddate = $(date +%s)
packager = MikeAdamRGB
size = $size
arch = $ARCH
license = MIT
depend = webkit2gtk-4.1
depend = gtk3
PKGI
  (
    cd "$d"
    export LANG=C
    bsdtar -czf .MTREE --format=mtree \
      --options='!all,use-set,type,uid,gid,mode,time,size,md5,sha256,link' \
      .PKGINFO usr
    bsdtar -cf - .MTREE .PKGINFO usr | zstd -19 -T0 -q -o "$OUT/$PKG-$VERSION-1-$ARCH.pkg.tar.zst"
  )
}

build_appimage() {
  local d="$WORK/AppDir"
  stage_usr "$d"
  cp "$ROOT/packaging/linux/$ID.desktop" "$d/$ID.desktop"
  cp "$ROOT/assets/icon-256.png" "$d/$ID.png"
  cp "$ROOT/assets/icon-256.png" "$d/.DirIcon"
  cat > "$d/AppRun" <<'RUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/maz-launcher" "$@"
RUN
  chmod +x "$d/AppRun"
  local tool="$WORK/appimagetool"
  curl -fsSL -o "$tool" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$ARCH.AppImage"
  chmod +x "$tool"
  ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$tool" --no-appstream "$d" "$OUT/MAZ-Launcher-$VERSION-$ARCH.AppImage"
}

build_flatpak() {
  local d="$WORK/flatpak"
  mkdir -p "$d/flatpak-src"
  cp "$BIN" "$d/flatpak-src/maz-launcher"
  cp "$ROOT/packaging/linux/$ID.desktop" "$ROOT/packaging/linux/$ID.metainfo.xml" "$d/flatpak-src/"
  cp "$ROOT/assets/icon-256.png" "$ROOT/assets/icon-512.png" "$d/flatpak-src/"
  cp "$ROOT/packaging/linux/$ID.yml" "$d/"
  (
    cd "$d"
    flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak-builder --user --install-deps-from=flathub --disable-rofiles-fuse --force-clean --repo=repo build "$ID.yml"
    flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo repo "$OUT/$PKG-$VERSION-$ARCH.flatpak" "$ID"
  )
}

build_snap() {
  local d="$WORK/snap"
  mkdir -p "$d/snap" "$d/snap-src/bin" "$d/snap-src/share/applications" "$d/snap-src/share/icons"
  cp "$BIN" "$d/snap-src/bin/maz-launcher"
  cp "$ROOT/assets/icon-256.png" "$d/snap-src/share/icons/maz-launcher.png"
  sed -e 's|^Icon=.*|Icon=${SNAP}/share/icons/maz-launcher.png|' -e 's|^Exec=.*|Exec=maz-launcher %U|' \
    "$ROOT/packaging/linux/$ID.desktop" > "$d/snap-src/share/applications/$ID.desktop"
  sed -e "s/@VERSION@/$VERSION/" -e "s/@ARCH@/$SNAP_ARCH/" "$ROOT/packaging/linux/snapcraft.yaml" > "$d/snap/snapcraft.yaml"
  (
    cd "$d"
    sudo snapcraft pack --destructive-mode --output "$OUT/${PKG}_${VERSION}_${SNAP_ARCH}.snap" || \
      sudo snapcraft --destructive-mode --output "$OUT/${PKG}_${VERSION}_${SNAP_ARCH}.snap"
  )
}

for f in $FORMATS; do
  echo "::group::$f"
  "build_$f"
  echo "::endgroup::"
done
ls -la "$OUT"
