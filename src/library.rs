//! Bibliothek: führt Steam, Epic und eigene Spiele zusammen.

use crate::model::{normalize_name, Art, CustomGame, Game, LaunchTarget, Settings, Source, StoreStatus};
use crate::problem::Problem;
use crate::{epic, icons, roblox, steam, sys};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ScanResult {
    pub games: Vec<Game>,
    pub steam: StoreStatus,
    pub epic: StoreStatus,
    pub roblox: StoreStatus,
    pub warnings: Vec<Problem>,
}

/// Komplette Bibliothek einlesen (läuft im Hintergrund-Thread).
pub fn scan(settings: &Settings) -> ScanResult {
    let mut result = ScanResult::default();

    if settings.steam_enabled {
        let s = steam::scan(&settings.steam_api_key, &settings.steam_id, settings.steam_show_uninstalled);
        result.games.extend(s.games);
        result.steam = s.status;
        if let Some(w) = s.warning {
            result.warnings.push(w);
        }
    } else {
        result.steam.note = Some(crate::i18n::t("store.steam_off"));
    }

    if settings.epic_enabled {
        let e = epic::scan(settings.epic_show_uninstalled);
        result.games.extend(e.games);
        result.epic = e.status;
    } else {
        result.epic.note = Some(crate::i18n::t("store.epic_off"));
    }

    if settings.roblox_enabled {
        let (games, status) = roblox::scan(&settings.roblox_games);
        result.games.extend(games);
        result.roblox = status;
    } else {
        result.roblox.note = Some(crate::i18n::t("roblox.off"));
    }

    for g in result.games.iter_mut() {
        match g.source {
            Source::Epic => {
                if let Some(exe) = &g.exe {
                    if let Some(icon) = icons::cached_icon(Path::new(exe)) {
                        g.art.icon.insert(0, sys::file_url(&icon));
                    }
                }
                apply_lookup(g, &settings.art_lookup);
            }
            Source::Steam => {
                // Kein Icon im Steam-Cache? Dann aus der Spiel-EXE holen.
                if g.art.icon.is_empty() && g.installed {
                    if let Some(dir) = &g.install_dir {
                        if let Some(exe) = icons::guess_main_exe(Path::new(dir), &g.name) {
                            if let Some(icon) = icons::cached_icon(&exe) {
                                g.art.icon.push(sys::file_url(&icon));
                            }
                        }
                    }
                }
            }
            Source::Roblox | Source::Custom => {}
        }
    }

    for c in &settings.custom_games {
        result.games.push(custom_game(c, &settings.art_lookup, true));
    }
    result
}

/// Wandelt ein eigenes Spiel in einen Bibliothekseintrag um.
pub fn custom_game(c: &CustomGame, lookup: &BTreeMap<String, u32>, with_icon: bool) -> Game {
    let mut art = Art::default();
    if !c.cover.trim().is_empty() {
        art.cover.push(sys::file_url(Path::new(c.cover.trim())));
    }
    let exe_path = Path::new(&c.exe);
    if with_icon {
        if let Some(icon) = icons::cached_icon(exe_path) {
            art.icon.push(sys::file_url(&icon));
        }
    }
    let mut g = Game {
        id: c.id.clone(),
        name: c.name.clone(),
        source: Source::Custom,
        installed: true,
        install_dir: exe_path.parent().map(|p| p.to_string_lossy().to_string()),
        exe: Some(c.exe.clone()),
        launch: LaunchTarget::Custom,
        art,
        size_bytes: None,
        playtime_min: None,
        steam_appid: None,
    };
    apply_lookup(&mut g, lookup);
    g
}

/// Hängt Steam-Store-Bilder an, wenn wir das Spiel dort gefunden haben.
pub fn apply_lookup(g: &mut Game, lookup: &BTreeMap<String, u32>) {
    if let Some(&appid) = lookup.get(&normalize_name(&g.name)) {
        if appid != 0 {
            let cdn = steam::cdn_art(appid);
            g.art.cover.extend(cdn.cover);
            g.art.hero.extend(cdn.hero);
            g.art.logo.extend(cdn.logo);
            g.art.header.extend(cdn.header);
        }
    }
}

/// Sucht für Epic- und eigene Spiele passende Cover im Steam-Store (Hintergrund).
/// Gibt neue Einträge für den Cache zurück: normalisierter Name → AppID (0 = nichts gefunden).
pub fn lookup_missing_art(names: Vec<String>, known: &BTreeMap<String, u32>) -> BTreeMap<String, u32> {
    let mut found = BTreeMap::new();
    let todo: Vec<String> = names
        .into_iter()
        .filter(|n| !known.contains_key(&normalize_name(n)))
        .take(40)
        .collect();
    if todo.is_empty() {
        return found;
    }
    let agent = steam::http_agent();
    for name in todo {
        match steam::store_lookup(&agent, &name) {
            Some(appid) => {
                found.insert(normalize_name(&name), appid);
            }
            None => break, // Offline o. Ä. → später erneut versuchen
        }
    }
    found
}
