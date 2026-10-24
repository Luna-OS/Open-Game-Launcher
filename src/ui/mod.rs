//! Oberfläche (Dioxus + Komponenten im rust-ui-Stil).

pub mod art;
pub mod components;
pub mod detail;
pub mod dialogs;
pub mod icons;
pub mod installer;
pub mod library;
pub mod settings;
pub mod state;

use crate::install::StartMode;
use crate::model::{layout, StoreStatus};
use crate::i18n::t;
use crate::{config, sys, theme};
use dioxus::desktop::wry::http::Response;
use dioxus::desktop::{use_asset_handler, window, LogicalSize};
use dioxus::prelude::*;
use icons::Icon;
use state::*;

/// Wurzel-Komponente: Theme, Titelleiste, Installer oder Launcher, Fenster, Toasts.
#[component]
pub fn Root() -> Element {
    let start = crate::START.get().cloned().unwrap_or(StartMode::Launcher);
    let mut mode = use_signal(|| start);

    let app = use_context_provider(|| App {
        settings: Signal::new(config::load()),
        games: Signal::new(Vec::new()),
        steam: Signal::new(StoreStatus::default()),
        epic: Signal::new(StoreStatus::default()),
        warnings: Signal::new(Vec::new()),
        scanning: Signal::new(false),
        loaded: Signal::new(false),
        page: Signal::new(Page::Library),
        filter: Signal::new(Filter::All),
        search: Signal::new(String::new()),
        selected: Signal::new(None),
        modal: Signal::new(None),
        toasts: Signal::new(Vec::new()),
    });

    // Lokale Bilder (Steam-Cache, Icons, eigene Cover) für das Fenster bereitstellen
    use_asset_handler("maz", move |req, responder| {
        let path = req.uri().path().to_string();
        std::thread::spawn(move || {
            let found = sys::decode_file_url(&path).and_then(|p| std::fs::read(&p).ok().map(|b| (p, b)));
            let resp = match found {
                Some((p, bytes)) => Response::builder()
                    .status(200)
                    .header("Content-Type", sys::mime_for(&p))
                    .header("Cache-Control", "max-age=604800")
                    .body(bytes),
                None => Response::builder().status(404).body(Vec::new()),
            };
            responder.respond(resp.unwrap_or_else(|_| Response::new(Vec::new())));
        });
    });

    let s = app.settings.read();
    let is_launcher = matches!(*mode.read(), StartMode::Launcher);
    let authentic = if is_launcher && s.authentic_colors && *app.page.read() == Page::Library {
        if s.layout == layout::STEAM {
            "authentic-steam"
        } else if s.layout == layout::EPIC {
            "authentic-epic"
        } else {
            ""
        }
    } else {
        ""
    };
    // Farben: eigenes Design oder fertiges Design (bei Original-Farben übernimmt das CSS)
    let custom = s.custom_themes.iter().find(|c| c.id == s.theme).cloned();
    let data_theme = if custom.is_some() { "custom".to_string() } else { s.theme.clone() };
    let mut style = String::new();
    if authentic.is_empty() {
        if let Some(c) = &custom {
            style.push_str(&c.css_vars());
        } else if let Some(p) = theme::preset(&s.theme) {
            style.push_str(&p.css_vars());
        }
        if !s.accent.is_empty() && custom.is_none() {
            let a = &s.accent;
            style.push_str(&format!("--primary:{a};--ring:{a};--glow:{a};--primary-foreground:{};", theme::contrast_text(a)));
        }
    }
    let anim = if s.animations { "" } else { "no-anim" };
    let wallpaper = if s.wallpaper.is_empty() || !is_launcher {
        None
    } else {
        let url = sys::file_url(std::path::Path::new(&s.wallpaper));
        let opacity = (100 - s.wallpaper_dim.min(90) as u32) as f32 / 100.0;
        let blur = if s.wallpaper_blur { "filter: blur(14px); transform: scale(1.06);" } else { "" };
        Some((url, format!("opacity: {opacity}; {blur}")))
    };
    let lang = if s.language.is_empty() { "auto".to_string() } else { s.language.clone() };
    drop(s);

    rsx! {
        div {
            "data-theme": "{data_theme}",
            class: "relative h-screen w-screen flex flex-col bg-background text-foreground font-sans antialiased overflow-hidden {authentic} {anim}",
            style: "{style}",
            {wallpaper.map(|(url, st)| rsx! {
                div { class: "absolute inset-0 pointer-events-none overflow-hidden z-0",
                    img { src: "{url}", class: "size-full object-cover", style: "{st}", "draggable": "false" }
                }
            })}
            // Bei Sprachwechsel alles neu aufbauen (Schlüssel = Sprache)
            {std::iter::once(rsx! {
                div { key: "{lang}", class: "relative z-[1] flex-1 min-h-0 flex flex-col",
                    TitleBar {}
                    div { class: "flex-1 min-h-0 relative",
                        {match mode() {
                            StartMode::Launcher => rsx! { LauncherShell {} },
                            StartMode::Installer { existing } => rsx! {
                                installer::InstallerView {
                                    existing,
                                    on_portable: move |_| {
                                        let w = window();
                                        w.window.set_inner_size(LogicalSize::new(1320.0, 840.0));
                                        w.window.set_min_inner_size(Some(LogicalSize::new(980.0, 620.0)));
                                        mode.set(StartMode::Launcher);
                                    },
                                }
                            },
                        }}
                    }
                    dialogs::ModalHost {}
                    Toasts {}
                }
            })}
        }
    }
}

#[component]
fn LauncherShell() -> Element {
    let app = use_app();
    use_effect(move || {
        if !*app.loaded.peek() {
            rescan(app);
        }
        if !app.settings.peek().welcome_done {
            app.open(Modal::Welcome);
        }
    });
    let page = *app.page.read();
    match page {
        Page::Library => rsx! { library::LibraryPage {} },
        Page::Settings(_) => rsx! { settings::SettingsPage {} },
    }
}

/// Eigene Titelleiste (ziehen zum Verschieben, Doppelklick = maximieren)
#[component]
fn TitleBar() -> Element {
    rsx! {
        div {
            class: "h-10 shrink-0 flex items-center bg-sidenav/90 border-b border-border/60 relative z-40",
            onmousedown: move |_| window().drag(),
            ondoubleclick: move |_| window().toggle_maximized(),
            library::TitleBrand {}
            div { class: "flex-1" }
            div {
                class: "flex h-full",
                onmousedown: move |e| e.stop_propagation(),
                ondoubleclick: move |e| e.stop_propagation(),
                button {
                    class: "titlebar-btn w-12 h-full grid place-items-center text-muted-foreground hover:bg-white/10 hover:text-foreground cursor-pointer",
                    title: t("win.minimize"),
                    onclick: move |_| window().window.set_minimized(true),
                    Icon { name: "minus", class: "size-4" }
                }
                button {
                    class: "titlebar-btn w-12 h-full grid place-items-center text-muted-foreground hover:bg-white/10 hover:text-foreground cursor-pointer",
                    title: t("win.maximize"),
                    onclick: move |_| window().toggle_maximized(),
                    Icon { name: "square", class: "size-3.5" }
                }
                button {
                    class: "titlebar-btn is-close w-12 h-full grid place-items-center text-muted-foreground hover:text-white cursor-pointer",
                    title: t("common.close"),
                    onclick: move |_| window().close(),
                    Icon { name: "x", class: "size-4" }
                }
            }
        }
    }
}

#[component]
fn Toasts() -> Element {
    let app = use_app();
    let toasts = app.toasts.read().clone();
    rsx! {
        div { class: "fixed bottom-5 right-5 z-[60] flex flex-col gap-2 pointer-events-none",
            {toasts.into_iter().map(|t| {
                let id = t.id;
                let text = t.text.clone();
                let icon = t.icon;
                rsx! {
                    div { key: "{id}", class: "pointer-events-auto relative overflow-hidden flex items-center gap-3 rounded-xl border border-border bg-popover/95 backdrop-blur pl-4 pr-2 py-3 text-sm shadow-2xl shadow-black/40 animate-slide-up max-w-sm",
                        div { class: "size-8 rounded-lg maz-gradient-bg text-white grid place-items-center shrink-0",
                            Icon { name: icon, class: "size-4" }
                        }
                        span { class: "flex-1", "{text}" }
                        button {
                            class: "p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent cursor-pointer",
                            onclick: move |_| {
                                let mut ts = app.toasts;
                                ts.write().retain(|x| x.id != id);
                            },
                            Icon { name: "x", class: "size-3.5" }
                        }
                        // Fortschrittsbalken: verschwindet in 5 Sekunden
                        div { class: "absolute left-0 bottom-0 h-0.5 maz-gradient-bg toast-progress" }
                    }
                }
            })}
        }
    }
}
