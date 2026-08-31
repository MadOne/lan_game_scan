use crate::custom_components::cvar::{Cvar, CvarFlag};
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

    let mut filter_popup_open = use_signal(|| false);

    // -------------------------------------------------------------------------
    // Autocomplete state
    // -------------------------------------------------------------------------

    let mut suggestions = use_signal(Vec::<Cvar>::new);

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
    let cvar_db = session.cvar_db.clone();

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

                        let essential_events = [
                            LogType::Chat,
                            LogType::Connection,
                            LogType::RoundWin,
                            LogType::GameOver,
                        ];

                        let essential_enabled =
                            essential_events
                                .iter()
                                .all(|event_type| {
                                    selected_events.contains(event_type)
                                })
                            && selected_events.len() == essential_events.len();

                        let customize_enabled =
                            !all_enabled
                                && !none_enabled
                                && !essential_enabled;

                        let enter_client = client.clone();
                        let send_client = client.clone();

                        rsx! {
                            div {
                                class: "flex flex-col h-full min-h-0",

                                // =================================================
                                // FILTER BAR
                                // =================================================

                                div {
                                    class: "relative shrink-0 bg-zinc-900/80 border-b border-zinc-800 px-4 py-2",

                                    div {
                                        class: "flex items-center gap-2",

                                        span {
                                            class: "text-[9px] font-black tracking-widest text-zinc-600 mr-1",
                                            "FILTER"
                                        }

                                        // -------------------------------------------------
                                        // ALL
                                        // -------------------------------------------------

                                        button {
                                            class: if all_enabled {
                                                "px-3 py-1 rounded text-[9px] font-black bg-indigo-600 text-white border border-indigo-500"
                                            } else {
                                                "px-3 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                                            },

                                            onclick: move |_| {
                                                visible_events.with_mut(|set| {
                                                    set.clear();
                                                    set.extend(LogType::all());
                                                });

                                                filter_popup_open.set(false);
                                            },

                                            "ALL"
                                        }

                                        // -------------------------------------------------
                                        // NONE
                                        // -------------------------------------------------

                                        button {
                                            class: if none_enabled {
                                                "px-3 py-1 rounded text-[9px] font-black bg-indigo-600 text-white border border-indigo-500"
                                            } else {
                                                "px-3 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                                            },

                                            onclick: move |_| {
                                                visible_events.with_mut(|set| {
                                                    set.clear();
                                                });

                                                filter_popup_open.set(false);
                                            },

                                            "NONE"
                                        }

                                        div {
                                            class: "w-px h-4 bg-zinc-800 mx-1"
                                        }

                                        // -------------------------------------------------
                                        // ESSENTIAL PRESET
                                        // -------------------------------------------------

                                        button {
                                            class: if essential_enabled {
                                                "px-3 py-1 rounded text-[9px] font-black bg-indigo-600 text-white border border-indigo-500"
                                            } else {
                                                "px-3 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                                            },

                                            onclick: move |_| {
                                                visible_events.with_mut(|set| {
                                                    set.clear();

                                                    set.insert(LogType::Chat);
                                                    set.insert(LogType::Connection);
                                                    set.insert(LogType::RoundWin);
                                                    set.insert(LogType::GameOver);
                                                });

                                                filter_popup_open.set(false);
                                            },

                                            "ESSENTIAL"
                                        }

                                        // -------------------------------------------------
                                        // CUSTOMIZE
                                        // -------------------------------------------------

                                        button {
                                            class: if filter_popup_open() || customize_enabled {
                                                "px-3 py-1 rounded text-[9px] font-black bg-indigo-950 text-indigo-300 border border-indigo-700"
                                            } else {
                                                "px-3 py-1 rounded text-[9px] font-black bg-zinc-800 text-zinc-500 border border-zinc-700 hover:text-zinc-300"
                                            },

                                            onclick: move |_| {
                                                let open = filter_popup_open();
                                                filter_popup_open.set(!open);
                                            },

                                            "CUSTOMIZE"
                                        }

                                        // -------------------------------------------------
                                        // FILTER SUMMARY
                                        // -------------------------------------------------

                                        span {
                                            class: "ml-auto text-[9px] text-zinc-600",

                                            "{selected_events.len()}/{LogType::all().count()}"
                                        }
                                    }

                                    // =================================================
                                    // CUSTOMIZE POPUP
                                    // =================================================

                                    if filter_popup_open() {
                                        div {
                                            class: "absolute z-50 top-full left-4 mt-2 w-80 bg-zinc-950 border border-zinc-700 rounded-lg shadow-2xl",

                                            div {
                                                class: "px-4 py-3 border-b border-zinc-800 flex items-center justify-between",

                                                div {
                                                    class: "text-[10px] font-black tracking-widest text-zinc-300",
                                                    "CUSTOMIZE FILTER"
                                                }

                                                button {
                                                    class: "text-zinc-600 hover:text-zinc-300 text-sm px-1",

                                                    onclick: move |_| {
                                                        filter_popup_open.set(false);
                                                    },

                                                    "×"
                                                }
                                            }

                                            div {
                                                class: "p-3 max-h-80 overflow-y-auto",

                                                div {
                                                    class: "grid grid-cols-2 gap-1",

                                                    for event_type in LogType::all() {
                                                        {
                                                            let enabled =
                                                                selected_events.contains(&event_type);

                                                            let label =
                                                                event_type.label();

                                                            let class = if enabled {
                                                                "w-full px-3 py-2 rounded text-left text-[9px] font-black bg-indigo-950 text-indigo-300 border border-indigo-800 hover:bg-indigo-900 transition-colors"
                                                            } else {
                                                                "w-full px-3 py-2 rounded text-left text-[9px] font-black bg-zinc-900 text-zinc-600 border border-zinc-800 hover:text-zinc-400 hover:border-zinc-700 transition-colors"
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

                                                                    div {
                                                                        class: "flex items-center gap-2",

                                                                        div {
                                                                            class: if enabled {
                                                                                "w-2 h-2 rounded-sm bg-indigo-500"
                                                                            } else {
                                                                                "w-2 h-2 rounded-sm border border-zinc-700"
                                                                            }
                                                                        }

                                                                        "{label}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            div {
                                                class: "px-3 py-2 border-t border-zinc-800 flex justify-between",

                                                button {
                                                    class: "px-2 py-1 text-[9px] font-black text-zinc-600 hover:text-zinc-300",

                                                    onclick: move |_| {
                                                        visible_events.with_mut(|set| {
                                                            set.clear();
                                                        });
                                                    },

                                                    "CLEAR"
                                                }

                                                button {
                                                    class: "px-2 py-1 text-[9px] font-black text-indigo-500 hover:text-indigo-300",

                                                    onclick: move |_| {
                                                        visible_events.with_mut(|set| {
                                                            set.clear();
                                                            set.extend(LogType::all());
                                                        });
                                                    },

                                                    "SELECT ALL"
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
                                    class: "relative shrink-0 border-t border-zinc-800 bg-zinc-900 p-3",

                                    // =================================================
                                    // AUTOCOMPLETE POPUP
                                    // =================================================

                                    if !suggestions.read().is_empty() {
                                        div {
                                            class: "absolute z-50 bottom-full left-3 right-3 mb-1 bg-zinc-950 border border-zinc-700 rounded-lg shadow-2xl overflow-visible",

                                            div {
                                                class: "px-3 py-2 border-b border-zinc-800 text-[9px] font-black tracking-widest text-zinc-600",

                                                "COMMANDS"
                                            }

                                            // -------------------------------------------------
                                            // CVar suggestions
                                            // -------------------------------------------------

                                            for suggestion in suggestions.read().iter() {
                                                {
                                                    let command = suggestion.name.clone();
                                                    let value = suggestion.value.clone();
                                                    let description = suggestion.description.clone();

                                                    let flags = suggestion
                                                        .flags
                                                        .iter()
                                                        .map(|flag| format!("{flag:?}"))
                                                        .collect::<Vec<_>>()
                                                        .join(", ");

                                                    rsx! {
                                                        div {
                                                            class: "relative group",

                                                            button {
                                                                key: "{command}",

                                                                class: "w-full text-left px-3 py-2 text-[10px] text-zinc-400 hover:bg-zinc-800 hover:text-indigo-300 transition-colors",

                                                                onclick: {
                                                                    let command = command.clone();

                                                                    move |_| {
                                                                        cmd_input.set(command.clone());
                                                                        suggestions.set(Vec::new());
                                                                    }
                                                                },

                                                                div {
                                                                    class: "flex items-center gap-2",

                                                                    // CVar name
                                                                    span {
                                                                        class: "text-indigo-300 shrink-0",
                                                                        "{command}"
                                                                    }

                                                                    // Current value
                                                                    span {
                                                                        class: "text-zinc-500 truncate",
                                                                        "{value}"
                                                                    }

                                                                    // Flags
                                                                    span {
                                                                        class: "ml-auto text-zinc-600 shrink-0",
                                                                        "[{flags}]"
                                                                    }
                                                                }
                                                            }

                                                            // -------------------------------------------------
                                                            // Tooltip
                                                            // -------------------------------------------------

                                                            div {
                                                                class: "absolute z-[100] left-3 bottom-full mb-1 hidden group-hover:block w-[calc(100%-24px)] rounded-lg border border-zinc-700 bg-zinc-900 p-3 text-left shadow-2xl pointer-events-none",

                                                                div {
                                                                    class: "text-[10px] font-bold text-indigo-300 mb-1",
                                                                    "{command}"
                                                                }

                                                                div {
                                                                    class: "text-[10px] text-zinc-400 whitespace-normal",
                                                                    "{description}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // =================================================
                                    // COMMAND INPUT ROW
                                    // =================================================

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

                                            oninput: {
                                                let cvar_db = cvar_db.clone();

                                                move |event| {
                                                    let input = event.value();

                                                    cmd_input.set(input.clone());

                                                    let query = input
                                                        .split_whitespace()
                                                        .next()
                                                        .unwrap_or("")
                                                        .to_string();

                                                    if query.is_empty()
                                                        || input.contains(char::is_whitespace)
                                                    {
                                                        suggestions.set(Vec::new());
                                                        return;
                                                    }

                                                    let Some(db) = cvar_db.clone() else {
                                                        suggestions.set(Vec::new());
                                                        return;
                                                    };

                                                    let mut suggestions_signal = suggestions;

                                                    spawn(async move {
                                                        let db = db.lock().await;

                                                        let mut filter = HashSet::new();

                                                        filter.insert(CvarFlag::MenuBarItem);
                                                        filter.insert(CvarFlag::VConsoleFuzzy);
                                                        filter.insert(CvarFlag::VConsoleSetFocus);
                                                        filter.insert(CvarFlag::DevelopmentOnly);

                                                        let results =
                                                            db.get_suggestions(&query, &filter);

                                                        suggestions_signal.set(results);
                                                    });
                                                }
                                            },

                                            onkeydown: {
                                                let enter_client = enter_client.clone();

                                                move |event| {
                                                    if event.key() == Key::Escape {
                                                        suggestions.set(Vec::new());
                                                        return;
                                                    }

                                                    if event.key() == Key::Enter {
                                                        let cmd = cmd_input();

                                                        if cmd.trim().is_empty() {
                                                            return;
                                                        }

                                                        cmd_input.set(String::new());
                                                        suggestions.set(Vec::new());

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
                                        }

                                        button {
                                            class: "bg-indigo-600 hover:bg-indigo-500 text-white px-5 py-2 rounded font-bold",

                                            onclick: move |_| {
                                                let cmd = cmd_input();

                                                if cmd.trim().is_empty() {
                                                    return;
                                                }

                                                cmd_input.set(String::new());
                                                suggestions.set(Vec::new());

                                                let client =
                                                    send_client.clone();

                                                let mut logs = logs;

                                                spawn(async move {
                                                    let mut client = client.lock().await;

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
