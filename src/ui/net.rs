//! Online-Funktionen der Oberfläche: Gratis-Spiele und Epic-Bibliothek (ohne MAZ-Konto).
//! Alle Netzwerkaufrufe laufen im Hintergrund, die Oberfläche bleibt flüssig.

use super::state::*;
use crate::freegames::FreeGame;
use crate::i18n::{t, tf};
use crate::sys;
use dioxus::core::spawn_forever;
use dioxus::prelude::*;

// ----------------------------------------------------------------------
// Epic-Konto: komplette Epic-Bibliothek
// ----------------------------------------------------------------------

/// Bibliothek mit dem gespeicherten Epic-Konto neu laden.
/// `force` = false → höchstens alle 6 Stunden.
pub fn sync_epic(app: App, force: bool) {
    let Some(auth) = app.settings.peek().epic_auth.clone() else { return };
    let last = app.settings.peek().epic_synced;
    if *app.epic_busy.peek() || (!force && sys::now_secs().saturating_sub(last) < 6 * 3600) {
        return;
    }
    run_epic(app, move || {
        let fresh = crate::epicacct::ensure_fresh(&auth)?;
        let list = crate::epicacct::fetch_library(&fresh, &crate::epicacct::load_cache(), &crate::i18n::current_code())?;
        Ok((fresh, list))
    });
}

/// Code aus dem Browser (oder die ganze kopierte Seite) → Konto verbinden
pub fn epic_connect(app: App, pasted: String) {
    if *app.epic_busy.peek() {
        return;
    }
    let Some(code) = crate::epicacct::extract_code(&pasted) else {
        let mut err = app.epic_error;
        err.set(Some(t("epic.err_code")));
        return;
    };
    run_epic(app, move || {
        let auth = crate::epicacct::exchange(&code)?;
        let list = crate::epicacct::fetch_library(&auth, &crate::epicacct::load_cache(), &crate::i18n::current_code())?;
        Ok((auth, list))
    });
}

fn run_epic<F>(app: App, job: F)
where
    F: FnOnce() -> Result<(crate::epicacct::EpicAuth, Vec<crate::epicacct::OwnedEpic>), String> + Send + 'static,
{
    let (mut busy, mut err) = (app.epic_busy, app.epic_error);
    busy.set(true);
    err.set(None);
    spawn_forever(async move {
        let res = tokio::task::spawn_blocking(job).await;
        let (mut busy, mut err) = (app.epic_busy, app.epic_error);
        busy.set(false);
        match res {
            Ok(Ok((auth, list))) => {
                let n = list.len();
                crate::epicacct::save_cache(&list);
                app.update(|s| {
                    s.epic_auth = Some(auth);
                    s.epic_synced = sys::now_secs();
                    s.epic_show_uninstalled = true;
                });
                app.toast(&tf("epic.synced_toast", &[("n", &n.to_string())]), "check");
                rescan(app);
            }
            Ok(Err(e)) => {
                if e == t("epic.err_expired") {
                    // Anmeldung abgelaufen: Liste bleibt (Cache), nur neu verbinden nötig
                    app.update(|s| s.epic_auth = None);
                }
                err.set(Some(e));
            }
            Err(_) => err.set(Some(t("common.err_unknown"))),
        }
    });
}

pub fn epic_disconnect(app: App) {
    crate::epicacct::clear_cache();
    app.update(|s| {
        s.epic_auth = None;
        s.epic_synced = 0;
    });
    let mut err = app.epic_error;
    err.set(None);
    rescan(app);
}

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
