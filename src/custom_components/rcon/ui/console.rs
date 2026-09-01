use crate::custom_components::cvar::CvarDatabase;
use crate::{
    custom_components::rcon::{
        code::RconLogEvent,
        ui::create_config::CreateConfig,
        ui::{
            console_filters::ConsoleFilters, console_input::RconCommandInput,
            console_logs::RconLogOutput, overview::RconOverview, RconSubTab,
        },
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

    let visible_events = use_signal(|| LogType::all().collect::<HashSet<LogType>>());

    let filter_popup_open = use_signal(|| false);

    // -------------------------------------------------------------------------
    // Command history
    //
    // This belongs to RconConsole so it survives switching between tabs.
    // -------------------------------------------------------------------------

    let command_history = use_signal(Vec::<String>::new);

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
    let cvar_db = session.cvar_db;

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

    let mut pw_input = use_signal(String::new);

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

                    // -----------------------------------------------------------------
                    // OVERVIEW
                    // -----------------------------------------------------------------

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

                    // -----------------------------------------------------------------
                    // TERMINAL
                    // -----------------------------------------------------------------

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

                    // -----------------------------------------------------------------
                    // CREATE CONFIG
                    // -----------------------------------------------------------------

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

                        rsx! {
                            div {
                                class: "flex flex-col h-full min-h-0",

                                // -------------------------------------------------
                                // FILTER BAR
                                // -------------------------------------------------

                                ConsoleFilters {
                                    visible_events,
                                    filter_popup_open,
                                }

                                // -------------------------------------------------
                                // LOG OUTPUT
                                // -------------------------------------------------

                                RconLogOutput {
                                    logs,
                                    selected_events,
                                }

                                // -------------------------------------------------
                                // COMMAND INPUT
                                // -------------------------------------------------

                                RconCommandInput {
                                    cvar_db,
                                    command_history,

                                    on_command: move |command: String| {
                                        let client = client.clone();
                                        let mut logs = logs;

                                        spawn(async move {
                                            let mut client = client.lock().await;

                                            match client.command(&command).await {
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
                                }
                            }
                        }
                    }

                    // =========================================================
                    // CREATE CONFIG
                    // =========================================================

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
