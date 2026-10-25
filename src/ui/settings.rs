//! Einstellungen: Design (inkl. Design-Editor), Sprache, Verbindungen,
//! eigene Spiele, Allgemein, Über.

use super::components::*;
use super::icons::{Icon, MazLogo};
use super::state::*;
use crate::i18n::{self, t, tf};
use crate::model::{default_checklist, layout, StoreStatus};
use crate::theme::{self, CustomTheme, PRESETS};
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
        (SettingsTab::Design, t("set.tab.design"), "palette"),
        (SettingsTab::Account, t("set.tab.account"), "user"),
        (SettingsTab::Language, t("set.tab.language"), "globe"),
        (SettingsTab::Connections, t("set.tab.connections"), "plug"),
        (SettingsTab::Custom, t("set.tab.custom"), "rocket"),
        (SettingsTab::General, t("set.tab.general"), "settings"),
        (SettingsTab::About, t("set.tab.about"), "info"),
    ];
    rsx! {
        div { class: "flex h-full min-h-0 maz-ambient",
            aside { class: "w-64 shrink-0 border-r border-border/60 bg-sidenav/60 p-4 flex flex-col gap-1",
                Button { variant: ButtonVariant::Ghost, class: "justify-start mb-4 -ml-1", onclick: move |_| app.go(Page::Library),
                    Icon { name: "arrow-left", class: "size-4" }
                    {t("set.back")}
                }
                div { class: "px-3 pb-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground", {t("nav.settings")} }
                {tabs.into_iter().map(|(tb, label, icon)| {
                    let active = tb == tab;
                    let cls = if active {
                        "w-full flex items-center gap-3 px-3 h-10 rounded-lg text-sm font-medium bg-sidenav-accent text-foreground cursor-pointer"
                    } else {
                        "w-full flex items-center gap-3 px-3 h-10 rounded-lg text-sm text-sidenav-foreground/80 hover:bg-sidenav-accent/60 hover:text-foreground cursor-pointer"
                    };
                    rsx! {
                        button { key: "{label}", class: "{cls}", onclick: move |_| app.go(Page::Settings(tb)),
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
                        SettingsTab::Language => rsx! { LanguageTab {} },
                        SettingsTab::Account => rsx! { AccountTab {} },
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

#[component]
fn PageTitle(#[props(into)] title: String, #[props(into)] sub: String) -> Element {
    rsx! {
        h1 { class: "text-3xl font-bold font-display mb-2", "{title}" }
        p { class: "text-muted-foreground mb-8", "{sub}" }
    }
}

// ----------------------------------------------------------------------
// Sprache
// ----------------------------------------------------------------------

/// Sprachauswahl (auch im Willkommensfenster)
#[component]
pub fn LanguagePicker() -> Element {
    let app = use_app();
    let current = app.settings.read().language.clone();
    let mut options: Vec<(String, String)> = vec![("auto".to_string(), t("lang.auto"))];
    options.extend(i18n::available());
    rsx! {
        div { class: "grid grid-cols-3 gap-2",
            {options.into_iter().map(|(code, name)| {
                let active = current == code || (current.is_empty() && code == "auto");
                let cls = if active {
                    "h-11 px-3 rounded-lg text-sm font-medium bg-primary text-primary-foreground cursor-pointer flex items-center gap-2"
                } else {
                    "h-11 px-3 rounded-lg text-sm font-medium bg-muted text-foreground hover:bg-accent cursor-pointer flex items-center gap-2"
                };
                let c2 = code.clone();
                rsx! {
                    button { key: "{code}", class: "{cls}",
                        onclick: move |_| {
                            let c = c2.clone();
                            i18n::set_language(&c);
                            app.update(|s| s.language = c);
                        },
                        if active {
                            Icon { name: "check", class: "size-4" }
                        }
                        span { class: "truncate", "{name}" }
                    }
                }
            })}
        }
    }
}

/// Kompakte Sprachauswahl (Dropdown), z. B. im Installer
#[component]
pub fn LanguageSelect() -> Element {
    let app = use_app();
    let current = app.settings.read().language.clone();
    let mut options: Vec<(String, String)> = vec![("auto".to_string(), t("lang.auto"))];
    options.extend(i18n::available());
    rsx! {
        div { class: "relative flex items-center gap-2 text-sm",
            Icon { name: "globe", class: "size-4 text-muted-foreground" }
            select {
                class: "h-9 rounded-md border border-input bg-background/60 px-2 text-sm outline-none cursor-pointer",
                onchange: move |e| {
                    let c = e.value();
                    i18n::set_language(&c);
                    app.update(|s| s.language = c);
                },
                {options.into_iter().map(|(code, name)| {
                    let sel = current == code || (current.is_empty() && code == "auto");
                    rsx! { option { key: "{code}", value: "{code}", selected: sel, "{name}" } }
                })}
            }
        }
    }
}

#[component]
fn LanguageTab() -> Element {
    let folder = i18n::user_lang_folder();
    let folder_s = folder.to_string_lossy().to_string();
    let app = use_app();
    rsx! {
        PageTitle { title: t("set.tab.language"), sub: t("lang.sub") }
        Section { title: t("lang.choose"),
            LanguagePicker {}
        }
        Section { title: t("lang.own"), description: t("lang.own_text"),
            div { class: "flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        let dir = i18n::user_lang_folder();
                        let _ = std::fs::create_dir_all(&dir);
                        if let Some(text) = i18n::builtin("en") {
                            let _ = std::fs::write(dir.join("template-en.txt"), text);
                        }
                        app.toast(&t("lang.template_saved"), "check-circle");
                        sys::open_folder(&dir.to_string_lossy());
                    },
                    Icon { name: "download", class: "size-4" }
                    {t("lang.template")}
                }
                Button { variant: ButtonVariant::Ghost,
                    onclick: move |_| {
                        let _ = std::fs::create_dir_all(&folder);
                        sys::open_folder(&folder_s);
                    },
                    Icon { name: "folder", class: "size-4" }
                    {t("lang.folder")}
                }
            }
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
        (layout::MAZ, t("layout.maz"), t("layout.maz_desc")),
        (layout::STEAM, t("layout.steam"), t("layout.steam_desc")),
        (layout::EPIC, t("layout.epic"), t("layout.epic_desc")),
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

/// Farbvorschau eines Designs (Kachel)
#[component]
fn ThemeTile(theme: CustomTheme, #[props(into)] name: String, active: bool, onselect: EventHandler<()>, #[props(optional)] onedit: Option<EventHandler<()>>) -> Element {
    let ring = if active { "ring-2 ring-primary" } else { "ring-1 ring-border hover:ring-primary/50" };
    let bg = format!(
        "background: linear-gradient(135deg, {} 0%, {} 55%, {} 150%);",
        theme.bg, theme.card, theme.accent
    );
    let side = format!("background: {};", theme.sidebar);
    let acc = format!("background: linear-gradient(90deg, {}, {});", theme.accent, theme.accent2);
    let txt = format!("background: {};", theme.text);
    let mtx = format!("background: {};", theme.muted);
    let has_edit = onedit.is_some();
    rsx! {
        div { class: "group relative rounded-xl overflow-hidden cursor-pointer text-left transition-all {ring}",
            onclick: move |_| onselect.call(()),
            div { class: "h-16 relative flex", style: "{bg}",
                div { class: "w-1/4 h-full opacity-90", style: "{side}" }
                div { class: "flex-1 p-2 flex flex-col gap-1.5",
                    div { class: "h-1.5 w-3/4 rounded-full", style: "{txt}" }
                    div { class: "h-1.5 w-1/2 rounded-full opacity-70", style: "{mtx}" }
                    div { class: "mt-auto h-3 w-10 rounded-full", style: "{acc}" }
                }
                if active {
                    div { class: "absolute top-2 right-2 size-5 rounded-full bg-white text-black grid place-items-center shadow",
                        Icon { name: "check", class: "size-3" }
                    }
                }
            }
            div { class: "px-3 py-2 text-xs font-medium bg-muted flex items-center gap-1",
                span { class: "truncate flex-1", "{name}" }
                if has_edit {
                    button { class: "p-1 -m-1 rounded text-muted-foreground hover:text-foreground hover:bg-accent cursor-pointer",
                        title: t("theme.edit"),
                        onclick: move |e| {
                            e.stop_propagation();
                            if let Some(h) = &onedit {
                                h.call(());
                            }
                        },
                        Icon { name: "pencil", class: "size-3.5" }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ColorField {
    Bg,
    Card,
    Sidebar,
    Text,
    Muted,
    Accent,
    Accent2,
    Border,
}

fn set_field(th: &mut CustomTheme, f: ColorField, v: String) {
    match f {
        ColorField::Bg => th.bg = v,
        ColorField::Card => th.card = v,
        ColorField::Sidebar => th.sidebar = v,
        ColorField::Text => th.text = v,
        ColorField::Muted => th.muted = v,
        ColorField::Accent => th.accent = v,
        ColorField::Accent2 => th.accent2 = v,
        ColorField::Border => th.border = v,
    }
}

fn update_theme(app: App, id: &str, f: impl FnOnce(&mut CustomTheme)) {
    let id = id.to_string();
    app.update(|s| {
        if let Some(th) = s.custom_themes.iter_mut().find(|c| c.id == id) {
            f(th);
        }
    });
}

/// Aktuelle Farben (fertiges oder eigenes Design) als Startwert für ein neues Design
fn current_colors(app: App) -> CustomTheme {
    let s = app.settings.peek();
    s.custom_themes
        .iter()
        .find(|c| c.id == s.theme)
        .cloned()
        .or_else(|| theme::preset(&s.theme))
        .unwrap_or_default()
}

fn new_custom_id() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("custom:{n:x}")
}

#[component]
fn ColorRow(#[props(into)] label: String, value: String, onchange: EventHandler<String>) -> Element {
    let hex = value.clone();
    rsx! {
        div { class: "flex items-center gap-3 py-2",
            input {
                r#type: "color",
                class: "h-9 w-12 shrink-0 rounded-md border border-input bg-transparent cursor-pointer",
                value: "{value}",
                oninput: move |e| onchange.call(e.value()),
            }
            span { class: "flex-1 text-sm", "{label}" }
            Input {
                class: "w-28 h-9 font-mono text-xs",
                value: hex,
                oninput: move |v: String| {
                    if theme::parse_hex(&v).is_some() {
                        onchange.call(v);
                    }
                },
            }
        }
    }
}

#[component]
fn ThemeEditor(id: String, on_done: EventHandler<()>) -> Element {
    let app = use_app();
    let Some(th) = app.settings.read().custom_themes.iter().find(|c| c.id == id).cloned() else {
        return rsx! {};
    };
    let fields = [
        (ColorField::Bg, t("theme.c.bg"), th.bg.clone()),
        (ColorField::Card, t("theme.c.card"), th.card.clone()),
        (ColorField::Sidebar, t("theme.c.sidebar"), th.sidebar.clone()),
        (ColorField::Text, t("theme.c.text"), th.text.clone()),
        (ColorField::Muted, t("theme.c.muted"), th.muted.clone()),
        (ColorField::Accent, t("theme.c.accent"), th.accent.clone()),
        (ColorField::Accent2, t("theme.c.accent2"), th.accent2.clone()),
        (ColorField::Border, t("theme.c.border"), th.border.clone()),
    ];
    let radius = match th.radius {
        0..=3 => "0",
        4..=9 => "6",
        10..=15 => "12",
        _ => "20",
    }
    .to_string();
    let (id_n, id_r, id_dup, id_del, id_code) = (id.clone(), id.clone(), id.clone(), id.clone(), id.clone());
    let name = th.name.clone();
    rsx! {
        Card { class: "mb-6 border-primary/50 ring-1 ring-primary/30 animate-slide-up",
            CardHeader {
                div { class: "flex items-center gap-2",
                    Icon { name: "palette", class: "size-5 text-primary" }
                    CardTitle { {t("theme.editor")} }
                }
                CardDescription { {t("theme.editor_text")} }
            }
            CardContent {
                div { class: "grid gap-2 mb-4",
                    Label { {t("theme.name")} }
                    Input { value: name, oninput: move |v: String| update_theme(app, &id_n, |th| th.name = v) }
                }
                div { class: "grid grid-cols-2 gap-x-6 divide-border",
                    {fields.into_iter().map(|(f, label, value)| {
                        let tid = id.clone();
                        let key = label.clone();
                        rsx! {
                            ColorRow { key: "{key}", label, value,
                                onchange: move |v: String| update_theme(app, &tid, |th| set_field(th, f, v)) }
                        }
                    })}
                }
                div { class: "flex items-center justify-between mt-4 py-2",
                    span { class: "text-sm", {t("theme.radius")} }
                    Segmented {
                        options: vec![
                            ("0".to_string(), t("theme.r0")),
                            ("6".to_string(), t("theme.r1")),
                            ("12".to_string(), t("theme.r2")),
                            ("20".to_string(), t("theme.r3")),
                        ],
                        value: radius,
                        onchange: move |v: String| {
                            let r = v.parse::<u8>().unwrap_or(12);
                            update_theme(app, &id_r, |th| th.radius = r);
                        },
                    }
                }
                div { class: "flex flex-wrap gap-2 mt-5",
                    Button { variant: ButtonVariant::Glow, onclick: move |_| on_done.call(()),
                        Icon { name: "check", class: "size-4" }
                        {t("theme.done")}
                    }
                    Button { variant: ButtonVariant::Outline,
                        onclick: move |_| {
                            let code = app.settings.peek().custom_themes.iter().find(|c| c.id == id_code).map(|c| c.to_code());
                            if let Some(code) = code {
                                if sys::copy_to_clipboard(&code) {
                                    app.toast(&t("theme.copied"), "copy");
                                }
                            }
                        },
                        Icon { name: "copy", class: "size-4" }
                        {t("theme.share")}
                    }
                    Button { variant: ButtonVariant::Outline,
                        onclick: move |_| {
                            let src = app.settings.peek().custom_themes.iter().find(|c| c.id == id_dup).cloned();
                            if let Some(mut c) = src {
                                c.id = new_custom_id();
                                c.name = tf("theme.copy_of", &[("name", &c.name)]);
                                let nid = c.id.clone();
                                app.update(|s| {
                                    s.custom_themes.push(c);
                                    s.theme = nid;
                                });
                            }
                        },
                        Icon { name: "layers", class: "size-4" }
                        {t("theme.duplicate")}
                    }
                    Button { variant: ButtonVariant::Ghost, class: "text-destructive",
                        onclick: move |_| {
                            let did = id_del.clone();
                            app.update(|s| {
                                s.custom_themes.retain(|c| c.id != did);
                                if s.theme == did {
                                    s.theme = "maz".into();
                                }
                            });
                            on_done.call(());
                        },
                        Icon { name: "trash", class: "size-4" }
                        {t("theme.delete")}
                    }
                }
            }
        }
    }
}

#[component]
fn DesignTab() -> Element {
    let app = use_app();
    let s = app.settings.read().clone();
    let mut editing = use_signal(|| None::<String>);
    let mut import_code = use_signal(String::new);
    let accent_value = if s.accent.is_empty() { "#8b5cf6".to_string() } else { s.accent.clone() };
    let has_accent = !s.accent.is_empty();
    let size = s.card_size.to_string();
    let wallpaper = s.wallpaper.clone();
    let has_wallpaper = !wallpaper.is_empty();
    let dim = s.wallpaper_dim;
    let presets: Vec<CustomTheme> = PRESETS.iter().filter_map(|(id, _)| theme::preset(id)).collect();
    let customs = s.custom_themes.clone();
    let editing_id = editing();
    let current = s.theme.clone();
    rsx! {
        PageTitle { title: t("set.tab.design"), sub: t("design.sub") }

        Section { title: t("design.layout"), description: t("design.layout_text"),
            LayoutPicker {}
            div { class: "mt-4",
                SwitchRow {
                    title: t("design.authentic"),
                    description: t("design.authentic_text"),
                    checked: s.authentic_colors,
                    onchange: move |v: bool| app.update(|s| s.authentic_colors = v),
                }
            }
        }

        {editing_id.map(|id| rsx! {
            ThemeEditor { id, on_done: move |_| editing.set(None) }
        })}

        Section { title: t("design.my_themes"), description: t("design.my_themes_text"),
            div { class: "grid grid-cols-4 gap-3",
                {customs.into_iter().map(|c| {
                    let active = current == c.id;
                    let (id1, id2) = (c.id.clone(), c.id.clone());
                    let name = if c.name.is_empty() { t("theme.unnamed") } else { c.name.clone() };
                    let key = c.id.clone();
                    rsx! {
                        ThemeTile { key: "{key}", theme: c, name, active,
                            onselect: move |_| { let i = id1.clone(); app.update(|s| s.theme = i); },
                            onedit: move |_| {
                                let i = id2.clone();
                                app.update(|s| s.theme = i.clone());
                                editing.set(Some(i));
                            },
                        }
                    }
                })}
                button {
                    class: "rounded-xl border-2 border-dashed border-border hover:border-primary hover:text-primary text-muted-foreground min-h-[100px] flex flex-col items-center justify-center gap-2 text-xs font-medium cursor-pointer transition-colors",
                    onclick: move |_| {
                        let mut c = current_colors(app);
                        c.id = new_custom_id();
                        let n = app.settings.peek().custom_themes.len() + 1;
                        c.name = tf("theme.new_name", &[("n", &n.to_string())]);
                        let nid = c.id.clone();
                        app.update(|s| {
                            s.custom_themes.push(c);
                            s.theme = nid.clone();
                        });
                        editing.set(Some(nid));
                    },
                    Icon { name: "plus", class: "size-6" }
                    {t("theme.new")}
                }
            }
            div { class: "flex gap-2 mt-5",
                Input { class: "font-mono text-xs", placeholder: t("theme.import_ph"), value: import_code(), oninput: move |v: String| import_code.set(v) }
                Button { variant: ButtonVariant::Secondary, class: "h-10",
                    onclick: move |_| {
                        let code = import_code.peek().clone();
                        match CustomTheme::from_code(&code) {
                            Some(mut c) => {
                                c.id = new_custom_id();
                                if c.name.is_empty() {
                                    c.name = t("theme.imported");
                                }
                                let nid = c.id.clone();
                                app.update(|s| {
                                    s.custom_themes.push(c);
                                    s.theme = nid;
                                });
                                import_code.set(String::new());
                                app.toast(&t("theme.import_ok"), "check-circle");
                            }
                            None => app.toast(&t("theme.import_bad"), "alert"),
                        }
                    },
                    Icon { name: "download", class: "size-4" }
                    {t("theme.import")}
                }
            }
        }

        Section { title: t("design.presets"), description: t("design.presets_text"),
            div { class: "grid grid-cols-4 gap-3",
                {presets.into_iter().map(|p| {
                    let active = current == p.id;
                    let id = p.id.clone();
                    let name = t(&format!("theme.{}", p.id));
                    let key = p.id.clone();
                    rsx! {
                        ThemeTile { key: "{key}", theme: p, name, active,
                            onselect: move |_| { let i = id.clone(); app.update(|s| s.theme = i); },
                        }
                    }
                })}
            }
            div { class: "mt-5 flex items-center justify-between gap-4 rounded-xl border border-border px-4 py-3",
                div {
                    div { class: "text-sm font-medium", {t("design.accent")} }
                    div { class: "text-xs text-muted-foreground mt-1", {t("design.accent_text")} }
                }
                div { class: "flex items-center gap-2",
                    input {
                        r#type: "color",
                        class: "h-9 w-14 rounded-md border border-input bg-transparent cursor-pointer",
                        value: "{accent_value}",
                        oninput: move |e| { let v = e.value(); app.update(|s| s.accent = v); },
                    }
                    if has_accent {
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| app.update(|s| s.accent.clear()), {t("common.reset")} }
                    }
                }
            }
        }

        Section { title: t("design.wallpaper"), description: t("design.wallpaper_text"),
            div { class: "flex flex-wrap items-center gap-2",
                Button { variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        if let Some(p) = pick_image_file() {
                            app.update(|s| s.wallpaper = p);
                        }
                    },
                    Icon { name: "image", class: "size-4" }
                    {t("design.wallpaper_pick")}
                }
                if has_wallpaper {
                    Button { variant: ButtonVariant::Ghost, onclick: move |_| app.update(|s| s.wallpaper.clear()),
                        Icon { name: "x", class: "size-4" }
                        {t("design.wallpaper_remove")}
                    }
                    span { class: "text-xs text-muted-foreground font-mono truncate max-w-xs", title: "{wallpaper}", "{wallpaper}" }
                }
            }
            if has_wallpaper {
                div { class: "flex items-center gap-4 mt-4",
                    span { class: "text-sm w-40", {t("design.wallpaper_dim")} }
                    input {
                        r#type: "range",
                        class: "flex-1 accent-[var(--primary)] cursor-pointer",
                        min: "0",
                        max: "90",
                        step: "5",
                        value: "{dim}",
                        oninput: move |e| {
                            let v = e.value().parse::<u8>().unwrap_or(60).min(90);
                            app.update(|s| s.wallpaper_dim = v);
                        },
                    }
                    span { class: "text-sm tabular-nums w-10 text-right", "{dim} %" }
                }
                SwitchRow {
                    title: t("design.wallpaper_blur"),
                    checked: s.wallpaper_blur,
                    onchange: move |v: bool| app.update(|s| s.wallpaper_blur = v),
                }
            }
        }

        Section { title: t("design.display"),
            div { class: "flex items-center justify-between py-3",
                div {
                    div { class: "text-sm font-medium", {t("design.card_size")} }
                    div { class: "text-xs text-muted-foreground mt-1", {t("design.card_size_text")} }
                }
                Segmented {
                    options: vec![("1".to_string(), t("design.small")), ("2".to_string(), t("design.medium")), ("3".to_string(), t("design.large"))],
                    value: size,
                    onchange: move |v: String| { let n = v.parse::<u8>().unwrap_or(2); app.update(|s| s.card_size = n); },
                }
            }
            Separator {}
            SwitchRow {
                title: t("design.names"),
                checked: s.show_names,
                onchange: move |v: bool| app.update(|s| s.show_names = v),
            }
            Separator {}
            SwitchRow {
                title: t("design.animations"),
                description: t("design.animations_text"),
                checked: s.animations,
                onchange: move |v: bool| app.update(|s| s.animations = v),
            }
        }
    }
}

// ----------------------------------------------------------------------
// Verbindungen
// ----------------------------------------------------------------------

/// Epic-Konto verbinden → komplette Epic-Bibliothek
#[component]
fn EpicAccount() -> Element {
    let app = use_app();
    let mut waiting = use_signal(|| false);
    let mut pasted = use_signal(String::new);
    let count = use_memo(move || {
        let _ = app.settings.read().epic_synced;
        crate::epicacct::load_cache().len()
    });
    // Während auf die Anmeldung gewartet wird: Zwischenablage beobachten.
    // Sobald die kopierte Epic-Seite darin ist, geht es automatisch weiter.
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(700)).await;
            if !*waiting.peek() || *app.epic_busy.peek() {
                continue;
            }
            if let Some(text) = sys::read_clipboard() {
                if text.contains("authorizationCode") && crate::epicacct::extract_code(&text).is_some() {
                    waiting.set(false);
                    // Code aus der Zwischenablage entfernen (er ist nur einmal gültig)
                    sys::copy_to_clipboard(" ");
                    super::net::epic_connect(app, text);
                }
            }
        }
    });

    let busy = *app.epic_busy.read();
    let err = app.epic_error.read().clone();
    let s = app.settings.read();
    let auth = s.epic_auth.clone();
    let synced = s.epic_synced;
    drop(s);
    let n = count();
    let steps = i18n::tl("epic.steps");

    rsx! {
        div { class: "mt-4 rounded-xl border border-border bg-muted/40 p-4",
            div { class: "flex items-start gap-3",
                div { class: "size-10 shrink-0 rounded-xl bg-primary/15 text-primary grid place-items-center",
                    Icon { name: "key", class: "size-5" }
                }
                div { class: "min-w-0 flex-1",
                    div { class: "font-semibold", {t("epic.title")} }
                    if let Some(a) = auth.clone() {
                        p { class: "text-sm text-success mt-0.5",
                            {tf("epic.connected_as", &[("name", if a.display_name.is_empty() { "Epic" } else { a.display_name.as_str() })])}
                        }
                        p { class: "text-sm text-muted-foreground mt-0.5",
                            {tf("epic.synced", &[("n", &n.to_string()), ("ago", &sys::format_ago(synced))])}
                        }
                    } else {
                        p { class: "text-sm text-muted-foreground mt-0.5", {t("epic.text")} }
                        if n > 0 {
                            p { class: "text-sm text-muted-foreground mt-1", {tf("epic.cached", &[("n", &n.to_string())])} }
                        }
                    }
                }
            }
            if auth.is_some() {
                div { class: "flex flex-wrap gap-2 mt-4",
                    Button { variant: ButtonVariant::Secondary, disabled: busy, onclick: move |_| super::net::sync_epic(app, true),
                        if busy { Spinner {} } else { Icon { name: "refresh", class: "size-4" } }
                        {t("epic.sync_now")}
                    }
                    Button { variant: ButtonVariant::Ghost, disabled: busy, onclick: move |_| super::net::epic_disconnect(app),
                        Icon { name: "log-out", class: "size-4" }
                        {t("epic.disconnect")}
                    }
                }
            } else {
                ol { class: "mt-4 space-y-2",
                    {steps.into_iter().enumerate().map(|(i, st)| {
                        let k = i + 1;
                        rsx! {
                            li { key: "{i}", class: "flex gap-3 items-start text-sm",
                                span { class: "size-6 shrink-0 rounded-full maz-gradient-bg text-white text-xs font-bold grid place-items-center", "{k}" }
                                span { class: "pt-0.5 leading-relaxed", "{st}" }
                            }
                        }
                    })}
                }
                div { class: "flex flex-wrap gap-2 mt-4",
                    Button { variant: ButtonVariant::Glow, disabled: busy,
                        onclick: move |_| {
                            let _ = sys::open_url(&crate::epicacct::login_url());
                            let mut e = app.epic_error;
                            e.set(None);
                            waiting.set(true);
                        },
                        if busy { Spinner {} } else { Icon { name: "external", class: "size-4" } }
                        {t("epic.connect")}
                    }
                }
                if waiting() || busy {
                    div { class: "mt-4 grid gap-2",
                        div { class: "flex items-center gap-2 text-sm text-muted-foreground",
                            Spinner {}
                            {t(if busy { "epic.loading" } else { "epic.waiting" })}
                        }
                        Input {
                            class: "font-mono text-xs",
                            placeholder: t("epic.paste_ph"),
                            value: pasted(),
                            oninput: move |v: String| {
                                pasted.set(v.clone());
                                if crate::epicacct::extract_code(&v).is_some() {
                                    waiting.set(false);
                                    pasted.set(String::new());
                                    super::net::epic_connect(app, v);
                                }
                            },
                        }
                    }
                }
            }
            if let Some(e) = err {
                div { class: "mt-3 flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm",
                    Icon { name: "alert", class: "size-4 mt-0.5 shrink-0 text-destructive" }
                    span { "{e}" }
                }
            }
        }
    }
}

#[component]
fn StatusLine(status: StoreStatus, #[props(into)] store: String) -> Element {
    let (cls, icon, text) = if status.found {
        ("text-success", "check-circle", tf("conn.connected", &[("store", &store)]))
    } else {
        ("text-muted-foreground", "x-circle", tf("conn.not_found", &[("store", &store)]))
    };
    let mut details: Vec<String> = Vec::new();
    if let Some(a) = &status.account {
        details.push(tf("conn.account", &[("name", a)]));
    }
    if status.found {
        details.push(tf("conn.installed", &[("n", &status.installed.to_string())]));
    }
    if status.owned > 0 {
        details.push(tf("conn.owned", &[("n", &status.owned.to_string())]));
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
    let key_steps = i18n::tl("conn.key_steps");
    rsx! {
        PageTitle { title: t("set.tab.connections"), sub: t("conn.sub") }

        Section { title: "Steam",
            StatusLine { status: steam, store: "Steam" }
            if !steam_found {
                div { class: "mt-3",
                    Button { variant: ButtonVariant::Outline, onclick: move |_| { let _ = sys::open_url("https://store.steampowered.com/about/"); },
                        Icon { name: "download", class: "size-4" }
                        {t("fix.steam_dl")}
                    }
                }
            }
            div { class: "mt-2",
                SwitchRow {
                    title: t("conn.show_steam"),
                    checked: s.steam_enabled,
                    onchange: move |v: bool| { app.update(|s| s.steam_enabled = v); rescan(app); },
                }
            }
            Separator { class: "my-2" }
            div { class: "py-3",
                div { class: "text-sm font-medium", {t("conn.all_title")} }
                p { class: "text-xs text-muted-foreground mt-1 mb-3 leading-relaxed", {t("conn.all_text")} }
                ol { class: "text-xs text-muted-foreground space-y-1 mb-4 list-decimal pl-5 leading-relaxed",
                    {key_steps.into_iter().enumerate().map(|(i, st)| rsx! { li { key: "{i}", "{st}" } })}
                }
                div { class: "grid gap-3",
                    div { class: "flex gap-2",
                        Input {
                            kind: "password",
                            class: "font-mono text-xs",
                            placeholder: t("conn.key_ph"),
                            value: s.steam_api_key.clone(),
                            oninput: move |v: String| app.update(|s| s.steam_api_key = v.trim().to_string()),
                        }
                        Button { variant: ButtonVariant::Secondary, class: "h-10", onclick: move |_| { let _ = sys::open_url("https://steamcommunity.com/dev/apikey"); },
                            Icon { name: "external", class: "size-4" }
                            {t("fix.api_key")}
                        }
                    }
                    Input {
                        class: "font-mono text-xs",
                        placeholder: t("conn.id_ph"),
                        value: s.steam_id.clone(),
                        oninput: move |v: String| app.update(|s| s.steam_id = v.trim().to_string()),
                    }
                }
                div { class: "mt-2",
                    SwitchRow {
                        title: t("conn.uninstalled"),
                        description: t("conn.uninstalled_text"),
                        checked: s.steam_show_uninstalled,
                        onchange: move |v: bool| { app.update(|s| s.steam_show_uninstalled = v); rescan(app); },
                    }
                }
            }
        }

        Section { title: "Epic Games",
            StatusLine { status: epic, store: "Epic Games" }
            if !epic_found {
                div { class: "mt-3",
                    Button { variant: ButtonVariant::Outline, onclick: move |_| { let _ = sys::open_url("https://store.epicgames.com/download"); },
                        Icon { name: "download", class: "size-4" }
                        {t("fix.epic_dl")}
                    }
                }
            }
            div { class: "mt-2",
                SwitchRow {
                    title: t("conn.show_epic"),
                    checked: s.epic_enabled,
                    onchange: move |v: bool| { app.update(|s| s.epic_enabled = v); rescan(app); },
                }
                SwitchRow {
                    title: t("conn.uninstalled"),
                    description: t("conn.epic_uninstalled_text"),
                    checked: s.epic_show_uninstalled,
                    onchange: move |v: bool| { app.update(|s| s.epic_show_uninstalled = v); rescan(app); },
                }
            }
            EpicAccount {}
        }

        Section { title: t("conn.images"),
            SwitchRow {
                title: t("conn.online_art"),
                description: t("conn.online_art_text"),
                checked: s.online_art,
                onchange: move |v: bool| app.update(|s| s.online_art = v),
            }
        }

        div { class: "flex justify-end",
            Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, disabled: scanning, onclick: move |_| rescan(app),
                if scanning { Spinner {} } else { Icon { name: "refresh", class: "size-4" } }
                {t("conn.reload_now")}
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
    let items = s.effective_checklist();
    let games = s.custom_games.clone();
    rsx! {
        PageTitle { title: t("set.tab.custom"), sub: t("custom.sub") }

        Section { title: t("custom.check_title"), description: t("custom.check_text"),
            div { class: "grid gap-2",
                {items.iter().cloned().enumerate().map(|(i, it)| rsx! {
                    div { key: "{i}", class: "flex gap-2",
                        Input {
                            value: it,
                            oninput: move |v: String| app.update(|s| {
                                if s.checklist.is_empty() {
                                    s.checklist = default_checklist();
                                }
                                if let Some(x) = s.checklist.get_mut(i) {
                                    *x = v;
                                }
                            }),
                        }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, class: "size-10", title: t("custom.delete_item"),
                            onclick: move |_| app.update(|s| {
                                if s.checklist.is_empty() {
                                    s.checklist = default_checklist();
                                }
                                if i < s.checklist.len() {
                                    s.checklist.remove(i);
                                }
                                if s.checklist.is_empty() {
                                    // Leere Liste würde wieder die Standardliste bedeuten → Platzhalter behalten
                                    s.checklist.push(String::new());
                                }
                            }),
                            Icon { name: "trash", class: "size-4" }
                        }
                    }
                })}
            }
            div { class: "flex gap-2 mt-4",
                Button { variant: ButtonVariant::Outline,
                    onclick: move |_| app.update(|s| {
                        if s.checklist.is_empty() {
                            s.checklist = default_checklist();
                        }
                        s.checklist.push(String::new());
                    }),
                    Icon { name: "plus", class: "size-4" }
                    {t("custom.add_item")}
                }
                Button { variant: ButtonVariant::Ghost, onclick: move |_| app.update(|s| s.checklist.clear()),
                    Icon { name: "refresh", class: "size-4" }
                    {t("custom.reset")}
                }
            }
        }

        Section { title: t("custom.games"), description: t("custom.games_text"),
            if games.is_empty() {
                div { class: "text-sm text-muted-foreground py-2", {t("custom.none")} }
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
                                {t("common.edit")}
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
                    {t("lib.add")}
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
        PageTitle { title: t("set.tab.general"), sub: t("general.sub") }

        Section { title: t("general.behavior"),
            SwitchRow {
                title: t("general.minimize"),
                description: t("general.minimize_text"),
                checked: s.minimize_on_launch,
                onchange: move |v: bool| app.update(|s| s.minimize_on_launch = v),
            }
        }

        Section { title: t("general.install"),
            div { class: "rounded-xl bg-muted p-4 mb-4",
                div { class: "text-xs text-muted-foreground", {t(if installed { "general.installed_in" } else { "general.portable" })} }
                div { class: "text-sm font-mono mt-1 truncate select-text", title: "{exe}", "{exe}" }
            }
            div { class: "flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Outline, onclick: move |_| sys::open_folder(&exe_dir),
                    Icon { name: "folder", class: "size-4" }
                    {t("fix.folder_app")}
                }
                Button { variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        let lnk = sys::desktop_dir().join(format!("{}.lnk", sys::APP_NAME));
                        match install::create_shortcut(&lnk, std::path::Path::new(&exe2)) {
                            Ok(()) => app.toast(&t("toast.shortcut"), "check-circle"),
                            Err(e) => app.show_problem(crate::problem::Problem::shortcut_failed(&exe2, &e), None),
                        }
                    },
                    Icon { name: "monitor", class: "size-4" }
                    {t("general.shortcut")}
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
                        {t("general.uninstall")}
                    }
                }
            }
        }

        Section { title: t("general.data"),
            div { class: "flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Outline, onclick: move |_| sys::open_folder(&sys::roaming_dir().to_string_lossy()),
                    Icon { name: "folder", class: "size-4" }
                    {t("general.settings_folder")}
                }
                Button { variant: ButtonVariant::Outline,
                    onclick: move |_| {
                        let _ = std::fs::remove_dir_all(sys::icon_cache_dir());
                        app.update(|s| s.art_lookup.clear());
                        app.toast(&t("toast.cache_cleared"), "refresh");
                        rescan(app);
                    },
                    Icon { name: "image", class: "size-4" }
                    {t("general.reload_images")}
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
    let version = tf("about.version", &[("v", sys::VERSION)]);
    rsx! {
        div { class: "flex flex-col items-center text-center pt-6",
            MazLogo { class: "size-24 text-2xl rounded-3xl" }
            h1 { class: "text-4xl font-black font-display mt-6", span { class: "maz-gradient-text", "MAZ Launcher" } }
            p { class: "text-muted-foreground mt-2", "{version}" }
            p { class: "text-sm text-muted-foreground mt-6 max-w-md leading-relaxed", {t("about.text")} }
            div { class: "flex gap-3 mt-8",
                Button { variant: ButtonVariant::Glow, onclick: move |_| { let _ = sys::open_url(sys::GITHUB_URL); },
                    Icon { name: "code", class: "size-4" }
                    {t("about.github")}
                }
                Button { variant: ButtonVariant::Outline, onclick: move |_| { let _ = sys::open_url(&format!("{}/issues", sys::GITHUB_URL)); },
                    Icon { name: "alert", class: "size-4" }
                    {t("fix.report")}
                }
            }
            p { class: "text-xs text-muted-foreground mt-12 max-w-md leading-relaxed", {t("about.legal")} }
        }
    }
}

// ----------------------------------------------------------------------
// Konto
// ----------------------------------------------------------------------

#[component]
fn AccountTab() -> Element {
    let app = use_app();
    let s = app.settings.read().clone();
    let me = app.me.read().clone();
    let mut username = use_signal(String::new);
    let mut display = use_signal(String::new);
    let mut server_url = use_signal(|| s.server_url.clone());
    let mut server_key = use_signal(|| s.server_key.clone());
    // Formular mit dem geladenen Profil füllen
    use_effect(move || {
        if let Some(m) = app.me.read().clone() {
            username.set(m.username.clone());
            display.set(m.display_name.clone().unwrap_or_default());
        }
    });
    use_effect(move || {
        if super::net::logged_in(app) && app.me.peek().is_none() {
            super::net::refresh_all(app);
        }
    });
    let session = s.session.clone();
    let (builtin_url, _) = crate::social::builtin_server();
    let has_builtin = !builtin_url.is_empty();
    rsx! {
        PageTitle { title: t("set.tab.account"), sub: t("acc.page_sub") }
        {match session {
            None => rsx! {
                Card { class: "mb-6 bg-card/80 backdrop-blur py-8",
                    super::friends::LoginPanel {}
                }
            },
            Some(sess) => {
                let name = me.as_ref().map(|m| m.name()).unwrap_or_else(|| sess.email.clone());
                let avatar = me.as_ref().and_then(|m| m.avatar_url.clone());
                let provider = match sess.provider.as_str() {
                    "google" => "Google",
                    "github" => "GitHub",
                    _ => "E-Mail",
                };
                let email = sess.email.clone();
                rsx! {
                    Section { title: t("acc.profile"),
                        div { class: "flex items-center gap-4 mb-5",
                            super::friends::Avatar { name: name.clone(), url: avatar, online: Some(true), class: "size-16" }
                            div { class: "min-w-0",
                                div { class: "text-lg font-semibold truncate", "{name}" }
                                div { class: "text-sm text-muted-foreground truncate", {tf("acc.signed_in_with", &[("provider", provider), ("email", &email)])} }
                            }
                        }
                        div { class: "grid grid-cols-2 gap-4",
                            div { class: "grid gap-2",
                                Label { {t("acc.username")} }
                                Input { value: username(), oninput: move |v: String| username.set(v) }
                                p { class: "text-xs text-muted-foreground", {t("acc.username_hint")} }
                            }
                            div { class: "grid gap-2",
                                Label { {t("acc.display_name")} }
                                Input { value: display(), oninput: move |v: String| display.set(v) }
                            }
                        }
                        div { class: "flex gap-2 mt-4",
                            Button { variant: ButtonVariant::Glow, onclick: move |_| super::net::save_profile(app, username.peek().clone(), display.peek().clone()),
                                Icon { name: "check", class: "size-4" }
                                {t("edit.save")}
                            }
                            Button { variant: ButtonVariant::Ghost, class: "text-destructive", onclick: move |_| super::net::logout(app),
                                Icon { name: "log-out", class: "size-4" }
                                {t("acc.logout")}
                            }
                        }
                    }
                    Section { title: t("acc.privacy_title"),
                        SwitchRow {
                            title: t("acc.share_playing"),
                            description: t("acc.share_playing_text"),
                            checked: s.share_playing,
                            onchange: move |v: bool| app.update(|s| s.share_playing = v),
                        }
                    }
                }
            }
        }}
        Section { title: t("acc.server"), description: t(if has_builtin { "acc.server_builtin" } else { "acc.server_text" }),
            div { class: "grid gap-3",
                Input { class: "font-mono text-xs", placeholder: "https://xxxx.supabase.co", value: server_url(), oninput: move |v: String| server_url.set(v) }
                Input { kind: "password", class: "font-mono text-xs", placeholder: t("acc.server_key"), value: server_key(), oninput: move |v: String| server_key.set(v) }
            }
            div { class: "flex flex-wrap gap-2 mt-4",
                Button { variant: ButtonVariant::Secondary,
                    onclick: move |_| {
                        let (u, k) = (server_url.peek().trim().to_string(), server_key.peek().trim().to_string());
                        if !u.is_empty() && crate::social::Backend::new(&u, &k).is_none() {
                            app.toast(&t("acc.server_invalid"), "alert");
                            return;
                        }
                        let changed = {
                            let s = app.settings.peek();
                            s.server_url.trim() != u || s.server_key.trim() != k
                        };
                        if changed {
                            // Neuer Server: alte Anmeldung verwerfen (Tokens nie an fremde Server schicken)
                            super::net::clear_session(app);
                        }
                        app.update(|s| {
                            s.server_url = u;
                            s.server_key = k;
                        });
                        app.toast(&t("acc.saved"), "check-circle");
                    },
                    Icon { name: "check", class: "size-4" }
                    {t("edit.save")}
                }
                Button { variant: ButtonVariant::Outline, onclick: move |_| { let _ = sys::open_url(&format!("{}/blob/main/docs/SERVER-SETUP.md", sys::GITHUB_URL)); },
                    Icon { name: "external", class: "size-4" }
                    {t("acc.setup_guide")}
                }
            }
        }
    }
}
