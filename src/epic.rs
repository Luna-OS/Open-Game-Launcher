//! Epic-Games-Anbindung: liest die Installations-Manifeste des Epic Games Launchers.

use crate::model::{Art, Game, LaunchTarget, Source, StoreStatus};
use crate::sys;
#[cfg(windows)]
use crate::sys::Hive;
use std::fs;
use std::path::{Path, PathBuf};

pub fn manifests_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let candidates = [
            sys::reg_str(Hive::LocalMachine, "SOFTWARE\\WOW6432Node\\Epic Games\\EpicGamesLauncher", "AppDataPath"),
            sys::reg_str(Hive::LocalMachine, "SOFTWARE\\Epic Games\\EpicGamesLauncher", "AppDataPath"),
        ];
        for c in candidates.into_iter().flatten() {
            let p = PathBuf::from(c).join("Manifests");
            if p.exists() {
                return Some(p);
            }
        }
        let p = sys::env_path("ProgramData")
            .unwrap_or_else(|| PathBuf::from("C:\\ProgramData"))
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data")
            .join("Manifests");
        p.exists().then_some(p)
    }
    #[cfg(target_os = "macos")]
    {
        let p = sys::home_dir().join("Library/Application Support/Epic/EpicGamesLauncher/Data/Manifests");
        p.exists().then_some(p)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        None
    }
}

/// Ist der Epic Games Launcher da? (Linux: Heroic Games Launcher)
pub fn launcher_installed() -> bool {
    #[cfg(windows)]
    {
        sys::protocol_registered("com.epicgames.launcher")
    }
    #[cfg(target_os = "macos")]
    {
        ["/Applications/Epic Games Launcher.app"]
            .iter()
            .map(PathBuf::from)
            .chain(std::iter::once(sys::home_dir().join("Applications/Epic Games Launcher.app")))
            .any(|p| p.exists())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        !heroic_dirs().is_empty()
    }
}

// ----------------------------------------------------------------------
// Linux: Epic-Spiele über den Heroic Games Launcher
// ----------------------------------------------------------------------

/// Konfigurationsordner von Heroic (normal und Flatpak)
pub fn heroic_dirs() -> Vec<PathBuf> {
    let h = sys::home_dir();
    let mut out: Vec<PathBuf> = Vec::new();
    // XDG_CONFIG_HOME zeigt im Flatpak auf die Sandbox → zusätzlich immer ~/.config prüfen
    for p in [
        sys::env_path("XDG_CONFIG_HOME").map(|c| c.join("heroic")),
        Some(h.join(".config/heroic")),
        Some(h.join(".var/app/com.heroicgameslauncher.hgl/config/heroic")),
    ]
    .into_iter()
    .flatten()
    {
        if p.join("legendaryConfig").exists() || p.join("store_cache").exists() {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

pub fn heroic_uri(app_name: &str) -> String {
    format!("heroic://launch?appName={}&runner=legendary", sys::url_encode(app_name))
}

/// Heroic: installed.json (installierte Spiele) + legendary_library.json (alle Spiele)
pub fn parse_heroic(installed_json: &str, library_json: &str, show_uninstalled: bool) -> Vec<Game> {
    let mut games: Vec<Game> = Vec::new();
    let installed: serde_json::Value = serde_json::from_str(installed_json).unwrap_or(serde_json::Value::Null);
    let library: serde_json::Value = serde_json::from_str(library_json).unwrap_or(serde_json::Value::Null);
    let lib_list: Vec<serde_json::Value> = library.get("library").and_then(|l| l.as_array()).cloned().unwrap_or_default();
    let art_for = |app: &str| -> Art {
        let Some(e) = lib_list.iter().find(|e| jstr(e, "app_name") == app) else { return Art::default() };
        let pick = |k: &str| -> Vec<String> {
            let u = jstr(e, k);
            if u.starts_with("https://") { vec![u.to_string()] } else { Vec::new() }
        };
        Art { cover: pick("art_square"), hero: pick("art_cover"), logo: pick("art_logo"), icon: pick("art_square"), header: pick("art_cover") }
    };
    if let Some(map) = installed.as_object() {
        for (app, v) in map {
            if v.get("is_dlc").and_then(|b| b.as_bool()) == Some(true) {
                continue;
            }
            let title = jstr(v, "title").trim().to_string();
            if title.is_empty() {
                continue;
            }
            let dir = jstr(v, "install_path").to_string();
            let exe_rel = jstr(v, "executable").to_string();
            games.push(Game {
                id: format!("epic:{app}"),
                name: title,
                source: Source::Epic,
                installed: true,
                exe: (!dir.is_empty() && !exe_rel.is_empty()).then(|| Path::new(&dir).join(&exe_rel).to_string_lossy().to_string()),
                install_dir: (!dir.is_empty()).then_some(dir),
                launch: LaunchTarget::Epic(heroic_uri(app)),
                art: art_for(app),
                size_bytes: v.get("install_size").and_then(|s| s.as_u64()).filter(|s| *s > 0),
                playtime_min: None,
                steam_appid: None,
            });
        }
    }
    if show_uninstalled {
        for e in &lib_list {
            let app = jstr(e, "app_name");
            let title = jstr(e, "title").trim().to_string();
            if app.is_empty() || title.is_empty() || games.iter().any(|g| g.id == format!("epic:{app}")) {
                continue;
            }
            if e.get("install").and_then(|i| i.get("is_dlc")).and_then(|b| b.as_bool()) == Some(true) {
                continue;
            }
            games.push(Game {
                id: format!("epic:{app}"),
                name: title,
                source: Source::Epic,
                installed: false,
                install_dir: None,
                exe: None,
                launch: LaunchTarget::EpicInstall(heroic_uri(app)),
                art: art_for(app),
                size_bytes: None,
                playtime_min: None,
                steam_appid: None,
            });
        }
    }
    games
}

fn scan_heroic(show_uninstalled: bool) -> EpicScan {
    let mut status = StoreStatus::default();
    let dirs = heroic_dirs();
    let Some(dir) = dirs.first() else {
        status.note = Some(crate::i18n::t("store.heroic_not_found"));
        return EpicScan { games: Vec::new(), status };
    };
    status.found = true;
    status.path = Some(dir.to_string_lossy().to_string());
    let installed = fs::read_to_string(dir.join("legendaryConfig/legendary/installed.json")).unwrap_or_default();
    let library = fs::read_to_string(dir.join("store_cache/legendary_library.json")).unwrap_or_default();
    let games = parse_heroic(&installed, &library, show_uninstalled);
    status.installed = games.iter().filter(|g| g.installed).count();
    status.owned = games.len();
    if games.is_empty() {
        status.note = Some(crate::i18n::t("store.heroic_note"));
    }
    EpicScan { games, status }
}

pub struct EpicScan {
    pub games: Vec<Game>,
    pub status: StoreStatus,
}

fn jstr<'a>(v: &'a serde_json::Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("")
}

/// Epic-Datenordner (…\EpicGamesLauncher\Data)
fn data_dir() -> Option<PathBuf> {
    manifests_dir().and_then(|m| m.parent().map(|p| p.to_path_buf())).or_else(|| {
        if cfg!(target_os = "macos") {
            let p = sys::home_dir().join("Library/Application Support/Epic/EpicGamesLauncher/Data");
            return p.exists().then_some(p);
        }
        let p = sys::env_path("ProgramData")
            .unwrap_or_else(|| PathBuf::from("C:\\ProgramData"))
            .join("Epic")
            .join("EpicGamesLauncher")
            .join("Data");
        p.exists().then_some(p)
    })
}

/// Ein Spiel aus Epics lokalem Katalog-Cache (= deine Epic-Bibliothek)
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogGame {
    pub app_name: String,
    pub namespace: String,
    pub item_id: String,
    pub title: String,
    pub art: Art,
}

fn key_image(v: &serde_json::Value, types: &[&str]) -> Vec<String> {
    let Some(list) = v.get("keyImages").and_then(|k| k.as_array()) else { return Vec::new() };
    let mut out = Vec::new();
    for t in types {
        for img in list {
            if jstr(img, "type") == *t {
                let url = jstr(img, "url");
                if !url.is_empty() {
                    out.push(url.to_string());
                }
            }
        }
    }
    out
}

/// Liest `Catalog/catcache.bin` (Base64 → JSON-Liste der Katalogeinträge).
pub fn parse_catalog(raw: &str) -> Vec<CatalogGame> {
    let Some(bytes) = crate::util::base64_decode(raw.trim()) else { return Vec::new() };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return Vec::new() };
    let Some(items) = v.as_array() else { return Vec::new() };
    let mut out: Vec<CatalogGame> = Vec::new();
    for it in items {
        // DLCs haben ein "mainGameItem"
        if it.get("mainGameItem").map(|m| !m.is_null()).unwrap_or(false) {
            continue;
        }
        let cats: Vec<String> = it
            .get("categories")
            .and_then(|c| c.as_array())
            .map(|a| a.iter().map(|c| jstr(c, "path").to_lowercase()).collect())
            .unwrap_or_default();
        if !cats.iter().any(|c| c == "games") {
            continue;
        }
        if cats.iter().any(|c| matches!(c.as_str(), "addons" | "engines" | "digitalextras" | "software" | "plugins")) {
            continue;
        }
        let app_name = it
            .get("releaseInfo")
            .and_then(|r| r.as_array())
            .and_then(|r| r.first())
            .map(|r| jstr(r, "appId").to_string())
            .unwrap_or_default();
        let title = jstr(it, "title").trim().to_string();
        if app_name.is_empty() || title.is_empty() || out.iter().any(|g| g.app_name == app_name) {
            continue;
        }
        let art = Art {
            cover: key_image(it, &["DieselGameBoxTall", "OfferImageTall", "Thumbnail"]),
            hero: key_image(it, &["DieselGameBox", "OfferImageWide", "DieselStoreFrontWide"]),
            logo: key_image(it, &["DieselGameBoxLogo"]),
            icon: key_image(it, &["DieselGameBoxTall"]).into_iter().map(|u| format!("{u}?w=64&h=64&resize=1")).collect(),
            header: key_image(it, &["DieselGameBox", "OfferImageWide"]),
        };
        out.push(CatalogGame {
            app_name,
            namespace: jstr(it, "namespace").to_string(),
            item_id: jstr(it, "id").to_string(),
            title,
            art,
        });
    }
    out
}

fn read_catalog() -> Vec<CatalogGame> {
    let Some(dir) = data_dir() else { return Vec::new() };
    let path = dir.join("Catalog").join("catcache.bin");
    match fs::read(&path) {
        Ok(bytes) if bytes.len() < 128 * 1024 * 1024 => parse_catalog(&String::from_utf8_lossy(&bytes)),
        _ => Vec::new(),
    }
}

pub fn epic_uri(ns: &str, item: &str, app: &str, action: &str) -> String {
    format!(
        "com.epicgames.launcher://apps/{}%3A{}%3A{}?action={action}&silent=true",
        sys::url_encode(ns),
        sys::url_encode(item),
        sys::url_encode(app)
    )
}

pub fn scan(show_uninstalled: bool) -> EpicScan {
    if cfg!(not(any(windows, target_os = "macos"))) {
        return scan_heroic(show_uninstalled);
    }
    let mut status = StoreStatus::default();
    let mut catalog = read_catalog();
    // Mit verbundenem Epic-Konto: komplette Online-Bibliothek (zwischengespeichert)
    for o in crate::epicacct::load_cache() {
        if !catalog.iter().any(|c| c.app_name == o.app_name) {
            catalog.push(CatalogGame {
                app_name: o.app_name.clone(),
                namespace: o.namespace.clone(),
                item_id: o.item_id.clone(),
                title: o.title.clone(),
                art: o.art(),
            });
        }
    }
    let Some(dir) = manifests_dir() else {
        status.found = launcher_installed();
        status.note = Some(if status.found {
            crate::i18n::t("store.epic_no_games")
        } else {
            crate::i18n::t("store.epic_not_found")
        });
        let mut games = Vec::new();
        add_uninstalled(&mut games, &catalog, show_uninstalled);
        status.owned = games.len();
        return EpicScan { games, status };
    };
    status.found = true;
    status.path = Some(dir.to_string_lossy().to_string());
    if catalog.is_empty() {
        status.note = Some(crate::i18n::t("store.epic_note"));
    }

    let mut games: Vec<Game> = Vec::new();
    for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("item")) != Some(true) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&p) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };

        let app_name = jstr(&v, "AppName");
        let main_app = jstr(&v, "MainGameAppName");
        // DLCs überspringen
        if !main_app.is_empty() && main_app != app_name {
            continue;
        }
        if v.get("bIsIncompleteInstall").and_then(|b| b.as_bool()) == Some(true) {
            continue;
        }
        let categories: Vec<String> = v
            .get("AppCategories")
            .and_then(|c| c.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_lowercase())).collect())
            .unwrap_or_default();
        if categories.iter().any(|c| c == "plugins" || c == "engines" || c == "addons" || c == "digitalextras") {
            continue;
        }
        let name = jstr(&v, "DisplayName").trim().to_string();
        if name.is_empty() || name.starts_with("Unreal Engine") {
            continue;
        }
        let ns = jstr(&v, "CatalogNamespace");
        let item = jstr(&v, "CatalogItemId");
        let install = sys::native_path(jstr(&v, "InstallLocation"));
        let exe_rel = sys::native_path(jstr(&v, "LaunchExecutable"));
        let exe = if !install.is_empty() && !exe_rel.is_empty() {
            Some(Path::new(&install).join(&exe_rel).to_string_lossy().to_string())
        } else {
            None
        };
        let uri = epic_uri(ns, item, app_name, "launch");
        let id = format!("epic:{app_name}");
        if games.iter().any(|g| g.id == id) {
            continue;
        }
        let size = v.get("InstallSize").and_then(|s| s.as_u64());
        games.push(Game {
            id,
            name,
            source: Source::Epic,
            installed: true,
            install_dir: (!install.is_empty()).then_some(install),
            exe,
            launch: LaunchTarget::Epic(uri),
            art: catalog.iter().find(|c| c.app_name == app_name).map(|c| c.art.clone()).unwrap_or_default(),
            size_bytes: size.filter(|s| *s > 0),
            playtime_min: None,
            steam_appid: None,
        });
    }
    status.installed = games.len();
    add_uninstalled(&mut games, &catalog, show_uninstalled);
    status.owned = catalog.len().max(games.len());
    EpicScan { games, status }
}

/// Nicht installierte Spiele aus dem Katalog hinzufügen
fn add_uninstalled(games: &mut Vec<Game>, catalog: &[CatalogGame], show: bool) {
    if !show {
        return;
    }
    for c in catalog {
        let id = format!("epic:{}", c.app_name);
        if games.iter().any(|g| g.id == id) {
            continue;
        }
        games.push(Game {
            id,
            name: c.title.clone(),
            source: Source::Epic,
            installed: false,
            install_dir: None,
            exe: None,
            launch: LaunchTarget::EpicInstall(epic_uri(&c.namespace, &c.item_id, &c.app_name, "install")),
            art: c.art.clone(),
            size_bytes: None,
            playtime_min: None,
            steam_appid: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_filters_dlc_and_engines() {
        let json = r#"[
          {"id":"i1","namespace":"ns1","title":"Cool Game","categories":[{"path":"games"},{"path":"applications"}],
           "releaseInfo":[{"appId":"CoolApp"}],
           "keyImages":[{"type":"DieselGameBoxTall","url":"https://cdn1.epicgames.com/tall.jpg"},{"type":"DieselGameBoxLogo","url":"https://x/logo.png"}]},
          {"id":"i2","namespace":"ns1","title":"Cool DLC","categories":[{"path":"games"}],"mainGameItem":{"id":"i1"},"releaseInfo":[{"appId":"Dlc"}]},
          {"id":"i3","namespace":"ue","title":"Unreal Engine","categories":[{"path":"engines"}],"releaseInfo":[{"appId":"UE_5"}]},
          {"id":"i4","namespace":"ns4","title":"No App","categories":[{"path":"games"}]}
        ]"#;
        let b64 = {
            const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let d = json.as_bytes();
            let mut s = String::new();
            for ch in d.chunks(3) {
                let n = ((ch[0] as u32) << 16) | ((*ch.get(1).unwrap_or(&0) as u32) << 8) | *ch.get(2).unwrap_or(&0) as u32;
                for i in 0..4 {
                    if i <= ch.len() {
                        s.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
                    } else {
                        s.push('=');
                    }
                }
            }
            s
        };
        let games = parse_catalog(&b64);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].app_name, "CoolApp");
        assert_eq!(games[0].art.cover[0], "https://cdn1.epicgames.com/tall.jpg");
        assert_eq!(games[0].art.logo.len(), 1);
        assert!(epic_uri("a b", "c", "d", "install").ends_with("?action=install&silent=true"));
        assert!(parse_catalog("kaputt!!").is_empty());
    }
}
