//! Original-Spiel-Icons: werden aus der EXE geholt und als PNG zwischengespeichert.

use crate::sys;
use std::fs;
use std::path::{Path, PathBuf};

/// Gibt den Pfad zu einem PNG mit dem Original-Icon der EXE zurück (wird gecacht).
pub fn cached_icon(exe: &Path) -> Option<PathBuf> {
    let meta = fs::metadata(exe).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dir = sys::icon_cache_dir();
    let key = sys::fnv1a(&format!("{}|{}", exe.to_string_lossy().to_lowercase(), mtime));
    let out = dir.join(format!("{key:016x}.png"));
    if out.exists() {
        return Some(out);
    }
    // Merker für "hat kein Icon", damit wir nicht jedes Mal neu suchen
    let none_marker = dir.join(format!("{key:016x}.none"));
    if none_marker.exists() {
        return None;
    }
    let _ = fs::create_dir_all(&dir);
    match extract_png(exe, &out) {
        Some(()) => Some(out),
        None => {
            let _ = fs::write(none_marker, b"");
            None
        }
    }
}

#[cfg(windows)]
fn extract_png(exe: &Path, out: &Path) -> Option<()> {
    let (w, h, rgba) = crate::winffi::extract_icon_rgba(&exe.to_string_lossy(), 256)?;
    let img = image::RgbaImage::from_raw(w, h, rgba)?;
    img.save_with_format(out, image::ImageFormat::Png).ok()
}

#[cfg(not(windows))]
fn extract_png(_exe: &Path, _out: &Path) -> Option<()> {
    None
}

/// Rät die Haupt-EXE eines Spielordners (für Icons von Steam-Spielen ohne Cache).
pub fn guess_main_exe(dir: &Path, game_name: &str) -> Option<PathBuf> {
    const SKIP: &[&str] = &[
        "unins", "uninst", "setup", "install", "crash", "report", "redist", "vcredist", "vc_redist", "dxsetup",
        "dotnet", "prereq", "helper", "update", "launcherhelper", "easyanticheat", "battleye", "be_service",
        "unitycrashhandler", "cefprocess", "webhelper", "touchup", "cleanup", "dxwebsetup", "physx", "oalinst",
    ];
    let wanted = crate::model::normalize_name(game_name);
    let mut best: Option<(u64, PathBuf)> = None;
    let subdirs = ["", "bin", "Bin", "bin\\win64", "Binaries\\Win64", "x64", "Game", "Win64"];
    for sub in subdirs {
        let d = if sub.is_empty() { dir.to_path_buf() } else { dir.join(sub) };
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("exe")) != Some(true) {
                continue;
            }
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if SKIP.iter().any(|s| stem.contains(s)) {
                continue;
            }
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            // Bonus, wenn der Dateiname zum Spielnamen passt
            let norm = crate::model::normalize_name(&stem);
            let score = if !norm.is_empty() && (wanted.contains(&norm) || norm.contains(&wanted)) {
                size + 10_000_000_000
            } else {
                size
            };
            if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                best = Some((score, p));
            }
        }
    }
    best.map(|(_, p)| p)
}
