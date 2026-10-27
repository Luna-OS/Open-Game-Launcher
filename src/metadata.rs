//! Automatische Spiel-Infos mit lokalem Zwischenspeicher. Zwei legale, öffentliche Quellen:
//!
//! 1. **Steam-Store** (für Steam-Spiele und Spiele, die es auch auf Steam gibt): Beschreibung,
//!    Genres, Entwickler, Erscheinungsdatum, Mindest-/Empfohlene Anforderungen, Steam-Deck-Status.
//! 2. **Wikipedia + Wikidata** (für alles andere, z. B. ROMs, GOG-, Xbox- oder eigene Spiele):
//!    Kurzbeschreibung (CC BY-SA, mit Quellenangabe), Genres, Entwickler, Erscheinungsjahr.
//!
//! Es wird nur gelesen, nie etwas hochgeladen. Antworten sind in der Größe begrenzt.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version des Cache-Formats – ältere Einträge werden einmal neu geladen
pub const VERSION: u32 = 3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Info {
    pub description: String,
    pub genres: Vec<String>,
    pub developer: String,
    pub release: String,
    /// Mindestanforderungen für dieses Betriebssystem: (Bezeichnung, Wert)
    pub req_min: Vec<(String, String)>,
    /// Empfohlene Anforderungen
    pub req_rec: Vec<(String, String)>,
    /// Steam Deck: 0 = unbekannt, 1 = nicht unterstützt, 2 = spielbar, 3 = verifiziert
    pub deck: u8,
    /// "steam" oder "wikipedia"
    pub source: String,
    /// Link zur Quelle (Wikipedia-Artikel)
    pub page: String,
    /// Bild aus der Quelle (nur genutzt, wenn das Spiel sonst kein Cover hat)
    pub cover: String,
    pub version: u32,
    /// Wann geladen (0 = nichts gefunden)
    pub fetched: u64,
}

impl Info {
    pub fn is_current(&self) -> bool {
        // Version 3 liest Steam-Anforderungen in mehr Formen und fragt bei Ländersperren den
        // US-Store. Ältere Einträge ohne Anforderungen werden deshalb einmal neu geladen.
        let no_reqs = self.req_min.is_empty() && self.req_rec.is_empty();
        self.version >= VERSION || (self.version >= 2 && self.fetched > 0 && !no_reqs)
    }
}

fn path() -> std::path::PathBuf {
    crate::sys::local_dir().join("metadata.json")
}

pub fn load_cache() -> BTreeMap<String, Info> {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_cache(cache: &BTreeMap<String, Info>) {
    let p = path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(t) = serde_json::to_string(cache) {
        let _ = crate::sys::write_atomic(&p, t.as_bytes());
    }
}

/// Einfache HTML-Reste entfernen ("&amp;", "<br>")
pub fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Steam-Anforderungen (HTML-Liste) → [(„Betriebssystem“, „Windows 10“), …]
///
/// Steam liefert das in mehreren Formen:
/// - `<li><strong>OS:</strong> Windows 10</li>` (üblich)
/// - `<li><strong>OS: Windows 10</strong></li>` (Bezeichnung und Wert fett, z. B. Terraria)
/// - ohne Liste: `<strong>Minimum:</strong><br>OS: Windows 10<br>Memory: 4 GB`
pub fn parse_requirements(html: &str) -> Vec<(String, String)> {
    let items: Vec<&str> = if html.contains("<li>") {
        html.split("<li>")
            .skip(1)
            .map(|i| i.split("</li>").next().unwrap_or(i))
            .collect()
    } else {
        html.split("<br>").collect()
    };
    let mut out = Vec::new();
    for item in items {
        let (mut label, mut value) = match (item.find("<strong>"), item.find("</strong>")) {
            (Some(a), Some(b)) if a < b => (plain(&item[a + 8..b]), plain(&item[b + 9..])),
            _ => (String::new(), plain(item)),
        };
        // "OS: Windows 10" steht ganz in der Bezeichnung oder ganz im Wert
        let whole = if value.is_empty() {
            Some(std::mem::take(&mut label))
        } else if label.is_empty() {
            Some(std::mem::take(&mut value))
        } else {
            None
        };
        if let Some(w) = whole {
            match w.split_once(':') {
                Some((l, v)) if !v.trim().is_empty() && l.chars().count() <= 40 => {
                    label = l.trim().to_string();
                    value = v.trim().to_string();
                }
                _ => value = w,
            }
        }
        let label = label.trim_end_matches(':').trim().to_string();
        // Überschriften wie "Minimum:" / "Empfohlen:" überspringen
        if value.is_empty() || is_heading(&label, &value) || value.len() >= 400 {
            continue;
        }
        out.push((label.chars().take(60).collect(), value));
        if out.len() >= 15 {
            break;
        }
    }
    out
}

fn is_heading(label: &str, value: &str) -> bool {
    let v = value.trim_end_matches(':').trim().to_lowercase();
    label.is_empty()
        && matches!(
            v.as_str(),
            "minimum"
                | "recommended"
                | "required"
                | "mindestanforderungen"
                | "empfohlen"
                | "minimale systemanforderungen"
                | "empfohlene systemanforderungen"
        )
}

fn os_req_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "mac_requirements"
    } else if cfg!(target_os = "linux") {
        "linux_requirements"
    } else {
        "pc_requirements"
    }
}

/// Antwort von store.steampowered.com/api/appdetails auswerten
pub fn parse_appdetails(appid: u32, json: &str) -> Option<Info> {
    parse_appdetails_for(appid, json, os_req_key())
}

pub fn parse_appdetails_for(appid: u32, json: &str, req_key: &str) -> Option<Info> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let d = v.get(appid.to_string())?;
    if d.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return None;
    }
    let data = d.get("data")?;
    let s = |k: &str| {
        data.get(k)
            .and_then(|x| x.as_str())
            .map(plain)
            .unwrap_or_default()
    };
    // Linux/Mac ohne eigene Angaben (oft leer) → Windows-Angaben nehmen (Proton/Wine)
    let reqs = data
        .get(req_key)
        .filter(|r| r.get("minimum").is_some())
        .or_else(|| data.get("pc_requirements"));
    let req = |k: &str| {
        reqs.and_then(|r| r.get(k))
            .and_then(|x| x.as_str())
            .map(parse_requirements)
            .unwrap_or_default()
    };
    Some(Info {
        description: s("short_description"),
        genres: data
            .get("genres")
            .and_then(|g| g.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|g| {
                        g.get("description")
                            .and_then(|x| x.as_str())
                            .map(|x| x.to_string())
                    })
                    .take(8)
                    .collect()
            })
            .unwrap_or_default(),
        developer: data
            .get("developers")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        release: data
            .get("release_date")
            .and_then(|r| r.get("date"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        req_min: req("minimum"),
        req_rec: req("recommended"),
        source: "steam".into(),
        version: VERSION,
        fetched: crate::sys::now_secs(),
        ..Default::default()
    })
}

/// Steam-Deck-Bewertung aus ajaxgetdeckappcompatibilityreport
pub fn parse_deck(json: &str) -> u8 {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("results")?.get("resolved_category")?.as_u64())
        .map(|c| c.min(3) as u8)
        .unwrap_or(0)
}

fn steam_lang(lang: &str) -> &'static str {
    match lang {
        "de" => "german",
        "fr" => "french",
        "es" => "spanish",
        "it" => "italian",
        "pt" => "brazilian",
        "pl" => "polish",
        "tr" => "turkish",
        "ru" => "russian",
        _ => "english",
    }
}

fn agent() -> ureq::Agent {
    use std::sync::OnceLock;
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT
        .get_or_init(|| {
            use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
            let tls = TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build();
            ureq::Agent::config_builder()
                .tls_config(tls)
                .timeout_global(Some(std::time::Duration::from_secs(12)))
                // Wikipedia verlangt einen aussagekräftigen User-Agent mit Kontakt
                .user_agent(concat!(
                    "Open-Game-Launcher/",
                    env!("CARGO_PKG_VERSION"),
                    " (https://github.com/Luna-OS/Open-Game-Launcher)"
                ))
                .build()
                .into()
        })
        .clone()
}

/// GET mit Größenlimit. `None` = keine Verbindung / Fehler
fn get_text(url: &str, limit: u64) -> Option<String> {
    let mut resp = agent().get(url).call().ok()?;
    resp.body_mut()
        .with_config()
        .limit(limit)
        .read_to_string()
        .ok()
}

/// Infos zu einer Steam-AppID laden (in der Sprache des Launchers).
/// `None` = keine Verbindung (später erneut versuchen), `fetched == 0` = nichts gefunden.
pub fn fetch(appid: u32, lang: &str) -> Option<Info> {
    let l = steam_lang(lang);
    let body = get_text(
        &format!("https://store.steampowered.com/api/appdetails?appids={appid}&l={l}"),
        4 * 1024 * 1024,
    )?;
    // Manche Spiele blendet Steam je nach Land aus (success: false) → noch einmal mit US-Store
    let parsed = parse_appdetails(appid, &body).or_else(|| {
        let us = get_text(
            &format!("https://store.steampowered.com/api/appdetails?appids={appid}&l={l}&cc=us"),
            4 * 1024 * 1024,
        )?;
        parse_appdetails(appid, &us)
    });
    let mut info = parsed.unwrap_or(Info {
        version: VERSION,
        ..Default::default()
    });
    if info.fetched > 0 {
        if let Some(d) = get_text(&format!("https://store.steampowered.com/saleaction/ajaxgetdeckappcompatibilityreport?nAppID={appid}&l={l}"), 256 * 1024) {
            info.deck = parse_deck(&d);
        }
    }
    Some(info)
}

// ----------------------------------------------------------------------
// Epic Games Store: Systemanforderungen für Spiele, die es nicht auf Steam gibt
// ----------------------------------------------------------------------

const EPIC_CONTENT: &str = "https://store-content-ipv4.ak.epicgames.com/api";

/// Store-Slug aus dem Titel raten ("Control: Ultimate Edition" → "control-ultimate-edition")
pub fn epic_slug_guess(title: &str) -> String {
    let mut out = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() && !matches!(c, '\'' | '’' | '®' | '™')
        {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Mögliche Slugs aus dem Titel: voller Titel, ohne Klammern ("(Beta)"), ohne Untertitel
/// ("Disco Elysium - The Final Cut" → "disco-elysium") und ohne "… Edition"
pub fn epic_slug_candidates(title: &str) -> Vec<String> {
    let mut base = String::new();
    let mut depth = 0;
    for c in title.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => base.push(c),
            _ => {}
        }
    }
    let base = base.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut forms = vec![title.to_string(), base.clone()];
    for sep in [" - ", " – ", ": "] {
        if let Some((head, _)) = base.split_once(sep) {
            forms.push(head.to_string());
        }
    }
    let lower = base.to_lowercase();
    if let Some(pos) = lower.rfind(" edition") {
        let words: Vec<&str> = base[..pos].split_whitespace().collect();
        // "… Standard/Definitive/Deluxe Edition" → Wort davor auch weglassen
        let keep = match words.last().map(|w| w.to_lowercase()) {
            Some(w)
                if matches!(
                    w.as_str(),
                    "standard" | "definitive" | "deluxe" | "complete" | "gold"
                ) =>
            {
                words.len() - 1
            }
            _ => words.len(),
        };
        forms.push(words[..keep].join(" "));
    }
    let mut out: Vec<String> = Vec::new();
    for f in forms {
        let s = epic_slug_guess(&f);
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// Nur harmlose Slugs (Teil einer URL); "spiel/home" → "spiel"
fn clean_slug(s: &str) -> Option<String> {
    let s = s.trim().trim_end_matches("/home").to_lowercase();
    let ok =
        !s.is_empty() && s.len() <= 120 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    ok.then_some(s)
}

fn epic_locale(lang: &str) -> &'static str {
    match lang.split(['-', '_']).next().unwrap_or("") {
        "de" => "de",
        "fr" => "fr",
        "es" => "es-ES",
        "it" => "it",
        "pl" => "pl",
        "pt" => "pt-BR",
        "ru" => "ru",
        "tr" => "tr",
        "ja" => "ja",
        "ko" => "ko",
        "zh" => "zh-CN",
        _ => "en-US",
    }
}

type Reqs = (Vec<(String, String)>, Vec<(String, String)>);

/// Antwort der Epic-Content-API → (Minimum, Empfohlen) für dieses Betriebssystem
pub fn parse_epic_requirements(json: &str, os: &str) -> Reqs {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Default::default();
    };
    let systems: Vec<&serde_json::Value> = v["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["data"]["requirements"]["systems"].as_array())
        .flatten()
        .collect();
    let kind = |s: &&serde_json::Value| s["systemType"].as_str().unwrap_or("").to_lowercase();
    let sys = systems
        .iter()
        .find(|s| kind(s).starts_with(os))
        .or_else(|| systems.iter().find(|s| kind(s).starts_with("windows")))
        .or(systems.first());
    let mut min = Vec::new();
    let mut rec = Vec::new();
    for d in sys
        .and_then(|s| s["details"].as_array())
        .into_iter()
        .flatten()
    {
        let title: String = plain(d["title"].as_str().unwrap_or(""))
            .chars()
            .take(60)
            .collect();
        for (key, list) in [("minimum", &mut min), ("recommended", &mut rec)] {
            let val = plain(d[key].as_str().unwrap_or(""));
            if !val.is_empty() && val.len() < 400 && list.len() < 15 {
                list.push((title.clone(), val));
            }
        }
    }
    (min, rec)
}

/// Systemanforderungen eines Epic-Spiels: erst über den Slug aus der Bibliothek,
/// sonst über einen aus dem Titel geratenen Slug
pub fn fetch_epic_requirements(app_name: &str, title: &str, lang: &str) -> Option<Reqs> {
    let known = crate::epicacct::load_cache()
        .into_iter()
        .find(|o| o.app_name == app_name)
        .map(|o| o.slug);
    let os = if cfg!(target_os = "macos") {
        "mac"
    } else {
        "windows"
    };
    let locale = epic_locale(lang);
    let mut tried: Vec<String> = Vec::new();
    for slug in known.into_iter().chain(epic_slug_candidates(title)) {
        let Some(slug) = clean_slug(&slug) else {
            continue;
        };
        if tried.contains(&slug) {
            continue;
        }
        tried.push(slug.clone());
        let Some(body) = get_text(
            &format!("{EPIC_CONTENT}/{locale}/content/products/{slug}"),
            4 * 1024 * 1024,
        ) else {
            continue;
        };
        let reqs = parse_epic_requirements(&body, os);
        if !reqs.0.is_empty() || !reqs.1.is_empty() {
            return Some(reqs);
        }
    }
    None
}

// ----------------------------------------------------------------------
// Zweite Quelle: Wikipedia + Wikidata
// ----------------------------------------------------------------------

/// Wörter, an denen man in der Kurzbeschreibung erkennt, dass es um ein Spiel geht
const GAME_WORDS: &[&str] = &[
    "video game",
    "videogame",
    "computer game",
    "videospiel",
    "computerspiel",
    "jeu vidéo",
    "videojuego",
    "videogioco",
    "jogo eletrônico",
    "jogo de vídeo",
    "gra komputerowa",
    "gra wideo",
    "video oyunu",
    "bilgisayar oyunu",
    "компьютерная игра",
    "видеоигра",
];

/// Name für die Suche säubern ("Spiel™ – Deluxe Edition" → "Spiel")
pub fn search_name(name: &str) -> String {
    let mut n: String = name
        .chars()
        .filter(|c| !matches!(c, '™' | '®' | '©'))
        .collect();
    for cut in [
        " - ",
        " – ",
        ": Game of the Year",
        " Deluxe Edition",
        " Definitive Edition",
        " GOTY",
        " Remastered",
    ] {
        if let Some(i) = n.find(cut) {
            if i > 2 {
                n.truncate(i);
            }
        }
    }
    n.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Ersten passenden Artikeltitel aus der Wikipedia-Suche
pub fn pick_search_result(json: &str, name: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let want = crate::model::normalize_name(name);
    let results = v.get("query")?.get("search")?.as_array()?;
    results
        .iter()
        .filter_map(|r| r.get("title")?.as_str())
        .find(|t| {
            let n = crate::model::normalize_name(t.split(" (").next().unwrap_or(t));
            !want.is_empty()
                && (n == want || n.starts_with(&want) || want.starts_with(&n) && n.len() > 3)
        })
        .map(|t| t.to_string())
}

/// Kurzfassung eines Artikels → (Text, Bild, Wikidata-ID, Seite). Nur, wenn es wirklich ein Spiel ist.
pub fn parse_summary(json: &str) -> Option<(String, String, String, String)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v.get("type").and_then(|t| t.as_str()) == Some("disambiguation") {
        return None;
    }
    let desc = v
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("")
        .to_lowercase();
    let extract = v
        .get("extract")
        .and_then(|d| d.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let head: String = extract.chars().take(300).collect::<String>().to_lowercase();
    if !GAME_WORDS
        .iter()
        .any(|w| desc.contains(w) || head.contains(w))
    {
        return None;
    }
    let thumb = v
        .get("thumbnail")
        .and_then(|t| t.get("source"))
        .and_then(|s| s.as_str())
        .filter(|u| u.starts_with("https://upload.wikimedia.org/"))
        .unwrap_or("")
        .to_string();
    let qid = v
        .get("wikibase_item")
        .and_then(|q| q.as_str())
        .unwrap_or("")
        .to_string();
    let page = v
        .get("content_urls")
        .and_then(|c| c.get("desktop"))
        .and_then(|d| d.get("page"))
        .and_then(|p| p.as_str())
        .filter(|u| u.starts_with("https://") && u.contains(".wikipedia.org/"))
        .unwrap_or("")
        .to_string();
    // Lange Texte kürzen (am Satzende)
    let mut text: String = extract.chars().take(700).collect();
    if extract.chars().count() > 700 {
        if let Some(i) = text.rfind(". ") {
            text.truncate(i + 1);
        }
    }
    Some((text, thumb, qid, page))
}

fn valid_qid(q: &str) -> bool {
    q.len() > 1 && q.len() < 15 && q.starts_with('Q') && q[1..].chars().all(|c| c.is_ascii_digit())
}

/// Aus den Wikidata-Aussagen: Genre-IDs (P136), Entwickler-IDs (P178), Erscheinungsjahr (P577)
pub fn parse_wikidata_claims(json: &str, qid: &str) -> (Vec<String>, Vec<String>, String) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Default::default();
    };
    let Some(claims) = v
        .get("entities")
        .and_then(|e| e.get(qid))
        .and_then(|e| e.get("claims"))
    else {
        return Default::default();
    };
    let ids = |p: &str| -> Vec<String> {
        claims
            .get(p)
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|c| {
                        c.get("mainsnak")?
                            .get("datavalue")?
                            .get("value")?
                            .get("id")?
                            .as_str()
                    })
                    .filter(|q| valid_qid(q))
                    .map(|q| q.to_string())
                    .take(6)
                    .collect()
            })
            .unwrap_or_default()
    };
    let year = claims
        .get("P577")
        .and_then(|a| a.as_array())
        .and_then(|a| {
            a.iter()
                .filter_map(|c| {
                    c.get("mainsnak")?
                        .get("datavalue")?
                        .get("value")?
                        .get("time")?
                        .as_str()
                })
                .map(|t| {
                    t.trim_start_matches('+')
                        .chars()
                        .take(4)
                        .collect::<String>()
                })
                .filter(|y| y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()))
                .min()
        })
        .unwrap_or_default();
    (ids("P136"), ids("P178"), year)
}

/// Beschriftungen zu Wikidata-IDs (in der Sprache, sonst Englisch)
pub fn parse_labels(json: &str, lang: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return out;
    };
    for (id, e) in v
        .get("entities")
        .and_then(|e| e.as_object())
        .into_iter()
        .flatten()
    {
        let labels = e.get("labels");
        let l = labels
            .and_then(|l| l.get(lang))
            .or_else(|| labels.and_then(|l| l.get("en")))
            .and_then(|l| l.get("value"))
            .and_then(|v| v.as_str());
        if let Some(l) = l {
            out.insert(id.clone(), l.chars().take(40).collect());
        }
    }
    out
}

fn first_upper(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Spiel-Infos aus Wikipedia/Wikidata. `None` = keine Verbindung, `fetched == 0` = nichts gefunden
pub fn fetch_wiki(name: &str, lang: &str) -> Option<Info> {
    let name = search_name(name);
    if name.chars().count() < 2 {
        return Some(Info {
            version: VERSION,
            ..Default::default()
        });
    }
    let langs: Vec<&str> = if lang == "en" {
        vec!["en"]
    } else {
        vec![lang, "en"]
    };
    let mut summary = None;
    let mut reached = false;
    for l in &langs {
        let l = if l.chars().all(|c| c.is_ascii_lowercase()) && l.len() == 2 {
            *l
        } else {
            "en"
        };
        let q = crate::sys::url_encode(&format!("{name} video game"));
        let Some(search) = get_text(&format!("https://{l}.wikipedia.org/w/api.php?action=query&list=search&srsearch={q}&srlimit=5&format=json&utf8=1"), 512 * 1024) else {
            continue;
        };
        reached = true;
        let Some(title) = pick_search_result(&search, &name) else {
            continue;
        };
        let t = crate::sys::url_encode(&title.replace(' ', "_"));
        if let Some(s) = get_text(
            &format!("https://{l}.wikipedia.org/api/rest_v1/page/summary/{t}"),
            512 * 1024,
        )
        .as_deref()
        .and_then(parse_summary)
        {
            summary = Some((l.to_string(), s));
            break;
        }
    }
    if !reached {
        return None;
    }
    let Some((wl, (text, thumb, qid, page))) = summary else {
        return Some(Info {
            version: VERSION,
            ..Default::default()
        });
    };
    let mut info = Info {
        description: text,
        cover: thumb,
        page,
        source: "wikipedia".into(),
        version: VERSION,
        fetched: crate::sys::now_secs(),
        ..Default::default()
    };
    if valid_qid(&qid) {
        if let Some(j) = get_text(
            &format!("https://www.wikidata.org/wiki/Special:EntityData/{qid}.json"),
            8 * 1024 * 1024,
        ) {
            let (genres, devs, year) = parse_wikidata_claims(&j, &qid);
            info.release = year;
            let ids: Vec<String> = genres.iter().chain(devs.iter()).cloned().collect();
            if !ids.is_empty() {
                let url = format!(
                    "https://www.wikidata.org/w/api.php?action=wbgetentities&ids={}&props=labels&languages={}|en&format=json",
                    ids.join("|"),
                    wl
                );
                if let Some(lj) = get_text(&url, 1024 * 1024) {
                    let labels = parse_labels(&lj, &wl);
                    info.genres = genres
                        .iter()
                        .filter_map(|g| labels.get(g))
                        .map(|g| first_upper(g))
                        .collect();
                    info.developer = devs
                        .iter()
                        .filter_map(|d| labels.get(d))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                }
            }
        }
    }
    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements_all_forms() {
        // Terraria: Bezeichnung und Wert zusammen fett
        let terraria = r#"<ul class="bb_ul"><li><strong>Betriebssystem: Windows Xp, Vista, 7</strong> <br></li><li><strong>Speicher: 512 MB</strong><br></li></ul>"#;
        assert_eq!(
            parse_requirements(terraria),
            vec![
                ("Betriebssystem".into(), "Windows Xp, Vista, 7".into()),
                ("Speicher".into(), "512 MB".into())
            ]
        );
        // Überschrift im <h2>, dann Liste
        let rec = r#"<h2 class="bb_tag" ><strong>RECOMMENDED</strong></h2><ul class="bb_ul"><li><strong>OS: Windows 10</strong> <br>		</li><li><strong>Memory: 4GB</strong></li></ul>"#;
        assert_eq!(parse_requirements(rec)[1], ("Memory".into(), "4GB".into()));
        // Ohne Liste, nur Zeilenumbrüche
        let br = "<strong>Minimum:</strong><br>OS: Windows 7<br>Memory: 2 GB RAM<br>Requires a 64-bit processor";
        assert_eq!(
            parse_requirements(br),
            vec![
                ("OS".into(), "Windows 7".into()),
                ("Memory".into(), "2 GB RAM".into()),
                ("".into(), "Requires a 64-bit processor".into())
            ]
        );
        // Bisheriges Format bleibt gleich
        let normal = "<ul><li><strong>OS:</strong> Windows 10</li></ul>";
        assert_eq!(
            parse_requirements(normal),
            vec![("OS".into(), "Windows 10".into())]
        );
    }

    #[test]
    fn epic_requirements() {
        let j = r#"{"pages":[{"data":{"requirements":{"systems":[
            {"systemType":"Mac","details":[{"title":"OS","minimum":"macOS 12","recommended":""}]},
            {"systemType":"Windows","details":[
                {"title":"Betriebssystem","minimum":"Windows 7 (64&nbsp;Bit)","recommended":"Windows 10"},
                {"title":"Prozessor","minimum":"i5-4690","recommended":""}]}]}}}]}"#;
        let (min, rec) = parse_epic_requirements(j, "windows");
        assert_eq!(
            min,
            vec![
                ("Betriebssystem".into(), "Windows 7 (64 Bit)".into()),
                ("Prozessor".into(), "i5-4690".into())
            ]
        );
        assert_eq!(rec, vec![("Betriebssystem".into(), "Windows 10".into())]);
        assert_eq!(parse_epic_requirements(j, "mac").0[0].1, "macOS 12");
        assert_eq!(
            parse_epic_requirements("kaputt", "windows"),
            Reqs::default()
        );
        assert_eq!(
            epic_slug_guess("Control: Ultimate Edition"),
            "control-ultimate-edition"
        );
        assert_eq!(
            epic_slug_guess("Assassin's Creed® Unity"),
            "assassins-creed-unity"
        );
        assert_eq!(clean_slug("control/home").as_deref(), Some("control"));
        assert_eq!(
            epic_slug_candidates("Disco Elysium - The Final Cut"),
            vec!["disco-elysium-the-final-cut", "disco-elysium"]
        );
        assert_eq!(
            epic_slug_candidates("Barony (Beta)"),
            vec!["barony-beta", "barony"]
        );
        assert_eq!(
            epic_slug_candidates("art of rally Standard Edition"),
            vec!["art-of-rally-standard-edition", "art-of-rally"]
        );
        assert_eq!(clean_slug("../x"), None);
    }

    #[test]
    fn refetch_only_steam_without_requirements() {
        let old = |source: &str, req: bool| Info {
            source: source.into(),
            version: 2,
            fetched: 1,
            req_min: if req {
                vec![("OS".into(), "10".into())]
            } else {
                vec![]
            },
            ..Default::default()
        };
        assert!(!old("steam", false).is_current());
        assert!(old("steam", true).is_current());
        assert!(!old("wikipedia", false).is_current());
        // Früher nichts gefunden → neu versuchen
        let none = Info {
            version: 2,
            ..Default::default()
        };
        assert!(!none.is_current());
    }

    #[test]
    fn appdetails() {
        let j = r#"{"570":{"success":true,"data":{"short_description":"Every day, millions &amp; more <b>play</b>.","genres":[{"id":"1","description":"Action"},{"id":"2","description":"Strategy"}],"developers":["Valve"],"release_date":{"date":"9 Jul, 2013"},
            "pc_requirements":{"minimum":"<strong>Minimum:</strong><br><ul class=\"bb_ul\"><li><strong>OS:</strong> Windows 10<br></li><li><strong>Memory:</strong> 4 GB RAM<br></li><li>Broadband</li></ul>","recommended":"<ul><li><strong>Graphics:</strong> GTX 1060</li></ul>"},
            "linux_requirements":[]}}}"#;
        let i = parse_appdetails_for(570, j, "linux_requirements").unwrap();
        assert_eq!(i.description, "Every day, millions & more play.");
        assert_eq!(i.genres, vec!["Action", "Strategy"]);
        assert_eq!(i.developer, "Valve");
        assert_eq!(
            i.req_min,
            vec![
                ("OS".into(), "Windows 10".into()),
                ("Memory".into(), "4 GB RAM".into()),
                ("".into(), "Broadband".into())
            ]
        );
        assert_eq!(i.req_rec, vec![("Graphics".into(), "GTX 1060".into())]);
        assert!(i.is_current());
        assert!(parse_appdetails(570, r#"{"570":{"success":false}}"#).is_none());
        assert_eq!(
            parse_deck(r#"{"success":1,"results":{"appid":570,"resolved_category":3}}"#),
            3
        );
        assert_eq!(parse_deck("kaputt"), 0);
    }

    #[test]
    fn wikipedia() {
        assert_eq!(
            search_name("The Witcher® 3: Wild Hunt – Game of the Year Edition"),
            "The Witcher 3: Wild Hunt"
        );
        let s = r#"{"query":{"search":[{"title":"Mario (Figur)"},{"title":"Super Mario World"},{"title":"Super Mario World 2"}]}}"#;
        assert_eq!(
            pick_search_result(s, "Super Mario World").as_deref(),
            Some("Super Mario World")
        );
        assert_eq!(pick_search_result(s, "Zelda"), None);
        let sum = r#"{"type":"standard","description":"1990 video game","extract":"Super Mario World is a platform game.","thumbnail":{"source":"https://upload.wikimedia.org/x.png"},"wikibase_item":"Q1137577","content_urls":{"desktop":{"page":"https://en.wikipedia.org/wiki/Super_Mario_World"}}}"#;
        let (t, img, q, page) = parse_summary(sum).unwrap();
        assert_eq!(t, "Super Mario World is a platform game.");
        assert_eq!(img, "https://upload.wikimedia.org/x.png");
        assert_eq!(q, "Q1137577");
        assert!(page.ends_with("Super_Mario_World"));
        assert!(parse_summary(
            r#"{"type":"standard","description":"Stadt in Bayern","extract":"Eine Stadt."}"#
        )
        .is_none());
        // Bild von fremder Seite wird ignoriert
        let evil = r#"{"description":"video game","extract":"x","thumbnail":{"source":"https://evil.example/a.png"}}"#;
        assert_eq!(parse_summary(evil).unwrap().1, "");
        let wd = r#"{"entities":{"Q1":{"claims":{"P136":[{"mainsnak":{"datavalue":{"value":{"id":"Q2"}}}}],"P178":[{"mainsnak":{"datavalue":{"value":{"id":"Q3"}}}}],"P577":[{"mainsnak":{"datavalue":{"value":{"time":"+1991-11-21T00:00:00Z"}}}},{"mainsnak":{"datavalue":{"value":{"time":"+1990-11-21T00:00:00Z"}}}}]}}}}"#;
        assert_eq!(
            parse_wikidata_claims(wd, "Q1"),
            (
                vec!["Q2".to_string()],
                vec!["Q3".to_string()],
                "1990".to_string()
            )
        );
        let labels = parse_labels(
            r#"{"entities":{"Q2":{"labels":{"en":{"value":"platform game"},"de":{"value":"Jump ’n’ Run"}}}}}"#,
            "de",
        );
        assert_eq!(labels.get("Q2").map(|s| s.as_str()), Some("Jump ’n’ Run"));
    }
}
