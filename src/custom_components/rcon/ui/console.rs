use crate::custom_components::code::StateUpdate;
use crate::custom_components::cvar::CvarFlag;
use crate::custom_components::ui::cvar_filters::CvarFilters;
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
    let mut state = use_context::<AppState>();

    let mut inner_tab = use_signal(|| RconSubTab::Overview);

    let rcon_state = match state.rcon_manager.state(&addr) {
        Some(state) => state,
        None => {
            return rsx! {
                div {
                    "Disconnected."
                }
            };
        }
    };
    let client = match state
        .rcon_manager
        .with_session(&addr, |session| session.client.clone())
    {
        Some(client) => client,
        None => {
            return rsx! {
                div {
                    "Disconnected."
                }
            };
        }
    };
    // -------------------------------------------------------------------------
    // Session state
    // -------------------------------------------------------------------------

    let cvar_filter_popup_open = use_signal(|| false);

    let visible_events = use_signal(|| LogType::all().collect::<HashSet<LogType>>());

    let filter_popup_open = use_signal(|| false);

    let cvar_filters = use_signal(|| {
        HashSet::from([
            CvarFlag::MenuBarItem,
            CvarFlag::VConsoleFuzzy,
            CvarFlag::VConsoleSetFocus,
            CvarFlag::DevelopmentOnly,
        ])
    });
    let status = rcon_state.status().read().cloned();
    /*
    let logs = rcon_state.logs;

    let cvar_db = rcon_state.cvar_db;
    let command_history = rcon_state.command_history;
    */

    // -------------------------------------------------------------------------
    // Server information
    // -------------------------------------------------------------------------

    let server = state.servers.read().get(&addr).cloned();

    let Some(server) = server.as_ref() else {
        tracing::error!("RCON server {} no longer exists in server list", addr);

        return rsx! {
            div {
                class: "h-full flex items-center justify-center bg-zinc-950 text-zinc-600",
                "Server no longer available."
            }
        };
    };

    let hostname = server
        .scanned
        .hostname
        .clone()
        .unwrap_or_else(|| "UNKNOWN SERVER".to_string());

    let map = server
        .scanned
        .map
        .clone()
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let player_count = server.scanned.players.unwrap_or(0);

    let player_max = server.scanned.players_max.unwrap_or(0);

    let protocol = server.scanned.protocol;

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

                                if password.is_empty() {
                                    return;
                                }

                                spawn(async move {
                                    state
                                        .rcon_manager
                                        .connect(addr, password, protocol)
                                        .await;
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
                            rcon_state,
                            player_count,
                            player_max,

                            get_maps: move |_| {
                                state.rcon_manager.with_session(&addr, |session| {
                                    session.get_maps();
                                });
                            },

                            on_command: move |command: String| {
                                let client = client.clone();
                                let sender = rcon_state.sender.read().clone();

                                spawn(async move {
                                    let mut client = client.lock().await;

                                    let update = match client.command(&command).await {
                                        Ok(response) => {
                                            StateUpdate::Log(RconLogEvent::RconResponse(response))
                                        }
                                        Err(error) => {
                                            StateUpdate::Log(RconLogEvent::Info(
                                                format!("[RCON] Command failed: {}", error),
                                            ))
                                        }
                                    };

                                    if sender.send(update).await.is_err() {
                                        tracing::error!("Failed to send RCON command result to state");
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

                                div {
                                    class: "shrink-0 flex items-center gap-2 px-3 py-2 bg-zinc-900 border-b border-zinc-800",

                                    ConsoleFilters {
                                        visible_events,
                                        filter_popup_open,
                                    }

                                    CvarFilters {
                                        filters: cvar_filters,
                                        popup_open: cvar_filter_popup_open,
                                    }
                                }

                                // -------------------------------------------------
                                // LOG OUTPUT
                                // -------------------------------------------------

                                RconLogOutput {
                                    //logs,
                                    rcon_state,
                                    selected_events,
                                }

                                // -------------------------------------------------
                                // COMMAND INPUT
                                // -------------------------------------------------

                                RconCommandInput {
                                    rcon_state,
                                    //cvar_db,
                                    cvar_filters,
                                    //command_history,

                                on_command: move |command: String| {
                                    let client = client.clone();
                                    let sender = rcon_state.sender.read().clone();

                                    spawn(async move {
                                        let mut client = client.lock().await;

                                        tracing::debug!(">>> SEND: {}", command);

                                        match client.command(&command).await {
                                            Ok(response) => {
                                                tracing::debug!(
                                                    "<<< RESPONSE FOR '{}': {:?}",
                                                    command,
                                                    response
                                                );

                                                if sender
                                                    .send(StateUpdate::Log(
                                                        RconLogEvent::RconResponse(response),
                                                    ))
                                                    .await
                                                    .is_err()
                                                {
                                                    tracing::error!("Failed to send RCON response to state");
                                                }
                                            }

                                            Err(error) => {
                                                if sender
                                                    .send(StateUpdate::Log(
                                                        RconLogEvent::Info(format!(
                                                            "[RCON] Command failed: {}",
                                                            error
                                                        )),
                                                    ))
                                                    .await
                                                    .is_err()
                                                {
                                                    tracing::error!("Failed to send RCON error to state");
                                                }
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
                            rcon_state,
                            get_maps: move |_| {
                                state.rcon_manager.with_session(&addr, |session| {
                                    session.get_maps();
                                });
                            },

                            on_command: move |command: String| {
                                let client = client.clone();
                                let sender = rcon_state.sender.read().clone();

                                spawn(async move {
                                    let mut client = client.lock().await;

                                    let update = match client.command(&command).await {
                                        Ok(response) => {
                                            StateUpdate::Log(RconLogEvent::RconResponse(response))
                                        }
                                        Err(error) => {
                                            StateUpdate::Log(RconLogEvent::Info(
                                                format!("[RCON] Command failed: {}", error),
                                            ))
                                        }
                                    };

                                    if sender.send(update).await.is_err() {
                                        tracing::error!("Failed to send RCON command result to state");
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
