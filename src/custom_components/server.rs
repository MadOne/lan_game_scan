use crate::app::Route;
use crate::misc::{connect_to_server, save_to_disk};

use crate::{state::AppState, GameServer, TableMode};
use cbz_rcon::RconStatus;
use dioxus::prelude::*;
use std::net::{IpAddr, SocketAddr};
use std::time::SystemTime;

// ============================================================
// SERVER TABLE
// ============================================================

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
        div {
            class: "flex-1 min-h-0 overflow-auto border border-zinc-800 rounded-xl bg-zinc-900/30 shadow-inner scrollbar-thin",

            table {
                class: "w-full text-left border-collapse table-fixed min-w-[320px]",

                thead {
                    class: "sticky top-0 bg-zinc-900 text-zinc-500 text-[10px] uppercase tracking-widest z-10",

                    tr {
                        // Action: Keep
                        th { class: "p-4 text-center w-16 md:w-20", "Action" }

                        // 🔒: Hidden on mobile
                        th { class: "hidden md:table-cell p-4 text-center w-10", "🔒" }

                        // Address: Hidden on mobile
                        th { class: "hidden md:table-cell p-4 w-40", "Address" }

                        // Game: Hidden on mobile/tablet
                        th { class: "hidden lg:table-cell p-4 w-28", "Game" }

                        // Server Name: Flexible
                        th { class: "p-4 w-auto", "Server Name" }

                        // Map: Hidden on small phones
                        th { class: "hidden sm:table-cell p-4 w-28 md:w-36", "Map" }

                        // Players: Keep
                        th { class: "p-4 text-center w-16 md:w-24", "Players" }

                        // Ping: Hidden on mobile
                        th { class: "hidden md:table-cell p-4 text-right w-20", "Ping" }
                    }
                }

                tbody {
                    class: "divide-y divide-zinc-800/50",

                    for srv in items {
                        if (now - srv.last_update.unwrap_or(0)) < 15
                            || mode == TableMode::Fav
                        {
                            ServerRow {
                                key: "{srv.socket_addr}",
                                srv: srv,
                                mode: mode,
                                selection: selection
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// SERVER ROW
// ============================================================

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
        "OFF".to_string()
    };
    let hostname_color_cls = if is_online {
        "text-zinc-200"
    } else {
        "text-zinc-600"
    };

    rsx! {
        tr {
            class: "cursor-pointer transition-colors {row_cls}",
            onclick: move |_| { selection.set(Some(addr)); },

            // ACTION
            td { class: "p-4 text-center",
                div {
                    class: "text-base transition-transform active:scale-125 inline-block",
                    onclick: move |e| {
                        e.stop_propagation();
                        state.servers.with_mut(|m| {
                            if let Some(s) = m.get_mut(&addr) {
                                s.is_favorite = !s.is_favorite;
                                save_to_disk(m);
                            }
                        });
                    },
                    if is_fav {
                        span { class: "text-yellow-500", "★" }
                    } else {
                        span { class: "text-zinc-800 hover:text-yellow-500/40", "☆" }
                    }
                }
            }

            // PASSWORD (Hidden on mobile)
            td { class: "hidden md:table-cell p-4 text-center",
                if srv.has_password {
                    span { class: "text-zinc-400 text-sm", "🔒" }
                } else {
                    span { class: "text-zinc-800 text-sm", "·" }
                }
            }

            // ADDRESS (Hidden on mobile)
            td { class: "hidden md:table-cell p-4 font-mono text-xs opacity-70 truncate", "{addr}" }

            // GAME (Hidden on mobile/tablet)
            td { class: "hidden lg:table-cell p-4 truncate",
                span {
                    class: "bg-zinc-800 text-zinc-400 px-2 py-0.5 rounded text-[9px] font-bold uppercase",
                    "{srv.game.clone().unwrap_or_else(|| \"---\".into())}"
                }
            }

            // SERVER NAME (Flexible)
            td { class: "p-4 font-semibold truncate {hostname_color_cls}",
                "{srv.hostname.clone().unwrap_or_default()}"
            }

            // MAP (Hidden on very small phones)
            td { class: "hidden sm:table-cell p-4 text-sm opacity-60 truncate",
                "{srv.map.clone().unwrap_or_default()}"
            }

            // PLAYERS
            td { class: "p-4 text-xs text-center opacity-70",
                "{srv.players.unwrap_or(0)}/{srv.players_max.unwrap_or(0)}"
            }

            // PING (Hidden on mobile)
            td { class: "hidden md:table-cell p-4 text-right font-mono",
                if is_online {
                    span { class: "text-emerald-500 font-bold text-xs", "{ping_txt}" }
                } else {
                    span { class: "text-zinc-700 text-[10px]", "{ping_txt}" }
                }
            }
        }
    }
}

// ============================================================
// LAN
// ============================================================

#[component]
pub fn LAN() -> Element {
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
        div {
            class: "h-full flex flex-col p-4 md:p-6 bg-zinc-950 overflow-hidden",

            // ------------------------------------------------
            // SERVER LIST
            // ------------------------------------------------

            ServerTable {
                mode: TableMode::Lan,
                items: items,
                selection: selection
            }

            // ------------------------------------------------
            // SERVER DETAILS
            // ------------------------------------------------

            if let Some(srv) = selected {
                div {
                    class: "shrink-0 mt-4",

                    ServerDetails {
                        key: "{srv.socket_addr}",
                        srv: srv
                    }
                }
            }
        }
    }
}

// ============================================================
// FAVOURITES
// ============================================================

#[component]
pub fn Favourites() -> Element {
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
        div {
            class: "h-full flex flex-col p-4 md:p-6 bg-zinc-950 overflow-hidden",

            // ------------------------------------------------
            // SERVER LIST
            // ------------------------------------------------

            ServerTable {
                mode: TableMode::Fav,
                items: items,
                selection: selection
            }

            // ------------------------------------------------
            // ADD SERVER
            // ------------------------------------------------

            div {
                class: "shrink-0 flex justify-end pt-2",

                button {
                    class: "bg-indigo-600 hover:bg-indigo-500 text-white text-[10px] font-bold px-4 py-2 rounded-lg",

                    onclick: move |_| {
                        show_form.set(!show_form());
                    },

                    if show_form() {
                        "CANCEL"
                    } else {
                        "ADD SERVER"
                    }
                }
            }

            // ------------------------------------------------
            // ADD SERVER FORM
            // ------------------------------------------------

            if show_form() {
                div {
                    class: "shrink-0 mt-2",

                    AddServerForm {
                        on_close: move |_| show_form.set(false)
                    }
                }
            }

            // ------------------------------------------------
            // SERVER DETAILS
            // ------------------------------------------------

            if let Some(srv) = selected {
                div {
                    class: "shrink-0 mt-4",

                    ServerDetails {
                        key: "{srv.socket_addr}",
                        srv: srv
                    }
                }
            }
        }
    }
}

// ============================================================
// ADD SERVER
// ============================================================

#[component]
fn AddServerForm(on_close: EventHandler<()>) -> Element {
    let mut state = use_context::<AppState>();

    let mut input_v = use_signal(String::new);

    rsx! {
        div {
            class: "bg-zinc-900 border border-zinc-800 p-4 rounded-xl flex flex-wrap gap-4 animate-in fade-in zoom-in-95",

            input {
                class: "flex-1 min-w-[180px] bg-zinc-950 border border-zinc-700 rounded-lg p-2 text-sm text-white outline-none focus:border-indigo-500",

                placeholder: "IP:PORT",

                value: "{input_v}",

                oninput: move |e| {
                    input_v.set(e.value());
                }
            }

            button {
                class: "bg-zinc-100 hover:bg-white text-black font-bold px-6 py-2 rounded-lg text-sm h-[38px]",

                onclick: move |_| {
                    if let Ok(addr) = input_v().parse::<SocketAddr>() {
                        let now = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64;

                        state.servers.with_mut(|m| {
                            m.insert(
                                addr,
                                GameServer {
                                    socket_addr: addr,
                                    hostname: Some("Custom Server".into()),
                                    game: None,
                                    map: None,
                                    players: None,
                                    players_max: None,
                                    query_port: Some(addr.port()),
                                    rcon: None,
                                    ping: None,
                                    last_update: Some(now),
                                    is_favorite: true,
                                    bots: None,
                                    has_password: false,
                                    password: None,
                                },
                            );

                            save_to_disk(m);
                        });

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

// ============================================================
// SERVER DETAILS
// ============================================================

#[component]
fn ServerDetails(srv: GameServer) -> Element {
    let nav = use_navigator();
    let mut state = use_context::<AppState>();

    let is_online = srv.ping.is_some();
    let addr = srv.socket_addr;

    // ------------------------------------------------------------
    // LOCAL PASSWORD EDITING STATE
    // ------------------------------------------------------------

    let mut server_password = use_signal(|| srv.password.clone().unwrap_or_default());

    let mut rcon_password = use_signal(|| srv.rcon.clone().unwrap_or_default());

    // ------------------------------------------------------------
    // KEEP LOCAL PASSWORDS IN SYNC WITH SELECTED SERVER
    // ------------------------------------------------------------

    use_effect({
        let password = srv.password.clone().unwrap_or_default();

        move || {
            server_password.set(password.clone());
        }
    });

    use_effect({
        let password = srv.rcon.clone().unwrap_or_default();

        move || {
            rcon_password.set(password.clone());
        }
    });

    // ------------------------------------------------------------
    // RCON CONNECT CALLBACK
    // ------------------------------------------------------------

    let connect_rcon = use_context::<Callback<(SocketAddr, String)>>();

    // ------------------------------------------------------------
    // CURRENT RCON SESSION
    // ------------------------------------------------------------

    let status = state
        .rcon_sessions
        .read()
        .get(&addr)
        .map(|session| *session.status.read())
        .unwrap_or(RconStatus::Disconnected);

    let is_authenticated = status == RconStatus::Authenticated;

    let is_connecting = status == RconStatus::Connecting;

    // ------------------------------------------------------------
    // SERVER INFORMATION
    // ------------------------------------------------------------

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

    // ------------------------------------------------------------
    // SAVED PASSWORD STATE
    // ------------------------------------------------------------

    let saved_password = srv.password.clone().unwrap_or_default();

    let password_changed = server_password() != saved_password;

    let saved_rcon_password = srv.rcon.clone().unwrap_or_default();

    let rcon_password_changed = rcon_password() != saved_rcon_password;

    rsx! {
            div {
                class: "bg-zinc-900 border border-zinc-800 rounded-xl p-4 md:p-6 shadow-2xl min-h-36 flex flex-col justify-center animate-in slide-in-from-bottom-4 duration-500",

                // ========================================================
                // HEADER
                // ========================================================

                div {
                    class: "flex justify-between items-center",

                    div {
                        h2 {
                            class: "text-xl font-black text-white tracking-tighter uppercase truncate max-w-md",
                            "{srv.hostname.clone().unwrap_or_default()}"
                        }

                        div {
                            class: "flex items-center gap-2 mt-0.5 text-zinc-500 font-mono text-xs",

                            p {
                                "{addr}"
                            }

                            if srv.has_password {
                                span {
                                    class: "text-zinc-400",
                                    title: "Password protected",
                                    "🔒"
                                }
                            }

                            if !is_online {
                                span {
                                    class: "text-[9px] bg-red-900/20 text-red-500 px-1.5 py-0.5 rounded font-bold border border-red-900/30 uppercase",
                                    "Offline"
                                }
                            }
                        }
                    }
                }

                // ========================================================
                // PASSWORDS
                // ========================================================

                div {
                    class: "mt-4 pt-4 border-t border-zinc-800 flex flex-wrap gap-x-8 gap-y-4",

                    // ====================================================
                    // SERVER PASSWORD
                    // ====================================================

                    div {
                        class: "hidden md:block flex-1 min-w-[300px]",

                        div {
                            class: "flex flex-wrap items-center gap-3",

                            span {
                                class: "text-[10px] text-zinc-500 font-black uppercase tracking-widest whitespace-nowrap",
                                "SERVER PASSWORD"
                            }

                            input {
                                r#type: "password",

                                class: "flex-1 min-w-[140px] max-w-xs bg-zinc-950 border border-zinc-700 rounded-lg px-3 py-2 text-xs text-white outline-none focus:border-indigo-500",

                                placeholder: if srv.has_password {
                                    "Enter server password"
                                } else {
                                    "No password"
                                },

                                value: "{server_password}",

                                oninput: move |event| {
                                    server_password.set(event.value());
                                }
                            }

                            if password_changed {
                                button {
                                    class: "bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2 rounded-lg text-[10px] font-black uppercase",

                                    onclick: move |_| {
                                        let value = server_password();

                                        state.servers.with_mut(|servers| {
                                            if let Some(server) =
                                                servers.get_mut(&addr)
                                            {
                                                server.password =
                                                    if value.is_empty() {
                                                        None
                                                    } else {
                                                        Some(value.clone())
                                                    };

                                                save_to_disk(servers);
                                            }
                                        });
                                    },

                                    "SAVE"
                                }
                            } else if !server_password().is_empty() {
                                span {
                                    class: "text-[10px] text-emerald-500 font-bold uppercase whitespace-nowrap",
                                    "SAVED"
                                }
                            } else {
                                span {
                                    class: "text-[10px] text-zinc-700 font-bold uppercase whitespace-nowrap",
                                    "NOT SET"
                                }
                            }

                            if !server_password().is_empty() {
                                button {
                                    class: "text-[10px] text-zinc-500 hover:text-red-400 font-bold uppercase whitespace-nowrap",

                                    onclick: move |_| {
                                        server_password.set(String::new());
                                    },

                                    "CLEAR"
                                }
                            }

                            // ------------------------------------------------
                            // JOIN
                            // ------------------------------------------------

                            button {
                                class: "bg-indigo-600 hover:bg-indigo-500 text-white px-4 py-2 rounded-lg text-[10px] font-black uppercase",

                                onclick: {
                                    let addr = addr;

                                    move |_| {
                                        let password = server_password();

                                        spawn(async move {
                                            connect_to_server(
                                                addr.to_string(),
                                                password,
                                            )
                                            .await;
                                        });
                                    }
                                },

                                "JOIN"
                            }
                        }
                    }

                    // ====================================================
                    // RCON PASSWORD / CONNECTION
                    // ====================================================

                    div {
                        class: "flex-1 min-w-[300px]",

                        div {
                            class: "flex flex-wrap items-center gap-3",

                            if !is_authenticated {
                                span {
                                    class: "text-[10px] text-zinc-500 font-black uppercase tracking-widest whitespace-nowrap",
                                    "RCON PASSWORD"
                                }

                                input {
                                    r#type: "password",

                                    class: "flex-1 min-w-[140px] max-w-xs bg-zinc-950 border border-zinc-700 rounded-lg px-3 py-2 text-xs text-white outline-none focus:border-indigo-500",

                                    placeholder: "RCON password",

                                    value: "{rcon_password}",

                                    disabled: is_connecting,

                                    oninput: move |event| {
                                        rcon_password.set(event.value());
                                    }
                                }

                                if rcon_password_changed {
                                    button {
                                        class: "bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2 rounded-lg text-[10px] font-black uppercase",

                                        disabled: is_connecting,

                                        onclick: move |_| {
                                            let value = rcon_password();

                                            state.servers.with_mut(|servers| {
                                                if let Some(server) =
                                                    servers.get_mut(&addr)
                                                {
                                                    server.rcon =
                                                        if value.is_empty() {
                                                            None
                                                        } else {
                                                            Some(value.clone())
                                                        };

                                                    save_to_disk(servers);
                                                }
                                            });
                                        },

                                        "SAVE"
                                    }
                                } else if !rcon_password().is_empty() {
                                    span {
                                        class: "text-[10px] text-emerald-500 font-bold uppercase whitespace-nowrap",
                                        "SAVED"
                                    }
                                } else {
                                    span {
                                        class: "text-[10px] text-zinc-700 font-bold uppercase whitespace-nowrap",
                                        "NOT SET"
                                    }
                                }

                                if !rcon_password().is_empty() {
                                    button {
                                        class: "text-[10px] text-zinc-500 hover:text-red-400 font-bold uppercase whitespace-nowrap",

                                        disabled: is_connecting,

                                        onclick: move |_| {
                                            rcon_password.set(String::new());
                                        },

                                        "CLEAR"
                                    }
                                }

                                // ------------------------------------------------
                                // LOGIN
                                // ------------------------------------------------

                                button {
                                    class: if is_connecting {
                                        "bg-zinc-700 text-zinc-500 px-4 py-2 rounded-lg text-[10px] font-black cursor-not-allowed"
                                    } else {
                                        "bg-indigo-600 hover:bg-indigo-500 text-white px-4 py-2 rounded-lg text-[10px] font-black"
                                    },

                                    disabled: is_connecting,

                                    onclick: {
                                        let addr = addr;
                                        let connect_rcon = connect_rcon;

                                        move |_| {
                                            if is_connecting {
                                                return;
                                            }

                                            let password = rcon_password();

                                            if password.is_empty() {
                                                return;
                                            }

                                            state.servers.with_mut(|servers| {
                                                if let Some(server) =
                                                    servers.get_mut(&addr)
                                                {
                                                    server.rcon =
                                                        Some(password.clone());

                                                    save_to_disk(servers);
                                                }
                                            });

                                            connect_rcon.call((addr, password));
                                        }
                                    },

                                    if is_connecting {
                                        "CONNECTING..."
                                    } else {
                                        "LOGIN"
                                    }
                                }
                            } else {
                                // ------------------------------------------------
                                // AUTHENTICATED
                                // ------------------------------------------------

                                button {
                                    class: "bg-emerald-700 hover:bg-emerald-600 text-white px-5 py-2 rounded-lg text-[10px] font-black uppercase flex items-center gap-2",

                                    onclick: move |_| {
                                        nav.push(Route::RconTab {});
                                    },

                                    span {
                                        "●"
                                    }

                                    "GO TO RCON"
                                }
                            }
                        }
                    }
                }

                // ========================================================
                // RCON STATUS
                // ========================================================

                if !is_authenticated {
                    match status {
                        RconStatus::Connecting => rsx! {
                            div {
                                class: "mt-3 text-[10px] text-blue-400 font-black uppercase tracking-widest animate-pulse",
                                "● RCON CONNECTING..."
                            }
                        },

                        RconStatus::Error => rsx! {
                            div {
                                class: "mt-3 text-[10px] text-red-400 font-black uppercase tracking-widest",
                                "● RCON AUTHENTICATION FAILED"
                            }
                        },

                        _ => rsx! {}
                    }
                }

                // ========================================================
                // SERVER DETAILS
                // ========================================================

                // ========================================================
    // SERVER DETAILS
    // ========================================================

    div {
        class: "hidden md:grid md:grid-cols-4 gap-6 mt-4",

        DetailBox {
            label: "Map".to_string(),
            value: srv.map.clone().unwrap_or_default()
        }

        DetailBox {
            label: "Engine".to_string(),
            value: "Source / GoldSrc".to_string()
        }

        DetailBox {
            label: "Status".to_string(),
            value: status_val
        }

        DetailBox {
            label: "Players".to_string(),
            value: player_val
        }
    }
            }
        }
}

// ============================================================
// DETAIL BOX
// ============================================================

#[component]
fn DetailBox(label: String, value: String) -> Element {
    rsx! {
        div {
            p {
                class: "text-[10px] uppercase tracking-widest text-zinc-600 font-black mb-1",
                "{label}"
            }

            p {
                class: "text-zinc-300 font-medium text-xs",
                "{value}"
            }
        }
    }
}
