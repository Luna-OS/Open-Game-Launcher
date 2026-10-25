//! Globaler Zustand der Oberfläche + alle Aktionen (Spielen, Favorit, Neu laden …).

use crate::model::{normalize_name, Game, LaunchTarget, Settings, Source, StoreStatus};
use crate::problem::{Fix, Problem};
use crate::i18n::{t, tf};
use crate::{config, launch, library, sys};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Installed,
    Steam,
    Epic,
    Custom,
    Favorites,
    Recent,
    Hidden,
}

impl Filter {
    pub fn label(self) -> String {
        t(match self {
            Filter::All => "filter.all",
            Filter::Installed => "filter.installed",
            Filter::Steam => "filter.steam",
            Filter::Epic => "filter.epic",
            Filter::Custom => "filter.custom",
            Filter::Favorites => "filter.favorites",
            Filter::Recent => "filter.recent",
            Filter::Hidden => "filter.hidden",
        })
    }
    pub fn icon(self) -> &'static str {
        match self {
            Filter::All => "library",
            Filter::Installed => "drive",
            Filter::Steam => "gamepad",
            Filter::Epic => "layers",
            Filter::Custom => "rocket",
            Filter::Favorites => "star",
            Filter::Recent => "clock",
            Filter::Hidden => "eye-off",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTab {
    Design,
    Language,
    Account,
    Connections,
    Custom,
    General,
    About,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Library,
    Free,
    Friends,
    Settings(SettingsTab),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Modal {
    /// Sicherheitsfrage vor dem Start eines eigenen Spiels
    Confirm(String),
    /// Nutzer hat "Nein" gesagt
    NotStarted(String),
    /// Fehler mit Erklärung (+ optional Spiel-ID für Aktionen)
    Problem(Problem, Option<String>),
    /// Eigenes Spiel hinzufügen (None) oder bearbeiten
    EditGame(Option<String>),
    DeleteGame(String),
    Detail(String),
    Welcome,
    /// Einfacher Hinweis (Titel, Text)
    Info(String, String),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Toast {
    pub id: u64,
    pub text: String,
    pub icon: &'static str,
}

#[derive(Clone, Copy)]
pub struct App {
    pub settings: Signal<Settings>,
    pub games: Signal<Vec<Game>>,
    pub steam: Signal<StoreStatus>,
    pub epic: Signal<StoreStatus>,
    pub warnings: Signal<Vec<Problem>>,
    pub scanning: Signal<bool>,
    pub loaded: Signal<bool>,
    pub page: Signal<Page>,
    pub filter: Signal<Filter>,
    pub search: Signal<String>,
    pub selected: Signal<Option<String>>,
    pub modal: Signal<Option<Modal>>,
    pub toasts: Signal<Vec<Toast>>,
    // Gratis-Spiele
    pub free: Signal<Vec<crate::freegames::FreeGame>>,
    pub free_state: Signal<LoadState>,
    // Konto & Freunde
    pub me: Signal<Option<crate::social::Profile>>,
    pub friends: Signal<Vec<crate::social::Friend>>,
    pub chat_with: Signal<Option<String>>,
    pub messages: Signal<Vec<crate::social::Message>>,
    pub social_busy: Signal<bool>,
    pub social_error: Signal<Option<String>>,
    pub playing_now: Signal<Option<String>>,
    // Epic-Konto
    pub epic_busy: Signal<bool>,
    pub epic_error: Signal<Option<String>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LoadState {
    #[default]
    Idle,
    Loading,
    Done,
    Failed,
}

pub fn use_app() -> App {
    use_context::<App>()
}

impl App {
    /// Einstellungen ändern und sofort speichern.
    pub fn update(&self, f: impl FnOnce(&mut Settings)) {
        let mut s = self.settings;
        f(&mut s.write());
        let snapshot = s.peek().clone();
        if let Err(e) = config::save(&snapshot) {
            self.toast(&tf("toast.save_failed", &[("error", &e.to_string())]), "alert");
        }
    }

    pub fn open(&self, m: Modal) {
        let mut modal = self.modal;
        modal.set(Some(m));
    }

    pub fn close_modal(&self) {
        let mut modal = self.modal;
        modal.set(None);
    }

    pub fn show_problem(&self, p: Problem, game_id: Option<String>) {
        self.open(Modal::Problem(p, game_id));
    }

    pub fn toast(&self, text: &str, icon: &'static str) {
        let id = sys::now_secs().wrapping_mul(1000) + (self.toasts.peek().len() as u64) + rand_small();
        let mut toasts = self.toasts;
        toasts.write().push(Toast { id, text: text.to_string(), icon });
        dioxus::core::spawn_forever(async move {
            tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
            toasts.write().retain(|t| t.id != id);
        });
    }

    pub fn game(&self, id: &str) -> Option<Game> {
        self.games.peek().iter().find(|g| g.id == id).cloned()
    }

    pub fn go(&self, page: Page) {
        let mut p = self.page;
        p.set(page);
    }
}

fn rand_small() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 % 997)
        .unwrap_or(0)
}

// ----------------------------------------------------------------------
// Bibliothek laden
// ----------------------------------------------------------------------

pub fn rescan(app: App) {
    if *app.scanning.peek() {
        return;
    }
    let mut scanning = app.scanning;
    scanning.set(true);
    let settings = app.settings.peek().clone();
    dioxus::core::spawn_forever(async move {
        let s2 = settings.clone();
        let res = tokio::task::spawn_blocking(move || library::scan(&s2)).await;
        let mut games = app.games;
        let mut steam = app.steam;
        let mut epic = app.epic;
        let mut warnings = app.warnings;
        let mut loaded = app.loaded;
        if let Ok(r) = res {
            games.set(r.games);
            steam.set(r.steam);
            epic.set(r.epic);
            warnings.set(r.warnings);
        }
        loaded.set(true);
        scanning.set(false);

        // Phase 2: schöne Cover für Epic- und eigene Spiele aus dem Steam-Store
        if settings.online_art {
            let names: Vec<String> = games
                .peek()
                .iter()
                .filter(|g| g.source != Source::Steam)
                .map(|g| g.name.clone())
                .collect();
            let known = settings.art_lookup.clone();
            if let Ok(found) = tokio::task::spawn_blocking(move || library::lookup_missing_art(names, &known)).await {
                if !found.is_empty() {
                    app.update(|s| s.art_lookup.extend(found.clone()));
                    let lookup = app.settings.peek().art_lookup.clone();
                    for g in games.write().iter_mut() {
                        if g.source != Source::Steam && found.contains_key(&normalize_name(&g.name)) {
                            library::apply_lookup(g, &lookup);
                        }
                    }
                }
            }
        }
    });
}

/// Bibliothek nach ein paar Sekunden erneut einlesen (z. B. nach Installation oder Gratis-Spiel)
pub fn rescan_later(app: App, delays_secs: &'static [u64]) {
    dioxus::core::spawn_forever(async move {
        let mut waited = 0;
        for d in delays_secs {
            tokio::time::sleep(std::time::Duration::from_secs(d - waited)).await;
            waited = *d;
            rescan(app);
        }
    });
}

/// Ein eigenes Spiel nach dem Hinzufügen/Bearbeiten neu in die Liste übernehmen.
pub fn refresh_custom(app: App, id: String) {
    let settings = app.settings.peek().clone();
    let Some(c) = settings.custom(&id).cloned() else {
        let mut games = app.games;
        games.write().retain(|g| g.id != id);
        return;
    };
    // Sofort anzeigen (ohne Icon), dann im Hintergrund Icon + Cover holen
    let quick = library::custom_game(&c, &settings.art_lookup, false);
    {
        let mut games = app.games;
        let mut w = games.write();
        match w.iter_mut().find(|g| g.id == id) {
            Some(g) => *g = quick,
            None => w.push(quick),
        }
    }
    let online = settings.online_art;
    dioxus::core::spawn_forever(async move {
        let name = c.name.clone();
        let known = settings.art_lookup.clone();
        let res = tokio::task::spawn_blocking(move || {
            let found = if online {
                library::lookup_missing_art(vec![name], &known)
            } else {
                Default::default()
            };
            let mut lookup = known.clone();
            lookup.extend(found.clone());
            (library::custom_game(&c, &lookup, true), found)
        })
        .await;
        if let Ok((game, found)) = res {
            if !found.is_empty() {
                app.update(|s| s.art_lookup.extend(found));
            }
            let mut games = app.games;
            let mut list = games.write();
            if let Some(g) = list.iter_mut().find(|g| g.id == game.id) {
                *g = game;
            }
            drop(list);
        }
    });
}

// ----------------------------------------------------------------------
// Spielen
// ----------------------------------------------------------------------

fn mark_played(app: App, id: &str) {
    let id = id.to_string();
    app.update(|s| {
        s.last_played.insert(id.clone(), sys::now_secs());
        *s.launch_count.entry(id).or_insert(0) += 1;
    });
}

fn minimize_window() {
    dioxus::desktop::window().window.set_minimized(true);
}

fn restore_window() {
    let w = dioxus::desktop::window();
    w.window.set_minimized(false);
    w.window.set_focus();
}

/// "Spielen" wurde geklickt.
pub fn play(app: App, id: String) {
    let Some(game) = app.game(&id) else { return };
    match game.source {
        Source::Steam | Source::Epic => match launch::launch_store(&game) {
            Ok(()) => {
                let installing = matches!(game.launch, LaunchTarget::SteamInstall(_) | LaunchTarget::EpicInstall(_));
                let store = if game.source == Source::Steam { "Steam" } else { "Epic Games" };
                if installing {
                    app.toast(&tf("toast.installing", &[("name", &game.name), ("store", store)]), "download");
                    // Nach der Installation taucht das Spiel automatisch als installiert auf
                    rescan_later(app, &[90, 300, 900]);
                } else {
                    mark_played(app, &id);
                    app.toast(&tf("toast.started_via", &[("name", &game.name), ("store", store)]), "rocket");
                    if app.settings.peek().minimize_on_launch {
                        minimize_window();
                    }
                    // Stürzt das Spiel direkt ab? Dann erklären, was los ist.
                    let g = game.clone();
                    let gid = id.clone();
                    dioxus::core::spawn_forever(async move {
                        let res = tokio::task::spawn_blocking(move || launch::watch_store(&g)).await;
                        if let Ok(Some(p)) = res {
                            restore_window();
                            app.show_problem(p, Some(gid));
                        }
                    });
                }
            }
            Err(p) => app.show_problem(p, Some(id)),
        },
        Source::Custom => {
            let Some(c) = app.settings.peek().custom(&id).cloned() else { return };
            match launch::precheck_custom(&c) {
                // Vor JEDEM Start eines eigenen Spiels: Sicherheitsfrage
                Ok(()) => app.open(Modal::Confirm(id)),
                Err(p) => app.show_problem(p, Some(id)),
            }
        }
    }
}

/// Nach "Ja, alles deaktiviert" – eigenes Spiel wirklich starten.
pub fn start_custom(app: App, id: String, force_admin: bool) {
    app.close_modal();
    let Some(c) = app.settings.peek().custom(&id).cloned() else { return };
    match launch::launch_custom(&c, force_admin) {
        Ok(child) => {
            mark_played(app, &id);
            app.toast(&tf("toast.started", &[("name", &c.name)]), "rocket");
            if app.settings.peek().minimize_on_launch {
                minimize_window();
            }
            if let Some(child) = child {
                let name = c.name.clone();
                let exe = c.exe.clone();
                dioxus::core::spawn_forever(async move {
                    let res = tokio::task::spawn_blocking(move || launch::watch(child, name, exe)).await;
                    if let Ok(Some(p)) = res {
                        restore_window();
                        app.show_problem(p, Some(id));
                    }
                });
            }
        }
        Err(p) => app.show_problem(p, Some(id)),
    }
}

/// Knopf in der Fehlererklärung wurde geklickt.
pub fn run_fix(app: App, fix: Fix, game_id: Option<String>) {
    match fix {
        Fix::RunAsAdmin => {
            if let Some(id) = game_id {
                let is_custom = app.game(&id).map(|g| g.source == Source::Custom).unwrap_or(false);
                if is_custom {
                    start_custom(app, id, true);
                } else {
                    app.close_modal();
                    play(app, id);
                }
            }
        }
        Fix::EditGame => {
            if let Some(id) = game_id {
                if app.settings.peek().custom(&id).is_some() {
                    app.open(Modal::EditGame(Some(id)));
                }
            }
        }
        Fix::OpenUrl { url, .. } => {
            let _ = sys::open_url(&url);
        }
        Fix::OpenFolder { path, .. } => sys::open_folder(&path),
        Fix::Rescan => {
            app.close_modal();
            rescan(app);
        }
        Fix::OpenSettings => {
            app.close_modal();
            app.go(Page::Settings(SettingsTab::Connections));
        }
        Fix::RemoveGame => {
            if let Some(id) = game_id {
                if app.settings.peek().custom(&id).is_some() {
                    app.open(Modal::DeleteGame(id));
                }
            }
        }
    }
}

/// "Spiel startet nicht?" – Hilfe mit allen typischen Lösungen
pub fn start_help(app: App, id: String) {
    let Some(game) = app.game(&id) else { return };
    let custom = game.source == Source::Custom;
    let dir = game
        .install_dir
        .clone()
        .or_else(|| app.settings.peek().custom(&id).and_then(|c| std::path::Path::new(c.exe.trim().trim_matches('"')).parent().map(|p| p.to_string_lossy().to_string())));
    let p = Problem::start_help(&game.name, &game.source.label(), launch::verify_fix(&game), dir, custom);
    app.show_problem(p, Some(id));
}

/// Spiel deinstallieren
pub fn uninstall(app: App, id: String) {
    let Some(game) = app.game(&id) else { return };
    match &game.launch {
        LaunchTarget::SteamRun(appid) => {
            // Steam fragt selbst noch einmal nach
            if sys::open_url(&format!("steam://uninstall/{appid}")).is_ok() {
                app.close_modal();
                app.toast(&tf("toast.uninstall_store", &[("name", &game.name), ("store", "Steam")]), "trash");
                rescan_later(app, &[20, 60, 180]);
            } else {
                app.show_problem(Problem::steam_missing(), Some(id));
            }
        }
        LaunchTarget::Epic(_) => {
            let _ = sys::open_url("com.epicgames.launcher://");
            app.show_problem(Problem::epic_uninstall(&game.name), Some(id));
            rescan_later(app, &[60, 180, 600]);
        }
        LaunchTarget::Custom => {
            let Some(c) = app.settings.peek().custom(&id).cloned() else { return };
            let dir = std::path::Path::new(c.exe.trim().trim_matches('"')).parent().map(|p| p.to_string_lossy().to_string());
            match launch::find_uninstaller(&c) {
                Some((prog, args)) => {
                    #[cfg(windows)]
                    let res = crate::winffi::shell_execute(Some("open"), &prog, Some(&args), None);
                    #[cfg(not(windows))]
                    let res: Result<(), i32> = { let _ = (&prog, &args); Err(0) };
                    match res {
                        Ok(()) => {
                            app.close_modal();
                            app.toast(&tf("toast.uninstall_custom", &[("name", &c.name)]), "trash");
                        }
                        Err(_) => app.show_problem(Problem::no_uninstaller(&c.name, dir), Some(id)),
                    }
                }
                None => app.show_problem(Problem::no_uninstaller(&c.name, dir), Some(id)),
            }
        }
        _ => {}
    }
}

pub fn toggle_favorite(app: App, id: &str) {
    let id = id.to_string();
    app.update(|s| {
        if let Some(pos) = s.favorites.iter().position(|f| *f == id) {
            s.favorites.remove(pos);
        } else {
            s.favorites.push(id);
        }
    });
}

pub fn toggle_hidden(app: App, id: &str) {
    let id = id.to_string();
    let mut hide = false;
    app.update(|s| {
        if let Some(pos) = s.hidden.iter().position(|f| *f == id) {
            s.hidden.remove(pos);
        } else {
            s.hidden.push(id);
            hide = true;
        }
    });
    app.toast(&t(if hide { "toast.hidden" } else { "toast.unhidden" }), "eye-off");
}

/// Eigenes Cover für irgendein Spiel wählen.
pub fn pick_cover(app: App, id: &str) {
    if let Some(path) = pick_image_file() {
        let id = id.to_string();
        app.update(|s| {
            s.cover_overrides.insert(id, path);
        });
        app.toast(&t("toast.cover_changed"), "image");
    }
}

pub fn reset_cover(app: App, id: &str) {
    let id = id.to_string();
    app.update(|s| {
        s.cover_overrides.remove(&id);
    });
}

// ----------------------------------------------------------------------
// Datei-Dialoge
// ----------------------------------------------------------------------

pub fn pick_game_file(start: Option<&str>) -> Option<String> {
    #[cfg(windows)]
    {
        let mut d = rfd::FileDialog::new()
            .set_title(t("pick.game_title"))
            .add_filter(t("pick.game_filter"), &["exe", "lnk", "bat", "cmd", "url"])
            .add_filter(t("pick.all_files"), &["*"]);
        if let Some(s) = start.filter(|s| !s.is_empty()) {
            if let Some(parent) = std::path::Path::new(s).parent() {
                d = d.set_directory(parent);
            }
        }
        d.pick_file().map(|p| p.to_string_lossy().to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = start;
        None
    }
}

pub fn pick_folder(title: &str) -> Option<String> {
    #[cfg(windows)]
    {
        rfd::FileDialog::new()
            .set_title(title)
            .pick_folder()
            .map(|p| p.to_string_lossy().to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = title;
        None
    }
}

pub fn pick_image_file() -> Option<String> {
    #[cfg(windows)]
    {
        rfd::FileDialog::new()
            .set_title(t("pick.cover_title"))
            .add_filter(t("pick.images"), &["jpg", "jpeg", "png", "webp", "gif", "bmp"])
            .pick_file()
            .map(|p| p.to_string_lossy().to_string())
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Bilder eines Spiels inkl. eigenem Cover
pub fn effective_cover(settings: &Settings, game: &Game) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(p) = settings.cover_overrides.get(&game.id) {
        v.push(sys::file_url(std::path::Path::new(p)));
    }
    v.extend(game.art.cover.iter().cloned());
    v
}
