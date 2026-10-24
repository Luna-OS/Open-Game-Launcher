//! Einstellungen laden und speichern (%APPDATA%\MAZ Launcher\config.json).

use crate::model::Settings;
use crate::sys;
use std::fs;
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    sys::roaming_dir().join("config.json")
}

pub fn load() -> Settings {
    let path = config_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return Settings::default();
    };
    match serde_json::from_str::<Settings>(&text) {
        Ok(s) => s,
        Err(_) => {
            // Kaputte Datei sichern statt überschreiben – so geht nichts verloren.
            let backup = path.with_extension(format!("defekt-{}.json", sys::now_secs()));
            let _ = fs::copy(&path, backup);
            Settings::default()
        }
    }
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let dir = sys::roaming_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    // Erst in eine Temp-Datei schreiben, dann umbenennen → nie halb geschriebene Datei.
    let tmp = dir.join("config.json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, config_path())
}

/// Kleine eindeutige ID für eigene Spiele.
pub fn new_id() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("custom:{:x}", t ^ (std::process::id() as u128) << 64)
}
