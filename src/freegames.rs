//! Gratis-Spiele: aktuelle (und kommende) kostenlose Spiele bei Epic und Steam.
//!
//! - Epic: offizielle Schnittstelle des Epic Stores (inkl. Start-/Enddatum)
//! - Steam: Store-Suche nach 100 % Rabatt (nur Spiele) + Enddaten von GamerPower

use crate::model::normalize_name;
use crate::util::parse_datetime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreeStore {
    Epic,
    Steam,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FreeGame {
    pub id: String,
    pub title: String,
    pub store: FreeStore,
    /// Hochformat- und Querformat-Bilder (Ersatzliste)
    pub images: Vec<String>,
    pub wide: Vec<String>,
    pub starts: Option<u64>,
    pub ends: Option<u64>,
    /// true = erst ab `starts` gratis
    pub upcoming: bool,
    /// Normaler Preis, z. B. "19,99 €"
    pub worth: String,
    pub description: String,
    pub epic_slug: Option<String>,
    pub steam_appid: Option<u32>,
    /// Webseite des Angebots (Ersatz, wenn der Launcher fehlt)
    pub url: String,
}

impl FreeGame {
    /// Link, der direkt im Epic Games Launcher bzw. in Steam öffnet
    pub fn launcher_url(&self) -> String {
        match (self.store, &self.epic_slug, self.steam_appid) {
            (FreeStore::Epic, Some(slug), _) => format!("com.epicgames.launcher://store/p/{slug}"),
            (FreeStore::Steam, _, Some(id)) => format!("steam://openurl/https://store.steampowered.com/app/{id}/"),
            _ => self.url.clone(),
        }
    }

    /// Gehört das Spiel schon zu einem Spiel in der Bibliothek?
    pub fn matches(&self, game_name: &str, steam_appid: Option<u32>) -> bool {
        if let (Some(a), Some(b)) = (self.steam_appid, steam_appid) {
            return a == b;
        }
        let a = normalize_name(&self.title);
        !a.is_empty() && a == normalize_name(game_name)
    }
}

fn s<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

/// Land für die Epic-Preise aus der Sprache ableiten
pub fn country_for(lang: &str) -> &'static str {
    match lang {
        "de" => "DE",
        "fr" => "FR",
        "es" => "ES",
        "it" => "IT",
        "pt" => "BR",
        "pl" => "PL",
        "tr" => "TR",
        _ => "US",
    }
}

fn epic_locale(lang: &str) -> &'static str {
    match lang {
        "de" => "de",
        "fr" => "fr",
        "es" => "es-ES",
        "it" => "it",
        "pt" => "pt-BR",
        "pl" => "pl",
        "tr" => "tr",
        "ru" => "ru",
        _ => "en-US",
    }
}

fn epic_slug(el: &serde_json::Value) -> Option<String> {
    let ok = |x: &str| !x.is_empty() && x != "[]" && !x.contains("bundle");
    let ps = s(el, "productSlug");
    if ok(ps) {
        return Some(ps.trim_end_matches("/home").to_string());
    }
    for path in [&["offerMappings"][..], &["catalogNs", "mappings"][..]] {
        let mut cur = el;
        for p in path {
            cur = cur.get(*p).unwrap_or(&serde_json::Value::Null);
        }
        if let Some(arr) = cur.as_array() {
            for m in arr {
                let slug = s(m, "pageSlug");
                if ok(slug) {
                    return Some(slug.to_string());
                }
            }
        }
    }
    let us = s(el, "urlSlug");
    ok(us).then(|| us.to_string())
}

fn epic_images(el: &serde_json::Value, types: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(list) = el.get("keyImages").and_then(|k| k.as_array()) {
        for t in types {
            for img in list {
                if s(img, "type") == *t && !s(img, "url").is_empty() {
                    out.push(s(img, "url").to_string());
                }
            }
        }
    }
    out
}

/// Epic-Antwort auswerten (getrennt, damit testbar)
pub fn parse_epic(v: &serde_json::Value, lang: &str) -> Vec<FreeGame> {
    let mut out = Vec::new();
    let empty = Vec::new();
    let els = v
        .pointer("/data/Catalog/searchStore/elements")
        .and_then(|e| e.as_array())
        .unwrap_or(&empty);
    for el in els {
        let title = s(el, "title").trim().to_string();
        if title.is_empty() {
            continue;
        }
        let price = el.pointer("/price/totalPrice");
        let discount_price = price.and_then(|p| p.get("discountPrice")).and_then(|x| x.as_i64()).unwrap_or(-1);
        let original = price.and_then(|p| p.get("originalPrice")).and_then(|x| x.as_i64()).unwrap_or(0);
        let fmt_original = price
            .and_then(|p| p.pointer("/fmtPrice/originalPrice"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let worth = if !fmt_original.is_empty() && fmt_original != "0" {
            fmt_original
        } else if original > 0 {
            let cur = price.and_then(|p| p.get("currencyCode")).and_then(|x| x.as_str()).unwrap_or("");
            let dec = crate::i18n::t("fmt.decimal");
            format!("{}{dec}{:02} {cur}", original / 100, original % 100)
        } else {
            String::new()
        };
        let promos = el.get("promotions");
        let windows = |key: &str| -> Vec<(Option<u64>, Option<u64>, i64)> {
            let mut v = Vec::new();
            if let Some(groups) = promos.and_then(|p| p.get(key)).and_then(|g| g.as_array()) {
                for g in groups {
                    if let Some(offers) = g.get("promotionalOffers").and_then(|o| o.as_array()) {
                        for o in offers {
                            let pct = o.pointer("/discountSetting/discountPercentage").and_then(|x| x.as_i64()).unwrap_or(-1);
                            v.push((parse_datetime(s(o, "startDate")), parse_datetime(s(o, "endDate")), pct));
                        }
                    }
                }
            }
            v
        };
        let current = windows("promotionalOffers");
        let upcoming = windows("upcomingPromotionalOffers");
        let (is_free_now, is_upcoming, starts, ends) = if discount_price == 0 && original > 0 && !current.is_empty() {
            (true, false, current[0].0, current[0].1)
        } else if let Some(u) = upcoming.iter().find(|u| u.2 == 0) {
            (false, true, u.0, u.1)
        } else {
            (false, false, None, None)
        };
        if !is_free_now && !is_upcoming {
            continue;
        }
        let slug = epic_slug(el);
        let url = match &slug {
            Some(sl) => format!("https://store.epicgames.com/{}/p/{sl}", epic_locale(lang)),
            None => "https://store.epicgames.com/free-games".to_string(),
        };
        out.push(FreeGame {
            id: format!("epic:{}", s(el, "id")),
            title,
            store: FreeStore::Epic,
            images: epic_images(el, &["OfferImageTall", "DieselGameBoxTall", "Thumbnail", "DieselStoreFrontTall"]),
            wide: epic_images(el, &["OfferImageWide", "DieselStoreFrontWide", "DieselGameBox"]),
            starts,
            ends,
            upcoming: is_upcoming,
            worth,
            description: s(el, "description").to_string(),
            epic_slug: slug,
            steam_appid: None,
            url,
        });
    }
    out
}

fn html_unescape(t: &str) -> String {
    t.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

fn between<'a>(hay: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = hay.find(start)? + start.len();
    let j = hay[i..].find(end)? + i;
    Some(&hay[i..j])
}

/// Steam-Suche (100 % Rabatt) auswerten
pub fn parse_steam_search(html: &str) -> Vec<FreeGame> {
    let mut out = Vec::new();
    for row in html.split("<a ").skip(1) {
        let Some(id) = between(row, "data-ds-appid=\"", "\"").and_then(|x| x.split(',').next()).and_then(|x| x.parse::<u32>().ok()) else {
            continue;
        };
        let Some(title) = between(row, "<span class=\"title\">", "</span>") else { continue };
        if !row.contains("-100%") {
            continue;
        }
        let worth = between(row, "discount_original_price\">", "</div>").map(html_unescape).unwrap_or_default();
        let url = between(row, "href=\"", "\"").map(|u| u.split('?').next().unwrap_or(u).to_string()).unwrap_or_else(|| format!("https://store.steampowered.com/app/{id}/"));
        let cdn = "https://cdn.cloudflare.steamstatic.com/steam/apps";
        out.push(FreeGame {
            id: format!("steam:{id}"),
            title: html_unescape(title.trim()),
            store: FreeStore::Steam,
            images: vec![format!("{cdn}/{id}/library_600x900_2x.jpg"), format!("{cdn}/{id}/library_600x900.jpg")],
            wide: vec![format!("{cdn}/{id}/header.jpg"), format!("{cdn}/{id}/capsule_616x353.jpg")],
            starts: None,
            ends: None,
            upcoming: false,
            worth,
            description: String::new(),
            epic_slug: None,
            steam_appid: Some(id),
            url,
        });
    }
    out
}

/// GamerPower-Liste: Titel → Enddatum (zum Ergänzen der Steam-Angebote)
pub fn parse_gamerpower_end_dates(v: &serde_json::Value) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    if let Some(arr) = v.as_array() {
        for g in arr {
            let raw_title = s(g, "title");
            let title = raw_title
                .replace("(Steam)", "")
                .replace("Giveaway", "")
                .replace("Free", "")
                .trim()
                .to_string();
            if let Some(end) = parse_datetime(s(g, "end_date")) {
                out.push((normalize_name(&title), end));
            }
        }
    }
    out
}

fn get_json(agent: &ureq::Agent, url: &str) -> Result<serde_json::Value, String> {
    agent
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json::<serde_json::Value>()
        .map_err(|e| e.to_string())
}

/// Alles laden. Fehler werden gesammelt, damit eine kaputte Quelle nicht alles blockiert.
pub fn fetch(lang: &str) -> (Vec<FreeGame>, Vec<String>) {
    let agent = crate::steam::http_agent();
    let country = country_for(lang);
    let mut games = Vec::new();
    let mut errors = Vec::new();

    let epic_url = format!(
        "https://store-site-backend-static.ak.epicgames.com/freeGamesPromotions?locale={}&country={country}&allowCountries={country}",
        epic_locale(lang)
    );
    match get_json(&agent, &epic_url) {
        Ok(v) => games.extend(parse_epic(&v, lang)),
        Err(e) => errors.push(format!("Epic: {e}")),
    }

    let steam_url = format!(
        "https://store.steampowered.com/search/results/?query&start=0&count=50&maxprice=free&specials=1&category1=998&infinite=1&cc={country}&l=english"
    );
    match get_json(&agent, &steam_url) {
        Ok(v) => {
            let mut steam = parse_steam_search(v.get("results_html").and_then(|h| h.as_str()).unwrap_or(""));
            if !steam.is_empty() {
                if let Ok(gp) = get_json(&agent, "https://www.gamerpower.com/api/giveaways?platform=steam&type=game") {
                    let ends = parse_gamerpower_end_dates(&gp);
                    for g in steam.iter_mut() {
                        let n = normalize_name(&g.title);
                        if let Some((_, end)) = ends.iter().find(|(t, _)| *t == n || (!t.is_empty() && n.starts_with(t.as_str()))) {
                            g.ends = Some(*end);
                        }
                    }
                }
            }
            games.extend(steam);
        }
        Err(e) => errors.push(format!("Steam: {e}")),
    }
    // Aktuelle zuerst, dann nach Ende sortieren
    games.sort_by_key(|g| (g.upcoming, g.ends.unwrap_or(u64::MAX)));
    (games, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epic_free_and_upcoming() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"data":{"Catalog":{"searchStore":{"elements":[
            {"title":"Home","id":"a1","productSlug":null,"urlSlug":"ff70",
             "offerMappings":[{"pageSlug":"home-19212a","pageType":"productHome"}],
             "keyImages":[{"type":"OfferImageTall","url":"https://cdn1/tall.png"}],
             "price":{"totalPrice":{"discountPrice":0,"originalPrice":449,"currencyCode":"EUR","fmtPrice":{"originalPrice":"4,49 €"}}},
             "promotions":{"promotionalOffers":[{"promotionalOffers":[{"startDate":"2026-10-22T15:00:00.000Z","endDate":"2026-10-29T15:00:00.000Z","discountSetting":{"discountPercentage":0}}]}],"upcomingPromotionalOffers":[]}},
            {"title":"Next","id":"a2","productSlug":"next-game",
             "price":{"totalPrice":{"discountPrice":1999,"originalPrice":1999}},
             "promotions":{"promotionalOffers":[],"upcomingPromotionalOffers":[{"promotionalOffers":[{"startDate":"2026-10-29T15:00:00.000Z","endDate":"2026-11-05T15:00:00.000Z","discountSetting":{"discountPercentage":0}}]}]}},
            {"title":"Not free","id":"a3","price":{"totalPrice":{"discountPrice":999,"originalPrice":1999}},"promotions":null}
            ]}}}}"#,
        )
        .unwrap();
        let g = parse_epic(&v, "de");
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].epic_slug.as_deref(), Some("home-19212a"));
        assert_eq!(g[0].worth, "4,49 €");
        assert_eq!(g[0].ends, parse_datetime("2026-10-29T15:00:00.000Z"));
        assert!(!g[0].upcoming);
        assert!(g[1].upcoming);
        assert_eq!(g[0].launcher_url(), "com.epicgames.launcher://store/p/home-19212a");
    }

    #[test]
    fn steam_search_rows() {
        let html = r#"<a href="https://store.steampowered.com/app/5340370/Popcorn/?snr=1" data-ds-appid="5340370" class="search_result_row">
          <span class="title">Popcorn &amp; Premieren</span><div class="discount_pct">-100%</div>
          <div class="discount_original_price">4,99€</div><div class="discount_final_price">0,00€</div></a>
          <a href="x" data-ds-appid="1"><span class="title">Not free</span><div class="discount_pct">-50%</div></a>"#;
        let g = parse_steam_search(html);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].title, "Popcorn & Premieren");
        assert_eq!(g[0].steam_appid, Some(5340370));
        assert_eq!(g[0].worth, "4,99€");
        assert!(g[0].launcher_url().starts_with("steam://openurl/"));
        assert!(g[0].matches("Popcorn & Premieren", None));
    }
}
