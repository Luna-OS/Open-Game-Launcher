//! Konto, Freunde, Chat und "spielt gerade" – über Supabase.
//!
//! Anmeldung: Google oder GitHub (OAuth mit PKCE über den Browser) oder E-Mail.
//! Alle Datenbank-Zugriffe laufen über die Funktionen in `supabase/schema.sql`.
//! Alles hier ist blockierend und wird von der Oberfläche im Hintergrund aufgerufen.

use crate::i18n::t;
use crate::util;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

/// Fester Port für die Rückleitung nach dem Login im Browser.
/// Muss in Supabase unter "Redirect URLs" eingetragen sein: http://127.0.0.1:47821/callback
pub const CALLBACK_PORT: u16 = 47821;

/// Beim Bauen eingetragene Server-Daten (GitHub-Secrets MAZ_SUPABASE_URL / MAZ_SUPABASE_KEY)
pub fn builtin_server() -> (String, String) {
    let url = option_env!("MAZ_SUPABASE_URL").unwrap_or("").trim().trim_end_matches('/').to_string();
    let key = option_env!("MAZ_SUPABASE_KEY").unwrap_or("").trim().to_string();
    (url, key)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub user_id: String,
    pub email: String,
    pub provider: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
#[serde(default)]
pub struct Profile {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl Profile {
    pub fn name(&self) -> String {
        self.display_name.clone().filter(|d| !d.trim().is_empty()).unwrap_or_else(|| self.username.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
#[serde(default)]
pub struct Friend {
    pub friendship_id: i64,
    /// "pending" oder "accepted"
    pub status: String,
    /// true = Anfrage an mich
    pub incoming: bool,
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub playing: Option<String>,
    pub playing_since: Option<String>,
    pub online: bool,
    pub unread: i64,
}

impl Friend {
    pub fn name(&self) -> String {
        self.display_name.clone().filter(|d| !d.trim().is_empty()).unwrap_or_else(|| self.username.clone())
    }
    pub fn accepted(&self) -> bool {
        self.status == "accepted"
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
#[serde(default)]
pub struct Message {
    pub id: i64,
    pub sender: String,
    pub receiver: String,
    pub body: String,
    pub created_at: String,
    pub read_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Backend {
    pub url: String,
    pub key: String,
}

fn agent() -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    let tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .user_agent("MAZ-Launcher")
        .build()
        .into()
}

/// Fehlermeldung aus einer Supabase-Antwort lesen
fn error_text(v: &serde_json::Value, status: u16) -> String {
    for k in ["error_description", "msg", "message", "error"] {
        if let Some(s) = v.get(k).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return friendly_error(s);
            }
        }
    }
    format!("HTTP {status}")
}

/// Häufige Fehler verständlich machen
fn friendly_error(raw: &str) -> String {
    let l = raw.to_lowercase();
    if l.contains("invalid login credentials") {
        t("acc.err_credentials")
    } else if l.contains("email not confirmed") {
        t("acc.err_not_confirmed")
    } else if l.contains("already registered") || l.contains("already been registered") {
        t("acc.err_exists")
    } else if l.contains("password should be") || l.contains("weak") {
        t("acc.err_password")
    } else if l.contains("not_friends") {
        t("acc.err_not_friends")
    } else if l.contains("provider is not enabled") || l.contains("unsupported provider") {
        t("acc.err_provider")
    } else if l.contains("rate limit") {
        t("acc.err_rate")
    } else {
        raw.to_string()
    }
}

impl Backend {
    pub fn new(url: &str, key: &str) -> Option<Backend> {
        let url = url.trim().trim_end_matches('/').to_string();
        let key = key.trim().to_string();
        (url.starts_with("https://") && key.len() > 20).then_some(Backend { url, key })
    }

    fn post(&self, path: &str, token: Option<&str>, body: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut req = agent()
            .post(&format!("{}{path}", self.url))
            .header("apikey", &self.key)
            .header("Content-Type", "application/json");
        if let Some(tok) = token {
            req = req.header("Authorization", &format!("Bearer {tok}"));
        }
        let mut resp = req.send_json(body).map_err(|e| format!("{}: {e}", t("acc.err_network")))?;
        let status = resp.status().as_u16();
        let text = resp.body_mut().read_to_string().unwrap_or_default();
        let v: serde_json::Value = if text.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text.clone()))
        };
        if status >= 400 {
            if status == 401 {
                return Err("401".into());
            }
            return Err(error_text(&v, status));
        }
        Ok(v)
    }

    // ------------------------------------------------------------------
    // Anmeldung
    // ------------------------------------------------------------------

    fn session_from(v: &serde_json::Value, provider: &str) -> Option<Session> {
        let access = v.get("access_token")?.as_str()?.to_string();
        let refresh = v.get("refresh_token").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let expires_in = v.get("expires_in").and_then(|x| x.as_u64()).unwrap_or(3600);
        let expires_at = v
            .get("expires_at")
            .and_then(|x| x.as_u64())
            .unwrap_or_else(|| crate::sys::now_secs() + expires_in);
        let user = v.get("user");
        Some(Session {
            access_token: access,
            refresh_token: refresh,
            expires_at,
            user_id: user.and_then(|u| u.get("id")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
            email: user.and_then(|u| u.get("email")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
            provider: provider.to_string(),
        })
    }

    /// E-Mail-Registrierung. `Ok(None)` = Bestätigungs-Mail wurde verschickt.
    pub fn signup(&self, email: &str, password: &str, username: &str) -> Result<Option<Session>, String> {
        let body = serde_json::json!({ "email": email.trim(), "password": password, "data": { "username": username.trim() } });
        let v = self.post("/auth/v1/signup", None, &body)?;
        Ok(Self::session_from(&v, "email"))
    }

    pub fn login(&self, email: &str, password: &str) -> Result<Session, String> {
        let body = serde_json::json!({ "email": email.trim(), "password": password });
        let v = self.post("/auth/v1/token?grant_type=password", None, &body)?;
        Self::session_from(&v, "email").ok_or_else(|| t("acc.err_unknown"))
    }

    pub fn reset_password(&self, email: &str) -> Result<(), String> {
        self.post("/auth/v1/recover", None, &serde_json::json!({ "email": email.trim() })).map(|_| ())
    }

    pub fn refresh(&self, s: &Session) -> Result<Session, String> {
        let v = self.post(
            "/auth/v1/token?grant_type=refresh_token",
            None,
            &serde_json::json!({ "refresh_token": s.refresh_token }),
        )?;
        let mut ns = Self::session_from(&v, &s.provider).ok_or_else(|| t("acc.err_unknown"))?;
        if ns.user_id.is_empty() {
            ns.user_id = s.user_id.clone();
        }
        if ns.email.is_empty() {
            ns.email = s.email.clone();
        }
        Ok(ns)
    }

    /// Anmeldung mit Google oder GitHub: öffnet den Browser und wartet auf die Rückkehr.
    pub fn oauth(&self, provider: &str) -> Result<Session, String> {
        let listener = TcpListener::bind(("127.0.0.1", CALLBACK_PORT)).map_err(|_| t("acc.err_port"))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let verifier = util::base64url(&util::random_bytes(48));
        let challenge = util::base64url(&util::sha256(verifier.as_bytes()));
        let redirect = format!("http://127.0.0.1:{CALLBACK_PORT}/callback");
        let url = format!(
            "{}/auth/v1/authorize?provider={provider}&redirect_to={}&code_challenge={challenge}&code_challenge_method=s256",
            self.url,
            crate::sys::url_encode(&redirect)
        );
        crate::sys::open_url(&url).map_err(|_| t("acc.err_browser"))?;

        let deadline = Instant::now() + Duration::from_secs(300);
        let code = loop {
            if Instant::now() > deadline {
                return Err(t("acc.err_timeout"));
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    let mut buf = [0u8; 8192];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let path = req.lines().next().unwrap_or("").split_whitespace().nth(1).unwrap_or("").to_string();
                    if !path.starts_with("/callback") {
                        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        continue;
                    }
                    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
                    let code = util::query_param(query, "code");
                    let err = util::query_param(query, "error_description").or_else(|| util::query_param(query, "error"));
                    let ok = code.is_some();
                    let page = callback_page(ok, err.as_deref());
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                        page.len()
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                    match (code, err) {
                        (Some(c), _) => break c,
                        (None, Some(e)) => return Err(friendly_error(&e)),
                        (None, None) => return Err(t("acc.err_unknown")),
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(150)),
                Err(e) => return Err(e.to_string()),
            }
        };
        let v = self.post(
            "/auth/v1/token?grant_type=pkce",
            None,
            &serde_json::json!({ "auth_code": code, "code_verifier": verifier }),
        )?;
        Self::session_from(&v, provider).ok_or_else(|| t("acc.err_unknown"))
    }

    // ------------------------------------------------------------------
    // Datenbank-Funktionen
    // ------------------------------------------------------------------

    pub fn rpc(&self, s: &Session, name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        self.post(&format!("/rest/v1/rpc/{name}"), Some(&s.access_token), &args)
    }

    pub fn my_profile(&self, s: &Session) -> Result<Option<Profile>, String> {
        let v = self.rpc(s, "my_profile", serde_json::json!({}))?;
        let list: Vec<Profile> = serde_json::from_value(v).unwrap_or_default();
        Ok(list.into_iter().next())
    }

    /// "ok" | "taken" | "invalid"
    pub fn update_profile(&self, s: &Session, username: &str, display_name: &str) -> Result<String, String> {
        let v = self.rpc(s, "update_profile", serde_json::json!({ "p_username": username.trim(), "p_display_name": display_name }))?;
        Ok(v.as_str().unwrap_or("invalid").to_string())
    }

    pub fn heartbeat(&self, s: &Session, playing: Option<&str>) -> Result<(), String> {
        self.rpc(s, "heartbeat", serde_json::json!({ "p_playing": playing })).map(|_| ())
    }

    pub fn go_offline(&self, s: &Session) {
        let _ = self.rpc(s, "go_offline", serde_json::json!({}));
    }

    pub fn search(&self, s: &Session, query: &str) -> Result<Vec<Profile>, String> {
        let v = self.rpc(s, "search_users", serde_json::json!({ "p_query": query }))?;
        Ok(serde_json::from_value(v).unwrap_or_default())
    }

    pub fn friends(&self, s: &Session) -> Result<Vec<Friend>, String> {
        let v = self.rpc(s, "friends_overview", serde_json::json!({}))?;
        serde_json::from_value(v).map_err(|e| e.to_string())
    }

    /// "sent" | "accepted" | "exists" | "self" | "not_found"
    pub fn add_friend(&self, s: &Session, user_id: &str) -> Result<String, String> {
        let v = self.rpc(s, "send_friend_request", serde_json::json!({ "p_user": user_id }))?;
        Ok(v.as_str().unwrap_or("not_found").to_string())
    }

    pub fn respond(&self, s: &Session, friendship_id: i64, accept: bool) -> Result<(), String> {
        self.rpc(s, "respond_friend_request", serde_json::json!({ "p_id": friendship_id, "p_accept": accept })).map(|_| ())
    }

    pub fn remove_friend(&self, s: &Session, friendship_id: i64) -> Result<(), String> {
        self.rpc(s, "remove_friend", serde_json::json!({ "p_id": friendship_id })).map(|_| ())
    }

    pub fn messages(&self, s: &Session, friend_id: &str, after: i64) -> Result<Vec<Message>, String> {
        let v = self.rpc(s, "get_messages", serde_json::json!({ "p_friend": friend_id, "p_after": after }))?;
        serde_json::from_value(v).map_err(|e| e.to_string())
    }

    pub fn send(&self, s: &Session, friend_id: &str, body: &str) -> Result<(), String> {
        self.rpc(s, "send_message", serde_json::json!({ "p_friend": friend_id, "p_body": body })).map(|_| ())
    }

    pub fn mark_read(&self, s: &Session, friend_id: &str) -> Result<(), String> {
        self.rpc(s, "mark_read", serde_json::json!({ "p_friend": friend_id })).map(|_| ())
    }
}

/// Seite, die der Browser nach dem Login anzeigt
fn callback_page(ok: bool, err: Option<&str>) -> String {
    let (title, text, color) = if ok {
        (t("acc.browser_ok_title"), t("acc.browser_ok_text"), "#22c55e")
    } else {
        (t("acc.browser_err_title"), err.map(friendly_error).unwrap_or_else(|| t("acc.err_unknown")), "#ef4444")
    };
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>MAZ Launcher</title></head>
<body style="margin:0;height:100vh;display:grid;place-items:center;background:#0a0a12;color:#ecebf5;font-family:Segoe UI,system-ui,sans-serif">
<div style="text-align:center;max-width:420px;padding:32px">
<div style="width:72px;height:72px;margin:0 auto 20px;border-radius:20px;background:linear-gradient(135deg,#8b5cf6,#22d3ee);display:grid;place-items:center;font-weight:900;color:#fff;font-size:20px">MAZ</div>
<h1 style="margin:0 0 10px;font-size:24px;color:{color}">{}</h1><p style="color:#9895b0;line-height:1.6">{}</p></div>
<script>setTimeout(function(){{window.close()}},2500)</script></body></html>"#,
        esc(&title),
        esc(&text)
    )
}

// ----------------------------------------------------------------------
// Was spiele ich gerade? (für "spielt gerade")
// ----------------------------------------------------------------------

/// Laufendes Spiel erkennen: Steam meldet es selbst, sonst laufende Prozesse vergleichen.
pub fn detect_playing(games: &[crate::model::Game]) -> Option<String> {
    let running_appid = crate::sys::reg_u32(crate::sys::Hive::CurrentUser, "Software\\Valve\\Steam", "RunningAppID").unwrap_or(0);
    if running_appid != 0 {
        if let Some(g) = games.iter().find(|g| g.steam_appid == Some(running_appid)) {
            return Some(g.name.clone());
        }
    }
    let procs = running_processes();
    if procs.is_empty() {
        return None;
    }
    for g in games.iter().filter(|g| g.installed) {
        if let Some(exe) = &g.exe {
            let file = std::path::Path::new(exe)
                .file_name()
                .map(|f| f.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !file.is_empty() && file.ends_with(".exe") && procs.iter().any(|p| *p == file) {
                return Some(g.name.clone());
            }
        }
    }
    None
}

fn running_processes() -> Vec<String> {
    #[cfg(windows)]
    {
        crate::winffi::process_names()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_parsing() {
        let v = serde_json::json!({
            "access_token": "abc", "refresh_token": "r", "expires_in": 3600,
            "user": { "id": "u1", "email": "a@b.de" }
        });
        let s = Backend::session_from(&v, "github").unwrap();
        assert_eq!(s.user_id, "u1");
        assert_eq!(s.provider, "github");
        assert!(s.expires_at > 3600);
        assert!(Backend::session_from(&serde_json::json!({"user": {}}), "x").is_none());
        assert!(Backend::new("http://unsicher", "1234567890123456789012345").is_none());
        assert!(Backend::new("https://abc.supabase.co/", "1234567890123456789012345").is_some());
    }

    #[test]
    fn friend_json() {
        let v = serde_json::json!([{
            "friendship_id": 3, "status": "accepted", "incoming": false, "id": "u2", "username": "lisa",
            "display_name": "Lisa M", "avatar_url": null, "playing": "Cyberpunk 2077",
            "playing_since": "2026-10-25T10:00:00+00:00", "online": true, "unread": 2
        }]);
        let f: Vec<Friend> = serde_json::from_value(v).unwrap();
        assert_eq!(f[0].name(), "Lisa M");
        assert!(f[0].accepted());
        assert_eq!(f[0].unread, 2);
    }
}
