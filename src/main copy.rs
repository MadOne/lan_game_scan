//main.rs

#![allow(non_snake_case)]
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder}; // Add this import

use dioxus::prelude::*;
use dioxus_primitives::tabs::*;
use lan_scanner::create_scaner;
use lan_scanner::misc::{connect_to_server, load_from_disk, save_to_disk};
use lan_scanner::server::GameServer;

use live_log::http_catcher::{self, catch_http};
use live_log::parser::LogEvent;
use regex::Regex;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc::Sender;

mod rcon;
use crate::rcon::*;

// --- ROUTING ---
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")] LAN {},
    #[route("/favourites")] Favourites {},
    #[route("/rcon")] RconTab {},
}

#[derive(PartialEq, Clone, Copy)]
enum TableMode {
    Lan,
    Fav,
}

#[derive(PartialEq, Clone, Copy)]
enum RconSubTab {
    Terminal,
    Maps,
    Players,
    Management,
}

// --- STATE ---
#[derive(Clone, Copy)]
struct AppState {
    servers: Signal<HashMap<SocketAddr, GameServer>>,
    rcon_sessions: Signal<HashMap<SocketAddr, RconSession>>,
    selected_rcon: Signal<Option<SocketAddr>>,
    query_tx: Signal<Option<Sender<SocketAddr>>>,
}

#[derive(Clone, Copy)]
struct RconManager {
    create_session: Callback<GameServer>,
}

#[derive(Debug, Clone, PartialEq)]
struct Player {
    id: String,
    name: String,
    ping: String,
    is_bot: bool,
}

// --- ASSETS ---
const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

// --- MAIN ---
fn main() {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon = image::load_from_memory(icon_bytes)
        .map(|img| {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            Icon::from_rgba(rgba.into_raw(), width, height).unwrap()
        })
        .ok();

    let window = WindowBuilder::new()
        .with_title("LAN GAME SCAN")
        .with_inner_size(LogicalSize::new(1200.0, 800.0))
        .with_window_icon(icon);

    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window))
        .launch(App);
}

#[component]
fn App() -> Element {
    let mut state = use_context_provider(|| AppState {
        servers: Signal::new(load_from_disk()),
        rcon_sessions: Signal::new(HashMap::new()),
        selected_rcon: Signal::new(None),
        query_tx: Signal::new(None),
    });

    use_context_provider(|| RconManager {
        create_session: Callback::new(move |srv: GameServer| {
            let addr = srv.socket_addr;
            state.rcon_sessions.with_mut(|m| {
                m.entry(addr).or_insert_with(|| RconSession {
                    server: srv,
                    logs: Signal::new(vec![format!("-- Terminal initialized for {} --", addr)]),
                    status: Signal::new(RconStatus::Disconnected),
                });
            });
            state.selected_rcon.set(Some(addr));
        }),
    });

    // 1. Discovery Scanner
    use_future(move || async move {
        let (rx, query_sender) = create_scaner().await;
        state.query_tx.set(Some(query_sender));
        let mut rx = rx.lock().await;
        while let Some(mut incoming) = rx.recv().await {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            incoming.last_update = Some(now);

            state.servers.with_mut(|map| {
                if let Some(existing) = map.get_mut(&incoming.socket_addr) {
                    existing.hostname = incoming.hostname;
                    existing.game = incoming.game;
                    existing.players = incoming.players;
                    existing.players_max = incoming.players_max;
                    existing.map = incoming.map;
                    existing.last_update = Some(now);
                    existing.ping = incoming.ping;
                    existing.bots = incoming.bots
                } else {
                    // Important: Don't set is_favorite to false if it's already in the map (loaded from disk)
                    if !map.contains_key(&incoming.socket_addr) {
                        incoming.is_favorite = false;
                        map.insert(incoming.socket_addr, incoming);
                    }
                }
            });
        }
    });

    // 2. Lifecycle logic
    use_future(move || async move {
        loop {
            let qry = match state.query_tx.cloned() {
                Some(q) => q,
                None => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            tokio::time::sleep(Duration::from_secs(6)).await;
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let timeout = 15;
            let mut to_ping = vec![];

            state.servers.with_mut(|map| {
                // Remove non-favorites that timed out
                map.retain(|_addr, srv| {
                    let elapsed = now - srv.last_update.unwrap_or(0);
                    elapsed < timeout || srv.is_favorite
                });

                for srv in map.values_mut() {
                    if srv.is_favorite {
                        let elapsed = now - srv.last_update.unwrap_or(0);
                        // Mark as offline in UI if timed out
                        if elapsed >= timeout {
                            srv.ping = None;
                        }
                        // Manual probe threshold
                        if elapsed >= 6 {
                            to_ping.push(srv.socket_addr);
                        }
                    }
                }
            });

            for addr in to_ping {
                let _ = qry.send(addr).await;
            }
        }
    });
    let state = use_context::<AppState>();

    let selected_addr = state.selected_rcon.read().clone();

    let session = selected_addr.and_then(|addr| state.rcon_sessions.read().get(&addr).cloned());
    if let Some(session) = session {
        let mut logs = session.logs;

        use_future(move || async move {
            let mut rx = catch_http().await;
            while let Some(parsed) = rx.recv().await {
                if !parsed.pretty.is_empty() {
                    logs.with_mut(|l| {
                        l.push(format!("[{}] {}", parsed.event.type_name(), parsed.pretty));
                    });
                }
            }
        });
    }

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div { class: "h-screen w-screen bg-zinc-950 text-zinc-300 font-sans overflow-hidden", Router::<Route> {} }
    }
}

#[component]
fn Navbar() -> Element {
    let state = use_context::<AppState>();
    let rcon_count = state.rcon_sessions.read().len();
    let route: Route = use_route();
    let base = "px-4 py-2 rounded-md text-sm font-medium transition-all flex items-center gap-2";

    let lan_cls = if matches!(route, Route::LAN {}) {
        format!("{} bg-zinc-800 text-white shadow-inner", base)
    } else {
        format!("{} text-zinc-500 hover:text-zinc-300", base)
    };
    let fav_cls = if matches!(route, Route::Favourites {}) {
        format!("{} bg-zinc-800 text-white shadow-inner", base)
    } else {
        format!("{} text-zinc-500 hover:text-zinc-300", base)
    };
    let rcon_cls = if matches!(route, Route::RconTab {}) {
        format!("{} bg-zinc-800 text-white shadow-inner", base)
    } else {
        format!("{} text-zinc-500 hover:text-zinc-300", base)
    };

    rsx! {
        div { class: "flex flex-col h-full",
            header { class: "flex items-center justify-between px-6 py-4 bg-zinc-900 border-b border-zinc-800 shadow-2xl z-30",
                div { class: "flex items-center gap-3",
                    div { class: "w-8 h-8 bg-indigo-600 rounded flex items-center justify-center text-white font-black shadow-lg", "L" }
                    h1 { class: "text-lg font-bold tracking-tighter text-white uppercase", "LAN GAME SCAN" }
                }
                nav { class: "flex items-center gap-1",
                    Link { to: Route::LAN {}, class: "{lan_cls}", "📡 LAN" }
                    Link { to: Route::Favourites {}, class: "{fav_cls}", "⭐ Favourites" }
                    Link { to: Route::RconTab {}, class: "{rcon_cls}", span { "⌨ RCON" }
                        if rcon_count > 0 { span { class: "bg-indigo-600 text-[10px] px-1.5 py-0.5 rounded-full text-white animate-pulse", "{rcon_count}" } }
                    }
                }
            }
            main { class: "flex-1 overflow-hidden", Outlet::<Route> {} }
        }
    }
}

#[component]
fn ServerTable(
    mode: TableMode,
    items: Vec<GameServer>,
    selection: Signal<Option<SocketAddr>>,
) -> Element {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    rsx! {
        div { class: "flex-1 overflow-auto border border-zinc-800 rounded-xl bg-zinc-900/30 shadow-inner scrollbar-thin",
            table { class: "w-full text-left border-collapse table-fixed",
                thead { class: "sticky top-0 bg-zinc-900 text-zinc-500 text-[10px] uppercase tracking-widest z-10",
                    tr {
                        th { class: "p-4 text-center w-24", "Action" }
                        th { class: "p-4 w-44", "Address" }
                        th { class: "p-4 w-32", "Game" }
                        th { class: "p-4", "Server Name" }
                        th { class: "p-4 w-40", "Map" }
                        th { class: "p-4 text-center w-28", "Players" }
                        th { class: "p-4 text-right w-24", "Ping" }
                    }
                }
                tbody { class: "divide-y divide-zinc-800/50",
                    for srv in items {
                        if (now - srv.last_update.unwrap_or(0)) < 15 || mode == TableMode::Fav {
                            ServerRow { key: "{srv.socket_addr}", srv: srv, mode: mode, selection: selection }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ServerRow(
    srv: GameServer,
    mode: TableMode,
    mut selection: Signal<Option<SocketAddr>>,
) -> Element {
    let mut state = use_context::<AppState>();
    let addr = srv.socket_addr;
    let is_selected = selection() == Some(addr);
    let is_fav = state
        .servers
        .read()
        .get(&addr)
        .map(|s| s.is_favorite)
        .unwrap_or(false);
    let is_online = srv.ping.is_some();

    let row_cls = if is_selected {
        "bg-indigo-600/10 text-white shadow-inner"
    } else if mode == TableMode::Fav && !is_online {
        "text-zinc-500 bg-red-950/5 opacity-80"
    } else {
        "hover:bg-zinc-800/40 text-zinc-400"
    };

    let ping_txt = if is_online {
        format!("{}ms", srv.ping.unwrap())
    } else {
        "OFFLINE".to_string()
    };
    let hostname_color_cls = if is_online {
        "text-zinc-200"
    } else {
        "text-zinc-600"
    };

    rsx! {
        tr { class: "cursor-pointer transition-colors {row_cls}", onclick: move |_| selection.set(Some(addr)),
            td { class: "p-4 text-center w-24",
                div { class: "text-lg transition-transform active:scale-125 inline-block",
                    onclick: move |e| {
                        e.stop_propagation();
                        state.servers.with_mut(|m| {
                            if let Some(s) = m.get_mut(&addr) {
                                s.is_favorite = !s.is_favorite;
                                save_to_disk(m);
                            }
                        });
                    },
                    if is_fav { span { class: "text-yellow-500 drop-shadow-md", "★" } }
                    else { span { class: "text-zinc-800 hover:text-yellow-500/40", "☆" } }
                }
            }
            td { class: "p-4 font-mono text-xs opacity-70 truncate", "{addr}" }
            td { class: "p-4 truncate", span { class: "bg-zinc-800 text-zinc-400 px-2 py-0.5 rounded text-[10px] font-bold uppercase", "{srv.game.clone().unwrap_or_else(|| \"---\".into())}" } }
            td { class: "p-4 font-semibold truncate {hostname_color_cls}", "{srv.hostname.clone().unwrap_or_default()}" }
            td { class: "p-4 text-sm opacity-60 truncate", "{srv.map.clone().unwrap_or_default()}" }
            td { class: "p-4 text-sm text-center opacity-70",
                "{srv.players.unwrap_or(0)}/{srv.players_max.unwrap_or(0)}"
                if let Some(b) = srv.bots {
                    if b > 0 {
                        {
                            let bot_label = if b == 1 { "Bot" } else { "Bots" };
                            rsx! { br {} span { class: "text-[10px] text-zinc-500", "({b} {bot_label})" } }
                        }
                    }
                }
            }
            td { class: "p-4 text-right font-mono",
                if is_online { span { class: "text-emerald-500 font-bold", "{ping_txt}" } }
                else { span { class: "text-zinc-700 text-[10px] font-black border border-zinc-800 px-1.5 py-0.5 rounded", "{ping_txt}" } }
            }
        }
    }
}

#[component]
fn LAN() -> Element {
    let selection = use_signal(|| None as Option<SocketAddr>);
    let state = use_context::<AppState>();

    let items: Vec<_> = state
        .servers
        .read()
        .values()
        .filter(|s| {
            let is_online = s.ping.is_some();
            let ip = s.socket_addr.ip();
            let is_local = match ip {
                IpAddr::V4(v4) => v4.is_private() || v4.is_loopback(),
                IpAddr::V6(v6) => v6.is_loopback(),
            };
            is_online && is_local
        })
        .cloned()
        .collect();

    let selected = selection().and_then(|a| state.servers.read().get(&a).cloned());

    rsx! {
        div { class: "h-full flex flex-col p-6 space-y-6 bg-zinc-950 overflow-hidden",
            div { class: "flex justify-between items-center", h2 { class: "text-lg font-bold text-white tracking-tight", "📡 LAN" } }
            ServerTable { mode: TableMode::Lan, items: items, selection: selection }
            if let Some(srv) = selected { ServerDetails { srv: srv } }
        }
    }
}

#[component]
fn Favourites() -> Element {
    let selection = use_signal(|| None as Option<SocketAddr>);
    let state = use_context::<AppState>();
    let items: Vec<_> = state
        .servers
        .read()
        .values()
        .filter(|s| s.is_favorite)
        .cloned()
        .collect();
    let mut show_form = use_signal(|| false);
    let selected = selection().and_then(|a| state.servers.read().get(&a).cloned());

    rsx! {
        div { class: "h-full flex flex-col p-6 space-y-6 bg-zinc-950 overflow-hidden",
            div { class: "flex justify-between items-center",
                h2 { class: "text-lg font-bold text-white tracking-tight", "⭐ FAVOURITES" }
                button { class: "bg-indigo-600 hover:bg-indigo-500 text-white text-[10px] font-bold px-4 py-2 rounded-lg", onclick: move |_| show_form.set(!show_form()), if show_form() { "CANCEL" } else { "ADD SERVER" } }
            }
            if show_form() { AddServerForm { on_close: move |_| show_form.set(false) } }
            ServerTable { mode: TableMode::Fav, items: items, selection: selection }
            if let Some(srv) = selected { ServerDetails { srv: srv } }
        }
    }
}

#[component]
fn AddServerForm(on_close: EventHandler<()>) -> Element {
    let mut state = use_context::<AppState>();
    let mut input_v = use_signal(String::new);
    rsx! {
        div { class: "bg-zinc-900 border border-zinc-800 p-4 rounded-xl flex gap-4 animate-in fade-in zoom-in-95",
            input { class: "flex-1 bg-zinc-950 border border-zinc-700 rounded-lg p-2 text-sm text-white outline-none focus:border-indigo-500", placeholder: "IP:PORT", value: "{input_v}", oninput: move |e| input_v.set(e.value()) }
            button { class: "bg-zinc-100 hover:bg-white text-black font-bold px-6 py-2 rounded-lg text-sm h-[38px]",
                onclick: move |_| {
                    if let Ok(addr) = input_v().parse::<SocketAddr>() {
                        // 1. Update state
                                    let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
                        state.servers.with_mut(|m| {
                            m.insert(addr, GameServer { socket_addr: addr, hostname: Some("Custom Server".into()), game: None, map: None, players: None, players_max: None, query_port: Some(addr.port()), rcon: None, ping: None, last_update: Some(now), is_favorite: true, bots: None });
                            save_to_disk(m);
                        });

                        // 2. TRIGGER IMMEDIATE PING
                        if let Some(tx) = state.query_tx.read().clone() {
                            spawn(async move {
                                let _ = tx.send(addr).await;

                            });
                        }

                        on_close.call(());
                    }
                },
                "ADD"
            }
        }
    }
}

#[component]
fn ServerDetails(srv: GameServer) -> Element {
    let nav = use_navigator();
    let manager = use_context::<RconManager>();
    let is_online = srv.ping.is_some();
    let addr = srv.socket_addr;
    let s_clone = srv.clone();

    let status_val = if is_online {
        format!("{}ms Latency", srv.ping.unwrap())
    } else {
        "Unreachable".to_string()
    };
    let player_val = format!(
        "{}/{}",
        srv.players.unwrap_or(0),
        srv.players_max.unwrap_or(0)
    );

    rsx! {
        div { class: "bg-zinc-900 border border-zinc-800 rounded-xl p-6 shadow-2xl h-36 flex flex-col justify-center animate-in slide-in-from-bottom-4 duration-500",
            div { class: "flex justify-between items-center",
                div {
                    h2 { class: "text-xl font-black text-white tracking-tighter uppercase truncate max-w-md", "{srv.hostname.clone().unwrap_or_default()}" }
                    div { class: "flex items-center gap-2 mt-0.5 text-zinc-500 font-mono text-xs", p { "{addr}" } if !is_online { span { class: "text-[9px] bg-red-900/20 text-red-500 px-1.5 py-0.5 rounded font-bold border border-red-900/30 uppercase", "Offline" } } }
                }
                div { class: "flex gap-2",
                    button { class: "bg-indigo-600 hover:bg-indigo-500 text-white px-5 py-2 rounded-lg font-bold text-xs", onclick: move |_| { spawn(async move { connect_to_server(addr.to_string()).await; }); }, "JOIN" }
                    button { class: "bg-zinc-800 hover:bg-zinc-700 text-zinc-300 px-5 py-2 rounded-lg font-bold border border-zinc-700 flex items-center gap-2 text-xs",
                        onclick: move |_| { manager.create_session.call(s_clone.clone()); nav.push(Route::RconTab {}); }, span { "⌨" } "RCON"
                    }
                }
            }
            div { class: "grid grid-cols-4 gap-6 mt-4",
                DetailBox { label: "Map".to_string(), value: srv.map.clone().unwrap_or_default() }
                DetailBox { label: "Engine".to_string(), value: "Source / GoldSrc".to_string() }
                DetailBox { label: "Status".to_string(), value: status_val }
                DetailBox { label: "Players".to_string(), value: player_val }
            }
        }
    }
}

#[component]
fn DetailBox(label: String, value: String) -> Element {
    rsx! { div { p { class: "text-[10px] uppercase tracking-widest text-zinc-600 font-black mb-1", "{label}" } p { class: "text-zinc-300 font-medium text-xs", "{value}" } } }
}

#[component]
pub fn RconTab() -> Element {
    let mut state = use_context::<AppState>();
    let sessions_owned = state.rcon_sessions.cloned();
    let mut session_list: Vec<_> = sessions_owned.into_iter().collect();
    session_list.sort_by_key(|(a, _)| a.to_string());
    let sel = state.selected_rcon.read();
    let active_val = sel.as_ref().map(|a| a.to_string()).unwrap_or_default();

    if session_list.is_empty() {
        return rsx! { div { class: "flex flex-col items-center justify-center h-full bg-zinc-950 text-zinc-700 p-20 text-center", p { "No active management sessions." } } };
    }
    rsx! {
        div { class: "flex flex-col h-full bg-zinc-950",
            Tabs { value: active_val.clone(), on_value_change: move |v: String| { if let Ok(a) = v.parse::<SocketAddr>() { state.selected_rcon.set(Some(a)); } },
                TabList { class: "flex items-center border-b border-zinc-800 bg-zinc-900 px-4",
                    for (idx, (addr, sess)) in session_list.iter().enumerate() {
                        {
                            let addr_val = *addr; let addr_str = addr_val.to_string(); let is_a = active_val == addr_str;
                            let t_cls = if is_a { "border-indigo-500 text-white bg-zinc-950" } else { "border-transparent text-zinc-500 hover:text-zinc-300" };
                            rsx! { TabTrigger { key: "{addr_str}", value: addr_str.clone(), index: idx, class: "px-4 py-3 text-[10px] font-black transition-all border-b-2 -mb-px flex items-center gap-2 {t_cls} uppercase tracking-tighter",
                                span { class: "truncate max-w-[140px]", "{sess.server.hostname.clone().unwrap_or_default()}" }
                                button { class: "hover:text-red-500 opacity-30 hover:opacity-100 ml-2 text-lg", onclick: move |e| { e.stop_propagation(); state.rcon_sessions.with_mut(|m| m.remove(&addr_val)); if state.selected_rcon.read().as_ref() == Some(&addr_val) { state.selected_rcon.set(None); } }, "×" }
                            }}
                        }
                    }
                }
                div { class: "flex-1 overflow-hidden",
                    for (idx, (addr, _)) in session_list.iter().enumerate() {
                        {
                            let a_str = addr.to_string();
                            rsx! { TabContent { key: "content-{a_str}", value: a_str, index: idx, class: "h-full", RconConsole { addr: *addr } } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RconConsole(addr: SocketAddr) -> Element {
    let state = use_context::<AppState>();
    let mut inner_tab = use_signal(|| RconSubTab::Terminal);
    let (logs, mut status_signal, game_name) = {
        let sessions = state.rcon_sessions.read();
        let session = match sessions.get(&addr) {
            Some(s) => s.clone(),
            None => return rsx! { div { "Disconnected." } },
        };
        (session.logs, session.status, session.server.game.clone())
    };
    let status = *status_signal.read();
    let mut cmd_input = use_signal(String::new);
    let mut pw_input = use_signal(String::new);
    let base_sub =
        "px-4 py-2 text-[10px] font-black tracking-widest cursor-pointer transition-all border-b-2";

    rsx! {
        div { class: "flex flex-col h-full bg-black font-mono text-xs",
            div { class: "p-3 bg-zinc-900 border-b border-zinc-800 flex justify-between items-center",
                div { class: "flex items-center gap-4 text-zinc-500", span { "ADDR: {addr}" }
                    match status {
                        RconStatus::Disconnected => rsx! { span { class: "text-yellow-600", "● OFFLINE" } },
                        RconStatus::Connecting => rsx! { span { class: "text-blue-500 animate-pulse", "● CONNECTING..." } },
                        RconStatus::Authenticated => rsx! { span { class: "text-emerald-500", "● AUTHENTICATED" } },
                        RconStatus::Error => rsx! { span { class: "text-red-500", "● AUTH FAILED" } },
                    }
                }
                if status != RconStatus::Authenticated {
                    div { class: "flex gap-2",
                        input { r#type: "password", class: "bg-black border border-zinc-700 px-2 py-1 rounded text-white w-32 outline-none focus:border-indigo-500", placeholder: "Password", value: "{pw_input}", oninput: move |e| pw_input.set(e.value()) }
                        button { class: "bg-indigo-600 text-white px-3 py-1 rounded text-[10px] font-bold", onclick: move |_| { let pw = pw_input(); spawn(async move { status_signal.set(RconStatus::Connecting); connect_rcon_task(addr, pw, logs, status_signal).await; }); }, "LOGIN" }
                    }
                }
            }
            if status == RconStatus::Authenticated {
                div { class: "flex bg-zinc-900/80 border-b border-zinc-800 px-4",
                    div { class: format!("{} {}", base_sub, if inner_tab() == RconSubTab::Terminal { "text-indigo-400 border-indigo-500 bg-zinc-900" } else { "text-zinc-600 border-transparent hover:text-zinc-400" }), onclick: move |_| inner_tab.set(RconSubTab::Terminal), "TERMINAL" }
                    div { class: format!("{} {}", base_sub, if inner_tab() == RconSubTab::Maps { "text-indigo-400 border-indigo-500 bg-zinc-900" } else { "text-zinc-600 border-transparent hover:text-zinc-400" }), onclick: move |_| inner_tab.set(RconSubTab::Maps), "MAPS" }
                    div { class: format!("{} {}", base_sub, if inner_tab() == RconSubTab::Players { "text-indigo-400 border-indigo-500 bg-zinc-900" } else { "text-zinc-600 border-transparent hover:text-zinc-400" }), onclick: move |_| inner_tab.set(RconSubTab::Players), "PLAYERS" }
                    div { class: format!("{} {}", base_sub, if inner_tab() == RconSubTab::Management { "text-indigo-400 border-indigo-500 bg-zinc-900" } else { "text-zinc-600 border-transparent hover:text-zinc-400" }), onclick: move |_| inner_tab.set(RconSubTab::Management), "MANAGEMENT" }
                }
            }
            div { class: "flex-1 overflow-hidden",
                match inner_tab() {
                    RconSubTab::Terminal => rsx! {
                        div { class: "flex flex-col h-full",
                            div { class: "flex-1 overflow-y-auto p-6 space-y-1 scrollbar-thin",
                                for log in logs.read().iter() {
                                    {
                                        let log_cls = if log.starts_with(">") { "text-zinc-600 italic" } else { "text-emerald-500" };
                                        rsx! { div { class: "{log_cls}", "{log}" } }
                                    }
                                }
                            }
                            if status == RconStatus::Authenticated {
                                div { class: "p-4 bg-zinc-900/50 border-t border-zinc-800 flex items-center gap-3 shadow-2xl",
                                    span { class: "text-indigo-500 font-bold", ">" }
                                    input { class: "flex-1 bg-transparent outline-none text-zinc-100 placeholder:text-zinc-800", placeholder: "Execute command...", value: "{cmd_input}", autofocus: true,
                                        oninput: move |e| cmd_input.set(e.value()),
                                        onkeydown: move |e| { if e.key() == Key::Enter && !cmd_input().is_empty() { let cmd = cmd_input().clone(); cmd_input.set(String::new()); spawn(async move { send_command_task(addr, cmd, logs).await; }); } }
                                    }
                                }
                            }
                        }
                    },
                    RconSubTab::Maps => rsx! { RconMapsView { addr, game: game_name, logs } },
                    RconSubTab::Players => rsx! { RconPlayersView { addr, logs } },
                    RconSubTab::Management => rsx! { RconManagementView { addr, logs } },
                }
            }
        }
    }
}

#[component]
fn RconMapsView(addr: SocketAddr, game: Option<String>, logs: Signal<Vec<String>>) -> Element {
    let maps = get_maps_for_game(game);
    rsx! {
        div { class: "p-6 grid grid-cols-2 md:grid-cols-4 gap-3 overflow-y-auto h-full scrollbar-thin",
            for map_name in maps {
                button { class: "bg-zinc-900 border border-zinc-800 p-4 rounded-lg hover:border-indigo-500 text-left transition-all group",
                    onclick: move |_| { let cmd = format!("map {}", map_name); spawn(async move { send_command_task(addr, cmd, logs).await; }); },
                    p { class: "text-zinc-600 text-[9px] font-black uppercase mb-1 group-hover:text-indigo-400", "Load Level" }
                    p { class: "text-zinc-200 font-bold", "{map_name}" }
                }
            }
        }
    }
}

#[component]
fn RconPlayersView(addr: SocketAddr, logs: Signal<Vec<String>>) -> Element {
    let players = parse_players_from_status(&logs.read());
    rsx! {
        div { class: "p-6 flex flex-col h-full bg-zinc-950",
            div { class: "flex justify-between items-center mb-6 px-2",
                div { h3 { class: "text-zinc-200 font-bold", "Live Players" } p { class: "text-zinc-500 text-[10px] uppercase", "Found {players.len()} entities" } }
                button { class: "bg-indigo-600 hover:bg-indigo-500 text-white text-[10px] font-black px-4 py-2 rounded-lg shadow-lg",
                    onclick: move |_| { spawn(async move { send_command_task(addr, "status".to_string(), logs).await; }); }, "REFRESH LIST"
                }
            }
            div { class: "flex-1 overflow-auto border border-zinc-800 rounded-xl bg-zinc-900/20",
                table { class: "w-full text-left border-collapse table-fixed",
                    thead { class: "sticky top-0 bg-zinc-900 text-zinc-500 text-[9px] uppercase font-black tracking-widest",
                        tr { th { class: "p-3 w-16", "ID" } th { class: "p-3", "Name" } th { class: "p-3 w-20", "Type" } th { class: "p-3 w-20", "Ping" } th { class: "p-3 w-24 text-right", "Action" } }
                    }
                    tbody { class: "divide-y divide-zinc-800/50",
                        if players.is_empty() { tr { td { colspan: 5, class: "p-10 text-center text-zinc-600 italic text-sm", "No players found. Refresh list." } } }
                        for p in players {
                            {
                                let p_id = p.id.clone();
                                rsx! {
                                    tr { class: "hover:bg-zinc-800/30 transition-colors group",
                                        td { class: "p-3 font-mono text-indigo-500", "#{p_id}" }
                                        td { class: "p-3 font-bold text-zinc-300", "{p.name}" }
                                        td { class: "p-3 text-[9px]", if p.is_bot { "BOT" } else { "HUMAN" } }
                                        td { class: "p-3 font-mono text-zinc-500", "{p.ping}ms" }
                                        td { class: "p-3 text-right", button { class: "opacity-0 group-hover:opacity-100 bg-red-900/20 text-red-500 border border-red-900/50 px-3 py-1 rounded-md text-[10px] font-black hover:bg-red-500 hover:text-white transition-all", onclick: move |_| { let cmd = format!("kickid {}", p_id); spawn(async move { send_command_task(addr, cmd, logs).await; }); }, "KICK" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RconManagementView(addr: SocketAddr, logs: Signal<Vec<String>>) -> Element {
    let mut cfg_name = use_signal(|| "server.cfg".to_string());
    let mut msg = use_signal(String::new);
    rsx! {
        div { class: "p-6 space-y-6 overflow-y-auto h-full scrollbar-thin",
            div { class: "space-y-3",
                h3 { class: "text-zinc-500 text-[10px] font-black uppercase tracking-widest", "Match Control" }
                div { class: "flex gap-3",
                    button { class: "flex-1 bg-zinc-900 border border-zinc-800 p-4 rounded-xl hover:border-indigo-500 text-left transition-all group", onclick: move |_| { spawn(async move { send_command_task(addr, "mp_restartgame 1".into(), logs).await; }); }, p { class: "text-zinc-600 text-[9px] uppercase font-bold", "Immediate" } p { class: "text-zinc-200 font-bold", "Restart Round" } }
                    button { class: "flex-1 bg-zinc-900 border border-zinc-800 p-4 rounded-xl hover:border-red-500 text-left transition-all group", onclick: move |_| { spawn(async move { send_command_task(addr, "mp_pause_match".into(), logs).await; }); }, p { class: "text-zinc-600 text-[9px] uppercase font-bold", "Pause" } p { class: "text-zinc-200 font-bold", "Pause Game" } }
                    button { class: "flex-1 bg-zinc-900 border border-zinc-800 p-4 rounded-xl hover:border-emerald-500 text-left transition-all group", onclick: move |_| { spawn(async move { send_command_task(addr, "mp_unpause_match".into(), logs).await; }); }, p { class: "text-zinc-600 text-[9px] uppercase font-bold", "Resume" } p { class: "text-zinc-200 font-bold", "Unpause Game" } }
                }
            }
            div { class: "space-y-3",
                h3 { class: "text-zinc-500 text-[10px] font-black uppercase tracking-widest", "Configuration" }
                div { class: "bg-zinc-900 border border-zinc-800 p-4 rounded-xl flex items-end gap-4",
                    div { class: "flex-1", label { class: "block text-zinc-600 text-[9px] mb-1 font-bold", "CONFIG FILENAME" } input { class: "w-full bg-black border border-zinc-700 rounded p-2 text-white outline-none focus:border-indigo-500", value: "{cfg_name}", oninput: move |e| cfg_name.set(e.value()) } }
                    button { class: "bg-zinc-100 hover:bg-white text-black font-bold px-4 py-2 rounded-lg text-xs h-[34px]", onclick: move |_| { let cmd = format!("exec {}", cfg_name()); spawn(async move { send_command_task(addr, cmd, logs).await; }); }, "EXECUTE" }
                }
            }
            div { class: "space-y-3",
                h3 { class: "text-zinc-500 text-[10px] font-black uppercase tracking-widest", "Communication" }
                div { class: "bg-zinc-900 border border-zinc-800 p-4 rounded-xl flex items-end gap-4",
                    div { class: "flex-1", label { class: "block text-zinc-600 text-[9px] mb-1 font-bold", "SAY MESSAGE" } input { class: "w-full bg-black border border-zinc-700 rounded p-2 text-white outline-none focus:border-indigo-500", value: "{msg}", oninput: move |e| msg.set(e.value()) } }
                    button { class: "bg-emerald-600 hover:bg-emerald-500 text-white font-bold px-4 py-2 rounded-lg text-xs h-[34px]", onclick: move |_| { let cmd = format!("say {}", msg()); msg.set(String::new()); spawn(async move { send_command_task(addr, cmd, logs).await; }); }, "SEND" }
                }
            }
        }
    }
}

// --- HELPERS ---

fn parse_players_from_status(logs: &[String]) -> Vec<Player> {
    let re = Regex::new(
        r"(?m)^\s*(\d+)\s+([\d:]+|BOT)\s+(\d+)\s+(\d+)\s+(\w+)\s+(\d+)\s+([\d.:]+|none)\s+'(.*)'",
    )
    .unwrap();
    for log in logs.iter().rev() {
        if log.contains("---------players--------") {
            let mut players = Vec::new();
            for cap in re.captures_iter(log) {
                players.push(Player {
                    id: cap[1].to_string(),
                    is_bot: &cap[2] == "BOT",
                    ping: cap[3].to_string(),
                    name: cap[8].to_string(),
                });
            }
            if !players.is_empty() {
                return players;
            }
        }
    }
    Vec::new()
}

fn get_maps_for_game(game: Option<String>) -> Vec<&'static str> {
    let g = game.unwrap_or_default().to_lowercase();
    if g.contains("cs2") {
        vec![
            "de_dust2",
            "de_mirage",
            "de_inferno",
            "de_nuke",
            "de_overpass",
            "de_ancient",
            "de_anubis",
        ]
    } else if g.contains("dod") {
        vec!["dod_donner", "dod_avalanche", "dod_flash", "dod_anzio"]
    } else {
        vec!["crossfire", "bounce", "data_core", "undertow"]
    }
}
