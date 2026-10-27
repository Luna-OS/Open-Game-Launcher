//! NoRisk-Übersicht: alle Profile nach Gruppen, Spielen, Ordner, Inhalte und der volle NoRisk Launcher.
//! Liest nur die Daten des NoRisk Launchers und startet ihn über seine Befehle – legt selbst nichts an.

use crate::i18n::{t, tf};
use crate::minecraft::norisk::{self as nr, GroupKind, NoRisk, NoRiskProfile, Open};
use crate::minecraft::{self as mcl, Starter};
use crate::sys;
use crate::ui::art::ArtImage;
use crate::ui::components::*;
use crate::ui::icons::Icon;
use crate::ui::state::*;
use dioxus::prelude::*;

/// Wie oft geprüft wird, ob der NoRisk Launcher Profile geändert hat
const POLL_SECS: u64 = 3;

fn group_title(kind: GroupKind, label: &str) -> String {
    match kind {
        GroupKind::NoRiskClient => "NoRisk Client".into(),
        GroupKind::Own => t("nr.group_own"),
        GroupKind::Modpacks => t("nr.group_modpacks"),
        GroupKind::Server => t("nr.group_server"),
        GroupKind::Custom => pretty_label(label),
    }
}

/// "MODRINTH" → "Modrinth" (NoRisk speichert manche Gruppen in Großbuchstaben)
pub(super) fn pretty_label(l: &str) -> String {
    if l.chars().any(|c| c.is_lowercase()) {
        return l.to_string();
    }
    l.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_string() + &c.as_str().to_lowercase())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Farbe je Loader (für das Profil-Symbol)
fn loader_color(l: &str) -> &'static str {
    match l {
        "fabric" => "bg-[#c6a875] text-black",
        "forge" => "bg-[#3e5a8a] text-white",
        "neoforge" => "bg-[#d0632b] text-white",
        "quilt" => "bg-[#8b5cf6] text-white",
        _ => "bg-[#3c8527] text-white",
    }
}

/// Starter-Befehl ausführen (Problem → freundlicher Hinweis)
fn run(app: App, starter: &Starter, args: Vec<String>, name: &str) -> bool {
    let g = mcl::command_game(name, starter, args);
    match crate::launch::launch_store(&g) {
        Ok(()) => true,
        Err(p) => {
            app.show_problem(p, None);
            false
        }
    }
}

fn play(app: App, starter: &Starter, p: &NoRiskProfile) {
    let Some(args) = nr::launch_args(starter, &p.id) else {
        return;
    };
    if run(app, starter, args, &p.name) {
        app.toast(&tf("nr.starting", &[("name", &p.name)]), "rocket");
    }
}

fn open_launcher(app: App, starter: &Starter, what: Open) {
    if run(
        app,
        starter,
        nr::open_args(starter, &what),
        "NoRisk Launcher",
    ) {
        app.toast(&t("nr.opening"), "external");
    }
}

/// Kacheln in der Bibliothek an den neuen Stand anpassen (zuletzt gespieltes Profil, Spielzeit …)
fn refresh_tiles(app: App, data: NoRisk) {
    let per_profile = app.settings.peek().norisk_profile_tiles;
    spawn(async move {
        let fresh = tokio::task::spawn_blocking(move || {
            let mut v: Vec<crate::model::Game> = mcl::norisk_tile(&data).into_iter().collect();
            if let Some(s) = data.starter.as_ref().filter(|_| per_profile) {
                v.extend(data.profiles.iter().filter_map(|p| mcl::profile_game(s, p)));
            }
            v
        })
        .await
        .unwrap_or_default();
        if fresh.is_empty() {
            return;
        }
        let mut games = app.games;
        let mut w = games.write();
        w.retain(|g| !g.id.starts_with("norisk:"));
        w.extend(fresh);
    });
}

#[component]
pub fn NoRiskHub() -> Element {
    let app = use_app();
    let mut data = use_signal(|| None::<NoRisk>);
    let mut query = use_signal(String::new);

    // Laden + beobachten: ändert der NoRisk Launcher Profile, wird neu eingelesen
    use_future(move || async move {
        let mut last = 0u64;
        loop {
            let stamp = tokio::task::spawn_blocking(nr::stamp).await.unwrap_or(0);
            if stamp != last || data.peek().is_none() {
                let first = data.peek().is_none();
                let fresh = tokio::task::spawn_blocking(nr::load)
                    .await
                    .unwrap_or_default();
                if !first && data.peek().as_ref() != Some(&fresh) {
                    refresh_tiles(app, fresh.clone());
                }
                data.set(Some(fresh));
                last = stamp;
            }
            tokio::time::sleep(std::time::Duration::from_secs(POLL_SECS)).await;
        }
    });

    let hero = use_hook(|| nr::artwork().map(|(_, h, _)| sys::file_url(&h)));
    let official = app
        .games
        .read()
        .iter()
        .find(|g| g.id == "minecraft:launcher")
        .cloned();

    let Some(d) = data.read().clone() else {
        return rsx! {
            div { class: "h-full grid place-items-center", Spinner { class: "size-6" } }
        };
    };
    let starter = d.starter.clone();
    let has_data = d.paths.is_some();
    let profiles_dir = d
        .paths
        .as_ref()
        .map(|p| p.profiles.clone())
        .filter(|p| p.is_dir());
    let last = d.last_played().cloned();
    let q = query().trim().to_lowercase();
    let shown: Vec<NoRiskProfile> = d
        .profiles
        .iter()
        .filter(|p| {
            q.is_empty()
                || p.name.to_lowercase().contains(&q)
                || p.game_version.contains(&q)
                || p.group.to_lowercase().contains(&q)
                || p.loader.contains(&q)
        })
        .cloned()
        .collect();
    let groups = nr::groups(&shown);
    let count = d.profiles.len();
    let hero_srcs: Vec<String> = hero.clone().into_iter().collect();

    rsx! {
        div { class: "h-full overflow-y-auto maz-ambient",
            div { class: "max-w-[1600px] mx-auto px-8 py-5",
                // Kopf
                div { class: "relative rounded-2xl overflow-hidden ring-1 ring-white/10 shadow-xl mb-6 min-h-[200px]",
                    ArtImage { srcs: hero_srcs, class: "absolute inset-0 size-full object-cover",
                        fallback: rsx! { div { class: "absolute inset-0 bg-linear-to-br from-rose-600 via-purple-800 to-slate-950" } } }
                    div { class: "absolute inset-0 bg-linear-to-r from-black/80 via-black/50 to-transparent" }
                    div { class: "relative p-7 flex flex-col gap-4 text-white",
                        div {
                            h1 { class: "text-3xl font-extrabold tracking-tight font-display", "NoRisk Client" }
                            p { class: "text-sm text-white/75 mt-1",
                                if starter.is_some() {
                                    {tf("nr.summary", &[("n", &count.to_string())])}
                                    if let Some(l) = last.as_ref() {
                                        " · "
                                        {tf("nr.last", &[("name", &l.name), ("ago", &sys::format_ago(l.last_played))])}
                                    }
                                } else {
                                    {t("nr.not_installed_sub")}
                                }
                            }
                        }
                        div { class: "flex flex-wrap gap-2",
                            if let Some(s) = starter.clone() {
                                if let Some(l) = last.clone() {
                                    {
                                        let s2 = s.clone();
                                        let label = tf("nr.continue", &[("name", &l.name)]);
                                        rsx! {
                                            Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, onclick: move |_| play(app, &s2, &l),
                                                Icon { name: "play", class: "size-5", filled: true }
                                                "{label}"
                                            }
                                        }
                                    }
                                }
                                {
                                    let (s1, s2) = (s.clone(), s.clone());
                                    rsx! {
                                        Button { variant: ButtonVariant::Secondary, size: ButtonSize::Lg, class: "bg-white/10 text-white hover:bg-white/20 backdrop-blur",
                                            onclick: move |_| open_launcher(app, &s1, Open::Home),
                                            Icon { name: "external", class: "size-4" }
                                            {t("nr.open_launcher")}
                                        }
                                        Button { variant: ButtonVariant::Secondary, size: ButtonSize::Lg, class: "bg-white/10 text-white hover:bg-white/20 backdrop-blur",
                                            onclick: move |_| open_launcher(app, &s2, Open::Create),
                                            Icon { name: "plus", class: "size-4" }
                                            {t("nr.new_profile")}
                                        }
                                    }
                                }
                                if let Some(dir) = profiles_dir.clone() {
                                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Lg, class: "text-white/80 hover:text-white hover:bg-white/10",
                                        onclick: move |_| sys::open_folder(&dir.to_string_lossy()),
                                        Icon { name: "folder", class: "size-4" }
                                        {t("nr.profiles_folder")}
                                    }
                                }
                            } else {
                                Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, onclick: move |_| { let _ = sys::open_url(nr::DOWNLOAD); },
                                    Icon { name: "download", class: "size-5" }
                                    {t("nr.install")}
                                }
                            }
                        }
                    }
                }

                // Nicht installiert
                if starter.is_none() {
                    div { class: "rounded-xl border border-border bg-card/60 p-5 text-sm mb-6 flex gap-3",
                        Icon { name: "info", class: "size-5 text-primary shrink-0" }
                        div {
                            div { class: "font-semibold mb-1", {t("nr.not_installed_title")} }
                            p { class: "text-muted-foreground leading-relaxed",
                                {t(if has_data { "nr.not_installed_data" } else { "nr.not_installed_text" })}
                            }
                            Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm, class: "mt-3", onclick: move |_| { let mut d = data; d.set(None); },
                                Icon { name: "refresh", class: "size-3.5" }
                                {t("nr.check_again")}
                            }
                        }
                    }
                }

                // Profile
                if count > 0 {
                    div { class: "flex flex-wrap items-center gap-3 mb-4",
                        h2 { class: "text-lg font-semibold mr-auto", {t("nr.profiles")} }
                        div { class: "relative w-72",
                            span { class: "absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none",
                                Icon { name: "search", class: "size-4" }
                            }
                            Input { class: "pl-9 h-9", placeholder: t("nr.search_ph"), value: query(), oninput: move |v: String| query.set(v) }
                        }
                    }
                    if groups.is_empty() {
                        div { class: "py-10 text-center text-sm text-muted-foreground", {t("nr.nothing")} }
                    }
                    for g in groups {
                        GroupSection { key: "{g.kind:?}:{g.label}", group: g, all: d.profiles.clone(), starter: starter.clone() }
                    }
                } else if starter.is_some() {
                    div { class: "rounded-xl border border-border bg-card/60 p-6 text-sm text-center mb-6",
                        div { class: "font-semibold mb-1", {t("nr.empty_title")} }
                        p { class: "text-muted-foreground", {t("nr.empty_text")} }
                    }
                }

                // Offizieller Minecraft Launcher (bleibt eine eigene Kachel)
                if let Some(g) = official {
                    {
                        let id = g.id.clone();
                        rsx! {
                            div { class: "mt-4 flex flex-wrap items-center gap-3 rounded-xl border border-border bg-card/60 px-4 py-3",
                                div { class: "size-10 rounded-xl bg-[#3c8527] text-white grid place-items-center", Icon { name: "cube", class: "size-5" } }
                                div { class: "mr-auto",
                                    div { class: "font-semibold text-sm", "Minecraft Launcher" }
                                    div { class: "text-xs text-muted-foreground", {t("nr.official_hint")} }
                                }
                                Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm, onclick: move |_| super::open_content(app, "official".into()),
                                    Icon { name: "package", class: "size-3.5" } {t("mc.open_content")}
                                }
                                Button { variant: ButtonVariant::Glow, size: ButtonSize::Sm, onclick: move |_| crate::ui::state::play(app, id.clone()),
                                    Icon { name: "play", class: "size-3.5", filled: true } {t("game.play")}
                                }
                            }
                        }
                    }
                }
                p { class: "mt-8 text-xs text-muted-foreground leading-relaxed", {t("nr.legal")} }
            }
        }
    }
}

/// Eine Gruppe (einklappbar)
#[component]
fn GroupSection(group: nr::Group, all: Vec<NoRiskProfile>, starter: Option<Starter>) -> Element {
    let mut open = use_signal(|| true);
    let title = group_title(group.kind, &group.label);
    let n = group.profiles.len();
    let chevron = if open() {
        "chevron-down"
    } else {
        "chevron-right"
    };
    rsx! {
        section { class: "mb-6",
            button { class: "flex items-center gap-2 mb-3 text-sm font-semibold text-muted-foreground hover:text-foreground cursor-pointer",
                onclick: move |_| open.toggle(),
                Icon { name: chevron, class: "size-4" }
                "{title}"
                span { class: "rounded-full bg-secondary px-2 py-0.5 text-[11px]", "{n}" }
            }
            if open() {
                div { class: "grid gap-3", style: "grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));",
                    for p in group.profiles {
                        ProfileCard { key: "{p.id}", shared_with: nr::sharing(&all, &p.dir).saturating_sub(1), profile: p, starter: starter.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn ProfileCard(profile: NoRiskProfile, starter: Option<Starter>, shared_with: usize) -> Element {
    let app = use_app();
    let p = profile;
    let initial: String = p
        .name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_else(|| "?".into());
    let color = loader_color(&p.loader);
    let detail = p.detail();
    let played = if p.last_played > 0 {
        sys::format_ago(p.last_played)
    } else {
        t("fmt.never")
    };
    let playtime = (p.playtime_secs >= 60).then(|| sys::format_playtime(p.playtime_secs / 60));
    let dir = p.dir.clone();
    let has_dir = dir.is_dir();
    let can_content = !dir.as_os_str().is_empty();
    let target = format!("norisk:{}", p.id);
    let (p1, p2) = (p.clone(), p.id.clone());
    let (s1, s2) = (starter.clone(), starter.clone());
    rsx! {
        div { class: "group flex flex-col gap-3 rounded-xl border border-border bg-card/60 p-4 hover:border-primary/50 transition-colors",
            div { class: "flex items-start gap-3",
                div { class: "size-11 shrink-0 rounded-lg grid place-items-center text-lg font-bold shadow {color}", "{initial}" }
                div { class: "min-w-0 flex-1",
                    div { class: "font-semibold truncate", title: "{p.name}", "{p.name}" }
                    div { class: "text-xs text-muted-foreground truncate", "{detail}" }
                    div { class: "text-xs text-muted-foreground mt-0.5 flex flex-wrap gap-x-2",
                        span { title: t("nr.last_played"), Icon { name: "clock", class: "size-3 inline -mt-0.5 mr-1" } "{played}" }
                        if let Some(pt) = playtime { span { Icon { name: "timer", class: "size-3 inline -mt-0.5 mr-1" } "{pt}" } }
                    }
                }
                if p.standard {
                    span { class: "rounded-md bg-primary/15 text-primary px-1.5 py-0.5 text-[10px] font-semibold", title: t("nr.standard_tip"), "NRC" }
                }
            }
            if shared_with > 0 {
                div { class: "text-[11px] text-muted-foreground flex items-center gap-1",
                    Icon { name: "layers", class: "size-3" }
                    {tf("nr.shared", &[("n", &shared_with.to_string())])}
                }
            }
            div { class: "flex flex-wrap gap-2 mt-auto",
                if let Some(s) = s1 {
                    Button { variant: ButtonVariant::Glow, size: ButtonSize::Sm, onclick: move |_| play(app, &s, &p1),
                        Icon { name: "play", class: "size-3.5", filled: true } {t("game.play")}
                    }
                }
                if can_content {
                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm, title: t("mc.open_content"), onclick: move |_| super::open_content(app, target.clone()),
                        Icon { name: "package", class: "size-3.5" } {t("nr.content")}
                    }
                }
                if has_dir {
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, title: t("mc.open_folder"), onclick: move |_| sys::open_folder(&dir.to_string_lossy()),
                        Icon { name: "folder", class: "size-4" }
                    }
                }
                if let Some(s) = s2 {
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, title: t("nr.open_in_launcher"), onclick: move |_| open_launcher(app, &s, Open::Profile(p2.clone())),
                        Icon { name: "external", class: "size-4" }
                    }
                }
            }
        }
    }
}
