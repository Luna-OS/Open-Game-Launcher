//! Linux: Windows-Spiele mit Wine oder Proton starten – pro Spiel wählbar.
//!
//! Gefunden werden: System-Wine, Proton aus Steam (offiziell und `compatibilitytools.d`,
//! z. B. GE-Proton), Wine-/Proton-Versionen von Lutris und Heroic. Es wird nichts
//! heruntergeladen; gestartet wird nur ein Programm, das in dieser Liste steht.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Wine,
    Proton,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Runner {
    /// Pfad zum Programm (wine oder das Proton-Skript "proton")
    pub path: String,
    pub name: String,
    pub kind: Kind,
}

fn steam_roots() -> Vec<PathBuf> {
    let h = crate::sys::home_dir();
    [
        h.join(".steam/root"),
        h.join(".steam/steam"),
        h.join(".local/share/Steam"),
        h.join(".var/app/com.valvesoftware.Steam/data/Steam"),
        h.join("snap/steam/common/.local/share/Steam"),
    ]
    .into_iter()
    .filter(|p| p.is_dir())
    .collect()
}

fn add(out: &mut Vec<Runner>, path: PathBuf, name: String, kind: Kind) {
    if !path.is_file() {
        return;
    }
    // Gleiche Datei (über Verknüpfungen) nur einmal
    let canon = std::fs::canonicalize(&path).unwrap_or(path.clone());
    if out
        .iter()
        .any(|r| std::fs::canonicalize(&r.path).ok().as_deref() == Some(canon.as_path()))
    {
        return;
    }
    out.push(Runner {
        path: path.to_string_lossy().to_string(),
        name,
        kind,
    });
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v.reverse(); // neueste Versionen zuerst
    v
}

fn dir_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Alle gefundenen Wine-/Proton-Versionen (nur unter Linux, sonst leer)
pub fn list() -> Vec<Runner> {
    let mut out = Vec::new();
    if !cfg!(target_os = "linux") {
        return out;
    }
    for sys_wine in [
        "/usr/bin/wine",
        "/usr/local/bin/wine",
        "/usr/bin/wine-stable",
    ] {
        add(
            &mut out,
            PathBuf::from(sys_wine),
            "Wine (System)".into(),
            Kind::Wine,
        );
    }
    for root in steam_roots() {
        for d in subdirs(&root.join("compatibilitytools.d")) {
            add(&mut out, d.join("proton"), dir_name(&d), Kind::Proton);
        }
        for d in subdirs(&root.join("steamapps/common")) {
            if dir_name(&d).starts_with("Proton") {
                add(&mut out, d.join("proton"), dir_name(&d), Kind::Proton);
            }
        }
    }
    let h = crate::sys::home_dir();
    for base in [
        h.join(".local/share/lutris/runners/wine"),
        h.join(".config/heroic/tools/wine"),
        h.join(".var/app/com.heroicgameslauncher.hgl/config/heroic/tools/wine"),
    ] {
        for d in subdirs(&base) {
            add(
                &mut out,
                d.join("bin/wine"),
                format!("{} (Wine)", dir_name(&d)),
                Kind::Wine,
            );
        }
    }
    for base in [
        h.join(".config/heroic/tools/proton"),
        h.join(".var/app/com.heroicgameslauncher.hgl/config/heroic/tools/proton"),
    ] {
        for d in subdirs(&base) {
            add(&mut out, d.join("proton"), dir_name(&d), Kind::Proton);
        }
    }
    out
}

/// SICHERHEIT: Nur Programme starten, die wir selbst gefunden haben – ein manipulierter
/// Eintrag in der Einstellungsdatei kann so kein beliebiges Programm starten.
#[cfg_attr(windows, allow(dead_code))]
pub fn find(path: &str) -> Option<Runner> {
    if path.is_empty() {
        return None;
    }
    list().into_iter().find(|r| r.path == path)
}

/// Eigener Wine-Prefix für ein Spiel (bei Proton nötig)
#[cfg_attr(windows, allow(dead_code))]
pub fn prefix_for(game_id: &str) -> PathBuf {
    let safe: String = game_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(60)
        .collect();
    // Nicht im Cache-Ordner: im Prefix liegen Spielstände
    let data = crate::sys::env_path("XDG_DATA_HOME")
        .unwrap_or_else(|| crate::sys::home_dir().join(".local/share"));
    data.join("maz-launcher").join("prefixes").join(safe)
}

/// Steam-Ordner, den Proton zum Starten braucht (nur unter Linux benutzt)
#[cfg_attr(windows, allow(dead_code))]
pub fn steam_root() -> Option<PathBuf> {
    steam_roots().into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_are_safe() {
        let p = prefix_for("custom:../../etc");
        assert!(p.ends_with("custom_______etc"));
        assert!(find("/bin/sh").is_none());
        assert!(find("").is_none());
    }
}
