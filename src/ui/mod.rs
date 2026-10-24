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
use crate::{config, sys};
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
    let theme = s.theme.clone();
    let accent = if !s.accent.is_empty() && authentic.is_empty() {
        let a = &s.accent;
        format!("--primary: {a}; --ring: {a}; --glow: {a};")
    } else {
        String::new()
    };
    let anim = if s.animations { "" } else { "no-anim" };
    drop(s);

    rsx! {
        div {
            "data-theme": "{theme}",
            class: "h-screen w-screen flex flex-col bg-background text-foreground font-sans antialiased overflow-hidden {authentic} {anim}",
            style: "{accent}",
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
    }
}

#[component]
fn LauncherShell() -> Element {
    let app = use_app();
    use_effect(move || {
        rescan(app);
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
                    title: "Minimieren",
                    onclick: move |_| window().window.set_minimized(true),
                    Icon { name: "minus", class: "size-4" }
                }
                button {
                    class: "titlebar-btn w-12 h-full grid place-items-center text-muted-foreground hover:bg-white/10 hover:text-foreground cursor-pointer",
                    title: "Maximieren",
                    onclick: move |_| window().toggle_maximized(),
                    Icon { name: "square", class: "size-3.5" }
                }
                button {
                    class: "titlebar-btn is-close w-12 h-full grid place-items-center text-muted-foreground hover:text-white cursor-pointer",
                    title: "Schließen",
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
                    div { key: "{id}", class: "flex items-center gap-3 rounded-xl border border-border bg-popover/95 backdrop-blur px-4 py-3 text-sm shadow-2xl shadow-black/40 animate-slide-up max-w-sm",
                        div { class: "size-8 rounded-lg maz-gradient-bg text-white grid place-items-center shrink-0",
                            Icon { name: icon, class: "size-4" }
                        }
                        span { "{text}" }
                    }
                }
            })}
        }
    }
}
