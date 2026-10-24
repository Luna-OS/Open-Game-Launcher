//! Datenmodell: Spiele, eigene Spiele und Einstellungen.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Steam,
    Epic,
    Custom,
}

impl Source {
    pub fn label(self) -> String {
        match self {
            Source::Steam => "Steam".into(),
            Source::Epic => "Epic Games".into(),
            Source::Custom => crate::i18n::t("source.custom"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LaunchTarget {
    /// steam://rungameid/<appid>
    SteamRun(u32),
    /// Nicht installiert → steam://install/<appid>
    SteamInstall(u32),
    /// com.epicgames.launcher://apps/…?action=launch
    Epic(String),
    /// Eigenes Spiel (Details stehen in den Einstellungen)
    Custom,
}

/// Bild-Kandidaten: Die Oberfläche probiert sie der Reihe nach durch.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Art {
    /// Hochformat-Cover (600×900)
    pub cover: Vec<String>,
    /// Großes Hintergrundbild (Steam "library_hero")
    pub hero: Vec<String>,
    /// Transparentes Spiel-Logo
    pub logo: Vec<String>,
    /// Kleines quadratisches Icon (Original-Spiel-Icon)
    pub icon: Vec<String>,
    /// Querformat-Banner (460×215)
    pub header: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub source: Source,
    pub installed: bool,
    pub install_dir: Option<String>,
    pub exe: Option<String>,
    pub launch: LaunchTarget,
    pub art: Art,
    pub size_bytes: Option<u64>,
    pub playtime_min: Option<u64>,
    pub steam_appid: Option<u32>,
}

impl Game {
    pub fn sort_key(&self) -> String {
        let n = self.name.to_lowercase();
        n.strip_prefix("the ").map(|s| s.to_string()).unwrap_or(n)
    }
}

/// Ein Spiel, das du selbst hinzugefügt hast.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CustomGame {
    pub id: String,
    pub name: String,
    /// Pfad zur EXE (oder .lnk/.bat/.url)
    pub exe: String,
    /// Startparameter, z. B. "-windowed"
    pub args: String,
    /// Leer = Ordner der EXE
    pub workdir: String,
    /// Eigenes Cover-Bild (leer = automatisch)
    pub cover: String,
    pub admin: bool,
    /// Zusätzliche Punkte für die Sicherheitsfrage (eine Zeile = ein Punkt)
    pub checklist: String,
    pub added: u64,
}

pub mod layout {
    pub const MAZ: &str = "maz";
    pub const STEAM: &str = "steam";
    pub const EPIC: &str = "epic";
}

/// Standard-Punkte der Sicherheitsfrage (in der gewählten Sprache)
pub fn default_checklist() -> Vec<String> {
    crate::i18n::tl("checklist.default")
}

/// Frühere deutsche Standardliste – wird beim Laden durch die übersetzte ersetzt
pub const OLD_GERMAN_CHECKLIST: &[&str] = &[
    "Antivirus / Echtzeitschutz (falls das Spiel das braucht)",
    "Overlays (Discord, Steam, GeForce Experience, Xbox Game Bar)",
    "Aufnahme- und Streaming-Programme (OBS, Medal, ShadowPlay)",
    "VPN und Proxy",
    "Unnötige Programme im Hintergrund (Browser, Downloads)",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Sprache ("auto" = Windows-Sprache)
    pub language: String,

    // Design
    pub theme: String,
    pub custom_themes: Vec<crate::theme::CustomTheme>,
    /// Hintergrundbild (leer = keins)
    pub wallpaper: String,
    /// Wie stark das Hintergrundbild abgedunkelt wird (0–90 %)
    pub wallpaper_dim: u8,
    pub wallpaper_blur: bool,
    pub layout: String,
    pub authentic_colors: bool,
    /// Eigene Akzentfarbe (#rrggbb), leer = Farbe des Themes
    pub accent: String,
    /// 1 = klein, 2 = mittel, 3 = groß
    pub card_size: u8,
    pub animations: bool,
    pub show_names: bool,

    // Verbindungen
    pub steam_enabled: bool,
    pub epic_enabled: bool,
    pub steam_api_key: String,
    pub steam_id: String,
    pub steam_show_uninstalled: bool,
    pub online_art: bool,

    // Eigene Spiele
    /// Leer = Standardliste in der gewählten Sprache
    pub checklist: Vec<String>,
    pub custom_games: Vec<CustomGame>,

    // Verhalten
    pub minimize_on_launch: bool,
    pub sort: String,

    // Gemerkte Daten
    pub favorites: Vec<String>,
    pub hidden: Vec<String>,
    pub last_played: BTreeMap<String, u64>,
    pub launch_count: BTreeMap<String, u32>,
    pub cover_overrides: BTreeMap<String, String>,
    /// Name (normalisiert) → Steam-AppID für Cover-Bilder (0 = nichts gefunden)
    pub art_lookup: BTreeMap<String, u32>,
    pub welcome_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "auto".into(),
            theme: "maz".into(),
            custom_themes: Vec::new(),
            wallpaper: String::new(),
            wallpaper_dim: 60,
            wallpaper_blur: false,
            layout: layout::MAZ.into(),
            authentic_colors: true,
            accent: String::new(),
            card_size: 2,
            animations: true,
            show_names: true,
            steam_enabled: true,
            epic_enabled: true,
            steam_api_key: String::new(),
            steam_id: String::new(),
            steam_show_uninstalled: true,
            online_art: true,
            checklist: Vec::new(),
            custom_games: Vec::new(),
            minimize_on_launch: true,
            sort: "name".into(),
            favorites: Vec::new(),
            hidden: Vec::new(),
            last_played: BTreeMap::new(),
            launch_count: BTreeMap::new(),
            cover_overrides: BTreeMap::new(),
            art_lookup: BTreeMap::new(),
            welcome_done: false,
        }
    }
}

impl Settings {
    pub fn is_favorite(&self, id: &str) -> bool {
        self.favorites.iter().any(|f| f == id)
    }
    pub fn is_hidden(&self, id: &str) -> bool {
        self.hidden.iter().any(|f| f == id)
    }
    pub fn custom(&self, id: &str) -> Option<&CustomGame> {
        self.custom_games.iter().find(|c| c.id == id)
    }
    /// Punkte der Sicherheitsfrage (eigene oder Standard)
    pub fn effective_checklist(&self) -> Vec<String> {
        if self.checklist.is_empty() {
            default_checklist()
        } else {
            self.checklist.clone()
        }
    }
    pub fn last_played(&self, id: &str) -> u64 {
        self.last_played.get(id).copied().unwrap_or(0)
    }
}

/// Status einer Plattform-Verbindung (für Einstellungen → Verbindungen).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct StoreStatus {
    pub found: bool,
    pub path: Option<String>,
    pub installed: usize,
    pub owned: usize,
    pub account: Option<String>,
    pub note: Option<String>,
}

/// Normalisierter Name für Vergleiche ("The Witcher® 3: Wild Hunt" → "thewitcher3wildhunt").
pub fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}
