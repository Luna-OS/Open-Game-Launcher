//! NoRisk Client: Erkennung, Profile und Start – nur über die Dateien und den offiziellen
//! Befehl des NoRisk Launchers (kein Code aus dem NoRisk-Projekt).
//!
//! - Datenbank: `<root>/meta/app.db` (WAL) – wird nur als *Kopie* gelesen.
//! - Profile: `<data>/profiles` bzw. `<custom_game_directory>/profiles` aus `launcher_config.json`.
//! - Standard-Versionen: `<root>/norisk_versions.json`.
//! - Start: `launch --profile <uuid>`, Launcher öffnen: `open [--profile <uuid>] [--theme <id>]`.
//!   Läuft der Launcher schon, reicht er die Argumente an das offene Fenster weiter.
//!
//! OGL legt hier nie selbst Profile oder Ordner an – neue Profile entstehen im NoRisk Launcher.

use super::{first_existing, flatpak_installed, Starter};
use crate::sys;
use std::path::{Path, PathBuf};

/// Kachel-ID der einen NoRisk-Kachel
pub const TILE_ID: &str = "norisk:client";
pub const DOWNLOAD: &str = "https://norisk.gg/";
/// Design, mit dem OGL den NoRisk Launcher öffnet
pub const THEME: &str = "lu_hub";
pub(super) const FLATPAK: &str = "gg.norisk.NoRiskClientLauncherV3";

/// Ein Profil aus dem NoRisk Launcher
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NoRiskProfile {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub loader: String,
    /// Gruppe wie im NoRisk Launcher (z. B. "NORISK CLIENT", "MODPACKS") – leer = keine
    pub group: String,
    /// Spielordner des Profils (für Mods, Shader …). Leer = unsicherer Pfad, nicht benutzen.
    pub dir: PathBuf,
    /// Unix-Sekunden, 0 = nie gespielt
    pub last_played: u64,
    pub playtime_secs: u64,
    /// Vorlage von NoRisk (Standard-Version) statt eigenes Profil
    pub standard: bool,
    /// Teilt den Spielordner mit anderen Profilen der Gruppe
    pub shared: bool,
}

impl NoRiskProfile {
    /// "1.21.4 · Fabric"
    pub fn detail(&self) -> String {
        [self.game_version.clone(), super::loader_label(&self.loader)]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// Wo der NoRisk Launcher seine Daten hat
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Paths {
    /// Hauptordner (Windows: `…\NoRiskClientV3`, sonst = data)
    pub root: PathBuf,
    pub data: PathBuf,
    pub db: PathBuf,
    /// Ordner aller Profile (beachtet einen eigenen Spielordner)
    pub profiles: PathBuf,
}

impl Paths {
    fn from_data(data: PathBuf) -> Self {
        let root = if cfg!(windows) {
            data.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| data.clone())
        } else {
            data.clone()
        };
        let custom = std::fs::read_to_string(root.join("launcher_config.json"))
            .ok()
            .and_then(|s| custom_game_dir(&s));
        let profiles = custom
            .map(|c| c.join("profiles"))
            .unwrap_or_else(|| data.join("profiles"));
        Paths {
            db: root.join("meta").join("app.db"),
            root,
            data,
            profiles,
        }
    }
}

/// `custom_game_directory` aus `launcher_config.json` – nur absolute, vorhandene Ordner
pub fn custom_game_dir(config_json: &str) -> Option<PathBuf> {
    let v: serde_json::Value = serde_json::from_str(config_json).ok()?;
    let p = PathBuf::from(v.get("custom_game_directory")?.as_str()?.trim());
    (p.is_absolute() && p.is_dir()).then_some(p)
}

/// Alles, was OGL über den NoRisk Client weiß
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NoRisk {
    pub starter: Option<Starter>,
    pub paths: Option<Paths>,
    /// Neueste zuerst
    pub profiles: Vec<NoRiskProfile>,
}

impl NoRisk {
    pub fn last_played(&self) -> Option<&NoRiskProfile> {
        self.profiles
            .iter()
            .filter(|p| p.last_played > 0)
            .max_by_key(|p| p.last_played)
    }
}

/// NoRisk Launcher, Datenordner und Profile einlesen (blockierend – im Hintergrund aufrufen)
pub fn load() -> NoRisk {
    let paths = data_dirs().into_iter().map(Paths::from_data).next();
    let profiles = paths.as_ref().map(read_all).unwrap_or_default();
    NoRisk {
        starter: starter(),
        paths,
        profiles,
    }
}

/// Ändert sich, sobald der NoRisk Launcher Profile ändert (Datenbank, WAL, Standard-Versionen, Einstellungen)
pub fn stamp() -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for d in data_dirs() {
        let p = Paths::from_data(d);
        let files = [
            p.db.clone(),
            p.db.with_file_name("app.db-wal"),
            p.root.join("norisk_versions.json"),
            p.root.join("launcher_config.json"),
        ];
        for f in files {
            let (len, mtime) = std::fs::metadata(&f)
                .map(|m| {
                    (
                        m.len(),
                        m.modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_nanos() as u64)
                            .unwrap_or(0),
                    )
                })
                .unwrap_or((0, 0));
            for x in [len, mtime] {
                h ^= x;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    h
}

// ----------------------------------------------------------------------
// Programm und Datenordner finden
// ----------------------------------------------------------------------

/// Programm des NoRisk Launchers
pub fn starter() -> Option<Starter> {
    let exe = |p: PathBuf| Starter {
        program: p.to_string_lossy().to_string(),
        prefix_args: Vec::new(),
    };
    if cfg!(windows) {
        // Eintrag unter "Apps" (DisplayIcon zeigt auf die EXE) – sonst übliche Ordner
        for e in crate::stores::uninstall_entries() {
            if e.display_name.to_lowercase().contains("norisk") {
                let icon = crate::stores::icon_path(&e.display_icon);
                if icon.to_lowercase().ends_with(".exe")
                    && Path::new(&icon).is_file()
                    && !icon.to_lowercase().ends_with("uninstall.exe")
                {
                    return Some(exe(PathBuf::from(icon)));
                }
                if !e.install_location.is_empty() {
                    if let Some(p) = find_exe_in(Path::new(&e.install_location)) {
                        return Some(exe(p));
                    }
                }
            }
        }
        let mut dirs = Vec::new();
        if let Some(l) = sys::env_path("LOCALAPPDATA") {
            dirs.push(l.join("NoRisk Launcher"));
            dirs.push(l.join("Programs").join("NoRisk Launcher"));
        }
        if let Some(p) = sys::env_path("ProgramFiles") {
            dirs.push(p.join("NoRisk Launcher"));
        }
        return dirs.iter().find_map(|d| find_exe_in(d)).map(exe);
    }
    if cfg!(target_os = "macos") {
        for app in [
            PathBuf::from("/Applications/NoRisk Launcher.app"),
            sys::home_dir().join("Applications/NoRisk Launcher.app"),
        ] {
            let macos = app.join("Contents").join("MacOS");
            let found = std::fs::read_dir(&macos)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .find(|p| p.is_file());
            if let Some(p) = found {
                return Some(exe(p));
            }
        }
        return None;
    }
    if let Some(p) = first_existing(vec![
        PathBuf::from("/usr/bin/noriskclient-launcher-v3"),
        PathBuf::from("/usr/local/bin/noriskclient-launcher-v3"),
        PathBuf::from("/opt/noriskclient-launcher-v3/noriskclient-launcher-v3"),
    ]) {
        return Some(exe(p));
    }
    if flatpak_installed(FLATPAK) {
        return Some(Starter {
            program: sys::unix_tool("flatpak").to_string_lossy().to_string(),
            prefix_args: vec!["run".into(), FLATPAK.into()],
        });
    }
    None
}

/// EXE des NoRisk Launchers in einem Ordner (Name je nach Version unterschiedlich)
fn find_exe_in(dir: &Path) -> Option<PathBuf> {
    [
        "noriskclient-launcher-v3.exe",
        "NoRisk Launcher.exe",
        "NoRiskClientLauncher.exe",
    ]
    .iter()
    .map(|c| dir.join(c))
    .find(|p| p.is_file())
}

/// Datenordner des NoRisk Launchers (dort liegen `profiles/`; `meta/app.db` eine Ebene höher auf Windows)
pub fn data_dirs() -> Vec<PathBuf> {
    let h = sys::home_dir();
    let v = if cfg!(windows) {
        sys::env_path("APPDATA")
            .map(|a| vec![a.join("norisk").join("NoRiskClientV3").join("data")])
            .unwrap_or_default()
    } else if cfg!(target_os = "macos") {
        vec![h.join("Library/Application Support/gg.norisk.NoRiskClientV3")]
    } else {
        vec![
            sys::env_path("XDG_DATA_HOME")
                .unwrap_or_else(|| h.join(".local/share"))
                .join("noriskclientv3"),
            h.join(".var/app").join(FLATPAK).join("data/noriskclientv3"),
        ]
    };
    v.into_iter().filter(|p| p.is_dir()).collect()
}

// ----------------------------------------------------------------------
// Profile lesen
// ----------------------------------------------------------------------

fn read_all(paths: &Paths) -> Vec<NoRiskProfile> {
    let mut out = read_db(paths);
    // Standard-Versionen von NoRisk, die (noch) nicht in der Datenbank stehen
    if let Ok(json) = std::fs::read_to_string(paths.root.join("norisk_versions.json")) {
        for p in read_standard(&json, &paths.profiles) {
            if !out.iter().any(|o| o.id == p.id) {
                out.push(p);
            }
        }
    }
    sort_profiles(&mut out);
    out
}

/// Neueste zuerst, dann Name
pub fn sort_profiles(v: &mut [NoRiskProfile]) {
    v.sort_by(|a, b| {
        b.last_played
            .cmp(&a.last_played)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Die Datenbank läuft im WAL-Modus und ist beim Lesen evtl. vom Launcher geöffnet. Damit auch gerade
/// erst angelegte Profile erscheinen und nichts gesperrt wird, wird eine Kopie (inkl. -wal) gelesen –
/// in einem eigenen Ordner pro Aufruf, damit parallele Aufrufe sich nicht stören.
fn read_db(paths: &Paths) -> Vec<NoRiskProfile> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let db = &paths.db;
    // Größenlimit: Profil-Datenbanken sind klein
    if !db.is_file()
        || std::fs::metadata(db)
            .map(|m| m.len() > 256 * 1024 * 1024)
            .unwrap_or(true)
    {
        return Vec::new();
    }
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = sys::local_dir().join(format!("norisk-copy-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    if std::fs::create_dir_all(&tmp).is_err() {
        return Vec::new();
    }
    let copy = tmp.join("app.db");
    let mut out = Vec::new();
    if std::fs::copy(db, &copy).is_ok() {
        let wal = db.with_file_name("app.db-wal");
        if wal.is_file() {
            let _ = std::fs::copy(&wal, tmp.join("app.db-wal"));
        }
        if let Some(conn) = crate::stores::open_db_wal(&copy) {
            out = read_profiles(&conn, &paths.profiles);
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    out
}

/// Profile aus einer geöffneten NoRisk-Datenbank. Ältere Datenbanken ohne die neueren Spalten
/// werden mit der einfachen Abfrage gelesen.
pub fn read_profiles(conn: &rusqlite::Connection, profiles_dir: &Path) -> Vec<NoRiskProfile> {
    type Row = (
        String,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        i64,
        i64,
        i64,
    );
    let full = "SELECT id, name, path, game_version, loader, last_played, group_name, use_shared_minecraft_folder, \
                is_standard_version, playtime_seconds FROM profiles LIMIT 2000";
    let basic = "SELECT id, name, path, game_version, loader, last_played, NULL, 0, 0, 0 FROM profiles LIMIT 2000";
    let rows: Vec<Row> = [full, basic]
        .iter()
        .find_map(|sql| {
            let mut st = conn.prepare(sql).ok()?;
            let rows = st
                .query_map([], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                        r.get(9)?,
                    ))
                })
                .ok()?
                .flatten()
                .collect();
            Some(rows)
        })
        .unwrap_or_default();
    rows.into_iter()
        .filter_map(
            |(id, name, path, gv, loader, last, group, shared, standard, playtime)| {
                make_profile(Raw {
                    id,
                    name,
                    path,
                    game_version: gv,
                    loader,
                    group: group.unwrap_or_default(),
                    shared: shared != 0,
                    standard: standard != 0,
                    last_played: last.map(nanos_to_secs).unwrap_or(0),
                    playtime_secs: playtime.max(0) as u64,
                    profiles_dir,
                })
            },
        )
        .collect()
}

/// Standard-Versionen aus `norisk_versions.json` (`{"profiles": [...]}`)
pub fn read_standard(json: &str, profiles_dir: &Path) -> Vec<NoRiskProfile> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let str_of = |p: &serde_json::Value, k: &str| {
        p.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string()
    };
    v.get("profiles")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .take(500)
        .filter_map(|p| {
            make_profile(Raw {
                id: str_of(p, "id"),
                name: str_of(p, "name"),
                path: str_of(p, "path"),
                game_version: str_of(p, "game_version"),
                loader: str_of(p, "loader"),
                group: str_of(p, "group"),
                shared: p
                    .get("use_shared_minecraft_folder")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
                standard: true,
                last_played: 0,
                playtime_secs: 0,
                profiles_dir,
            })
        })
        .collect()
}

struct Raw<'a> {
    id: String,
    name: String,
    path: String,
    game_version: String,
    loader: String,
    group: String,
    shared: bool,
    standard: bool,
    last_played: u64,
    playtime_secs: u64,
    profiles_dir: &'a Path,
}

fn make_profile(r: Raw) -> Option<NoRiskProfile> {
    if !valid_profile_id(&r.id) || r.name.trim().is_empty() {
        return None;
    }
    let game_version: String = r
        .game_version
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .take(24)
        .collect();
    let group: String = r
        .group
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(40)
        .collect();
    Some(NoRiskProfile {
        dir: instance_dir(r.profiles_dir, &r.path, &group, r.shared, &game_version),
        id: r.id.to_ascii_lowercase(),
        name: r
            .name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(80)
            .collect(),
        loader: r
            .loader
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_lowercase())
            .take(12)
            .collect(),
        game_version,
        group,
        last_played: r.last_played,
        playtime_secs: r.playtime_secs,
        standard: r.standard,
        shared: r.shared,
    })
}

/// NoRisk speichert `last_played` in Nanosekunden. Alte Einträge in Sekunden/Millisekunden bleiben lesbar.
pub fn nanos_to_secs(raw: i64) -> u64 {
    let v = raw.max(0) as u64;
    match v {
        0..=99_999_999_999 => v,                           // Sekunden
        100_000_000_000..=99_999_999_999_999 => v / 1_000, // Millisekunden
        100_000_000_000_000..=99_999_999_999_999_999 => v / 1_000_000,
        _ => v / 1_000_000_000,
    }
}

/// SICHERHEIT: Die ID wird als Argument an den NoRisk Launcher übergeben → nur UUIDs
pub fn valid_profile_id(id: &str) -> bool {
    id.len() == 36
        && id.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

/// Spielordner eines Profils – wie im NoRisk Launcher:
/// geteilter Ordner (außer Gruppen "server"/"modpacks") → `noriskclient/{legacy,new}` bzw.
/// `groups/<gruppe>`, sonst `<profiles>/<path>`. Immer innerhalb des Profile-Ordners.
pub fn instance_dir(
    profiles_dir: &Path,
    path: &str,
    group: &str,
    shared: bool,
    game_version: &str,
) -> PathBuf {
    let g = group.trim().to_lowercase();
    if shared && !g.is_empty() && g != "server" && g != "modpacks" {
        if matches!(g.as_str(), "nrc" | "noriskclient" | "norisk client") {
            return profiles_dir
                .join("noriskclient")
                .join(if is_legacy_version(game_version) {
                    "legacy"
                } else {
                    "new"
                });
        }
        let s = sanitize_group(&g);
        return if s.is_empty() {
            PathBuf::new()
        } else {
            profiles_dir.join("groups").join(s)
        };
    }
    profile_dir(profiles_dir, path)
}

/// Pfad eines Profils (`path`-Spalte) – nur innerhalb des Profile-Ordners (kein "..", kein Laufwerk)
pub fn profile_dir(profiles_dir: &Path, path: &str) -> PathBuf {
    let clean = path.trim().replace('\\', "/");
    let parts: Vec<&str> = clean
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();
    let ok = !parts.is_empty()
        && !clean.starts_with('/')
        && !parts.iter().any(|p| *p == ".." || p.contains(':'));
    if ok {
        parts
            .iter()
            .fold(profiles_dir.to_path_buf(), |acc, p| acc.join(p))
    } else {
        PathBuf::new()
    }
}

/// Gruppenname als Ordnername (wie `sanitize_filename`: verbotene Zeichen weg, keine Punkte/Leerzeichen am Ende)
pub fn sanitize_group(g: &str) -> String {
    let s: String = g
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '/' | '\\' | '?' | '<' | '>' | ':' | '*' | '|' | '"')
        })
        .collect();
    let s = s.trim_end_matches(['.', ' ']).to_string();
    if s == "." || s == ".." {
        String::new()
    } else {
        s
    }
}

/// Minecraft 1.12.x und älter
pub fn is_legacy_version(v: &str) -> bool {
    let mut it = v.split('.');
    match (
        it.next().and_then(|x| x.parse::<u32>().ok()),
        it.next().and_then(|x| x.parse::<u32>().ok()),
    ) {
        (Some(0), _) => true,
        (Some(1), Some(minor)) => minor <= 12,
        _ => false,
    }
}

// ----------------------------------------------------------------------
// Gruppen für die Übersicht
// ----------------------------------------------------------------------

/// Art einer Gruppe – bestimmt Reihenfolge und Beschriftung
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupKind {
    NoRiskClient,
    /// Eigene Profile ohne Gruppe
    Own,
    /// Eigene Gruppe
    Custom,
    Modpacks,
    Server,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub kind: GroupKind,
    /// Anzeigename (nur bei `Custom` – sonst übersetzt die Oberfläche)
    pub label: String,
    pub profiles: Vec<NoRiskProfile>,
}

pub fn group_kind(group: &str) -> GroupKind {
    match group.trim().to_lowercase().as_str() {
        "" => GroupKind::Own,
        "nrc" | "noriskclient" | "norisk client" => GroupKind::NoRiskClient,
        "modpacks" | "modpack" => GroupKind::Modpacks,
        "server" | "servers" => GroupKind::Server,
        _ => GroupKind::Custom,
    }
}

/// Profile nach Gruppe (Groß-/Kleinschreibung egal). Gruppen mit zuletzt gespielten Profilen zuerst;
/// nie gespielte Gruppen in fester Reihenfolge. Innerhalb: neueste zuerst.
pub fn groups(profiles: &[NoRiskProfile]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for p in profiles {
        let kind = group_kind(&p.group);
        let label = if kind == GroupKind::Custom {
            p.group.clone()
        } else {
            String::new()
        };
        match out
            .iter_mut()
            .find(|g| g.kind == kind && g.label.to_lowercase() == label.to_lowercase())
        {
            Some(g) => g.profiles.push(p.clone()),
            None => out.push(Group {
                kind,
                label,
                profiles: vec![p.clone()],
            }),
        }
    }
    for g in out.iter_mut() {
        sort_profiles(&mut g.profiles);
    }
    let latest = |g: &Group| g.profiles.iter().map(|p| p.last_played).max().unwrap_or(0);
    out.sort_by(|a, b| {
        latest(b)
            .cmp(&latest(a))
            .then(a.kind.cmp(&b.kind))
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
    out
}

/// Wie viele Profile teilen sich diesen Spielordner?
pub fn sharing(profiles: &[NoRiskProfile], dir: &Path) -> usize {
    if dir.as_os_str().is_empty() {
        return 0;
    }
    profiles.iter().filter(|p| p.dir == dir).count()
}

// ----------------------------------------------------------------------
// Befehle an den NoRisk Launcher
// ----------------------------------------------------------------------

/// `launch --profile <uuid>` – startet das Spiel (auch wenn der Launcher schon offen ist)
pub fn launch_args(starter: &Starter, profile: &str) -> Option<Vec<String>> {
    if !valid_profile_id(profile) {
        return None;
    }
    let mut a = starter.prefix_args.clone();
    a.extend([
        "launch".to_string(),
        "--profile".to_string(),
        profile.to_string(),
    ]);
    Some(a)
}

/// Was im NoRisk Launcher geöffnet werden soll
#[derive(Clone, Debug, PartialEq)]
pub enum Open {
    Home,
    Profile(String),
    Create,
}

/// `open [--profile <uuid> | --create] --theme <id>` – öffnet bzw. holt das Launcher-Fenster nach vorn.
/// Ältere Launcher kennen `open` nicht und öffnen dann einfach die Startseite.
pub fn open_args(starter: &Starter, what: &Open) -> Vec<String> {
    let mut a = starter.prefix_args.clone();
    a.push("open".into());
    match what {
        Open::Profile(id) if valid_profile_id(id) => {
            a.extend(["--profile".to_string(), id.clone()])
        }
        Open::Create => a.push("--create".into()),
        _ => {}
    }
    a.extend(["--theme".to_string(), THEME.to_string()]);
    a
}

// ----------------------------------------------------------------------
// Bilder für die Kachel
// ----------------------------------------------------------------------

/// Selbst gezeichnete Bilder der NoRisk-Kachel → (cover, hero, header)
pub fn artwork() -> Option<(PathBuf, PathBuf, PathBuf)> {
    super::art::artwork()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "0b5c1a8e-4f1e-4c55-9d6b-2b9f0c3a1e22";
    const B: &str = "1b5c1a8e-4f1e-4c55-9d6b-2b9f0c3a1e22";
    const C: &str = "2b5c1a8e-4f1e-4c55-9d6b-2b9f0c3a1e22";
    const D: &str = "3b5c1a8e-4f1e-4c55-9d6b-2b9f0c3a1e22";

    fn db() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        c.execute_batch(&format!(
            "CREATE TABLE profiles(id TEXT, name TEXT, path TEXT, game_version TEXT, loader TEXT, last_played INTEGER,
               group_name TEXT, use_shared_minecraft_folder INTEGER, is_standard_version INTEGER, playtime_seconds INTEGER);
             INSERT INTO profiles VALUES ('{A}','Survival','survival','1.21.4','fabric',1761763378621243600,NULL,0,0,120),
               ('kaputt; rm -rf','Böse','x','1.0','vanilla',NULL,NULL,0,0,0),
               ('{B}','Ausbruch','../../etc','1.20.1','forge',NULL,'SERVER',0,0,0),
               ('{C}','NRC 1.8.9','noriskclient/legacy','1.8.9','forge',NULL,'NORISK CLIENT',1,1,0),
               ('{D}','Pack','Pack','1.21.1','neoforge',1700000000000000000,'My Friends',1,0,0);"
        ))
        .unwrap();
        c
    }

    #[test]
    fn reads_profiles() {
        let list = read_profiles(&db(), Path::new("/p"));
        assert_eq!(list.len(), 4);
        assert_eq!(list[0].name, "Survival");
        assert_eq!(list[0].dir, PathBuf::from("/p/survival"));
        assert_eq!(list[0].last_played, 1_761_763_378);
        assert_eq!(list[0].playtime_secs, 120);
        assert_eq!(list[1].dir, PathBuf::new());
        assert_eq!(list[2].dir, PathBuf::from("/p/noriskclient/legacy"));
        assert!(list[2].standard && list[2].shared);
        assert_eq!(list[3].dir, PathBuf::from("/p/groups/my friends"));
        assert!(!valid_profile_id("--profile"));
        assert!(!valid_profile_id("------------------------------------"));
    }

    #[test]
    fn old_schema() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        c.execute_batch(&format!(
            "CREATE TABLE profiles(id TEXT, name TEXT, path TEXT, game_version TEXT, loader TEXT, last_played INTEGER);
             INSERT INTO profiles VALUES ('{A}','Alt','alt','1.20','vanilla',1700000000);"
        ))
        .unwrap();
        let list = read_profiles(&c, Path::new("/p"));
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].last_played, 1_700_000_000);
        assert_eq!(list[0].group, "");
    }

    #[test]
    fn standard_versions() {
        let json = format!(
            r#"{{"profiles":[{{"id":"{A}","name":"NRC 1.21.4","path":"noriskclient/new","game_version":"1.21.4","loader":"fabric","group":"NORISK CLIENT","use_shared_minecraft_folder":false}},{{"id":"x","name":"y"}}]}}"#
        );
        let list = read_standard(&json, Path::new("/p"));
        assert_eq!(list.len(), 1);
        assert!(list[0].standard);
        assert_eq!(list[0].dir, PathBuf::from("/p/noriskclient/new"));
        assert!(read_standard("kaputt", Path::new("/p")).is_empty());
    }

    #[test]
    fn instance_paths() {
        let p = Path::new("/p");
        assert_eq!(
            instance_dir(p, "a/b", "", true, "1.21"),
            PathBuf::from("/p/a/b")
        );
        assert_eq!(
            instance_dir(p, "a\\b", "", false, "1.21"),
            PathBuf::from("/p/a/b")
        );
        assert_eq!(
            instance_dir(p, "x", "NRC", true, "1.8.9"),
            PathBuf::from("/p/noriskclient/legacy")
        );
        assert_eq!(
            instance_dir(p, "x", "norisk client", true, "1.21"),
            PathBuf::from("/p/noriskclient/new")
        );
        assert_eq!(
            instance_dir(p, "x", "Server", true, "1.21"),
            PathBuf::from("/p/x")
        );
        assert_eq!(
            instance_dir(p, "x", "A/B:C", true, "1.21"),
            PathBuf::from("/p/groups/abc")
        );
        assert_eq!(instance_dir(p, "x", "..", true, "1.21"), PathBuf::new());
        assert_eq!(profile_dir(p, "C:/Windows"), PathBuf::new());
        assert_eq!(profile_dir(p, "/etc"), PathBuf::new());
        assert_eq!(profile_dir(p, "./a//b"), PathBuf::from("/p/a/b"));
        assert_eq!(profile_dir(p, ""), PathBuf::new());
        assert!(
            is_legacy_version("1.12.2")
                && !is_legacy_version("1.13")
                && !is_legacy_version("26.1")
                && !is_legacy_version("24w10a")
        );
    }

    #[test]
    fn timestamps() {
        assert_eq!(nanos_to_secs(1_761_763_378_621_243_600), 1_761_763_378);
        assert_eq!(nanos_to_secs(1_761_763_378_621), 1_761_763_378);
        assert_eq!(nanos_to_secs(1_761_763_378), 1_761_763_378);
        assert_eq!(nanos_to_secs(-5), 0);
    }

    #[test]
    fn grouping() {
        let mk = |id: &str, g: &str, last: u64| NoRiskProfile {
            id: id.into(),
            name: id.into(),
            group: g.into(),
            last_played: last,
            ..Default::default()
        };
        let list = vec![
            mk("a", "MODPACKS", 0),
            mk("b", "", 50),
            mk("c", "modpacks", 10),
            mk("d", "NORISK CLIENT", 0),
            mk("e", "Friends", 0),
            mk("f", "friends", 0),
            mk("g", "SERVER", 0),
        ];
        let g = groups(&list);
        let kinds: Vec<GroupKind> = g.iter().map(|g| g.kind).collect();
        assert_eq!(
            kinds,
            vec![
                GroupKind::Own,
                GroupKind::Modpacks,
                GroupKind::NoRiskClient,
                GroupKind::Custom,
                GroupKind::Server
            ]
        );
        assert_eq!(
            g[1].profiles
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "a"]
        );
        assert_eq!(g[3].profiles.len(), 2);
        assert_eq!(g[3].label, "Friends");
    }

    #[test]
    fn commands() {
        let s = Starter {
            program: "nrc".into(),
            prefix_args: vec!["run".into()],
        };
        assert_eq!(
            launch_args(&s, A).unwrap(),
            vec!["run", "launch", "--profile", A]
        );
        assert!(launch_args(&s, "--evil").is_none());
        assert_eq!(
            open_args(&s, &Open::Profile(A.into())),
            vec!["run", "open", "--profile", A, "--theme", THEME]
        );
        assert_eq!(
            open_args(&s, &Open::Profile("--x".into())),
            vec!["run", "open", "--theme", THEME]
        );
        assert_eq!(
            open_args(&s, &Open::Create),
            vec!["run", "open", "--create", "--theme", THEME]
        );
    }

    #[test]
    fn config() {
        assert_eq!(custom_game_dir(r#"{"custom_game_directory":null}"#), None);
        assert_eq!(
            custom_game_dir(r#"{"custom_game_directory":"relative/x"}"#),
            None
        );
        let tmp = std::env::temp_dir();
        let j = serde_json::json!({ "custom_game_directory": tmp.to_string_lossy() }).to_string();
        assert_eq!(custom_game_dir(&j), Some(tmp));
    }
}
