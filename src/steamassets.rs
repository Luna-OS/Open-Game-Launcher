//! Echte Steam-Bildpfade für neuere Spiele.
//!
//! Steam legt die Bilder neuerer Spiele nur noch unter
//! `…/store_item_assets/steam/apps/<id>/<hash>/library_capsule.jpg` ab – die alten Pfade
//! (`…/apps/<id>/library_600x900.jpg`) liefern dort 404, und die Kachel bleibt ohne Cover.
//! Die richtigen Pfade kommen aus der öffentlichen Store-API `IStoreBrowseService/GetItems`
//! (kein Schlüssel nötig) und werden lokal zwischengespeichert.

use crate::model::Art;
use crate::sys;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const API: &str = "https://api.steampowered.com/IStoreBrowseService/GetItems/v1/";
const ASSET_BASE: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";
/// So viele Spiele pro Anfrage
const BATCH: usize = 50;
/// Spiele ohne Treffer erst nach dieser Zeit erneut fragen
const RETRY_SECS: u64 = 7 * 24 * 3600;

/// Fertige Bild-URLs eines Spiels (leer = Steam kennt keine)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Assets {
    #[serde(default)]
    pub cover: Vec<String>,
    #[serde(default)]
    pub hero: Vec<String>,
    #[serde(default)]
    pub header: Vec<String>,
    /// Zeitpunkt der letzten Abfrage (Sekunden)
    #[serde(default)]
    pub checked: u64,
}

impl Assets {
    fn is_empty(&self) -> bool {
        self.cover.is_empty() && self.hero.is_empty() && self.header.is_empty()
    }
}

pub type Cache = BTreeMap<u32, Assets>;

fn path() -> std::path::PathBuf {
    sys::local_dir().join("steam-assets.json")
}

pub fn load() -> Cache {
    std::fs::read(path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save(cache: &Cache) {
    if let Ok(json) = serde_json::to_vec(cache) {
        let _ = sys::write_atomic(&path(), &json);
    }
}

/// AppIDs, für die noch nichts (oder nur ein alter Fehlschlag) im Cache steht
pub fn missing(ids: impl IntoIterator<Item = u32>, cache: &Cache, now: u64) -> Vec<u32> {
    let mut out: Vec<u32> = ids
        .into_iter()
        .filter(|id| match cache.get(id) {
            None => true,
            Some(a) => a.is_empty() && now.saturating_sub(a.checked) > RETRY_SECS,
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Antwort von `GetItems` auswerten: AppID → Bilder
pub fn parse(json: &str, now: u64) -> Cache {
    let mut out = Cache::new();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return out;
    };
    let Some(items) = v["response"]["store_items"].as_array() else {
        return out;
    };
    for it in items {
        let Some(appid) = it["appid"].as_u64().and_then(|a| u32::try_from(a).ok()) else {
            continue;
        };
        let a = &it["assets"];
        let format = a["asset_url_format"].as_str().unwrap_or("");
        // Nur das erwartete Muster zulassen (keine fremden Hosts oder Pfade)
        let prefix = format!("steam/apps/{appid}/");
        let mut assets = Assets {
            checked: now,
            ..Default::default()
        };
        if format.starts_with(&prefix) && format.contains("${FILENAME}") {
            let url = |key: &str| -> Option<String> {
                let file = a[key].as_str()?;
                let ok = !file.is_empty()
                    && !file.contains("..")
                    && file
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '.' | '-'));
                ok.then(|| format!("{ASSET_BASE}{}", format.replace("${FILENAME}", file)))
            };
            let pick = |keys: &[&str]| keys.iter().filter_map(|k| url(k)).collect::<Vec<_>>();
            assets.cover = pick(&["library_capsule_2x", "library_capsule"]);
            assets.hero = pick(&["library_hero", "library_hero_2x"]);
            assets.header = pick(&["header", "header_2x"]);
        }
        out.insert(appid, assets);
    }
    out
}

fn request(ids: &[u32], country: &str) -> Option<String> {
    let list: Vec<String> = ids
        .iter()
        .map(|id| format!(r#"{{"appid":{id}}}"#))
        .collect();
    let input = format!(
        r#"{{"ids":[{}],"context":{{"language":"english","country_code":"{country}"}},"data_request":{{"include_assets":true}}}}"#,
        list.join(",")
    );
    let url = format!("{API}?input_json={}", sys::url_encode(&input));
    let mut res = crate::steam::http_agent().get(&url).call().ok()?;
    res.body_mut()
        .with_config()
        .limit(4 * 1024 * 1024)
        .read_to_string()
        .ok()
}

/// Bilder für die AppIDs online holen. Steam blendet manche Spiele je nach Land aus,
/// daher wird für Fehlende noch einmal mit "US" gefragt.
pub fn fetch(ids: &[u32]) -> Cache {
    let now = sys::now_secs();
    let mut out = Cache::new();
    for chunk in ids.chunks(BATCH) {
        let mut got = request(chunk, "DE")
            .map(|j| parse(&j, now))
            .unwrap_or_default();
        let retry: Vec<u32> = chunk
            .iter()
            .copied()
            .filter(|id| got.get(id).is_none_or(Assets::is_empty))
            .collect();
        if !retry.is_empty() {
            if let Some(j) = request(&retry, "US") {
                for (id, a) in parse(&j, now) {
                    if !a.is_empty() {
                        got.insert(id, a);
                    }
                }
            }
        }
        // Nicht beantwortete IDs als "nichts gefunden" merken
        for id in chunk {
            got.entry(*id).or_insert_with(|| Assets {
                checked: now,
                ..Default::default()
            });
        }
        out.extend(got);
    }
    out
}

/// Bilder vor die alten CDN-Pfade setzen (lokale Steam-Bilder bleiben vorn)
pub fn apply(art: &mut Art, a: &Assets) {
    fn insert(list: &mut Vec<String>, new: &[String]) {
        let at = list
            .iter()
            .position(|u| u.starts_with("https://"))
            .unwrap_or(list.len());
        let fresh: Vec<String> = new.iter().filter(|u| !list.contains(u)).cloned().collect();
        list.splice(at..at, fresh);
    }
    insert(&mut art.cover, &a.cover);
    insert(&mut art.hero, &a.hero);
    insert(&mut art.header, &a.header);
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"response":{"store_items":[
        {"appid":1912410,"success":1,"assets":{"asset_url_format":"steam/apps/1912410/${FILENAME}?t=1",
         "library_capsule":"47d3/library_capsule.jpg","library_capsule_2x":"47d3/library_capsule_2x.jpg",
         "library_hero":"3553/library_hero.jpg","header":"86cb/header.jpg"}},
        {"appid":3602290,"success":15,"visible":false},
        {"appid":5,"assets":{"asset_url_format":"https://evil.example/${FILENAME}","library_capsule":"a.jpg"}},
        {"appid":6,"assets":{"asset_url_format":"steam/apps/6/${FILENAME}","library_capsule":"../../x.jpg"}}
    ]}}"#;

    #[test]
    fn parses_assets() {
        let c = parse(JSON, 7);
        let a = &c[&1912410];
        assert_eq!(
            a.cover,
            vec![
                format!("{ASSET_BASE}steam/apps/1912410/47d3/library_capsule_2x.jpg?t=1"),
                format!("{ASSET_BASE}steam/apps/1912410/47d3/library_capsule.jpg?t=1"),
            ]
        );
        assert_eq!(a.hero.len(), 1);
        assert_eq!(a.header.len(), 1);
        assert!(c[&3602290].is_empty());
        assert_eq!(c[&3602290].checked, 7);
    }

    #[test]
    fn rejects_foreign_urls() {
        let c = parse(JSON, 0);
        assert!(c[&5].is_empty());
        assert!(c[&6].is_empty());
        assert!(parse("kaputt", 0).is_empty());
    }

    #[test]
    fn missing_retries_only_old_failures() {
        let mut c = Cache::new();
        c.insert(
            1,
            Assets {
                cover: vec!["x".into()],
                checked: 0,
                ..Default::default()
            },
        );
        c.insert(
            2,
            Assets {
                checked: 100,
                ..Default::default()
            },
        );
        c.insert(
            3,
            Assets {
                checked: 0,
                ..Default::default()
            },
        );
        assert_eq!(missing([1, 2, 3, 4, 4], &c, 200), vec![4]);
        assert_eq!(missing([1, 2, 3, 4], &c, RETRY_SECS + 50), vec![3, 4]);
    }

    #[test]
    fn apply_keeps_local_first() {
        let mut art = Art {
            cover: vec!["/maz/f/local.jpg".into(), "https://cdn/old.jpg".into()],
            ..Default::default()
        };
        let a = Assets {
            cover: vec!["https://new.jpg".into()],
            ..Default::default()
        };
        apply(&mut art, &a);
        apply(&mut art, &a);
        assert_eq!(
            art.cover,
            vec!["/maz/f/local.jpg", "https://new.jpg", "https://cdn/old.jpg"]
        );
    }
}
