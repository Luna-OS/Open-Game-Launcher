//! Spiel-Details: als Fenster (MAZ/Epic) oder direkt in der Seite (Steam-Stil).

use super::art::{gradient_for, ArtImage, CoverFallback, GameIcon};
use super::components::*;
use super::icons::Icon;
use super::library::SourceBadge;
use super::state::*;
use crate::model::{Game, LaunchTarget, Source};
use crate::sys;
use dioxus::prelude::*;

struct InfoRow {
    label: &'static str,
    value: String,
}

fn info_rows(app: App, g: &Game) -> Vec<InfoRow> {
    let s = app.settings.read();
    let mut rows = vec![InfoRow { label: "Plattform", value: g.source.label().to_string() }];
    rows.push(InfoRow {
        label: "Status",
        value: if g.installed { "Installiert".into() } else { "Nicht installiert".into() },
    });
    if let Some(min) = g.playtime_min.filter(|m| *m > 0) {
        rows.push(InfoRow { label: "Spielzeit", value: sys::format_playtime(min) });
    }
    rows.push(InfoRow { label: "Zuletzt gespielt", value: sys::format_ago(s.last_played(&g.id)) });
    if let Some(n) = s.launch_count.get(&g.id) {
        rows.push(InfoRow { label: "Über MAZ gestartet", value: format!("{n}×") });
    }
    if let Some(b) = g.size_bytes {
        rows.push(InfoRow { label: "Größe", value: sys::format_bytes(b) });
    }
    if let Some(d) = &g.install_dir {
        rows.push(InfoRow { label: "Ordner", value: d.clone() });
    }
    if let Some(c) = s.custom(&g.id) {
        if !c.args.trim().is_empty() {
            rows.push(InfoRow { label: "Startparameter", value: c.args.clone() });
        }
        rows.push(InfoRow {
            label: "Als Administrator",
            value: if c.admin { "Ja".into() } else { "Nein".into() },
        });
    }
    rows
}

#[component]
fn ActionList(game: Game, #[props(default = false)] steam_style: bool) -> Element {
    let app = use_app();
    let s = app.settings.read();
    let fav = s.is_favorite(&game.id);
    let hidden = s.is_hidden(&game.id);
    let has_override = s.cover_overrides.contains_key(&game.id);
    let is_custom = s.custom(&game.id).is_some();
    drop(s);
    let btn = if steam_style { "w-full justify-start rounded-sm" } else { "w-full justify-start" };
    let (id1, id2, id3, id4, id5, id6) = (
        game.id.clone(),
        game.id.clone(),
        game.id.clone(),
        game.id.clone(),
        game.id.clone(),
        game.id.clone(),
    );
    let dir = game.install_dir.clone();
    let appid = game.steam_appid;
    let fav_label = if fav { "Aus Favoriten entfernen" } else { "Zu Favoriten" };
    let star_cls = if fav { "size-4 text-yellow-400" } else { "size-4" };
    let hide_icon = if hidden { "eye" } else { "eye-off" };
    let hide_label = if hidden { "Wieder einblenden" } else { "Ausblenden" };
    rsx! {
        div { class: "flex flex-col gap-2",
            Button { variant: ButtonVariant::Outline, class: btn, onclick: move |_| toggle_favorite(app, &id1),
                Icon { name: "star", class: star_cls, filled: fav }
                "{fav_label}"
            }
            {dir.map(|d| rsx! {
                Button { variant: ButtonVariant::Outline, class: btn, onclick: move |_| sys::open_folder(&d),
                    Icon { name: "folder", class: "size-4" }
                    "Spielordner öffnen"
                }
            })}
            Button { variant: ButtonVariant::Outline, class: btn, onclick: move |_| pick_cover(app, &id2),
                Icon { name: "image", class: "size-4" }
                "Eigenes Cover wählen"
            }
            if has_override {
                Button { variant: ButtonVariant::Ghost, class: btn, onclick: move |_| reset_cover(app, &id3),
                    Icon { name: "refresh", class: "size-4" }
                    "Original-Cover wiederherstellen"
                }
            }
            {appid.map(|a| rsx! {
                Button { variant: ButtonVariant::Outline, class: btn, onclick: move |_| { let _ = sys::open_url(&format!("steam://store/{a}")); },
                    Icon { name: "external", class: "size-4" }
                    "Im Steam-Store ansehen"
                }
            })}
            if is_custom {
                Button { variant: ButtonVariant::Outline, class: btn, onclick: move |_| app.open(Modal::EditGame(Some(id4.clone()))),
                    Icon { name: "pencil", class: "size-4" }
                    "Spiel bearbeiten"
                }
            }
            Button { variant: ButtonVariant::Ghost, class: btn, onclick: move |_| toggle_hidden(app, &id5),
                Icon { name: hide_icon, class: "size-4" }
                "{hide_label}"
            }
            if is_custom {
                Button { variant: ButtonVariant::Ghost, class: "{btn} text-destructive hover:text-destructive", onclick: move |_| app.open(Modal::DeleteGame(id6.clone())),
                    Icon { name: "trash", class: "size-4" }
                    "Aus der Liste entfernen"
                }
            }
        }
    }
}

/// Details als Fenster (MAZ- und Epic-Ansicht)
#[component]
pub fn DetailModal(id: String) -> Element {
    let app = use_app();
    let Some(game) = app.games.read().iter().find(|g| g.id == id).cloned() else {
        return rsx! {};
    };
    let s = app.settings.read();
    let covers = effective_cover(&s, &game);
    drop(s);
    let mut hero = game.art.hero.clone();
    hero.extend(game.art.header.iter().cloned());
    let logos = game.art.logo.clone();
    let rows = info_rows(app, &game);
    let name = game.name.clone();
    let grad = gradient_for(&game.name);
    let installed = game.installed;
    let play_label = if installed { "Spielen" } else { "Installieren" };
    let play_icon = if installed { "play" } else { "download" };
    let id_play = game.id.clone();
    let g2 = game.clone();
    let g3 = game.clone();
    let source = game.source;
    let note = match (&game.launch, game.source) {
        (LaunchTarget::SteamInstall(_), _) => Some("Dieses Spiel gehört dir, ist aber nicht installiert. „Installieren“ öffnet Steam."),
        (_, Source::Custom) => Some("Vor dem Start wirst du gefragt, ob du alles Wichtige deaktiviert hast."),
        _ => None,
    };
    rsx! {
        Dialog { on_close: move |_| app.close_modal(), class: "max-w-4xl",
            div { class: "relative h-72 overflow-hidden rounded-t-2xl",
                ArtImage {
                    srcs: hero,
                    class: "absolute inset-0 size-full object-cover",
                    fallback: rsx! { div { class: "absolute inset-0 {grad}" } },
                }
                div { class: "absolute inset-0 hero-fade" }
                div { class: "absolute inset-x-0 bottom-0 p-8 flex items-end gap-6",
                    div { class: "relative w-32 aspect-[2/3] rounded-lg overflow-hidden shadow-2xl ring-1 ring-white/10 shrink-0",
                        ArtImage {
                            srcs: covers,
                            class: "absolute inset-0 size-full object-cover",
                            fallback: rsx! { CoverFallback { game: g2.clone(), show_name: false } },
                        }
                    }
                    div { class: "min-w-0 pb-1",
                        div { class: "flex items-center gap-2 mb-3",
                            SourceBadge { source }
                            if !installed {
                                Badge { variant: BadgeVariant::Warning, "NICHT INSTALLIERT" }
                            }
                        }
                        div { class: "h-20 flex items-end",
                            ArtImage {
                                srcs: logos,
                                class: "max-h-20 max-w-[380px] object-contain object-left drop-shadow-2xl",
                                fallback: rsx! { h2 { class: "text-3xl font-black leading-tight font-display drop-shadow-lg line-clamp-2", "{name}" } },
                            }
                        }
                    }
                }
            }
            div { class: "p-8 pt-6 grid grid-cols-[1fr_260px] gap-8",
                div { class: "min-w-0",
                    div { class: "flex items-center gap-3",
                        Button { variant: ButtonVariant::Glow, size: ButtonSize::Xl, class: "min-w-52", onclick: move |_| { app.close_modal(); play(app, id_play.clone()); },
                            Icon { name: play_icon, class: "size-5", filled: true }
                            "{play_label}"
                        }
                        GameIcon { game: g3.clone(), class: "size-12 rounded-xl ring-1 ring-border" }
                    }
                    {note.map(|n| rsx! {
                        p { class: "mt-4 text-sm text-muted-foreground flex items-start gap-2",
                            Icon { name: "info", class: "size-4 mt-0.5 shrink-0" }
                            "{n}"
                        }
                    })}
                    div { class: "mt-6 rounded-xl border border-border divide-y divide-border overflow-hidden",
                        {rows.into_iter().map(|r| {
                            let label = r.label;
                            let value = r.value;
                            rsx! {
                                div { key: "{label}", class: "flex items-center gap-4 px-4 py-2.5 text-sm",
                                    span { class: "w-40 shrink-0 text-muted-foreground", "{label}" }
                                    span { class: "min-w-0 truncate select-text", title: "{value}", "{value}" }
                                }
                            }
                        })}
                    }
                }
                ActionList { game }
            }
        }
    }
}

/// Details direkt in der Seite – im Look der Steam-Bibliothek
#[component]
pub fn SteamDetail(game: Game) -> Element {
    let app = use_app();
    let s = app.settings.read();
    let mut hero = game.art.hero.clone();
    hero.extend(game.art.header.iter().cloned());
    hero.extend(effective_cover(&s, &game));
    let last = sys::format_ago(s.last_played(&game.id));
    drop(s);
    let logos = game.art.logo.clone();
    let name = game.name.clone();
    let grad = gradient_for(&game.name);
    let installed = game.installed;
    let id = game.id.clone();
    let play_cls = if installed {
        "steam-play-btn is-green h-12 px-8 rounded-sm text-white font-semibold tracking-wide text-[15px] flex items-center gap-3 shadow-lg hover:brightness-110 cursor-pointer"
    } else {
        "steam-play-btn h-12 px-8 rounded-sm text-white font-semibold tracking-wide text-[15px] flex items-center gap-3 shadow-lg hover:brightness-110 cursor-pointer"
    };
    let label = if installed { "SPIELEN" } else { "INSTALLIEREN" };
    let icon = if installed { "play" } else { "download" };
    let playtime = game.playtime_min.filter(|m| *m > 0).map(sys::format_playtime);
    let rows = info_rows(app, &game);
    let source = game.source;
    rsx! {
        div { class: "animate-fade-in",
            div { class: "relative h-[380px] overflow-hidden",
                ArtImage {
                    srcs: hero,
                    class: "absolute inset-0 size-full object-cover",
                    fallback: rsx! { div { class: "absolute inset-0 {grad}" } },
                }
                div { class: "absolute inset-0 steam-hero-fade" }
                div { class: "absolute left-10 bottom-10 max-w-[480px] h-36 flex items-end",
                    ArtImage {
                        srcs: logos,
                        class: "max-h-36 max-w-full object-contain object-left drop-shadow-[0_4px_12px_rgba(0,0,0,0.8)]",
                        fallback: rsx! { h1 { class: "text-5xl font-black text-white drop-shadow-lg font-display", "{name}" } },
                    }
                }
            }
            div { class: "flex items-center gap-8 px-10 py-4 bg-black/25 border-y border-black/30",
                button { class: "{play_cls}", onclick: move |_| play(app, id.clone()),
                    Icon { name: icon, class: "size-5", filled: true }
                    "{label}"
                }
                div { class: "flex flex-col",
                    span { class: "text-[11px] uppercase tracking-wider text-muted-foreground", "Zuletzt gespielt" }
                    span { class: "text-sm text-foreground", "{last}" }
                }
                {playtime.map(|pt| rsx! {
                    div { class: "flex flex-col",
                        span { class: "text-[11px] uppercase tracking-wider text-muted-foreground", "Spielzeit" }
                        span { class: "text-sm text-foreground", "{pt}" }
                    }
                })}
                div { class: "flex-1" }
                SourceBadge { source }
            }
            div { class: "px-10 py-6 grid grid-cols-[1fr_260px] gap-8",
                div { class: "rounded-sm bg-black/20 p-5",
                    div { class: "text-[13px] uppercase tracking-wider text-muted-foreground mb-3", "Informationen" }
                    {rows.into_iter().map(|r| {
                        let label = r.label;
                        let value = r.value;
                        rsx! {
                            div { key: "{label}", class: "flex gap-4 py-1.5 text-[13px] border-b border-white/5 last:border-0",
                                span { class: "w-40 shrink-0 text-muted-foreground", "{label}" }
                                span { class: "min-w-0 truncate select-text", title: "{value}", "{value}" }
                            }
                        }
                    })}
                }
                ActionList { game, steam_style: true }
            }
        }
    }
}
