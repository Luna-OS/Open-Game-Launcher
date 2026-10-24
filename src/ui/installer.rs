//! Installationsmenü: Ordner wählen, Desktop-Verknüpfung, Startmenü – danach
//! "Jetzt öffnen" oder "App-Ordner öffnen".

use super::components::*;
use super::icons::{Icon, MazLogo};
use super::state::{pick_folder, use_app};
use crate::install::{self, InstallOptions, InstallReport};
use crate::problem::Problem;
use crate::sys;
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
enum Step {
    Options,
    Working,
    Done(InstallReport),
    Failed(Problem),
}

#[component]
pub fn InstallerView(#[props(!optional)] existing: Option<String>, on_portable: EventHandler<()>) -> Element {
    let app = use_app();
    let default_dir = existing.clone().unwrap_or_else(|| sys::default_install_dir().to_string_lossy().to_string());
    let mut dir = use_signal(|| default_dir);
    let mut desktop = use_signal(|| true);
    let mut start_menu = use_signal(|| true);
    let mut step = use_signal(|| Step::Options);
    let exe_size = install::current_exe()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| sys::format_bytes(m.len()))
        .unwrap_or_else(|| "wenige MB".into());

    let do_install = move |_: MouseEvent| {
        let opts = InstallOptions {
            dir: dir.peek().clone(),
            desktop: *desktop.peek(),
            start_menu: *start_menu.peek(),
        };
        step.set(Step::Working);
        spawn(async move {
            let res = tokio::task::spawn_blocking(move || install::install(&opts)).await;
            match res {
                Ok(Ok(report)) => step.set(Step::Done(report)),
                Ok(Err(p)) => step.set(Step::Failed(p)),
                Err(e) => step.set(Step::Failed(Problem::install_failed(&dir.peek(), &e.to_string()))),
            }
        });
    };

    let current = step();
    let version = sys::VERSION;
    rsx! {
        div { class: "h-full flex",
            // Linke Seite: Logo + Vorteile
            div { class: "w-[340px] shrink-0 relative overflow-hidden maz-gradient-bg text-white p-9 flex flex-col",
                div { class: "absolute -right-24 -top-24 size-72 rounded-full bg-white/10 blur-2xl" }
                div { class: "absolute -left-16 bottom-0 size-60 rounded-full bg-black/20 blur-2xl" }
                div { class: "relative",
                    MazLogo { class: "size-16 text-lg bg-white/15 ring-1 ring-white/30" }
                    h1 { class: "text-3xl font-black mt-6 leading-tight font-display", "MAZ Launcher" }
                    p { class: "text-white/80 mt-2 text-sm leading-relaxed", "Alle deine Spiele. Ein Launcher." }
                }
                div { class: "relative mt-10 space-y-4 text-sm",
                    Perk { icon: "gamepad", text: "Steam- und Epic-Spiele automatisch" }
                    Perk { icon: "rocket", text: "Eigene Spiele mit einem Klick" }
                    Perk { icon: "palette", text: "Viele Designs zum Umschalten" }
                    Perk { icon: "wrench", text: "Fehler werden einfach erklärt" }
                }
                div { class: "flex-1" }
                div { class: "relative text-xs text-white/60", "Version {version} · 100 % Rust" }
            }
            // Rechte Seite
            div { class: "flex-1 min-w-0 p-10 flex flex-col overflow-y-auto",
                {match current {
                    Step::Options => rsx! {
                        h2 { class: "text-2xl font-bold font-display", "Installation" }
                        p { class: "text-sm text-muted-foreground mt-1 mb-7", "Nur noch ein Klick. Du brauchst keine Administrator-Rechte." }
                        {existing.clone().map(|ex| { let ex2 = ex.clone(); rsx! {
                            div { class: "mb-6 rounded-xl border border-primary/40 bg-primary/10 p-4 text-sm",
                                div { class: "font-semibold flex items-center gap-2",
                                    Icon { name: "info", class: "size-4 text-primary" }
                                    "Der MAZ Launcher ist schon installiert"
                                }
                                p { class: "text-muted-foreground mt-1 text-xs font-mono truncate", "{ex}" }
                                p { class: "text-muted-foreground mt-2 text-xs", "Mit „Installieren“ wird er auf diese Version aktualisiert. Deine Einstellungen bleiben erhalten." }
                                div { class: "mt-3",
                                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm,
                                        onclick: move |_| {
                                            let exe = std::path::Path::new(&ex2).join(sys::EXE_NAME);
                                            if std::process::Command::new(&exe).spawn().is_ok() {
                                                std::process::exit(0);
                                            }
                                        },
                                        "Installierte Version öffnen"
                                    }
                                }
                            }
                        }})}
                        div { class: "grid gap-2",
                            Label { "Installationsordner" }
                            div { class: "flex gap-2",
                                Input { class: "font-mono text-xs", value: dir(), oninput: move |v: String| dir.set(v) }
                                Button { variant: ButtonVariant::Secondary, class: "h-10",
                                    onclick: move |_| {
                                        if let Some(p) = pick_folder("Wo soll der MAZ Launcher installiert werden?") {
                                            let p = std::path::PathBuf::from(p);
                                            let p = if p.ends_with(sys::APP_NAME) { p } else { p.join(sys::APP_NAME) };
                                            dir.set(p.to_string_lossy().to_string());
                                        }
                                    },
                                    Icon { name: "folder", class: "size-4" }
                                    "Ändern…"
                                }
                            }
                            p { class: "text-xs text-muted-foreground", "Tipp: Einfach so lassen – der Standardordner passt perfekt." }
                        }
                        div { class: "mt-6 rounded-xl border border-border px-4 divide-y divide-border",
                            SwitchRow { title: "Desktop-Verknüpfung erstellen", description: "Starte den Launcher mit Doppelklick vom Desktop.", checked: desktop(), onchange: move |v: bool| desktop.set(v) }
                            SwitchRow { title: "Im Startmenü anzeigen", description: "Findest du dann auch über die Windows-Suche.", checked: start_menu(), onchange: move |v: bool| start_menu.set(v) }
                        }
                        div { class: "flex-1 min-h-6" }
                        div { class: "flex items-center gap-3 mt-6",
                            Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, class: "min-w-44", onclick: do_install,
                                Icon { name: "download", class: "size-4" }
                                "Installieren"
                            }
                            Button { variant: ButtonVariant::Ghost, onclick: move |_| on_portable.call(()),
                                "Ohne Installation starten"
                            }
                            div { class: "flex-1" }
                            span { class: "text-xs text-muted-foreground", "Größe: {exe_size}" }
                        }
                    },
                    Step::Working => rsx! {
                        div { class: "flex-1 flex flex-col items-center justify-center text-center",
                            Spinner { class: "size-10 text-primary" }
                            h2 { class: "text-xl font-bold mt-5", "Wird installiert …" }
                            p { class: "text-sm text-muted-foreground mt-1", "Das dauert nur einen Moment." }
                        }
                    },
                    Step::Done(report) => {
                        let exe = report.exe.clone();
                        let folder = report.dir.clone();
                        let warnings = report.warnings.clone();
                        rsx! {
                            div { class: "flex-1 flex flex-col items-center justify-center text-center animate-pop-in",
                                div { class: "size-20 rounded-full bg-success/15 text-success grid place-items-center",
                                    Icon { name: "check", class: "size-10" }
                                }
                                h2 { class: "text-2xl font-bold mt-5 font-display", "Fertig! 🎉" }
                                p { class: "text-sm text-muted-foreground mt-2 max-w-sm", "Der MAZ Launcher ist installiert. Ab jetzt kannst du ihn über die Desktop-Verknüpfung starten." }
                                {warnings.into_iter().enumerate().map(|(i, w)| {
                                    let t = w.title.clone();
                                    rsx! {
                                        div { key: "{i}", class: "mt-5 w-full max-w-md rounded-xl border border-warning/40 bg-warning/10 p-3 text-left text-sm flex items-center gap-3",
                                            Icon { name: "alert", class: "size-4 text-warning shrink-0" }
                                            span { class: "flex-1", "{t}" }
                                            Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm, onclick: move |_| app.show_problem(w.clone(), None), "Was tun?" }
                                        }
                                    }
                                })}
                                div { class: "flex gap-3 mt-8",
                                    Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg,
                                        onclick: move |_| {
                                            if std::process::Command::new(&exe).spawn().is_ok() {
                                                std::process::exit(0);
                                            }
                                        },
                                        Icon { name: "play", class: "size-4", filled: true }
                                        "Jetzt öffnen"
                                    }
                                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Lg, onclick: move |_| sys::open_folder(&folder),
                                        Icon { name: "folder", class: "size-4" }
                                        "App-Ordner öffnen"
                                    }
                                }
                                Button { variant: ButtonVariant::Ghost, class: "mt-3", onclick: move |_| dioxus::desktop::window().close(), "Schließen" }
                            }
                        }
                    }
                    Step::Failed(p) => {
                        let p2 = p.clone();
                        let title = p.title.clone();
                        let what = p.what_happened.clone();
                        rsx! {
                            div { class: "flex-1 flex flex-col items-center justify-center text-center",
                                div { class: "size-20 rounded-full bg-destructive/15 text-destructive grid place-items-center",
                                    Icon { name: "x", class: "size-10" }
                                }
                                h2 { class: "text-2xl font-bold mt-5", "{title}" }
                                p { class: "text-sm text-muted-foreground mt-2 max-w-sm", "{what}" }
                                div { class: "flex gap-3 mt-8",
                                    Button { variant: ButtonVariant::Glow, onclick: move |_| app.show_problem(p2.clone(), None), "So behebst du es" }
                                    Button { variant: ButtonVariant::Outline, onclick: move |_| step.set(Step::Options), "Nochmal versuchen" }
                                }
                            }
                        }
                    }
                }}
            }
        }
    }
}

#[component]
fn Perk(icon: &'static str, #[props(into)] text: String) -> Element {
    rsx! {
        div { class: "flex items-center gap-3",
            div { class: "size-8 rounded-lg bg-white/15 grid place-items-center",
                Icon { name: icon, class: "size-4" }
            }
            span { class: "text-white/90", "{text}" }
        }
    }
}
