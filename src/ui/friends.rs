//! Freunde: Anmeldung, Freundesliste, "spielt gerade", Anfragen und Chat.

use super::art::{gradient_for, initials};
use super::components::*;
use super::icons::Icon;
use super::net;
use super::state::*;
use crate::i18n::{t, tf};
use crate::social::{Friend, Profile};
use crate::sys;
use dioxus::prelude::*;

/// Profilbilder nur von Google und GitHub laden. Sonst könnte jemand über ein eigenes
/// Bild herausfinden, wann und von welcher IP-Adresse sein Profil angesehen wird.
fn trusted_avatar(url: &str) -> bool {
    const HOSTS: [&str; 3] = [
        "https://lh3.googleusercontent.com/",
        "https://avatars.githubusercontent.com/",
        "https://lh4.googleusercontent.com/",
    ];
    HOSTS.iter().any(|h| url.starts_with(h)) && !url.contains('"')
}

/// Rundes Profilbild (Bild oder Initialen) mit optionalem Online-Punkt
#[component]
pub fn Avatar(
    #[props(into)] name: String,
    #[props(!optional)] url: Option<String>,
    #[props(into, default)] class: String,
    #[props(!optional)] online: Option<bool>,
) -> Element {
    let cls = if class.is_empty() { "size-10".to_string() } else { class };
    let grad = gradient_for(&name);
    let ini = initials(&name);
    let mut broken = use_signal(|| false);
    let dot = match online {
        Some(true) => "bg-success",
        Some(false) => "bg-muted-foreground/50",
        None => "",
    };
    rsx! {
        div { class: "relative shrink-0 {cls}",
            {match url.filter(|u| trusted_avatar(u) && !broken()) {
                Some(u) => rsx! {
                    img { src: "{u}", class: "size-full rounded-full object-cover", "draggable": "false", onerror: move |_| broken.set(true) }
                },
                None => rsx! {
                    div { class: "size-full rounded-full {grad} grid place-items-center text-white text-xs font-bold", "{ini}" }
                },
            }}
            if !dot.is_empty() {
                span { class: "absolute -bottom-0.5 -right-0.5 size-3 rounded-full ring-2 ring-sidenav {dot}" }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Anmeldung
// ----------------------------------------------------------------------

#[component]
pub fn LoginPanel() -> Element {
    let app = use_app();
    let mut mode = use_signal(|| "login");
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut username = use_signal(String::new);
    let busy = *app.social_busy.read();
    let err = app.social_error.read().clone();
    let has_backend = app.settings.read().backend().is_some();
    if !has_backend {
        return rsx! { NoServer {} };
    }
    let is_signup = mode() == "signup";
    let submit = move || {
        let (e, p, u) = (email.peek().clone(), password.peek().clone(), username.peek().clone());
        let mut er = app.social_error;
        if !e.contains('@') {
            er.set(Some(t("acc.err_email")));
            return;
        }
        if p.len() < 6 {
            er.set(Some(t("acc.err_password")));
            return;
        }
        if *mode.peek() == "signup" {
            net::signup_email(app, e, p, u);
        } else {
            net::login_email(app, e, p);
        }
    };
    rsx! {
        div { class: "w-full max-w-md mx-auto",
            div { class: "text-center mb-6",
                div { class: "mx-auto size-16 rounded-2xl maz-gradient-bg text-white grid place-items-center shadow-xl mb-4",
                    Icon { name: "users", class: "size-8" }
                }
                h2 { class: "text-2xl font-bold font-display", {t("acc.title")} }
                p { class: "text-sm text-muted-foreground mt-2 leading-relaxed", {t("acc.sub")} }
            }
            div { class: "grid gap-3",
                Button { variant: ButtonVariant::Outline, size: ButtonSize::Lg, class: "w-full bg-white text-[#1f1f1f] hover:bg-white/90 hover:text-[#1f1f1f] border-transparent", disabled: busy,
                    onclick: move |_| net::login_oauth(app, "google"),
                    span { class: "font-black text-[#4285f4]", "G" }
                    {t("acc.google")}
                }
                Button { variant: ButtonVariant::Outline, size: ButtonSize::Lg, class: "w-full bg-[#24292f] text-white hover:bg-[#32383f] hover:text-white border-transparent", disabled: busy,
                    onclick: move |_| net::login_oauth(app, "github"),
                    Icon { name: "code", class: "size-4" }
                    {t("acc.github")}
                }
            }
            div { class: "flex items-center gap-3 my-6 text-xs text-muted-foreground",
                div { class: "flex-1 h-px bg-border" }
                {t("acc.or_email")}
                div { class: "flex-1 h-px bg-border" }
            }
            div { class: "grid gap-3",
                if is_signup {
                    Input { placeholder: t("acc.username"), value: username(), oninput: move |v: String| username.set(v) }
                }
                Input { kind: "email", placeholder: t("acc.email"), value: email(), oninput: move |v: String| email.set(v) }
                Input { kind: "password", placeholder: t("acc.password"), value: password(), oninput: move |v: String| password.set(v),
                    onenter: move |_| submit() }
                Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, class: "w-full", disabled: busy, onclick: move |_| submit(),
                    if busy { Spinner {} } else { Icon { name: "mail", class: "size-4" } }
                    {t(if is_signup { "acc.signup" } else { "acc.login" })}
                }
            }
            {err.map(|e| rsx! {
                div { class: "mt-4 flex items-start gap-3 rounded-xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm animate-slide-up",
                    Icon { name: "alert", class: "size-4 mt-0.5 shrink-0 text-destructive" }
                    span { "{e}" }
                }
            })}
            if busy {
                p { class: "mt-4 text-center text-xs text-muted-foreground", {t("acc.waiting_browser")} }
            }
            div { class: "flex justify-between mt-5 text-sm",
                button { class: "text-primary hover:underline cursor-pointer",
                    onclick: move |_| { mode.set(if is_signup { "login" } else { "signup" }); let mut er = app.social_error; er.set(None); },
                    {t(if is_signup { "acc.have_account" } else { "acc.no_account" })}
                }
                if !is_signup {
                    button { class: "text-muted-foreground hover:text-foreground hover:underline cursor-pointer",
                        onclick: move |_| {
                            let e = email.peek().clone();
                            if e.contains('@') {
                                net::reset_password(app, e);
                            } else {
                                let mut er = app.social_error;
                                er.set(Some(t("acc.err_email")));
                            }
                        },
                        {t("acc.forgot")}
                    }
                }
            }
            p { class: "mt-8 text-xs text-muted-foreground text-center leading-relaxed", {t("acc.privacy")} }
        }
    }
}

#[component]
fn NoServer() -> Element {
    let app = use_app();
    rsx! {
        div { class: "w-full max-w-lg mx-auto text-center",
            div { class: "mx-auto size-16 rounded-2xl bg-muted text-muted-foreground grid place-items-center mb-4",
                Icon { name: "server", class: "size-8" }
            }
            h2 { class: "text-xl font-bold", {t("acc.no_server_title")} }
            p { class: "text-sm text-muted-foreground mt-3 leading-relaxed", {t("acc.no_server_text")} }
            div { class: "flex justify-center gap-3 mt-6",
                Button { variant: ButtonVariant::Glow, onclick: move |_| app.go(Page::Settings(SettingsTab::Account)),
                    Icon { name: "settings", class: "size-4" }
                    {t("acc.server_settings")}
                }
                Button { variant: ButtonVariant::Outline, onclick: move |_| { let _ = sys::open_url(&format!("{}/blob/main/docs/SERVER-SETUP.md", sys::GITHUB_URL)); },
                    Icon { name: "external", class: "size-4" }
                    {t("acc.setup_guide")}
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Freunde-Seite
// ----------------------------------------------------------------------

#[component]
pub fn FriendsPage() -> Element {
    let app = use_app();
    let logged_in = net::logged_in(app);
    use_effect(move || {
        if net::logged_in(app) {
            net::refresh_all(app);
            net::start_loop(app);
        }
    });
    if !logged_in {
        return rsx! {
            div { class: "h-full overflow-y-auto maz-ambient grid place-items-center p-8", LoginPanel {} }
        };
    }
    rsx! {
        div { class: "flex h-full min-h-0",
            FriendList {}
            ChatPanel {}
        }
    }
}

#[component]
fn FriendList() -> Element {
    let app = use_app();
    let friends = app.friends.read().clone();
    let me = app.me.read().clone();
    let chat = app.chat_with.read().clone();
    let playing = app.playing_now.read().clone();
    let mut query = use_signal(String::new);
    let mut results = use_signal(Vec::<Profile>::new);
    let mut searching = use_signal(|| false);
    let incoming: Vec<Friend> = friends.iter().filter(|f| !f.accepted() && f.incoming).cloned().collect();
    let outgoing: Vec<Friend> = friends.iter().filter(|f| !f.accepted() && !f.incoming).cloned().collect();
    let accepted: Vec<Friend> = friends.iter().filter(|f| f.accepted()).cloned().collect();
    let online = accepted.iter().filter(|f| f.online).count();
    let mut do_search = move || {
        let q = query.peek().trim().to_string();
        if q.len() < 2 {
            results.set(Vec::new());
            return;
        }
        searching.set(true);
        dioxus::core::spawn_forever(async move {
            let q2 = q.clone();
            let r = net::call(app, move |b, s| b.search(s, &q2)).await;
            searching.set(false);
            match r {
                Ok(list) => {
                    if list.is_empty() {
                        app.toast(&t("friends.no_results"), "search");
                    }
                    results.set(list);
                }
                Err(e) => app.toast(&e, "alert"),
            }
        });
    };
    let me_name = me.as_ref().map(|m| m.name()).unwrap_or_default();
    let me_user = me.as_ref().map(|m| m.username.clone()).unwrap_or_default();
    let me_avatar = me.as_ref().and_then(|m| m.avatar_url.clone());
    let my_status = match &playing {
        Some(g) => tf("friends.playing", &[("game", g)]),
        None => t("friends.online"),
    };
    rsx! {
        aside { class: "w-80 shrink-0 flex flex-col bg-sidenav border-r border-border/60 min-h-0",
            // Ich
            div { class: "p-4 border-b border-border/60 flex items-center gap-3",
                Avatar { name: me_name.clone(), url: me_avatar, online: Some(true), class: "size-11" }
                div { class: "min-w-0 flex-1",
                    div { class: "font-semibold truncate", "{me_name}" }
                    div { class: "text-xs text-muted-foreground truncate", "@{me_user} · {my_status}" }
                }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, title: t("set.tab.account"), onclick: move |_| app.go(Page::Settings(SettingsTab::Account)),
                    Icon { name: "settings", class: "size-4" }
                }
            }
            // Freund hinzufügen
            div { class: "p-3 border-b border-border/60",
                div { class: "flex gap-2",
                    Input { class: "h-9", placeholder: t("friends.search"), value: query(), oninput: move |v: String| query.set(v), onenter: move |_| do_search() }
                    Button { variant: ButtonVariant::Secondary, size: ButtonSize::Icon, title: t("friends.add"), onclick: move |_| do_search(),
                        if searching() { Spinner {} } else { Icon { name: "user-plus", class: "size-4" } }
                    }
                }
                if !results.read().is_empty() {
                    div { class: "mt-2 rounded-lg border border-border bg-popover divide-y divide-border max-h-60 overflow-y-auto",
                        {results.read().clone().into_iter().map(|p| {
                            let (pid, pname) = (p.id.clone(), p.name());
                            let user = p.username.clone();
                            let key = p.id.clone();
                            rsx! {
                                div { key: "{key}", class: "flex items-center gap-2 px-3 py-2",
                                    Avatar { name: pname.clone(), url: p.avatar_url.clone(), online: None, class: "size-8" }
                                    div { class: "min-w-0 flex-1",
                                        div { class: "text-sm font-medium truncate", "{pname}" }
                                        div { class: "text-xs text-muted-foreground truncate", "@{user}" }
                                    }
                                    Button { variant: ButtonVariant::Default, size: ButtonSize::Sm,
                                        onclick: move |_| { net::add_friend(app, pid.clone(), pname.clone()); results.set(Vec::new()); query.set(String::new()); },
                                        Icon { name: "user-plus", class: "size-3.5" }
                                        {t("friends.add")}
                                    }
                                }
                            }
                        })}
                    }
                }
            }
            div { class: "flex-1 overflow-y-auto p-2 space-y-4",
                if !incoming.is_empty() {
                    div {
                        div { class: "px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-warning", {tf("friends.requests", &[("n", &incoming.len().to_string())])} }
                        {incoming.into_iter().map(|f| {
                            let (id1, id2) = (f.friendship_id, f.friendship_id);
                            let key = f.id.clone();
                            rsx! {
                                div { key: "{key}", class: "flex items-center gap-2 rounded-lg px-2 py-2 bg-warning/10",
                                    Avatar { name: f.name(), url: f.avatar_url.clone(), online: None, class: "size-9" }
                                    div { class: "min-w-0 flex-1",
                                        div { class: "text-sm font-medium truncate", {f.name()} }
                                        div { class: "text-xs text-muted-foreground truncate", {t("friends.wants")} }
                                    }
                                    Button { variant: ButtonVariant::Success, size: ButtonSize::IconSm, title: t("friends.accept"), onclick: move |_| net::respond(app, id1, true),
                                        Icon { name: "check", class: "size-4" }
                                    }
                                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, title: t("friends.decline"), onclick: move |_| net::respond(app, id2, false),
                                        Icon { name: "x", class: "size-4" }
                                    }
                                }
                            }
                        })}
                    }
                }
                div {
                    div { class: "px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground",
                        {tf("friends.list_title", &[("online", &online.to_string()), ("total", &accepted.len().to_string())])}
                    }
                    if accepted.is_empty() {
                        div { class: "px-2 py-6 text-center text-sm text-muted-foreground leading-relaxed", {t("friends.empty")} }
                    }
                    {accepted.into_iter().map(|f| {
                        let active = chat.as_deref() == Some(f.id.as_str());
                        let cls = if active { "w-full flex items-center gap-3 rounded-lg px-2 py-2 text-left bg-sidenav-accent cursor-pointer" } else { "w-full flex items-center gap-3 rounded-lg px-2 py-2 text-left hover:bg-sidenav-accent/60 cursor-pointer" };
                        let fid = f.id.clone();
                        let status = match (&f.playing, f.online) {
                            (Some(g), _) => tf("friends.playing", &[("game", g)]),
                            (None, true) => t("friends.online"),
                            (None, false) => t("friends.offline"),
                        };
                        let status_cls = if f.playing.is_some() { "text-xs truncate text-success font-medium" } else { "text-xs truncate text-muted-foreground" };
                        let unread = f.unread;
                        let key = f.id.clone();
                        rsx! {
                            button { key: "{key}", class: "{cls}", onclick: move |_| net::open_chat(app, fid.clone()),
                                Avatar { name: f.name(), url: f.avatar_url.clone(), online: Some(f.online), class: "size-10" }
                                div { class: "min-w-0 flex-1",
                                    div { class: "text-sm font-medium truncate", {f.name()} }
                                    div { class: "{status_cls}", "{status}" }
                                }
                                if unread > 0 {
                                    span { class: "min-w-5 h-5 px-1.5 rounded-full bg-primary text-primary-foreground text-[11px] font-bold grid place-items-center", "{unread}" }
                                }
                            }
                        }
                    })}
                }
                if !outgoing.is_empty() {
                    div {
                        div { class: "px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground", {t("friends.pending")} }
                        {outgoing.into_iter().map(|f| {
                            let (fsid, fid) = (f.friendship_id, f.id.clone());
                            let key = f.id.clone();
                            rsx! {
                                div { key: "{key}", class: "flex items-center gap-2 rounded-lg px-2 py-2 opacity-80",
                                    Avatar { name: f.name(), url: f.avatar_url.clone(), online: None, class: "size-8" }
                                    div { class: "min-w-0 flex-1 text-sm truncate", {f.name()} }
                                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| net::remove_friend(app, fsid, fid.clone()),
                                        {t("friends.cancel_request")}
                                    }
                                }
                            }
                        })}
                    }
                }
            }
        }
    }
}

#[component]
fn ChatPanel() -> Element {
    let app = use_app();
    // Hooks immer zuerst (feste Reihenfolge)
    let mut draft = use_signal(String::new);
    let mut confirm_remove = use_signal(|| false);
    use_effect(move || {
        // Bei neuen Nachrichten nach unten scrollen
        let _ = app.messages.read().len();
        let _ = dioxus::desktop::window().webview.evaluate_script(
            "setTimeout(function(){var e=document.getElementById('maz-chat');if(e)e.scrollTop=e.scrollHeight},30)",
        );
    });
    let chat = app.chat_with.read().clone();
    let friends = app.friends.read().clone();
    let Some(fid) = chat else {
        return rsx! {
            div { class: "flex-1 grid place-items-center maz-ambient",
                div { class: "text-center max-w-sm px-6",
                    div { class: "mx-auto size-16 rounded-2xl bg-muted text-muted-foreground grid place-items-center mb-4",
                        Icon { name: "message", class: "size-8" }
                    }
                    h2 { class: "text-lg font-semibold", {t("chat.pick_title")} }
                    p { class: "text-sm text-muted-foreground mt-2 leading-relaxed", {t("chat.pick_text")} }
                }
            }
        };
    };
    let Some(friend) = friends.iter().find(|f| f.id == fid).cloned() else {
        return rsx! { div { class: "flex-1" } };
    };
    let msgs = app.messages.read().clone();
    let mut send = move || {
        let body = draft.peek().clone();
        if !body.trim().is_empty() {
            net::send_message(app, body);
            draft.set(String::new());
        }
    };
    let status = match (&friend.playing, friend.online) {
        (Some(g), _) => format!("{} · {}", tf("friends.playing", &[("game", g)]), net::playing_for(&friend.playing_since)),
        (None, true) => t("friends.online"),
        (None, false) => t("friends.offline"),
    };
    let (rm_id, rm_fid) = (friend.friendship_id, friend.id.clone());
    rsx! {
        div { class: "flex-1 min-w-0 flex flex-col maz-ambient",
            div { class: "h-16 shrink-0 px-6 flex items-center gap-3 border-b border-border/60 bg-card/40 backdrop-blur",
                Avatar { name: friend.name(), url: friend.avatar_url.clone(), online: Some(friend.online), class: "size-10" }
                div { class: "min-w-0 flex-1",
                    div { class: "font-semibold truncate", {friend.name()} }
                    div { class: "text-xs text-muted-foreground truncate", "@{friend.username} · {status}" }
                }
                if confirm_remove() {
                    span { class: "text-sm text-muted-foreground", {t("friends.remove_confirm")} }
                    Button { variant: ButtonVariant::Destructive, size: ButtonSize::Sm, onclick: move |_| { confirm_remove.set(false); net::remove_friend(app, rm_id, rm_fid.clone()); }, {t("common.yes")} }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| confirm_remove.set(false), {t("common.no")} }
                } else {
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, class: "text-muted-foreground", onclick: move |_| confirm_remove.set(true),
                        Icon { name: "trash", class: "size-4" }
                        {t("friends.remove")}
                    }
                }
            }
            div { id: "maz-chat", class: "flex-1 overflow-y-auto px-6 py-5 space-y-2 select-text",
                if msgs.is_empty() {
                    div { class: "h-full grid place-items-center text-sm text-muted-foreground", {tf("chat.empty", &[("name", &friend.name())])} }
                }
                {msgs.into_iter().map(|m| {
                    let mine = m.sender != fid;
                    let row = if mine { "flex justify-end" } else { "flex justify-start" };
                    let bubble = if mine {
                        "max-w-[70%] rounded-2xl rounded-br-md px-4 py-2 bg-primary text-primary-foreground shadow"
                    } else {
                        "max-w-[70%] rounded-2xl rounded-bl-md px-4 py-2 bg-card border border-border shadow"
                    };
                    let time = net::message_time(&m.created_at);
                    let body = m.body.clone();
                    let key = m.id;
                    rsx! {
                        div { key: "{key}", class: "{row}",
                            div { class: "{bubble}",
                                div { class: "text-sm whitespace-pre-wrap break-words leading-relaxed", "{body}" }
                                div { class: "text-[10px] opacity-60 mt-1 text-right", "{time}" }
                            }
                        }
                    }
                })}
            }
            div { class: "shrink-0 p-4 border-t border-border/60 bg-card/40 backdrop-blur flex gap-2",
                Input { class: "h-11", placeholder: tf("chat.placeholder", &[("name", &friend.name())]), value: draft(),
                    oninput: move |v: String| draft.set(v), onenter: move |_| send() }
                Button { variant: ButtonVariant::Glow, size: ButtonSize::Lg, onclick: move |_| send(),
                    Icon { name: "send", class: "size-4" }
                    {t("chat.send")}
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// Kleine Freundesliste für die Seitenleiste
// ----------------------------------------------------------------------

#[component]
pub fn SidebarFriends() -> Element {
    let app = use_app();
    if !net::logged_in(app) {
        return rsx! {};
    }
    let friends: Vec<Friend> = app.friends.read().iter().filter(|f| f.accepted() && f.online).take(6).cloned().collect();
    if friends.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "space-y-0.5",
            div { class: "px-3 pb-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground", {t("friends.online_now")} }
            {friends.into_iter().map(|f| {
                let fid = f.id.clone();
                let status = f.playing.clone().map(|g| tf("friends.playing", &[("game", &g)])).unwrap_or_else(|| t("friends.online"));
                let key = f.id.clone();
                rsx! {
                    button { key: "{key}", class: "w-full flex items-center gap-2.5 px-3 py-1.5 rounded-lg hover:bg-sidenav-accent/60 cursor-pointer text-left",
                        onclick: move |_| { app.go(Page::Friends); net::open_chat(app, fid.clone()); },
                        Avatar { name: f.name(), url: f.avatar_url.clone(), online: Some(true), class: "size-7" }
                        div { class: "min-w-0",
                            div { class: "text-xs font-medium truncate", {f.name()} }
                            div { class: "text-[11px] text-muted-foreground truncate", "{status}" }
                        }
                    }
                }
            })}
        }
    }
}
