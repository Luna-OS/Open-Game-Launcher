//! Seite "Gratis-Spiele": was es gerade bei Epic und Steam kostenlos gibt.

use super::art::{gradient_for, ArtImage};
use super::components::*;
use super::icons::Icon;
use super::net;
use super::state::*;
use crate::freegames::{FreeGame, FreeStore};
use crate::i18n::t;
use crate::sys;
use dioxus::prelude::*;

#[component]
pub fn FreePage() -> Element {
    let app = use_app();
    use_effect(move || net::load_free(app, false));
    let state = *app.free_state.read();
    let all = app.free.read().clone();
    let now: Vec<FreeGame> = all.iter().filter(|g| !g.upcoming).cloned().collect();
    let soon: Vec<FreeGame> = all.iter().filter(|g| g.upcoming).cloned().collect();
    let loading = state == LoadState::Loading;
    rsx! {
        div { class: "h-full overflow-y-auto maz-ambient",
            div { class: "max-w-[1600px] mx-auto px-8 py-7",
                div { class: "flex items-center gap-4 mb-2",
                    div { class: "size-12 rounded-2xl maz-gradient-bg text-glow-fg grid place-items-center shadow-lg",
                        Icon { name: "gift", class: "size-6" }
                    }
                    div { class: "mr-auto",
                        h1 { class: "text-2xl font-bold tracking-tight font-display", {t("free.title")} }
                        p { class: "text-sm text-muted-foreground", {t("free.sub")} }
                    }
                    Button { variant: ButtonVariant::Outline, disabled: loading, onclick: move |_| net::load_free(app, true),
                        if loading { Spinner {} } else { Icon { name: "refresh", class: "size-4" } }
                        {t("lib.reload")}
                    }
                }
                div { class: "mt-4 mb-8 flex items-start gap-3 rounded-xl border border-border bg-card/60 px-4 py-3 text-sm text-muted-foreground",
                    Icon { name: "info", class: "size-4 mt-0.5 shrink-0 text-primary" }
                    span { {t("free.how")} }
                }
                {match state {
                    LoadState::Loading | LoadState::Idle if all.is_empty() => rsx! {
                        div { class: "grid gap-6", style: "grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));",
                            for i in 0..6 { div { key: "{i}", class: "h-[380px] rounded-2xl skeleton" } }
                        }
                    },
                    LoadState::Failed => rsx! {
                        div { class: "flex flex-col items-center text-center py-20",
                            div { class: "size-16 rounded-2xl bg-muted grid place-items-center text-muted-foreground mb-4",
                                Icon { name: "alert", class: "size-8" }
                            }
                            h2 { class: "text-lg font-bold", {t("free.error_title")} }
                            p { class: "text-sm text-muted-foreground mt-2 max-w-md", {t("free.error_text")} }
                            Button { variant: ButtonVariant::Glow, class: "mt-5", onclick: move |_| net::load_free(app, true), {t("inst.retry")} }
                        }
                    },
                    _ if all.is_empty() => rsx! {
                        div { class: "text-center py-20 text-muted-foreground", {t("free.none")} }
                    },
                    _ => rsx! {
                        if !now.is_empty() {
                            SectionTitle { title: t("free.now_title"), count: now.len() }
                            div { class: "grid gap-6 mb-12", style: "grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));",
                                {now.into_iter().map(|g| { let k = g.id.clone(); rsx! { FreeCard { key: "{k}", game: g } } })}
                            }
                        }
                        if !soon.is_empty() {
                            SectionTitle { title: t("free.soon_title"), count: soon.len() }
                            div { class: "grid gap-6", style: "grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));",
                                {soon.into_iter().map(|g| { let k = g.id.clone(); rsx! { FreeCard { key: "{k}", game: g } } })}
                            }
                        }
                    },
                }}
                p { class: "text-xs text-muted-foreground mt-12", {t("free.sources")} }
            }
        }
    }
}

#[component]
fn SectionTitle(#[props(into)] title: String, count: usize) -> Element {
    rsx! {
        div { class: "flex items-center gap-3 mb-4",
            h2 { class: "text-lg font-semibold", "{title}" }
            Badge { variant: BadgeVariant::Secondary, "{count}" }
            div { class: "flex-1 h-px bg-border" }
        }
    }
}

#[component]
fn FreeCard(game: FreeGame) -> Element {
    let app = use_app();
    let owned = net::owned_game(app, &game);
    let mut images = game.images.clone();
    images.extend(game.wide.iter().cloned());
    let title = game.title.clone();
    let worth = game.worth.clone();
    let left = net::remaining(&game);
    let grad = gradient_for(&game.title);
    let (store_badge, store_name) = match game.store {
        FreeStore::Epic => (BadgeVariant::Epic, "EPIC GAMES"),
        FreeStore::Steam => (BadgeVariant::Steam, "STEAM"),
    };
    let upcoming = game.upcoming;
    let g_claim = game.clone();
    let url = game.url.clone();
    let urgent = !upcoming && game.ends.map(|e| e.saturating_sub(sys::now_secs()) < 86_400).unwrap_or(false);
    let timer_cls = if upcoming {
        "bg-black/70 text-white"
    } else if urgent {
        "bg-destructive text-white"
    } else {
        "bg-success text-success-foreground"
    };
    rsx! {
        div { class: "group flex flex-col rounded-2xl overflow-hidden bg-card border border-border shadow-lg shadow-black/20 animate-pop-in",
            div { class: "relative h-[300px] overflow-hidden bg-muted",
                ArtImage {
                    srcs: images,
                    class: "absolute inset-0 size-full object-cover transition-transform duration-500 group-hover:scale-105",
                    alt: title.clone(),
                    fallback: rsx! {
                        div { class: "absolute inset-0 {grad} grid place-items-center p-6 text-center",
                            span { class: "text-white font-black text-xl drop-shadow", "{title}" }
                        }
                    },
                }
                div { class: "absolute inset-x-0 bottom-0 h-1/2 bg-linear-to-t from-black/80 to-transparent" }
                div { class: "absolute top-3 left-3", Badge { variant: store_badge, "{store_name}" } }
                div { class: "absolute top-3 right-3 inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-semibold shadow {timer_cls}",
                    Icon { name: "timer", class: "size-3.5" }
                    "{left}"
                }
                if owned.is_some() {
                    div { class: "absolute bottom-3 left-3 inline-flex items-center gap-1.5 rounded-full bg-black/70 px-2.5 py-1 text-xs font-semibold text-white",
                        Icon { name: "check-circle", class: "size-3.5 text-success" }
                        {t("free.owned")}
                    }
                }
            }
            div { class: "p-4 flex flex-col gap-3 flex-1",
                div { class: "font-semibold leading-snug line-clamp-2", "{title}" }
                div { class: "flex items-center gap-2 text-sm",
                    if !worth.is_empty() {
                        span { class: "line-through text-muted-foreground", "{worth}" }
                    }
                    span { class: "font-bold text-success", {t(if upcoming { "free.soon_free" } else { "free.free" })} }
                }
                div { class: "flex-1" }
                {match (owned, upcoming) {
                    (Some(g), _) => {
                        let id = g.id.clone();
                        let label = t(if g.installed { "game.play" } else { "game.install" });
                        let icon = if g.installed { "play" } else { "download" };
                        rsx! {
                            Button { variant: ButtonVariant::Secondary, class: "w-full", onclick: move |_| play(app, id.clone()),
                                Icon { name: icon, class: "size-4", filled: true }
                                "{label}"
                            }
                        }
                    }
                    (None, false) => rsx! {
                        Button { variant: ButtonVariant::Glow, class: "w-full", onclick: move |_| net::claim_free(app, &g_claim),
                            Icon { name: "plus", class: "size-4" }
                            {t("free.claim")}
                        }
                    },
                    (None, true) => rsx! {
                        Button { variant: ButtonVariant::Outline, class: "w-full", onclick: move |_| { let _ = sys::open_url(&url); },
                            Icon { name: "external", class: "size-4" }
                            {t("free.view")}
                        }
                    },
                }}
            }
        }
    }
}

/// Anzahl der aktuell gratis Spiele, die man noch nicht hat (für die Seitenleiste)
pub fn unclaimed_count(app: App) -> usize {
    let free = app.free.read();
    free.iter().filter(|g| !g.upcoming && net::owned_game(app, g).is_none()).count()
}

