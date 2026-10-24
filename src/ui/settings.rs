//! Einstellungen: Design, Verbindungen (Steam/Epic), eigene Spiele, Allgemein, Über.

use super::components::*;
use super::icons::{Icon, MazLogo};
use super::state::*;
use crate::model::{default_checklist, layout, StoreStatus, THEMES};
use crate::{install, sys};
use dioxus::prelude::*;

#[component]
pub fn SettingsPage() -> Element {
    let app = use_app();
    let tab = match *app.page.read() {
        Page::Settings(t) => t,
        _ => SettingsTab::Design,
    };
    let tabs = [
        (SettingsTab::Design, "Design", "palette"),
        (SettingsTab::Connections, "Verbindungen", "plug"),
        (SettingsTab::Custom, "Eigene Spiele", "rocket"),
        (SettingsTab::General, "Allgemein", "settings"),
        (SettingsTab::About, "Über", "info"),
    ];
    rsx! {
        div { class: "flex h-full min-h-0 maz-ambient",
            aside { class: "w-64 shrink-0 border-r border-border/60 bg-sidenav/60 p-4 flex flex-col gap-1",
                Button { variant: ButtonVariant::Ghost, class: "justify-start mb-4 -ml-1", onclick: move |_| app.go(Page::Library),
                    Icon { name: "arrow-left", class: "size-4" }
                    "Zurück zur Bibliothek"
                }
                div { class: "px-3 pb-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground", "Einstellungen" }
                {tabs.into_iter().map(|(t, label, icon)| {
                    let active = t == tab;
                    let cls = if active {
                        "w-full flex items-center gap-3 px-3 h-10 rounded-lg text-sm font-medium bg-sidenav-accent text-foreground cursor-pointer"
                    } else {
                        "w-full flex items-center gap-3 px-3 h-10 rounded-lg text-sm text-sidenav-foreground/80 hover:bg-sidenav-accent/60 hover:text-foreground cursor-pointer"
                    };
                    rsx! {
                        button { key: "{label}", class: "{cls}", onclick: move |_| app.go(Page::Settings(t)),
                            Icon { name: icon, class: "size-4" }
                            "{label}"
                        }
                    }
                })}
            }
            main { class: "flex-1 min-w-0 overflow-y-auto",
                div { class: "max-w-3xl mx-auto px-10 py-8 animate-fade-in",
                    {match tab {
                        SettingsTab::Design => rsx! { DesignTab {} },
                        SettingsTab::Connections => rsx! { ConnectionsTab {} },
                        SettingsTab::Custom => rsx! { CustomTab {} },
                        SettingsTab::General => rsx! { GeneralTab {} },
                        SettingsTab::About => rsx! { AboutTab {} },
                    }}
                }
            }
        }
    }
}

#[component]
fn Section(#[props(into)] title: String, #[props(into, default)] description: String, children: Element) -> Element {
    rsx! {
        Card { class: "mb-6 bg-card/80 backdrop-blur",
            CardHeader {
                CardTitle { "{title}" }
                if !description.is_empty() {
                    CardDescription { "{description}" }
                }
            }
            CardContent { {children} }
        }
    }
}

// ----------------------------------------------------------------------
// Design
// ----------------------------------------------------------------------

/// Drei Vorschau-Karten: MAZ / Steam / Epic
#[component]
pub fn LayoutPicker() -> Element {
    let app = use_app();
    let current = app.settings.read().layout.clone();
    let options = [
        (layout::MAZ, "MAZ (eigener Stil)", "Modern mit großem Weiterspielen-Banner"),
        (layout::STEAM, "Wie Steam", "Spieleliste links, Details mit großem Bild"),
        (layout::EPIC, "Wie Epic Games", "Große Karten, Filter oben"),
    ];
    rsx! {
        div { class: "grid grid-cols-3 gap-4",
            {options.into_iter().map(|(id, title, desc)| {
                let active = current == id;
                let ring = if active { "ring-2 ring-primary" } else { "ring-1 ring-border hover:ring-primary/50" };
                rsx! {
                    button {
                        key: "{id}",
                        class: "text-left rounded-xl overflow-hidden bg-muted cursor-pointer transition-all {ring}",
                        onclick: move |_| app.update(|s| s.layout = id.to_string()),
                        LayoutPreview { kind: id }
                        div { class: "p-3",
                            div { class: "flex items-center gap-2 text-sm font-semibold",
                                if active {
                                    Icon { name: "check-circle", class: "size-4 text-primary" }
                                }
                                "{title}"
                            }
                            div { class: "text-xs text-muted-foreground mt-1", "{desc}" }
                        }
                    }
                }
            })}
        }
    }
}

#[component]
fn LayoutPreview(kind: &'static str) -> Element {
    match kind {
        "steam" => rsx! {
            div { class: "h-28 flex bg-[#1b2838]",
                div { class: "w-1/3 bg-[#171d25] p-1.5 space-y-1",
                    for i in 0..7 { div { key: "{i}", class: "h-1.5 rounded-sm bg-white/15" } }
                }
                div { class: "flex-1 p-2",
                    div { class: "h-10 rounded-sm bg-linear-to-r from-[#2a475e] to-[#66c0f4]/40" }
                    div { class: "mt-2 h-3 w-12 rounded-sm bg-[#75b022]" }
                    div { class: "mt-2 grid grid-cols-4 gap-1",
                        for i in 0..4 { div { key: "{i}", class: "aspect-[2/3] rounded-[2px] bg-white/10" } }
                    }
                }
            }
        },
        "epic" => rsx! {
            div { class: "h-28 flex bg-[#101014]",
                div { class: "w-1/4 bg-[#18181c] p-1.5 space-y-1.5",
                    for i in 0..3 { div { key: "{i}", class: "h-2 rounded bg-white/15" } }
                }
                div { class: "flex-1 p-2",
                    div { class: "h-2.5 w-16 rounded bg-white/70 mb-2" }
                    div { class: "flex gap-1 mb-2",
                        for i in 0..4 { div { key: "{i}", class: "h-1.5 w-6 rounded-full bg-white/20" } }
                    }
                    div { class: "grid grid-cols-4 gap-1.5",
                        for i in 0..4 { div { key: "{i}", class: "aspect-[3/4] rounded bg-white/15" } }
                    }
                }
            }
        },
        _ => rsx! {
            div { class: "h-28 flex bg-background",
                div { class: "w-1/4 bg-sidenav p-1.5 space-y-1",
                    div { class: "h-2 rounded maz-gradient-bg mb-2" }
                    for i in 0..5 { div { key: "{i}", class: "h-1.5 rounded bg-foreground/15" } }
                }
                div { class: "flex-1 p-2",
                    div { class: "h-9 rounded-md maz-gradient-bg opacity-80" }
                    div { class: "mt-2 grid grid-cols-5 gap-1",
                        for i in 0..5 { div { key: "{i}", class: "aspect-[2/3] rounded bg-foreground/15" } }
                    }
                }
            }
        },
    }
}

#[component]
fn DesignTab() -> Element {
    let app = use_app();
    let s = app.settings.read().clone();
    let accent_value = if s.accent.is_empty() { "#8b5cf6".to_string() } else { s.accent.clone() };
    let has_accent = !s.accent.is_empty();
    let size = s.card_size.to_string();
    rsx! {
        h1 { class: "text-3xl font-bold font-display mb-2", "Design" }
        p { class: "text-muted-foreground mb-8", "Mach den Launcher zu deinem. Änderungen werden sofort übernommen." }

        Section { title: "Bibliotheks-Ansicht", description: "Wie sollen deine Spiele angezeigt werden?",
            LayoutPicker {}
            div { class: "mt-4",
                SwitchRow {
                    title: "Original-Farben im Steam- und Epic-Stil",
                    description: "An = sieht aus wie das Original. Aus = nutzt dein Farbschema (unten).",
                    checked: s.authentic_colors,
                    onchange: move |v: bool| app.update(|s| s.authentic_colors = v),
                }
            }
        }

        Section { title: "Farbschema", description: "Wähle deine Lieblingsfarben.",
            div { class: "grid grid-cols-4 gap-3",
                {THEMES.iter().map(|(id, name, bg, acc)| {
                    let active = s.theme == *id;
                    let ring = if active { "ring-2 ring-primary" } else { "ring-1 ring-border hover:ring-primary/50" };
                    let id = id.to_string();
                    let name = name.to_string();
                    let style = format!("background: linear-gradient(135deg, {bg} 55%, {acc} 160%);");
                    let dot = format!("background: {acc};");
                    rsx! {
                        button {
                            key: "{id}",
                            class: "rounded-xl overflow-hidden cursor-pointer text-left transition-all {ring}",
                            onclick: move |_| { let t = id.clone(); app.update(|s| s.theme = t); },
                            div { class: "h-16 relative", style: "{style}",
                                div { class: "absolute bottom-2 left-2 size-5 rounded-full ring-2 ring-white/40", style: "{dot}" }
                                if active {
                                    div { class: "absolute top-2 right-2 size-5 rounded-full bg-white text-black grid place-items-center",
                                        Icon { name: "check", class: "size-3" }
                                    }
                                }
                            }
                            div { class: "px-3 py-2 text-xs font-medium bg-muted", "{name}" }
                        }
                    }
                })}
            }
            div { class: "mt-5 flex items-center justify-between gap-4 rounded-xl border border-border px-4 py-3",
                div {
                    div { class: "text-sm font-medium", "Eigene Akzentfarbe" }
                    div { class: "text-xs text-muted-foreground mt-1", "Färbt Knöpfe, Markierungen und Leuchteffekte." }
                }
                div { class: "flex items-center gap-2",
                    input {
                        r#type: "color",
                        class: "h-9 w-14 rounded-md border border-input bg-transparent cursor-pointer",
                        value: "{accent_value}",
                        oninput: move |e| { let v = e.value(); app.update(|s| s.accent = v); },
                    }
                    if has_accent {
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| app.update(|s| s.accent.clear()), "Zurücksetzen" }
                    }
                }
            }
        }

        Section { title: "Darstellung",
            div { class: "flex items-center justify-between py-3",
                div {
                    div { class: "text-sm font-medium", "Größe der Spiele-Karten" }
                    div { class: "text-xs text-muted-foreground mt-1", "S = viele kleine, L = wenige große" }
                }
                Segmented {
                    options: vec![("1".to_string(), "Klein".to_string()), ("2".to_string(), "Mittel".to_string()), ("3".to_string(), "Groß".to_string())],
                    value: size,
                    onchange: move |v: String| { let n = v.parse::<u8>().unwrap_or(2); app.update(|s| s.card_size = n); },
                }
            }
            Separator {}
            SwitchRow {
                title: "Spielnamen unter den Covern anzeigen",
                checked: s.show_names,
                onchange: move |v: bool| app.update(|s| s.show_names = v),
            }
            Separator {}
            SwitchRow {
                title: "Animationen",
                description: "Ausschalten, wenn dein PC langsam ist oder dich Bewegungen stören.",
                checked: s.animations,
                onchange: move |v: bool| app.update(|s| s.animations = v),
            }
        }
    }
}

// ----------------------------------------------------------------------
// Verbindungen
// ----------------------------------------------------------------------

#[component]
fn StatusLine(status: StoreStatus, #[props(into)] store: String) -> Element {
    let (cls, icon, text) = if status.found {
        ("text-success", "check-circle", format!("{store} ist verbunden"))
    } else {
        ("text-muted-foreground", "x-circle", format!("{store} wurde nicht gefunden"))
    };
    let mut details: Vec<String> = Vec::new();
    if let Some(a) = &status.account {
        details.push(format!("Angemeldet als {a}"));
    }
    if status.found {
        details.push(format!("{} installierte Spiele", status.installed));
    }
    if status.owned > 0 {
        details.push(format!("{} Spiele insgesamt im Konto", status.owned));
    }
    let detail = details.join(" · ");
    let path = status.path.clone().unwrap_or_default();
    let note = status.note.clone().unwrap_or_default();
    rsx! {
        div { class: "rounded-xl bg-muted p-4",
            div { class: "flex items-center gap-2 font-semibold {cls}",
                Icon { name: icon, class: "size-5" }
                "{text}"
            }
            if !detail.is_empty() {
                div { class: "text-sm mt-1.5", "{detail}" }
            }
            if !path.is_empty() {
                div { class: "text-xs text-muted-foreground mt-1 font-mono truncate select-text", title: "{path}", "{path}" }
            }
            if !note.is_empty() {
                div { class: "text-xs text-muted-foreground mt-2 leading-relaxed", "{note}" }
            }
        }
    }
}

#[component]
fn ConnectionsTab() -> Element {
    let app = use_app();
    let s = app.settings.read().clone();
    let steam = app.steam.read().clone();
    let epic = app.epic.read().clone();
    let scanning = *app.scanning.read();
    let steam_found = steam.found;
    let epic_found = epic.found;
    rsx! {
        h1 { class: "text-3xl font-bold font-display mb-2", "Verbindungen" }
        p { class: "text-muted-foreground mb-8",
            "Der MAZ Launcher findet deine Spiele automatisch auf deinem PC. Du musst dich nirgends anmelden – deine Passwörter bleiben bei Steam und Epic."
        }

        Section { title: "Steam",
            StatusLine { status: steam, store: "Steam" }
            if !steam_found {
                div { class: "mt-3",
                    Button { variant: ButtonVariant::Outline, onclick: move |_| { let _ = sys::open_url("https://store.steampowered.com/about/"); },
                        Icon { name: "download", class: "size-4" }
                        "Steam herunterladen"
                    }
                }
            }
            div { class: "mt-2",
                SwitchRow {
                    title: "Steam-Spiele anzeigen",
                    checked: s.steam_enabled,
                    onchange: move |v: bool| { app.update(|s| s.steam_enabled = v); rescan(app); },
                }
            }
            Separator { class: "my-2" }
            div { class: "py-3",
                div { class: "text-sm font-medium", "Alle gekauften Steam-Spiele anzeigen (optional)" }
                p { class: "text-xs text-muted-foreground mt-1 mb-3 leading-relaxed",
                    "Ohne Schlüssel siehst du alle installierten Spiele. Mit einem kostenlosen Steam-API-Schlüssel siehst du ALLE Spiele in deinem Konto – auch nicht installierte – plus deine Spielzeit."
                }
                ol { class: "text-xs text-muted-foreground space-y-1 mb-4 list-decimal pl-5 leading-relaxed",
                    li { "Klick auf „API-Schlüssel holen“ und melde dich bei Steam an." }
                    li { "Bei „Domainname“ einfach localhost eintragen, Haken setzen, „Registrieren“ klicken." }
                    li { "Den Schlüssel (32 Zeichen) kopieren und hier einfügen. Fertig!" }
                }
                div { class: "grid gap-3",
                    div { class: "flex gap-2",
                        Input {
                            kind: "password",
                            class: "font-mono text-xs",
                            placeholder: "Steam-API-Schlüssel",
                            value: s.steam_api_key.clone(),
                            oninput: move |v: String| app.update(|s| s.steam_api_key = v.trim().to_string()),
                        }
                        Button { variant: ButtonVariant::Secondary, class: "h-10", onclick: move |_| { let _ = sys::open_url("https://steamcommunity.com/dev/apikey"); },
                            Icon { name: "external", class: "size-4" }
                            "API-Schlüssel holen"
                        }
                    }
                    Input {
                        class: "font-mono text-xs",
                        placeholder: "Steam-ID (leer lassen = wird automatisch erkannt)",
                        value: s.steam_id.clone(),
                        oninput: move |v: String| app.update(|s| s.steam_id = v.trim().to_string()),
                    }
                }
                div { class: "mt-2",
                    SwitchRow {
                        title: "Nicht installierte Spiele zeigen",
                        description: "Werden grau angezeigt. Ein Klick öffnet die Installation in Steam.",
                        checked: s.steam_show_uninstalled,
                        onchange: move |v: bool| app.update(|s| s.steam_show_uninstalled = v),
                    }
                }
            }
        }

        Section { title: "Epic Games",
            StatusLine { status: epic, store: "Epic Games" }
            if !epic_found {
                div { class: "mt-3",
                    Button { variant: ButtonVariant::Outline, onclick: move |_| { let _ = sys::open_url("https://store.epicgames.com/de/download"); },
                        Icon { name: "download", class: "size-4" }
                        "Epic Games Launcher herunterladen"
                    }
                }
            }
            div { class: "mt-2",
                SwitchRow {
                    title: "Epic-Spiele anzeigen",
                    checked: s.epic_enabled,
                    onchange: move |v: bool| { app.update(|s| s.epic_enabled = v); rescan(app); },
                }
            }
        }

        Section { title: "Bilder",
            SwitchRow {
                title: "Cover-Bilder aus dem Internet laden",
                description: "Holt schöne Original-Cover (auch für Epic- und eigene Spiele aus dem Steam-Store). Aus = nur Bilder, die schon auf deinem PC sind.",
                checked: s.online_art,
                onchange: move |v: bool| app.update(|s| s.online_art = v),
            }
        }

        div { class: "flex justify-end",
            Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, disabled: scanning, onclick: move |_| rescan(app),
                if scanning { Spinner {} } else { Icon { name: "refresh", class: "size-4" } }
                "Jetzt neu laden"
            }
        }
    }
}

// ----------------------------------------------------------------------
// Eigene Spiele
// ----------------------------------------------------------------------

#[component]
fn CustomTab() -> Element {
    let app = use_app();
    let s = app.settings.read().clone();
    let items = s.checklist.clone();
    let games = s.custom_games.clone();
    rsx! {
        h1 { class: "text-3xl font-bold font-display mb-2", "Eigene Spiele" }
        p { class: "text-muted-foreground mb-8", "Spiele, die du selbst hinzugefügt hast, und die Sicherheitsfrage vor dem Start." }

        Section {
            title: "Sicherheitsfrage vor jedem Start",
            description: "Bevor ein eigenes Spiel startet, fragt der Launcher: „Hast du alles Wichtige deaktiviert?“ Bei „Nein“ startet das Spiel nicht. Hier legst du fest, welche Punkte dabei angezeigt werden.",
            div { class: "grid gap-2",
                {items.iter().cloned().enumerate().map(|(i, it)| rsx! {
                    div { key: "{i}", class: "flex gap-2",
                        Input {
                            value: it,
                            oninput: move |v: String| app.update(|s| {
                                if let Some(x) = s.checklist.get_mut(i) {
                                    *x = v;
                                }
                            }),
                        }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, class: "size-10", title: "Punkt löschen",
                            onclick: move |_| app.update(|s| { if i < s.checklist.len() { s.checklist.remove(i); } }),
                            Icon { name: "trash", class: "size-4" }
                        }
                    }
                })}
            }
            div { class: "flex gap-2 mt-4",
                Button { variant: ButtonVariant::Outline, onclick: move |_| app.update(|s| s.checklist.push(String::new())),
                    Icon { name: "plus", class: "size-4" }
                    "Punkt hinzufügen"
                }
                Button { variant: ButtonVariant::Ghost, onclick: move |_| app.update(|s| s.checklist = default_checklist()),
                    Icon { name: "refresh", class: "size-4" }
                    "Standard wiederherstellen"
                }
            }
        }

        Section { title: "Deine Spiele", description: "Bearbeiten oder entfernen (das Spiel selbst wird dabei nie gelöscht).",
            if games.is_empty() {
                div { class: "text-sm text-muted-foreground py-2", "Du hast noch keine eigenen Spiele hinzugefügt." }
            }
            div { class: "divide-y divide-border",
                {games.into_iter().map(|g| {
                    let (id1, id2) = (g.id.clone(), g.id.clone());
                    let name = g.name.clone();
                    let exe = g.exe.clone();
                    let key = g.id.clone();
                    rsx! {
                        div { key: "{key}", class: "flex items-center gap-3 py-3",
                            div { class: "min-w-0 flex-1",
                                div { class: "text-sm font-medium truncate", "{name}" }
                                div { class: "text-xs text-muted-foreground font-mono truncate", title: "{exe}", "{exe}" }
                            }
                            Button { variant: ButtonVariant::Outline, size: ButtonSize::Sm, onclick: move |_| app.open(Modal::EditGame(Some(id1.clone()))),
                                Icon { name: "pencil", class: "size-3.5" }
                                "Bearbeiten"
                            }
                            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, class: "text-destructive", onclick: move |_| app.open(Modal::DeleteGame(id2.clone())),
                                Icon { name: "trash", class: "size-3.5" }
                            }
                        }
                    }
                })}
            }
            div { class: "mt-4",
                Button { variant: ButtonVariant::Glow, onclick: move |_| app.open(Modal::EditGame(None)),
                    Icon { name: "plus", class: "size-4" }
                    "Spiel hinzufügen"
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Allgemein
// ----------------------------------------------------------------------

#[component]
fn GeneralTab() -> Element {
    let app = use_app();
    let s = app.settings.read().clone();
    let exe = install::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let exe_dir = install::current_exe()
        .and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string()))
        .unwrap_or_default();
    let installed = install::installed_location().is_some();
    let exe2 = exe.clone();
    rsx! {
        h1 { class: "text-3xl font-bold font-display mb-2", "Allgemein" }
        p { class: "text-muted-foreground mb-8", "Verhalten, Installation und Daten." }

        Section { title: "Verhalten",
            SwitchRow {
                title: "Launcher minimieren, wenn ein Spiel startet",
                description: "Macht Platz fürs Spiel. Stürzt ein eigenes Spiel direkt ab, kommt der Launcher mit einer Erklärung zurück.",
                checked: s.minimize_on_launch,
                onchange: move |v: bool| app.update(|s| s.minimize_on_launch = v),
            }
        }

        Section { title: "Installation",
            div { class: "rounded-xl bg-muted p-4 mb-4",
                div { class: "text-xs text-muted-foreground", if installed { "Installiert in" } else { "Läuft ohne Installation (portabel) aus" } }
                div { class: "text-sm font-mono mt-1 truncate select-text", title: "{exe}", "{exe}" }
            }
            div { class: "flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Outline, onclick: move |_| sys::open_folder(&exe_dir),
                    Icon { name: "folder", class: "size-4" }
                    "App-Ordner öffnen"
                }
                Button { variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        let lnk = sys::desktop_dir().join(format!("{}.lnk", sys::APP_NAME));
                        match install::create_shortcut(&lnk, std::path::Path::new(&exe2)) {
                            Ok(()) => app.toast("Desktop-Verknüpfung erstellt", "check-circle"),
                            Err(e) => app.show_problem(crate::problem::Problem::shortcut_failed(&exe2, &e), None),
                        }
                    },
                    Icon { name: "monitor", class: "size-4" }
                    "Desktop-Verknüpfung erstellen"
                }
                if installed {
                    Button { variant: ButtonVariant::Ghost, class: "text-destructive",
                        onclick: move |_| {
                            install::uninstall_interactive();
                            if install::installed_location().is_none() {
                                std::process::exit(0);
                            }
                        },
                        Icon { name: "trash", class: "size-4" }
                        "Deinstallieren"
                    }
                }
            }
        }

        Section { title: "Daten",
            div { class: "flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Outline, onclick: move |_| sys::open_folder(&sys::roaming_dir().to_string_lossy()),
                    Icon { name: "folder", class: "size-4" }
                    "Einstellungsordner öffnen"
                }
                Button { variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        let _ = std::fs::remove_dir_all(sys::icon_cache_dir());
                        app.update(|s| s.art_lookup.clear());
                        app.toast("Bilder-Cache geleert – lade neu …", "refresh");
                        rescan(app);
                    },
                    Icon { name: "image", class: "size-4" }
                    "Bilder neu laden"
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Über
// ----------------------------------------------------------------------

#[component]
fn AboutTab() -> Element {
    let version = sys::VERSION;
    rsx! {
        div { class: "flex flex-col items-center text-center pt-6",
            MazLogo { class: "size-24 text-2xl rounded-3xl" }
            h1 { class: "text-4xl font-black font-display mt-6", span { class: "maz-gradient-text", "MAZ Launcher" } }
            p { class: "text-muted-foreground mt-2", "Version {version}" }
            p { class: "text-sm text-muted-foreground mt-6 max-w-md leading-relaxed",
                "Alle deine Spiele von Steam, Epic Games und deine eigenen an einem Ort. Gebaut zu 100 % in Rust mit Dioxus und Komponenten im Stil von rust-ui."
            }
            div { class: "flex gap-3 mt-8",
                Button { variant: ButtonVariant::Glow, onclick: move |_| { let _ = sys::open_url(sys::GITHUB_URL); },
                    Icon { name: "code", class: "size-4" }
                    "Auf GitHub ansehen"
                }
                Button { variant: ButtonVariant::Outline, onclick: move |_| { let _ = sys::open_url(&format!("{}/issues", sys::GITHUB_URL)); },
                    Icon { name: "alert", class: "size-4" }
                    "Fehler melden"
                }
            }
            p { class: "text-xs text-muted-foreground mt-12 max-w-md leading-relaxed",
                "Steam ist eine Marke der Valve Corporation, Epic Games ist eine Marke von Epic Games, Inc. Der MAZ Launcher ist ein unabhängiges Projekt und mit keinem der beiden verbunden. Es werden keine Spiele heruntergeladen und kein Kopierschutz umgangen – Spiele starten immer über den jeweiligen Original-Launcher."
            }
        }
    }
}
