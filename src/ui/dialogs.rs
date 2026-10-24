//! Alle Fenster: Sicherheitsfrage, Fehlererklärung, Spiel hinzufügen/bearbeiten …

use super::components::*;
use super::detail::DetailModal;
use super::settings::LayoutPicker;
use super::icons::{Icon, MazLogo};
use super::state::*;
use crate::model::CustomGame;
use crate::problem::{Fix, Problem, Severity};
use crate::{config, sys};
use dioxus::prelude::*;
use std::path::Path;

#[component]
pub fn ModalHost() -> Element {
    let app = use_app();
    let modal = app.modal.read().clone();
    match modal {
        None => rsx! {},
        Some(Modal::Confirm(id)) => rsx! { ConfirmDialog { id } },
        Some(Modal::NotStarted(id)) => rsx! { NotStartedDialog { id } },
        Some(Modal::Problem(p, gid)) => rsx! { ProblemDialog { problem: p, game_id: gid } },
        Some(Modal::EditGame(id)) => rsx! { EditGameDialog { id } },
        Some(Modal::DeleteGame(id)) => rsx! { DeleteDialog { id } },
        Some(Modal::Detail(id)) => rsx! { DetailModal { id } },
        Some(Modal::Welcome) => rsx! { WelcomeDialog {} },
    }
}

fn checklist_for(app: App, id: &str) -> Vec<String> {
    let s = app.settings.read();
    let mut items: Vec<String> = s.checklist.iter().filter(|x| !x.trim().is_empty()).cloned().collect();
    if let Some(c) = s.custom(id) {
        for line in c.checklist.lines() {
            let l = line.trim();
            if !l.is_empty() {
                items.push(l.to_string());
            }
        }
    }
    items
}

#[component]
fn Checklist(items: Vec<String>) -> Element {
    rsx! {
        ul { class: "space-y-2 text-left",
            {items.into_iter().enumerate().map(|(i, it)| rsx! {
                li { key: "{i}", class: "flex items-start gap-3 rounded-lg bg-muted px-4 py-2.5 text-sm",
                    span { class: "mt-0.5 size-5 shrink-0 rounded-full bg-primary/15 text-primary grid place-items-center",
                        Icon { name: "check", class: "size-3" }
                    }
                    span { class: "leading-relaxed", "{it}" }
                }
            })}
        }
    }
}

// ----------------------------------------------------------------------
// Sicherheitsfrage vor jedem Start eines eigenen Spiels
// ----------------------------------------------------------------------

#[component]
fn ConfirmDialog(id: String) -> Element {
    let app = use_app();
    let name = app.settings.read().custom(&id).map(|c| c.name.clone()).unwrap_or_default();
    let items = checklist_for(app, &id);
    let (id_yes, id_no) = (id.clone(), id.clone());
    rsx! {
        Dialog { on_close: move |_| app.close_modal(), close_on_backdrop: false, class: "max-w-lg",
            div { class: "p-8 text-center",
                div { class: "mx-auto size-16 rounded-2xl maz-gradient-bg grid place-items-center text-white shadow-xl mb-5",
                    Icon { name: "shield", class: "size-8" }
                }
                h2 { class: "text-2xl font-bold font-display", "Hast du alles Wichtige deaktiviert?" }
                p { class: "text-muted-foreground mt-2 mb-6 text-sm leading-relaxed",
                    "Bevor „{name}“ startet: Bitte prüfe kurz diese Punkte."
                }
                Checklist { items }
                div { class: "grid grid-cols-2 gap-3 mt-8",
                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Lg, class: "w-full",
                        onclick: move |_| app.open(Modal::NotStarted(id_no.clone())),
                        Icon { name: "x", class: "size-4" }
                        "Nein"
                    }
                    Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, class: "w-full",
                        onclick: move |_| start_custom(app, id_yes.clone(), false),
                        Icon { name: "check", class: "size-4" }
                        "Ja, starten"
                    }
                }
            }
        }
    }
}

#[component]
fn NotStartedDialog(id: String) -> Element {
    let app = use_app();
    let name = app.settings.read().custom(&id).map(|c| c.name.clone()).unwrap_or_default();
    let items = checklist_for(app, &id);
    let id2 = id.clone();
    rsx! {
        Dialog { on_close: move |_| app.close_modal(), class: "max-w-lg",
            div { class: "p-8 text-center",
                div { class: "mx-auto size-16 rounded-2xl bg-warning/15 text-warning grid place-items-center mb-5",
                    Icon { name: "hand", class: "size-8" }
                }
                h2 { class: "text-2xl font-bold font-display", "Okay – das Spiel wurde nicht gestartet" }
                p { class: "text-muted-foreground mt-2 mb-6 text-sm leading-relaxed",
                    "Kein Problem! „{name}“ bleibt aus. So geht es weiter:"
                }
                ol { class: "space-y-2 text-left text-sm mb-6",
                    li { class: "flex gap-3", span { class: "font-bold text-primary", "1." } span { "Deaktiviere bzw. schließe die Punkte aus der Liste unten." } }
                    li { class: "flex gap-3", span { class: "font-bold text-primary", "2." } span { "Komm zurück in den MAZ Launcher." } }
                    li { class: "flex gap-3", span { class: "font-bold text-primary", "3." } span { "Klick noch einmal auf „Spielen“ und dann auf „Ja, starten“." } }
                }
                Checklist { items }
                div { class: "grid grid-cols-2 gap-3 mt-8",
                    Button { variant: ButtonVariant::Outline, size: ButtonSize::Lg, class: "w-full", onclick: move |_| app.close_modal(), "Okay, verstanden" }
                    Button { variant: ButtonVariant::Secondary, size: ButtonSize::Lg, class: "w-full", onclick: move |_| app.open(Modal::Confirm(id2.clone())),
                        "Ich bin jetzt bereit"
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Fehlererklärung – ausführlich und Schritt für Schritt
// ----------------------------------------------------------------------

fn fix_label(f: &Fix) -> (String, &'static str) {
    match f {
        Fix::RunAsAdmin => ("Als Administrator starten".into(), "shield"),
        Fix::EditGame => ("Spiel bearbeiten".into(), "pencil"),
        Fix::OpenUrl { label, .. } => (label.clone(), "external"),
        Fix::OpenFolder { label, .. } => (label.clone(), "folder"),
        Fix::Rescan => ("Bibliothek neu laden".into(), "refresh"),
        Fix::OpenSettings => ("Einstellungen öffnen".into(), "settings"),
    }
}

#[component]
fn ProblemDialog(problem: Problem, #[props(!optional)] game_id: Option<String>) -> Element {
    let app = use_app();
    let mut copied = use_signal(|| false);
    let (ring, icon) = match problem.severity {
        Severity::Error => ("bg-destructive/15 text-destructive", "x-circle"),
        Severity::Warning => ("bg-warning/15 text-warning", "alert"),
        Severity::Info => ("bg-info/15 text-info", "info"),
    };
    let title = problem.title.clone();
    let what = problem.what_happened.clone();
    let why = problem.why.clone();
    let tech = problem.technical.clone();
    let clip = problem.to_clipboard_text();
    let steps = problem.steps.clone();
    let fixes = problem.fixes.clone();
    let copy_icon = if copied() { "check" } else { "copy" };
    let can_edit = game_id.as_ref().map(|id| app.settings.read().custom(id).is_some()).unwrap_or(false);
    rsx! {
        Dialog { on_close: move |_| app.close_modal(), class: "max-w-2xl",
            div { class: "p-8",
                div { class: "flex items-start gap-4 pr-8",
                    div { class: "size-14 shrink-0 rounded-2xl grid place-items-center {ring}",
                        Icon { name: icon, class: "size-7" }
                    }
                    div {
                        div { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-1", "Da hat etwas nicht geklappt" }
                        h2 { class: "text-2xl font-bold leading-tight font-display", "{title}" }
                    }
                }
                div { class: "grid gap-4 mt-6",
                    div { class: "rounded-xl bg-muted p-4",
                        div { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-1.5", "Was ist passiert?" }
                        p { class: "text-[15px] leading-relaxed", "{what}" }
                    }
                    div { class: "rounded-xl bg-muted p-4",
                        div { class: "text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-1.5", "Warum passiert das?" }
                        p { class: "text-[15px] leading-relaxed text-foreground/85", "{why}" }
                    }
                }
                h3 { class: "mt-7 mb-4 text-lg font-bold flex items-center gap-2",
                    Icon { name: "wrench", class: "size-5 text-primary" }
                    "So behebst du es – Schritt für Schritt"
                }
                ol { class: "space-y-3",
                    {steps.into_iter().enumerate().map(|(i, st)| {
                        let n = i + 1;
                        rsx! {
                            li { key: "{i}", class: "flex gap-4 items-start",
                                span { class: "size-8 shrink-0 rounded-full maz-gradient-bg text-white font-bold grid place-items-center text-sm shadow", "{n}" }
                                p { class: "pt-1 text-[15px] leading-relaxed select-text", "{st}" }
                            }
                        }
                    })}
                }
                if !fixes.is_empty() {
                    div { class: "mt-7 flex flex-wrap gap-2",
                        {fixes.into_iter().enumerate().filter(|(_, f)| !matches!(f, Fix::EditGame) || can_edit).map(|(i, f)| {
                            let (label, icon) = fix_label(&f);
                            let variant = if i == 0 { ButtonVariant::Glow } else { ButtonVariant::Outline };
                            let gid = game_id.clone();
                            rsx! {
                                Button { key: "{i}", variant, onclick: move |_| run_fix(app, f.clone(), gid.clone()),
                                    Icon { name: icon, class: "size-4" }
                                    "{label}"
                                }
                            }
                        })}
                    }
                }
                details { class: "mt-7 rounded-xl border border-border group",
                    summary { class: "cursor-pointer px-4 py-3 text-sm text-muted-foreground hover:text-foreground select-none",
                        "Technische Details (für Profis oder für eine Fehlermeldung auf GitHub)"
                    }
                    pre { class: "px-4 pb-4 text-xs text-muted-foreground whitespace-pre-wrap break-all select-text font-mono", "{tech}" }
                }
                div { class: "mt-6 flex justify-between gap-3",
                    Button { variant: ButtonVariant::Ghost, onclick: move |_| {
                            if sys::copy_to_clipboard(&clip) {
                                copied.set(true);
                            }
                        },
                        Icon { name: copy_icon, class: "size-4" }
                        if copied() { "Kopiert!" } else { "Alles kopieren" }
                    }
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| app.close_modal(), "Schließen" }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Eigenes Spiel hinzufügen / bearbeiten
// ----------------------------------------------------------------------

fn pretty_name(path: &str) -> String {
    let stem = Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
    let mut out = String::new();
    let mut prev_lower = false;
    for c in stem.chars() {
        if c == '_' || c == '-' || c == '.' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase();
        out.push(c);
    }
    let out = out
        .replace(" Win64", "")
        .replace(" Shipping", "")
        .replace(" x64", "")
        .replace(" Launcher", "");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[component]
fn Field(#[props(into)] label: String, #[props(into, default)] hint: String, children: Element) -> Element {
    rsx! {
        div { class: "grid gap-2",
            Label { "{label}" }
            {children}
            if !hint.is_empty() {
                p { class: "text-xs text-muted-foreground leading-relaxed", "{hint}" }
            }
        }
    }
}

#[component]
fn EditGameDialog(#[props(!optional)] id: Option<String>) -> Element {
    let app = use_app();
    let existing: Option<CustomGame> = id.as_ref().and_then(|i| app.settings.peek().custom(i).cloned());
    let is_new = existing.is_none();
    let init = existing.clone().unwrap_or_default();
    let mut name = use_signal(|| init.name.clone());
    let mut exe = use_signal(|| init.exe.clone());
    let mut args = use_signal(|| init.args.clone());
    let mut workdir = use_signal(|| init.workdir.clone());
    let mut cover = use_signal(|| init.cover.clone());
    let mut admin = use_signal(|| init.admin);
    let mut checklist = use_signal(|| init.checklist.clone());
    let mut error = use_signal(String::new);

    let save = move |_: MouseEvent| {
        let n = name.peek().trim().to_string();
        let e = exe.peek().trim().trim_matches('"').to_string();
        if n.is_empty() {
            error.set("Bitte gib dem Spiel einen Namen (oben im ersten Feld).".into());
            return;
        }
        if e.is_empty() {
            error.set("Bitte wähle die Start-Datei des Spiels aus. Klick dazu bei „Programm“ auf „Durchsuchen…“.".into());
            return;
        }
        if !Path::new(&e).exists() {
            error.set("Diese Datei gibt es nicht. Klick bei „Programm“ auf „Durchsuchen…“ und wähle die Datei neu aus.".into());
            return;
        }
        let w = workdir.peek().trim().trim_matches('"').to_string();
        if !w.is_empty() && !Path::new(&w).is_dir() {
            error.set("Den Arbeitsordner gibt es nicht. Lass das Feld am besten einfach leer.".into());
            return;
        }
        let mut game = existing.clone().unwrap_or_else(|| CustomGame {
            id: config::new_id(),
            added: sys::now_secs(),
            ..Default::default()
        });
        game.name = n.clone();
        game.exe = e;
        game.args = args.peek().trim().to_string();
        game.workdir = w;
        game.cover = cover.peek().trim().trim_matches('"').to_string();
        game.admin = *admin.peek();
        game.checklist = checklist.peek().clone();
        let gid = game.id.clone();
        app.update(|s| match s.custom_games.iter_mut().find(|c| c.id == gid) {
            Some(c) => *c = game,
            None => s.custom_games.push(game),
        });
        refresh_custom(app, gid);
        app.close_modal();
        app.toast(
            &if is_new { format!("„{n}“ wurde hinzugefügt") } else { format!("„{n}“ wurde gespeichert") },
            "check-circle",
        );
    };

    let title = if is_new { "Eigenes Spiel hinzufügen" } else { "Spiel bearbeiten" };
    let head_icon = if is_new { "plus" } else { "pencil" };
    let err = error();
    rsx! {
        Dialog { on_close: move |_| app.close_modal(), close_on_backdrop: false, class: "max-w-2xl",
            div { class: "p-8",
                div { class: "flex items-center gap-3 mb-1",
                    div { class: "size-10 rounded-xl maz-gradient-bg grid place-items-center text-white",
                        Icon { name: head_icon, class: "size-5" }
                    }
                    h2 { class: "text-2xl font-bold font-display", "{title}" }
                }
                p { class: "text-sm text-muted-foreground mb-6",
                    "Für Spiele, die nicht über Steam oder Epic laufen – z. B. Minecraft, Emulatoren, Indie-Spiele oder Spiele von CD."
                }
                div { class: "grid gap-5",
                    Field { label: "Programm (Start-Datei) *", hint: "Tipp: Die Start-Datei liegt im Ordner des Spiels und heißt meistens wie das Spiel (endet auf .exe). Nicht „unins000.exe“ oder „setup.exe“ nehmen!",
                        div { class: "flex gap-2",
                            Input { class: "font-mono text-xs", placeholder: "C:\\Spiele\\MeinSpiel\\MeinSpiel.exe", value: exe(),
                                oninput: move |v: String| exe.set(v) }
                            Button { variant: ButtonVariant::Secondary, class: "h-10",
                                onclick: move |_| {
                                    let start = exe.peek().clone();
                                    if let Some(p) = pick_game_file(Some(&start)) {
                                        if name.peek().trim().is_empty() {
                                            name.set(pretty_name(&p));
                                        }
                                        exe.set(p);
                                        error.set(String::new());
                                    }
                                },
                                Icon { name: "folder", class: "size-4" }
                                "Durchsuchen…"
                            }
                        }
                    }
                    Field { label: "Name *",
                        Input { placeholder: "z. B. Minecraft", value: name(), oninput: move |v: String| name.set(v) }
                    }
                    div { class: "grid grid-cols-2 gap-4",
                        Field { label: "Startparameter (optional)", hint: "Nur wenn du weißt, was das ist – z. B. -windowed",
                            Input { class: "font-mono text-xs", placeholder: "leer lassen", value: args(), oninput: move |v: String| args.set(v) }
                        }
                        Field { label: "Arbeitsordner (optional)", hint: "Leer lassen = Ordner der Start-Datei (passt fast immer)",
                            div { class: "flex gap-2",
                                Input { class: "font-mono text-xs", placeholder: "automatisch", value: workdir(), oninput: move |v: String| workdir.set(v) }
                                Button { variant: ButtonVariant::Secondary, size: ButtonSize::Icon, class: "size-10", title: "Ordner wählen",
                                    onclick: move |_| {
                                        if let Some(p) = pick_folder("Arbeitsordner wählen") {
                                            workdir.set(p);
                                        }
                                    },
                                    Icon { name: "folder", class: "size-4" }
                                }
                            }
                        }
                    }
                    Field { label: "Cover-Bild (optional)", hint: "Leer lassen = der Launcher sucht automatisch ein passendes Bild bzw. nimmt das Original-Icon.",
                        div { class: "flex gap-2",
                            Input { class: "font-mono text-xs", placeholder: "automatisch", value: cover(), oninput: move |v: String| cover.set(v) }
                            Button { variant: ButtonVariant::Secondary, class: "h-10",
                                onclick: move |_| {
                                    if let Some(p) = pick_image_file() {
                                        cover.set(p);
                                    }
                                },
                                Icon { name: "image", class: "size-4" }
                                "Bild wählen"
                            }
                        }
                    }
                    div { class: "rounded-xl border border-border px-4",
                        SwitchRow {
                            title: "Immer als Administrator starten",
                            description: "Nur einschalten, wenn das Spiel sonst nicht startet. Windows fragt dann jedes Mal nach.",
                            checked: admin(),
                            onchange: move |v: bool| admin.set(v),
                        }
                    }
                    Field { label: "Zusätzliche Punkte für die Sicherheitsfrage (optional)", hint: "Ein Punkt pro Zeile. Wird zusätzlich zu den allgemeinen Punkten vor jedem Start gefragt.",
                        textarea {
                            class: "min-h-20 w-full rounded-md border border-input bg-background/60 px-3 py-2 text-sm outline-none focus-visible:ring-[3px] focus-visible:ring-ring/40",
                            placeholder: "z. B. Mod-Manager geschlossen",
                            value: "{checklist}",
                            oninput: move |e| checklist.set(e.value()),
                        }
                    }
                }
                if !err.is_empty() {
                    div { class: "mt-5 flex items-start gap-3 rounded-xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm animate-slide-up",
                        Icon { name: "alert", class: "size-5 text-destructive shrink-0" }
                        span { "{err}" }
                    }
                }
                div { class: "mt-7 flex justify-end gap-3",
                    Button { variant: ButtonVariant::Ghost, onclick: move |_| app.close_modal(), "Abbrechen" }
                    Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, onclick: save,
                        Icon { name: "check", class: "size-4" }
                        if is_new { "Hinzufügen" } else { "Speichern" }
                    }
                }
            }
        }
    }
}

#[component]
fn DeleteDialog(id: String) -> Element {
    let app = use_app();
    let name = app.settings.read().custom(&id).map(|c| c.name.clone()).unwrap_or_default();
    let id2 = id.clone();
    rsx! {
        Dialog { on_close: move |_| app.close_modal(), class: "max-w-md",
            div { class: "p-8",
                h2 { class: "text-xl font-bold", "„{name}“ aus der Liste entfernen?" }
                p { class: "text-sm text-muted-foreground mt-3 leading-relaxed",
                    "Keine Sorge: Das Spiel selbst wird NICHT gelöscht. Es verschwindet nur aus dem MAZ Launcher und du kannst es jederzeit wieder hinzufügen."
                }
                div { class: "mt-7 flex justify-end gap-3",
                    Button { variant: ButtonVariant::Ghost, onclick: move |_| app.close_modal(), "Abbrechen" }
                    Button { variant: ButtonVariant::Destructive,
                        onclick: move |_| {
                            let gid = id2.clone();
                            app.update(|s| {
                                s.custom_games.retain(|c| c.id != gid);
                                s.favorites.retain(|f| *f != gid);
                            });
                            refresh_custom(app, id2.clone());
                            app.close_modal();
                            app.toast("Spiel aus der Liste entfernt", "trash");
                        },
                        Icon { name: "trash", class: "size-4" }
                        "Entfernen"
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Willkommen (erster Start)
// ----------------------------------------------------------------------

#[component]
fn WelcomeDialog() -> Element {
    let app = use_app();
    let finish = move || {
        app.update(|s| s.welcome_done = true);
        app.close_modal();
    };
    rsx! {
        Dialog { on_close: move |_| finish(), close_on_backdrop: false, class: "max-w-3xl",
            div { class: "p-10",
                div { class: "flex items-center gap-4",
                    MazLogo { class: "size-14 text-base" }
                    div {
                        h2 { class: "text-3xl font-black font-display", "Willkommen beim " span { class: "maz-gradient-text", "MAZ Launcher" } }
                        p { class: "text-muted-foreground mt-1", "Alle deine Spiele an einem Ort – Steam, Epic Games und deine eigenen." }
                    }
                }
                div { class: "grid grid-cols-3 gap-4 mt-8",
                    WelcomeStep { icon: "plug", title: "Automatisch verbunden", text: "Steam- und Epic-Spiele werden von selbst gefunden. Du musst nichts einrichten." }
                    WelcomeStep { icon: "plus", title: "Eigene Spiele", text: "Mit „Spiel hinzufügen“ packst du jedes andere Spiel dazu." }
                    WelcomeStep { icon: "palette", title: "Dein Look", text: "Wähle unten deinen Stil. Mehr Farben gibt es in den Einstellungen." }
                }
                h3 { class: "mt-8 mb-3 font-semibold", "Wie soll deine Bibliothek aussehen?" }
                LayoutPicker {}
                div { class: "mt-8 flex justify-end",
                    Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, onclick: move |_| finish(),
                        "Los geht's"
                        Icon { name: "chevron-right", class: "size-4" }
                    }
                }
            }
        }
    }
}

#[component]
fn WelcomeStep(icon: &'static str, #[props(into)] title: String, #[props(into)] text: String) -> Element {
    rsx! {
        div { class: "rounded-xl bg-muted p-5",
            div { class: "size-10 rounded-lg bg-primary/15 text-primary grid place-items-center mb-3",
                Icon { name: icon, class: "size-5" }
            }
            div { class: "font-semibold text-sm", "{title}" }
            p { class: "text-xs text-muted-foreground mt-1.5 leading-relaxed", "{text}" }
        }
    }
}
