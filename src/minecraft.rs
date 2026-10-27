//! Minecraft: offizieller Minecraft Launcher und NoRisk Client.
//!
//! - **Minecraft Launcher**: Microsoft Store/Xbox-App, alter `MinecraftLauncher.exe`, macOS-App,
//!   Linux (`minecraft-launcher` oder Flatpak `com.mojang.Minecraft`).
//! - **NoRisk Client** (<https://norisk.gg>): *eine* Kachel; ein Klick öffnet die NoRisk-Übersicht
//!   in OGL (Profile, Spielen, Inhalte). Details in [`norisk`]. Auf Wunsch zusätzlich eine Kachel
//!   pro Profil. Das Microsoft-Konto bleibt im jeweiligen Launcher.

use crate::model::{Art, Game, LaunchTarget, Source, StoreStatus};
use crate::sys;
use std::path::{Path, PathBuf};

mod art;
pub mod norisk;
pub use norisk::NoRiskProfile;

pub const STORE_PACKAGE: &str = "Microsoft.4297127D64EC6";
pub const NORISK_DOWNLOAD: &str = norisk::DOWNLOAD;
pub const MC_DOWNLOAD: &str = "https://www.minecraft.net/download";
const MC_FLATPAK: &str = "com.mojang.Minecraft";

/// Wie ein Launcher gestartet wird: Programm + feste Anfangs-Argumente
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Starter {
    pub program: String,
    pub prefix_args: Vec<String>,
}

pub(crate) fn flatpak_installed(app: &str) -> bool {
    let h = sys::home_dir();
    [
        PathBuf::from("/var/lib/flatpak/app").join(app),
        h.join(".local/share/flatpak/app").join(app),
    ]
    .iter()
    .any(|p| p.is_dir())
}

pub(crate) fn first_existing(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.into_iter().find(|p| p.is_file())
}

// ----------------------------------------------------------------------
// Offizieller Minecraft Launcher
// ----------------------------------------------------------------------

/// Wie der offizielle Minecraft Launcher gestartet wird
pub fn official_launch() -> Option<(LaunchTarget, String)> {
    if cfg!(windows) {
        if let Some(pkg) = crate::appx::find(STORE_PACKAGE) {
            let app = pkg.app_id("Minecraft")?;
            return Some((
                LaunchTarget::Appx {
                    family: pkg.family.clone(),
                    app,
                },
                format!("Microsoft Store: {}", pkg.full_name),
            ));
        }
        let mut c = Vec::new();
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = sys::env_path(var) {
                c.push(p.join("Minecraft Launcher").join("MinecraftLauncher.exe"));
            }
        }
        if let Some(a) = sys::env_path("APPDATA") {
            c.push(
                a.join(".minecraft")
                    .join("launcher")
                    .join("MinecraftLauncher.exe"),
            );
        }
        return first_existing(c).map(|p| {
            let s = p.to_string_lossy().to_string();
            (
                LaunchTarget::Exe {
                    path: s.clone(),
                    args: Vec::new(),
                    dir: String::new(),
                },
                s,
            )
        });
    }
    if cfg!(target_os = "macos") {
        let app = [
            "/Applications/Minecraft.app".to_string(),
            sys::home_dir()
                .join("Applications/Minecraft.app")
                .to_string_lossy()
                .to_string(),
        ]
        .into_iter()
        .find(|p| Path::new(p).is_dir())?;
        return Some((
            LaunchTarget::Exe {
                path: "/usr/bin/open".into(),
                args: vec!["-a".into(), app.clone()],
                dir: String::new(),
            },
            app,
        ));
    }
    if let Some(p) = first_existing(vec![
        PathBuf::from("/usr/bin/minecraft-launcher"),
        PathBuf::from("/opt/minecraft-launcher/minecraft-launcher"),
        PathBuf::from("/usr/local/bin/minecraft-launcher"),
    ]) {
        let s = p.to_string_lossy().to_string();
        return Some((
            LaunchTarget::Exe {
                path: s.clone(),
                args: Vec::new(),
                dir: String::new(),
            },
            s,
        ));
    }
    if flatpak_installed(MC_FLATPAK) {
        let fp = sys::unix_tool("flatpak").to_string_lossy().to_string();
        return Some((
            LaunchTarget::Exe {
                path: fp,
                args: vec!["run".into(), MC_FLATPAK.into()],
                dir: String::new(),
            },
            format!("Flatpak: {MC_FLATPAK}"),
        ));
    }
    None
}

/// `.minecraft`-Ordner des offiziellen Launchers
pub fn official_game_dir() -> Option<PathBuf> {
    let h = sys::home_dir();
    let candidates = if cfg!(windows) {
        sys::env_path("APPDATA")
            .map(|a| vec![a.join(".minecraft")])
            .unwrap_or_default()
    } else if cfg!(target_os = "macos") {
        vec![h.join("Library/Application Support/minecraft")]
    } else {
        vec![
            h.join(".minecraft"),
            h.join(".var/app").join(MC_FLATPAK).join(".minecraft"),
        ]
    };
    candidates.into_iter().find(|p| p.is_dir())
}

// ----------------------------------------------------------------------
// Bibliothek
// ----------------------------------------------------------------------

fn mc_game(id: String, name: String, launch: LaunchTarget, dir: Option<String>) -> Game {
    Game {
        id,
        name,
        source: Source::Minecraft,
        installed: true,
        install_dir: dir,
        exe: None,
        launch,
        art: Art::default(),
        size_bytes: None,
        playtime_min: None,
        steam_appid: None,
    }
}

/// Loader schön schreiben ("fabric" → "Fabric")
pub fn loader_label(l: &str) -> String {
    match l {
        "neoforge" => "NeoForge".into(),
        "vanilla" | "" => "Vanilla".into(),
        other => {
            let mut c = other.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        }
    }
}

/// Befehl für den NoRisk Launcher als Startziel
pub fn norisk_target(starter: &Starter, args: Vec<String>) -> LaunchTarget {
    LaunchTarget::Exe {
        path: starter.program.clone(),
        args,
        dir: String::new(),
    }
}

/// Einmaliger Befehl an den NoRisk Launcher als Spiel (für den Start über die normale Fehlerbehandlung)
pub fn command_game(name: &str, starter: &Starter, args: Vec<String>) -> Game {
    mc_game(
        norisk::TILE_ID.into(),
        name.to_string(),
        norisk_target(starter, args),
        None,
    )
}

/// Spielbares NoRisk-Profil als Spiel – für "Spielen" in der Übersicht und für Einzel-Kacheln
pub fn profile_game(starter: &Starter, p: &NoRiskProfile) -> Option<Game> {
    let args = norisk::launch_args(starter, &p.id)?;
    let detail = p.detail();
    let name = if detail.is_empty() {
        format!("NoRisk · {}", p.name)
    } else {
        format!("NoRisk · {} ({detail})", p.name)
    };
    let dir = (!p.dir.as_os_str().is_empty()).then(|| p.dir.to_string_lossy().to_string());
    Some(mc_game(
        format!("norisk:{}", p.id),
        name,
        norisk_target(starter, args),
        dir,
    ))
}

/// Die eine NoRisk-Kachel. "Spielen" startet das zuletzt gespielte Profil, sonst öffnet sich der Launcher.
pub fn norisk_tile(nr: &norisk::NoRisk) -> Option<Game> {
    let starter = nr.starter.as_ref()?;
    let args = match nr
        .last_played()
        .and_then(|p| norisk::launch_args(starter, &p.id))
    {
        Some(a) => a,
        None => norisk::open_args(starter, &norisk::Open::Home),
    };
    let dir = nr
        .paths
        .as_ref()
        .map(|p| p.profiles.clone())
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().to_string());
    let mut g = mc_game(
        norisk::TILE_ID.into(),
        "NoRisk Client".into(),
        norisk_target(starter, args),
        dir,
    );
    g.exe = Some(starter.program.clone()).filter(|p| Path::new(p).is_file());
    g.playtime_min =
        Some(nr.profiles.iter().map(|p| p.playtime_secs).sum::<u64>() / 60).filter(|m| *m > 0);
    if let Some(icon) = g
        .exe
        .as_ref()
        .and_then(|e| crate::icons::cached_icon(Path::new(e)))
    {
        g.art.icon.push(sys::file_url(&icon));
    }
    if let Some((cover, hero, header)) = norisk::artwork() {
        g.art.cover.push(sys::file_url(&cover));
        g.art.hero.push(sys::file_url(&hero));
        g.art.header.push(sys::file_url(&header));
    }
    Some(g)
}

/// Ist das die NoRisk-Kachel (Klick öffnet die Übersicht statt der Detailansicht)?
pub fn is_norisk_tile(id: &str) -> bool {
    id == norisk::TILE_ID
}

/// Minecraft-Kacheln: offizieller Launcher + eine NoRisk-Kachel;
/// `profile_tiles` = zusätzlich eine Kachel pro NoRisk-Profil (Einstellung, Standard aus)
pub fn scan_with(profile_tiles: bool) -> (Vec<Game>, StoreStatus) {
    let mut games = Vec::new();
    let mut status = StoreStatus::default();
    if let Some((launch, path)) = official_launch() {
        games.push(mc_game(
            "minecraft:launcher".into(),
            "Minecraft".into(),
            launch,
            official_game_dir().map(|d| d.to_string_lossy().to_string()),
        ));
        status.path = Some(path);
    }
    let nr = norisk::load();
    if let Some(tile) = norisk_tile(&nr) {
        games.push(tile);
        if let Some(starter) = nr.starter.as_ref().filter(|_| profile_tiles) {
            games.extend(nr.profiles.iter().filter_map(|p| profile_game(starter, p)));
        }
        if status.path.is_none() {
            status.path = nr.starter.as_ref().map(|s| s.program.clone());
        }
    }
    status.found = !games.is_empty();
    status.installed = games.len();
    status.owned = games.len();
    (games, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(loader_label("neoforge"), "NeoForge");
        assert_eq!(loader_label("fabric"), "Fabric");
        assert_eq!(loader_label(""), "Vanilla");
    }

    fn sample() -> norisk::NoRisk {
        let p = |id: &str, last: u64| NoRiskProfile {
            id: id.into(),
            name: "P".into(),
            game_version: "1.21".into(),
            loader: "fabric".into(),
            last_played: last,
            playtime_secs: 600,
            ..Default::default()
        };
        norisk::NoRisk {
            starter: Some(Starter {
                program: "nrc".into(),
                prefix_args: Vec::new(),
            }),
            paths: None,
            profiles: vec![
                p("0b5c1a8e-4f1e-4c55-9d6b-2b9f0c3a1e22", 5),
                p("1b5c1a8e-4f1e-4c55-9d6b-2b9f0c3a1e22", 9),
            ],
        }
    }

    #[test]
    fn one_tile() {
        let nr = sample();
        let g = norisk_tile(&nr).unwrap();
        assert!(is_norisk_tile(&g.id));
        assert_eq!(g.playtime_min, Some(20));
        // Spielen = zuletzt gespieltes Profil
        match g.launch {
            LaunchTarget::Exe { args, .. } => assert_eq!(
                args,
                vec![
                    "launch",
                    "--profile",
                    "1b5c1a8e-4f1e-4c55-9d6b-2b9f0c3a1e22"
                ]
            ),
            _ => panic!("kein Exe-Start"),
        }
        // Ohne gespieltes Profil: Launcher öffnen
        let mut nr2 = nr.clone();
        nr2.profiles.iter_mut().for_each(|p| p.last_played = 0);
        match norisk_tile(&nr2).unwrap().launch {
            LaunchTarget::Exe { args, .. } => assert_eq!(args[0], "open"),
            _ => panic!("kein Exe-Start"),
        }
        // Nicht installiert → keine Kachel
        assert!(norisk_tile(&norisk::NoRisk {
            starter: None,
            ..nr
        })
        .is_none());
    }

    #[test]
    fn profile_tiles() {
        let nr = sample();
        let s = nr.starter.clone().unwrap();
        let g = profile_game(&s, &nr.profiles[0]).unwrap();
        assert_eq!(g.id, "norisk:0b5c1a8e-4f1e-4c55-9d6b-2b9f0c3a1e22");
        assert_eq!(g.name, "NoRisk · P (1.21 · Fabric)");
        assert_eq!(g.install_dir, None);
    }
}
