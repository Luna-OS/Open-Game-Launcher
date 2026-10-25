//! Online-Funktionen der Oberfläche: Gratis-Spiele, Anmeldung, Freunde, Chat.
//! Alle Netzwerkaufrufe laufen im Hintergrund, die Oberfläche bleibt flüssig.

use super::state::*;
use crate::freegames::FreeGame;
use crate::i18n::{t, tf};
use crate::social::{Backend, Session};
use crate::sys;
use dioxus::core::spawn_forever;
use dioxus::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static LOOP_STARTED: AtomicBool = AtomicBool::new(false);
static FRIENDS_LOADED: AtomicBool = AtomicBool::new(false);

// ----------------------------------------------------------------------
// Gratis-Spiele
// ----------------------------------------------------------------------

pub fn load_free(app: App, force: bool) {
    let state = *app.free_state.peek();
    if state == LoadState::Loading || (state == LoadState::Done && !force) {
        return;
    }
    let mut st = app.free_state;
    st.set(LoadState::Loading);
    let lang = crate::i18n::current_code();
    spawn_forever(async move {
        let res = tokio::task::spawn_blocking(move || crate::freegames::fetch(&lang)).await;
        let mut free = app.free;
        let mut st = app.free_state;
        match res {
            Ok((games, errors)) => {
                let failed = games.is_empty() && !errors.is_empty();
                free.set(games);
                st.set(if failed { LoadState::Failed } else { LoadState::Done });
            }
            Err(_) => st.set(LoadState::Failed),
        }
    });
}

/// Ist das Gratis-Spiel schon in der Bibliothek?
pub fn owned_game(app: App, fg: &FreeGame) -> Option<crate::model::Game> {
    let source = match fg.store {
        crate::freegames::FreeStore::Epic => crate::model::Source::Epic,
        crate::freegames::FreeStore::Steam => crate::model::Source::Steam,
    };
    app.games
        .read()
        .iter()
        .find(|g| g.source == source && fg.matches(&g.name, g.steam_appid))
        .cloned()
}

/// "Zur Bibliothek hinzufügen": öffnet das Spiel direkt im Epic Launcher bzw. in Steam.
pub fn claim_free(app: App, fg: &FreeGame) {
    let launcher = fg.launcher_url();
    let opened = match fg.store {
        crate::freegames::FreeStore::Epic if crate::epic::launcher_installed() => sys::open_url(&launcher).is_ok(),
        crate::freegames::FreeStore::Steam if sys::protocol_registered("steam") => sys::open_url(&launcher).is_ok(),
        _ => false,
    };
    if !opened {
        let _ = sys::open_url(&fg.url);
    }
    let store = match fg.store {
        crate::freegames::FreeStore::Epic => "Epic Games",
        crate::freegames::FreeStore::Steam => "Steam",
    };
    app.toast(&tf("free.claim_toast", &[("store", store)]), "download");
    // Sobald es im Store hinzugefügt ist, taucht es automatisch in der Bibliothek auf
    rescan_later(app, &[30, 90, 240, 600]);
}

// ----------------------------------------------------------------------
// Sitzung
// ----------------------------------------------------------------------

pub fn backend(app: App) -> Option<Backend> {
    app.settings.peek().backend()
}

pub fn logged_in(app: App) -> bool {
    app.settings.read().session.is_some()
}

/// Gültige Sitzung holen (läuft sie bald ab, wird sie erneuert)
async fn ensure_session(app: App) -> Option<(Backend, Session)> {
    let b = backend(app)?;
    let s = app.settings.peek().session.clone()?;
    if s.expires_at > sys::now_secs() + 90 {
        return Some((b, s));
    }
    refresh_session(app, b, s).await
}

async fn refresh_session(app: App, b: Backend, s: Session) -> Option<(Backend, Session)> {
    let (b2, s2) = (b.clone(), s.clone());
    match tokio::task::spawn_blocking(move || b2.refresh(&s2)).await {
        Ok(Ok(ns)) => {
            let saved = ns.clone();
            app.update(|st| st.session = Some(saved));
            Some((b, ns))
        }
        Ok(Err(e)) if e == "401" || e.to_lowercase().contains("refresh") || e.to_lowercase().contains("invalid") => {
            // Sitzung ungültig → abmelden
            clear_session(app);
            app.toast(&t("acc.expired"), "user");
            None
        }
        // Netzwerkproblem: alte Sitzung weiter versuchen
        _ => Some((b, s)),
    }
}

/// Datenbank-Aufruf mit automatischer Erneuerung der Sitzung
pub async fn call<T, F>(app: App, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: Fn(&Backend, &Session) -> Result<T, String> + Send + Sync + 'static,
{
    let Some((b, s)) = ensure_session(app).await else {
        return Err(t("acc.err_logged_out"));
    };
    let f = Arc::new(f);
    let (f1, b1, s1) = (f.clone(), b.clone(), s.clone());
    let r = tokio::task::spawn_blocking(move || f1(&b1, &s1)).await.map_err(|e| e.to_string())?;
    if matches!(&r, Err(e) if e == "401") {
        if let Some((b2, s2)) = refresh_session(app, b, s).await {
            return tokio::task::spawn_blocking(move || f(&b2, &s2)).await.map_err(|e| e.to_string())?;
        }
        return Err(t("acc.expired"));
    }
    r
}

fn clear_session(app: App) {
    app.update(|s| s.session = None);
    let (mut me, mut fr, mut chat, mut msgs) = (app.me, app.friends, app.chat_with, app.messages);
    me.set(None);
    fr.set(Vec::new());
    chat.set(None);
    msgs.set(Vec::new());
    FRIENDS_LOADED.store(false, Ordering::Relaxed);
}

fn after_login(app: App, s: Session) {
    app.update(|st| st.session = Some(s));
    let mut busy = app.social_busy;
    busy.set(false);
    let mut err = app.social_error;
    err.set(None);
    app.toast(&t("acc.welcome"), "check-circle");
    refresh_all(app);
    start_loop(app);
}

fn login_failed(app: App, e: String) {
    let mut busy = app.social_busy;
    busy.set(false);
    let mut err = app.social_error;
    err.set(Some(e));
}

pub fn login_oauth(app: App, provider: &'static str) {
    let Some(b) = backend(app) else { return };
    let (mut busy, mut err) = (app.social_busy, app.social_error);
    busy.set(true);
    err.set(None);
    spawn_forever(async move {
        let res = tokio::task::spawn_blocking(move || b.oauth(provider)).await;
        let w = dioxus::desktop::window();
        w.window.set_minimized(false);
        w.window.set_focus();
        match res {
            Ok(Ok(s)) => after_login(app, s),
            Ok(Err(e)) => login_failed(app, e),
            Err(e) => login_failed(app, e.to_string()),
        }
    });
}

pub fn login_email(app: App, email: String, password: String) {
    let Some(b) = backend(app) else { return };
    let (mut busy, mut err) = (app.social_busy, app.social_error);
    busy.set(true);
    err.set(None);
    spawn_forever(async move {
        match tokio::task::spawn_blocking(move || b.login(&email, &password)).await {
            Ok(Ok(s)) => after_login(app, s),
            Ok(Err(e)) => login_failed(app, e),
            Err(e) => login_failed(app, e.to_string()),
        }
    });
}

pub fn signup_email(app: App, email: String, password: String, username: String) {
    let Some(b) = backend(app) else { return };
    let (mut busy, mut err) = (app.social_busy, app.social_error);
    busy.set(true);
    err.set(None);
    spawn_forever(async move {
        match tokio::task::spawn_blocking(move || b.signup(&email, &password, &username)).await {
            Ok(Ok(Some(s))) => after_login(app, s),
            Ok(Ok(None)) => {
                let mut busy = app.social_busy;
                busy.set(false);
                app.open(Modal::Info(t("acc.confirm_title"), t("acc.confirm_text")));
            }
            Ok(Err(e)) => login_failed(app, e),
            Err(e) => login_failed(app, e.to_string()),
        }
    });
}

pub fn reset_password(app: App, email: String) {
    let Some(b) = backend(app) else { return };
    spawn_forever(async move {
        let _ = tokio::task::spawn_blocking(move || b.reset_password(&email)).await;
        app.toast(&t("acc.reset_sent"), "check-circle");
    });
}

pub fn logout(app: App) {
    let b = backend(app);
    let s = app.settings.peek().session.clone();
    if let (Some(b), Some(s)) = (b, s) {
        std::thread::spawn(move || b.go_offline(&s));
    }
    clear_session(app);
    app.toast(&t("acc.logged_out"), "user");
}

/// Beim Schließen des Launchers kurz "offline" melden (max. 1,5 s)
pub fn offline_on_exit(app: App) {
    let b = backend(app);
    let s = app.settings.peek().session.clone();
    if let (Some(b), Some(s)) = (b, s) {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            b.go_offline(&s);
            let _ = tx.send(());
        });
        let _ = rx.recv_timeout(std::time::Duration::from_millis(1500));
    }
}

// ----------------------------------------------------------------------
// Profil, Freunde, Chat
// ----------------------------------------------------------------------

pub fn refresh_all(app: App) {
    spawn_forever(async move {
        if let Ok(p) = call(app, |b, s| b.my_profile(s)).await {
            let mut me = app.me;
            me.set(p);
        }
        refresh_friends_now(app).await;
    });
}

async fn refresh_friends_now(app: App) {
    let Ok(list) = call(app, |b, s| b.friends(s)).await else { return };
    let old = app.friends.peek().clone();
    let chat = app.chat_with.peek().clone();
    let on_friends_page = *app.page.peek() == Page::Friends;
    if FRIENDS_LOADED.swap(true, Ordering::Relaxed) {
        for f in &list {
            let before = old.iter().find(|o| o.id == f.id);
            // Neue Nachricht
            let old_unread = before.map(|o| o.unread).unwrap_or(0);
            let chat_open = on_friends_page && chat.as_deref() == Some(f.id.as_str());
            if f.unread > old_unread && !chat_open {
                app.toast(&tf("friends.new_message", &[("name", &f.name())]), "message");
            }
            // Neue Freundschaftsanfrage
            if f.incoming && !f.accepted() && before.is_none() {
                app.toast(&tf("friends.new_request", &[("name", &f.name())]), "user");
            }
            // Anfrage wurde angenommen
            if f.accepted() && before.map(|o| !o.accepted()).unwrap_or(false) && !f.incoming {
                app.toast(&tf("friends.accepted", &[("name", &f.name())]), "check-circle");
            }
            // Freund hat ein Spiel gestartet
            if let Some(game) = &f.playing {
                let was = before.and_then(|o| o.playing.clone());
                if was.as_deref() != Some(game.as_str()) && before.is_some() {
                    app.toast(&tf("friends.started_playing", &[("name", &f.name()), ("game", game)]), "gamepad");
                }
            }
        }
    }
    let mut fr = app.friends;
    fr.set(list);
}

pub fn refresh_friends(app: App) {
    spawn_forever(async move { refresh_friends_now(app).await });
}

pub fn add_friend(app: App, user_id: String, name: String) {
    spawn_forever(async move {
        let uid = user_id.clone();
        match call(app, move |b, s| b.add_friend(s, &uid)).await {
            Ok(r) => {
                let key = match r.as_str() {
                    "sent" => "friends.sent",
                    "accepted" => "friends.now_friends",
                    "exists" => "friends.exists",
                    "self" => "friends.self",
                    _ => "friends.not_found",
                };
                app.toast(&tf(key, &[("name", &name)]), "user");
                refresh_friends_now(app).await;
            }
            Err(e) => app.toast(&e, "alert"),
        }
    });
}

pub fn respond(app: App, friendship_id: i64, accept: bool) {
    spawn_forever(async move {
        match call(app, move |b, s| b.respond(s, friendship_id, accept)).await {
            Ok(()) => refresh_friends_now(app).await,
            Err(e) => app.toast(&e, "alert"),
        }
    });
}

pub fn remove_friend(app: App, friendship_id: i64, friend_id: String) {
    spawn_forever(async move {
        match call(app, move |b, s| b.remove_friend(s, friendship_id)).await {
            Ok(()) => {
                if app.chat_with.peek().as_deref() == Some(friend_id.as_str()) {
                    let (mut c, mut m) = (app.chat_with, app.messages);
                    c.set(None);
                    m.set(Vec::new());
                }
                app.toast(&t("friends.removed"), "trash");
                refresh_friends_now(app).await;
            }
            Err(e) => app.toast(&e, "alert"),
        }
    });
}

pub fn open_chat(app: App, friend_id: String) {
    let (mut c, mut m) = (app.chat_with, app.messages);
    c.set(Some(friend_id.clone()));
    m.set(Vec::new());
    spawn_forever(async move {
        let fid = friend_id.clone();
        if let Ok(list) = call(app, move |b, s| b.messages(s, &fid, 0)).await {
            if app.chat_with.peek().as_deref() == Some(friend_id.as_str()) {
                let mut m = app.messages;
                m.set(list);
            }
        }
        let fid = friend_id.clone();
        let _ = call(app, move |b, s| b.mark_read(s, &fid)).await;
        let mut fr = app.friends;
        if let Some(f) = fr.write().iter_mut().find(|f| f.id == friend_id) {
            f.unread = 0;
        }
    });
}

/// Neue Nachrichten im offenen Chat holen
async fn poll_chat(app: App) {
    let Some(fid) = app.chat_with.peek().clone() else { return };
    let after = app.messages.peek().last().map(|m| m.id).unwrap_or(0);
    let f2 = fid.clone();
    let Ok(new) = call(app, move |b, s| b.messages(s, &f2, after)).await else { return };
    if new.is_empty() || app.chat_with.peek().as_deref() != Some(fid.as_str()) {
        return;
    }
    let from_friend = new.iter().any(|m| m.sender == fid);
    {
        let mut m = app.messages;
        let mut w = m.write();
        for msg in new {
            if !w.iter().any(|x| x.id == msg.id) {
                w.push(msg);
            }
        }
    }
    if from_friend {
        let f3 = fid.clone();
        let _ = call(app, move |b, s| b.mark_read(s, &f3)).await;
    }
}

pub fn send_message(app: App, body: String) {
    let body = body.trim().to_string();
    let Some(fid) = app.chat_with.peek().clone() else { return };
    if body.is_empty() {
        return;
    }
    spawn_forever(async move {
        let (f2, b2) = (fid.clone(), body.clone());
        match call(app, move |b, s| b.send(s, &f2, &b2)).await {
            Ok(()) => poll_chat(app).await,
            Err(e) => app.toast(&e, "alert"),
        }
    });
}

pub fn save_profile(app: App, username: String, display: String) {
    spawn_forever(async move {
        let (u, d) = (username.clone(), display.clone());
        match call(app, move |b, s| b.update_profile(s, &u, &d)).await {
            Ok(r) if r == "ok" => {
                app.toast(&t("acc.saved"), "check-circle");
                refresh_all(app);
            }
            Ok(r) if r == "taken" => app.toast(&t("acc.err_taken"), "alert"),
            Ok(_) => app.toast(&t("acc.err_username"), "alert"),
            Err(e) => app.toast(&e, "alert"),
        }
    });
}

// ----------------------------------------------------------------------
// Hintergrund-Abgleich
// ----------------------------------------------------------------------

/// Startet (einmalig) den Abgleich: alle 30 s online melden + Freunde, alle 3 s offener Chat.
pub fn start_loop(app: App) {
    if LOOP_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    // Online-Status, "spielt gerade", Freundesliste
    spawn_forever(async move {
        loop {
            if app.settings.peek().session.is_some() {
                let games = app.games.peek().clone();
                let share = app.settings.peek().share_playing;
                let playing = tokio::task::spawn_blocking(move || crate::social::detect_playing(&games)).await.ok().flatten();
                {
                    let mut pn = app.playing_now;
                    if *pn.peek() != playing {
                        pn.set(playing.clone());
                    }
                }
                let shown = if share { playing } else { None };
                let _ = call(app, move |b, s| b.heartbeat(s, shown.as_deref())).await;
                refresh_friends_now(app).await;
            }
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
    // Offener Chat
    spawn_forever(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if app.settings.peek().session.is_some() && *app.page.peek() == Page::Friends && app.chat_with.peek().is_some() {
                poll_chat(app).await;
            }
        }
    });
}

/// Uhrzeit einer Nachricht in Ortszeit ("14:05" oder "24.10. 14:05")
pub fn message_time(iso: &str) -> String {
    let Some(ts) = crate::util::parse_datetime(iso) else { return String::new() };
    #[cfg(windows)]
    let off = crate::winffi::utc_offset_secs();
    #[cfg(not(windows))]
    let off = 0i64;
    let local = ts as i64 + off;
    let now = sys::now_secs() as i64 + off;
    let (_, mo, d, h, mi) = crate::util::civil(local);
    let (_, nmo, nd, _, _) = crate::util::civil(now);
    if (mo, d) == (nmo, nd) {
        format!("{h:02}:{mi:02}")
    } else {
        format!("{d:02}.{mo:02}. {h:02}:{mi:02}")
    }
}

/// Seit wann spielt ein Freund ("seit 25 Min.")
pub fn playing_for(iso: &Option<String>) -> String {
    let Some(ts) = iso.as_deref().and_then(crate::util::parse_datetime) else { return String::new() };
    let mins = sys::now_secs().saturating_sub(ts) / 60;
    sys::format_playtime(mins.max(1))
}

/// Restzeit eines Gratis-Angebots
pub fn remaining(fg: &FreeGame) -> String {
    let now = sys::now_secs();
    let (key_d, key_h, key_m, target) = if fg.upcoming {
        ("free.in_days", "free.in_hours", "free.in_min", fg.starts)
    } else {
        ("free.left_days", "free.left_hours", "free.left_min", fg.ends)
    };
    let Some(target) = target else {
        return if fg.upcoming { t("free.soon") } else { t("free.now") };
    };
    let secs = target.saturating_sub(now);
    let n;
    let key = if secs >= 2 * 86_400 {
        n = secs / 86_400;
        key_d
    } else if secs >= 3600 {
        n = secs / 3600;
        key_h
    } else {
        n = (secs / 60).max(1);
        key_m
    };
    tf(key, &[("n", &n.to_string())])
}
