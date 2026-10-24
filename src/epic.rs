//! Epic-Games-Anbindung: liest die Installations-Manifeste des Epic Games Launchers.

use crate::model::{Art, Game, LaunchTarget, Source, StoreStatus};
use crate::sys::{self, Hive};
use std::fs;
use std::path::{Path, PathBuf};

pub fn manifests_dir() -> Option<PathBuf> {
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

pub fn launcher_installed() -> bool {
    sys::protocol_registered("com.epicgames.launcher")
}

pub struct EpicScan {
    pub games: Vec<Game>,
    pub status: StoreStatus,
}

fn jstr<'a>(v: &'a serde_json::Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("")
}

pub fn scan() -> EpicScan {
    let mut status = StoreStatus::default();
    let Some(dir) = manifests_dir() else {
        status.found = launcher_installed();
        status.note = Some(if status.found {
            crate::i18n::t("store.epic_no_games")
        } else {
            crate::i18n::t("store.epic_not_found")
        });
        return EpicScan { games: Vec::new(), status };
    };
    status.found = true;
    status.path = Some(dir.to_string_lossy().to_string());
    status.note = Some(crate::i18n::t("store.epic_note"));

    let mut games: Vec<Game> = Vec::new();
    let Ok(rd) = fs::read_dir(&dir) else {
        return EpicScan { games, status };
    };
    for e in rd.flatten() {
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
        let install = jstr(&v, "InstallLocation").replace('/', "\\");
        let exe_rel = jstr(&v, "LaunchExecutable").replace('/', "\\");
        let exe = if !install.is_empty() && !exe_rel.is_empty() {
            Some(Path::new(&install).join(&exe_rel).to_string_lossy().to_string())
        } else {
            None
        };
        let uri = format!(
            "com.epicgames.launcher://apps/{}%3A{}%3A{}?action=launch&silent=true",
            sys::url_encode(ns),
            sys::url_encode(item),
            sys::url_encode(app_name)
        );
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
            art: Art::default(),
            size_bytes: size.filter(|s| *s > 0),
            playtime_min: None,
            steam_appid: None,
        });
    }
    status.installed = games.len();
    EpicScan { games, status }
}
