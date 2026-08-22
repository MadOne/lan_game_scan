use crate::{
    custom_components::{
        rcon::{
            code::RconLogEvent,
            ui::{overview::RconOverview, RconSubTab},
        },
        ui::{create_config::CreateConfig, pretty_log},
    },
    state::AppState,
};
use cbz_rcon::RconStatus;
use dioxus::prelude::*;
use live_log::parser::LogType;
use std::{collections::HashSet, net::SocketAddr};

#[component]
pub fn RconConsole(addr: SocketAddr) -> Element {
    let state = use_context::<AppState>();

    let mut inner_tab = use_signal(|| RconSubTab::Overview);

    let mut visible_events = use_signal(|| LogType::all().collect::<HashSet<LogType>>());

    let sessions = state.rcon_sessions.read();

    let session = match sessions.get(&addr) {
        Some(session) => session,
        None => {
            return rsx! {
                div {
                    class: "h-full flex items-center justify-center bg-zinc-950 text-zinc-600",
                    "Disconnected."
                }
            };
        }
    };

    // -------------------------------------------------------------------------
    // Session state
    // -------------------------------------------------------------------------

    let logs = session.logs;
    let status_signal = session.status;
    let players = session.players;
    let score = session.score;
    let paused = session.match_paused;
    let client = session.client.clone();
    let maps = session.maps;

    // -------------------------------------------------------------------------
    // Server information
    // -------------------------------------------------------------------------

    let server = state.servers.read().get(&addr).cloned();

    let hostname = server
        .as_ref()
        .and_then(|server| server.hostname.clone())
        .unwrap_or_else(|| "UNKNOWN SERVER".to_string());

    let map = server
        .as_ref()
        .and_then(|server| server.map.clone())
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let player_count = server
        .as_ref()
        .and_then(|server| server.players)
        .unwrap_or(0);

    let player_max = server
        .as_ref()
        .and_then(|server| server.players_max)
        .unwrap_or(0);

    drop(sessions);

    let status = status_signal();

    // These are Signals<String>, not Option<String>.
    let mut pw_input = use_signal(String::new);
    let mut cmd_input = use_signal(String::new);

    let base_sub =
        "px-4 py-2 text-[10px] font-black tracking-widest cursor-pointer transition-all border-b-2";

    rsx! {
                div {
                    class: "flex flex-col h-full min-h-0 bg-black font-mono text-xs",

                    // =================================================================
                    // LOGIN / CONNECTION HEADER
                    // =================================================================

                    if status != RconStatus::Authenticated {
                        div {
                            class: "shrink-0 p-3 bg-zinc-900 border-b border-zinc-800 flex justify-between items-center",

                            div {
                                class: "flex items-center gap-4 text-zinc-500",

                                span {
                                    "ADDR: {addr}"
                                }

                                match status {
                                    RconStatus::Disconnected => rsx! {
                                        span {
                                            class: "text-yellow-600",
                                            "● OFFLINE"
                                        }
                                    },

                                    RconStatus::Connecting => rsx! {
                                        span {
                                            class: "text-blue-500 animate-pulse",
                                            "● CONNECTING..."
                                        }
                                    },

                                    RconStatus::Authenticated => rsx! {
                                        span {
                                            class: "text-emerald-500",
                                            "● RUNNING"
                                        }
                                    },

                                    RconStatus::Error => rsx! {
                                        span {
                                            class: "text-red-500",
                                            "● AUTH FAILED"
                                        }
                                    },
                                }
                            }

                            div {
                                class: "flex gap-2",

                                input {
                                    r#type: "password",
                                    class: "bg-black border border-zinc-700 px-2 py-1 rounded text-white w-32 outline-none focus:border-indigo-500",
                                    placeholder: "Password",
                                    value: "{pw_input}",

                                    oninput: move |event| {
                                        pw_input.set(event.value());
                                    }
                                }

                                button {
                                    class: "bg-indigo-600 text-white px-3 py-1 rounded text-[10px] font-bold",

                                    onclick: move |_| {
                                        let password = pw_input();
                                        let mut state = state;

                                        spawn(async move {
                                            let session =
                                                crate::custom_components::rcon::code::RconSession::connect(
                                                    addr,
                                                    password,
                                                )
                                                .await;

                                            if let Some(session) = session {
                                                state.rcon_sessions.with_mut(|sessions| {
                                                    sessions.insert(addr, session);
                                                });
                                            }
                                        });
                                    },

                                    "LOGIN"
                                }
                            }
                        }
                    }

                    // =================================================================
                    // SUB TABS
                    // =================================================================

                    if status == RconStatus::Authenticated {
                        div {
                            class: "shrink-0 flex bg-zinc-900/80 border-b border-zinc-800 px-4",

                            div {
                                class: format!(
                                    "{} {}",
                                    base_sub,
                                    if inner_tab() == RconSubTab::Overview {
                                        "text-indigo-400 border-indigo-500 bg-zinc-900"
                                    } else {
                                        "text-zinc-600 border-transparent hover:text-zinc-400"
                                    }
                                ),

                                onclick: move |_| {
                                    inner_tab.set(RconSubTab::Overview);
                                },

                                "OVERVIEW"
                            }

                            div {
                                class: format!(
                                    "{} {}",
                                    base_sub,
                                    if inner_tab() == RconSubTab::Terminal {
                                        "text-indigo-400 border-indigo-500 bg-zinc-900"
                                    } else {
                                        "text-zinc-600 border-transparent hover:text-zinc-400"
                                    }
                                ),

                                onclick: move |_| {
                                    inner_tab.set(RconSubTab::Terminal);
                                },

                                "TERMINAL"
                            }
                            div {
                                class: format!(
                                    "{} {}",
                                    base_sub,
                                    if inner_tab() == RconSubTab::CreateConfig {
                                        "text-indigo-400 border-indigo-500 bg-zinc-900"
                                    } else {
                                        "text-zinc-600 border-transparent hover:text-zinc-400"
                                    }
                                ),

                                onclick: move |_| {
                                    inner_tab.set(RconSubTab::CreateConfig);
                                },

                                "CREATE CONFIG"
                            }

                        }
                    }

                    // =================================================================
                    // CONTENT
                    // =================================================================

                    div {
                        class: "flex-1 min-h-0 overflow-hidden",

                        match inner_tab() {

                            // =========================================================
                            // OVERVIEW
                            // =========================================================

                            RconSubTab::Overview => rsx! {
                                RconOverview {
                                    addr,
                                    hostname,
                                    map,
                                    status: status_signal,
                                    score,
                                    player_count,
                                    player_max,
                                    logs,
                                    players,
                                    paused,
                                    maps,

                                    get_maps: move |_| {
            let sessions = state.rcon_sessions.read();

            if let Some(session) = sessions.get(&addr) {
                session.get_maps();
            }
        },

                                    on_command: move |command: String| {
                                        let client = client.clone();
                                        let mut logs = logs;

                                        spawn(async move {
                                            let mut client = client.lock().await;

                                            match client.command(&command).await {
                                                Ok(response) => {
                                                    logs.write().push(
                                                        RconLogEvent::RconResponse(response)
                                                    );
                                                }

                                                Err(error) => {
                                                    logs.write().push(
                                                        RconLogEvent::Info(
                                                            format!(
                                                                "[RCON] Command failed: {}",
                                                                error
                                                            )
                                                        )
                                                    );
                                                }
                                            }
                                        });
                                    },
                                }
                            },

                            // =========================================================
                            // TERMINAL
                            // =========================================================

                            RconSubTab::Terminal => {
                                let selected_events =
                                    visible_events.read().clone();

                                let all_enabled =
                                    LogType::all().all(|event_type| {
                                        selected_events.contains(&event_type)
                                    });

                                let none_enabled =
                                    selected_events.is_empty();

                                let enter_client = client.clone();
                                let send_client = client.clone();

                                rsx! {
                                    div {
                                        class: "flex flex-col h-full min-h-0",

                                        // =================================================
                                        // FILTER BAR
                                        // =================================================

                                        div {
                                            class: "shrink-0 bg-zinc-900/80 border-b border-zinc-800 px-4 py-2",

                                            div {
                                                class: "flex items-center gap-2 flex-wrap",

                                                span {
                                                    class: "text-[9px] font-black tracking-widest text-zinc-600 mr-2",
                                                    "FILTER"
                                                }

                                                button {
                                                    class: if all_enabled {
                                                        "px-2 py-1 rounded text-[9px] font-black bg-indigo-600 text-white border border-indigo-500"
                                                    } else {
                                                        "px-2 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                                                    },

                                                    onclick: move |_| {
                                                        visible_events.with_mut(|set| {
                                                            set.clear();
                                                            set.extend(LogType::all());
                                                        });
                                                    },

                                                    "ALL"
                                                }

                                                button {
                                                    class: if none_enabled {
                                                        "px-2 py-1 rounded text-[9px] font-black bg-indigo-600 text-white border border-indigo-500"
                                                    } else {
                                                        "px-2 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                                                    },

                                                    onclick: move |_| {
                                                        visible_events.with_mut(|set| {
                                                            set.clear();
                                                        });
                                                    },

                                                    "NONE"
                                                }

                                                div {
                                                    class: "w-px h-4 bg-zinc-800 mx-1"
                                                }

                                                for event_type in LogType::all() {
                                                    {
                                                        let enabled =
                                                            selected_events.contains(&event_type);

                                                        let label =
                                                            event_type.label();

                                                        let class = if enabled {
                                                            "px-2 py-1 rounded text-[9px] font-black bg-indigo-950 text-indigo-300 border border-indigo-800 hover:bg-indigo-900 transition-colors"
                                                        } else {
                                                            "px-2 py-1 rounded text-[9px] font-black bg-zinc-950 text-zinc-600 border border-zinc-800 hover:text-zinc-400 hover:border-zinc-700 transition-colors"
                                                        };

                                                        rsx! {
                                                            button {
                                                                key: "{label}",
                                                                class: "{class}",

                                                                onclick: move |_| {
                                                                    visible_events.with_mut(|set| {
                                                                        if set.contains(&event_type) {
                                                                            set.remove(&event_type);
                                                                        } else {
                                                                            set.insert(event_type);
                                                                        }
                                                                    });
                                                                },

                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // =================================================
                                        // LOG OUTPUT
                                        // =================================================

                                        div {
                                            class: "flex-1 min-h-0 overflow-y-auto overflow-x-hidden p-6 space-y-1 scrollbar-thin",

                                            for event in logs.read().iter() {
                                                {
                                                    match event {
                                                        RconLogEvent::RconResponse(text) => rsx! {
                                                            div {
                                                                class: "whitespace-pre-wrap break-words text-zinc-300",
                                                                "{text}"
                                                            }

                                                            div {
                                                                class: "h-2"
                                                            }
                                                        },

                                                        RconLogEvent::Info(text) => rsx! {
                                                            div {
                                                                class: "whitespace-pre-wrap break-words text-zinc-500",
                                                                "{text}"
                                                            }

                                                            div {
                                                                class: "h-2"
                                                            }
                                                        },

                                                        RconLogEvent::LiveLog(parsed) => {
                                                            if selected_events.contains(
                                                                &parsed.log_type
                                                            ) {
                                                                if parsed.log_type
                                                                    == LogType::Unknown
                                                                {
                                                                    rsx! {
                                                                        div {
                                                                            class: "whitespace-pre-wrap break-words text-zinc-300",
                                                                            "{parsed.raw}"
                                                                        }

                                                                        div {
                                                                            class: "h-2"
                                                                        }
                                                                    }
                                                                } else {
                                                                    rsx! {
                                                                        div {
                                                                            class: "whitespace-pre-wrap break-words",
                                                                            {pretty_log(&parsed.event)}
                                                                        }

                                                                        div {
                                                                            class: "h-2"
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                rsx! {}
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // =================================================
                                        // COMMAND INPUT
                                        // =================================================

                                        div {
                                            class: "shrink-0 border-t border-zinc-800 bg-zinc-900 p-3",

                                            div {
                                                class: "flex gap-2",

                                                span {
                                                    class: "text-indigo-500 font-bold py-2",
                                                    ">"
                                                }

                                                input {
                                                    class: "flex-1 bg-black border border-zinc-700 rounded px-3 py-2 text-zinc-200 outline-none focus:border-indigo-500",

                                                    placeholder: "Enter RCON command...",

                                                    value: "{cmd_input}",

                                                    oninput: move |event| {
                                                        cmd_input.set(event.value());
                                                    },

                                                    onkeydown: move |event| {
                                                        if event.key() == Key::Enter {
                                                            let cmd = cmd_input();

                                                            if cmd.trim().is_empty() {
                                                                return;
                                                            }

                                                            cmd_input.set(String::new());

                                                            let client =
                                                                enter_client.clone();

                                                            let mut logs = logs;

                                                            spawn(async move {
                                                                let mut client =
                                                                    client.lock().await;

                                                                match client.command(&cmd).await {
                                                                    Ok(response) => {
                                                                        logs.write().push(
                                                                            RconLogEvent::RconResponse(
                                                                                response,
                                                                            ),
                                                                        );
                                                                    }

                                                                    Err(error) => {
                                                                        logs.write().push(
                                                                            RconLogEvent::Info(
                                                                                format!(
                                                                                    "[RCON] Command failed: {}",
                                                                                    error
                                                                                ),
                                                                            ),
                                                                        );
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    }
                                                }

                                                button {
                                                    class: "bg-indigo-600 hover:bg-indigo-500 text-white px-5 py-2 rounded font-bold",

                                                    onclick: move |_| {
                                                        let cmd = cmd_input();

                                                        if cmd.trim().is_empty() {
                                                            return;
                                                        }

                                                        cmd_input.set(String::new());

                                                        let client =
                                                            send_client.clone();

                                                        let mut logs = logs;

                                                        spawn(async move {
                                                            let mut client =
                                                                client.lock().await;

                                                            match client.command(&cmd).await {
                                                                Ok(response) => {
                                                                    logs.write().push(
                                                                        RconLogEvent::RconResponse(
                                                                            response,
                                                                        ),
                                                                    );
                                                                }

                                                                Err(error) => {
                                                                    logs.write().push(
                                                                        RconLogEvent::Info(
                                                                            format!(
                                                                                "[RCON] Command failed: {}",
                                                                                error
                                                                            ),
                                                                        ),
                                                                    );
                                                                }
                                                            }
                                                        });
                                                    },

                                                    "SEND"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            RconSubTab::CreateConfig => rsx! {
                                CreateConfig {
                                    addr,
                                    hostname,
                                    map,
                                    status: status_signal,
                                    score,
                                    player_count,
                                    player_max,
                                    logs,
                                    players,
                                    paused,
                                    maps,

                                    get_maps: move |_| {
            let sessions = state.rcon_sessions.read();

            if let Some(session) = sessions.get(&addr) {
                session.get_maps();
            }
        },

                                    on_command: move |command: String| {
                                        let client = client.clone();
                                        let mut logs = logs;

                                        spawn(async move {
                                            let mut client = client.lock().await;

                                            match client.command(&command).await {
                                                Ok(response) => {
                                                    logs.write().push(
                                                        RconLogEvent::RconResponse(response)
                                                    );
                                                }

                                                Err(error) => {
                                                    logs.write().push(
                                                        RconLogEvent::Info(
                                                            format!(
                                                                "[RCON] Command failed: {}",
                                                                error
                                                            )
                                                        )
                                                    );
                                                }
                                            }
                                        });
                                    },
                                }
                            },
                    }
                }
            }
    }
}
