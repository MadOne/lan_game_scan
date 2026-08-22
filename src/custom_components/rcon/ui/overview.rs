use dioxus::prelude::*;
use std::net::SocketAddr;

use crate::custom_components::{
    code::{RconLogEvent, RconPlayers, Team, TeamScore},
    ui::RconChat,
};

use cbz_rcon::RconStatus;

#[component]
pub fn RconOverview(
    addr: SocketAddr,
    hostname: String,
    map: String,
    status: Signal<RconStatus>,
    score: Signal<TeamScore>,
    player_count: u8,
    player_max: u8,
    logs: Signal<Vec<RconLogEvent>>,
    players: Signal<RconPlayers>,
    paused: Signal<bool>,
    maps: Signal<Vec<String>>,
    on_command: EventHandler<String>,
    get_maps: EventHandler<()>,
) -> Element {
    // Keep the RconPlayers value alive while we borrow player data from it.
    let current_players = players();

    let ct_players: Vec<_> = current_players
        .players()
        .values()
        .filter(|player| player.team == Team::CT)
        .collect();

    let t_players: Vec<_> = current_players
        .players()
        .values()
        .filter(|player| player.team == Team::Terrorist)
        .collect();

    let current_score = score();
    let is_paused = paused();

    let mut show_map_change = use_signal(|| false);
    let mut map_input = use_signal(String::new);
    let mut chat_input = use_signal(String::new);

    let current_maps = maps();

    // -------------------------------------------------------------------------
    // Map groups
    // -------------------------------------------------------------------------

    let de_maps: Vec<String> = current_maps
        .iter()
        .filter(|map| map.starts_with("de_"))
        .cloned()
        .collect();

    let cs_maps: Vec<String> = current_maps
        .iter()
        .filter(|map| map.starts_with("cs_"))
        .cloned()
        .collect();

    let ar_maps: Vec<String> = current_maps
        .iter()
        .filter(|map| map.starts_with("ar_"))
        .cloned()
        .collect();

    rsx! {
        div {
            class: "flex flex-col h-full min-h-0 bg-zinc-950",

            // ========================================================
            // SERVER CONTROL BAR
            // ========================================================

            div {
                class: "shrink-0 bg-zinc-900 border-b border-zinc-800 px-5 py-3",

                div {
                    class: "text-white font-black text-sm truncate",
                    "{hostname}"
                }

                div {
                    class: "flex items-center gap-4 mt-2",

                    // ------------------------------------------------
                    // MAP
                    // ------------------------------------------------

                    div {
                        class: "flex items-center gap-2 relative",

                        span {
                            class: "text-zinc-400 text-[10px] font-bold",
                            "{map}"
                        }

                        button {
                            class: "px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-[9px] font-black text-zinc-300 hover:border-indigo-500 hover:text-indigo-300",

                            onclick: move |_| {
                                let was_open = show_map_change();

                                if !was_open {
                                    show_map_change.set(true);

                                    if current_maps.is_empty() {
                                        get_maps.call(());
                                    }
                                } else {
                                    show_map_change.set(false);
                                }
                            },

                            "CHANGE"
                        }

                        // ------------------------------------------------
                        // MAP SELECTOR
                        // ------------------------------------------------

                        if show_map_change() {
                            div {
                                class: "absolute top-8 left-0 z-50 bg-zinc-900 border border-zinc-700 rounded-lg p-3 shadow-xl w-[720px] max-w-[calc(100vw-2rem)]",

                                // ----------------------------------------
                                // AVAILABLE MAPS
                                // ----------------------------------------

                                if current_maps.is_empty() {
                                    div {
                                        class: "text-zinc-600 text-[10px] px-2 py-2 animate-pulse text-center",
                                        "LOADING MAPS..."
                                    }
                                } else {
                                    div {
                                        class: "grid grid-cols-4 gap-3",

                                        // =================================================
                                        // MAP HEADERS
                                        // =================================================

                                        div {
                                            class: "col-span-2 text-blue-400 text-[10px] font-black tracking-widest text-center pb-1",
                                            "DE"
                                        }

                                        div {
                                            class: "text-red-400 text-[10px] font-black tracking-widest text-center pb-1",
                                            "CS"
                                        }

                                        div {
                                            class: "text-amber-400 text-[10px] font-black tracking-widest text-center pb-1",
                                            "AR"
                                        }

                                        // =================================================
                                        // DE MAPS
                                        // =================================================

                                        div {
                                            class: "col-span-2 grid grid-cols-2 gap-1 max-h-64 overflow-y-auto",

                                            for available_map in de_maps.iter() {
                                                {
                                                    let map_name = available_map.clone();

                                                    rsx! {
                                                        button {
                                                            key: "{map_name}",

                                                            class: "
                                                                w-full text-left
                                                                px-2 py-1.5
                                                                rounded
                                                                text-[10px]
                                                                font-bold
                                                                text-blue-300/80
                                                                hover:bg-blue-500/10
                                                                hover:text-blue-200
                                                                truncate
                                                                transition-colors
                                                            ",

                                                            onclick: move |_| {
                                                                on_command.call(
                                                                    format!("changelevel {}", map_name)
                                                                );

                                                                show_map_change.set(false);
                                                            },

                                                            "{available_map}"
                                                        }
                                                    }
                                                }
                                            }

                                            if de_maps.is_empty() {
                                                div {
                                                    class: "col-span-2 text-center text-zinc-700 text-[10px] py-2",
                                                    "NO MAPS"
                                                }
                                            }
                                        }

                                        // =================================================
                                        // CS MAPS
                                        // =================================================

                                        div {
                                            class: "max-h-64 overflow-y-auto space-y-1",

                                            for available_map in cs_maps.iter() {
                                                {
                                                    let map_name = available_map.clone();

                                                    rsx! {
                                                        button {
                                                            key: "{map_name}",

                                                            class: "
                                                                w-full text-left
                                                                px-2 py-1.5
                                                                rounded
                                                                text-[10px]
                                                                font-bold
                                                                text-red-300/80
                                                                hover:bg-red-500/10
                                                                hover:text-red-200
                                                                truncate
                                                                transition-colors
                                                            ",

                                                            onclick: move |_| {
                                                                on_command.call(
                                                                    format!("changelevel {}", map_name)
                                                                );

                                                                show_map_change.set(false);
                                                            },

                                                            "{available_map}"
                                                        }
                                                    }
                                                }
                                            }

                                            if cs_maps.is_empty() {
                                                div {
                                                    class: "text-center text-zinc-700 text-[10px] py-2",
                                                    "NO MAPS"
                                                }
                                            }
                                        }

                                        // =================================================
                                        // AR MAPS
                                        // =================================================

                                        div {
                                            class: "max-h-64 overflow-y-auto space-y-1",

                                            for available_map in ar_maps.iter() {
                                                {
                                                    let map_name = available_map.clone();

                                                    rsx! {
                                                        button {
                                                            key: "{map_name}",

                                                            class: "
                                                                w-full text-left
                                                                px-2 py-1.5
                                                                rounded
                                                                text-[10px]
                                                                font-bold
                                                                text-amber-300/80
                                                                hover:bg-amber-500/10
                                                                hover:text-amber-200
                                                                truncate
                                                                transition-colors
                                                            ",

                                                            onclick: move |_| {
                                                                on_command.call(
                                                                    format!("changelevel {}", map_name)
                                                                );

                                                                show_map_change.set(false);
                                                            },

                                                            "{available_map}"
                                                        }
                                                    }
                                                }
                                            }

                                            if ar_maps.is_empty() {
                                                div {
                                                    class: "text-center text-zinc-700 text-[10px] py-2",
                                                    "NO MAPS"
                                                }
                                            }
                                        }
                                    }
                                }

                                // ----------------------------------------
                                // CUSTOM MAP
                                // ----------------------------------------

                                div {
                                    class: "border-t border-zinc-800 mt-3 pt-2 flex items-center gap-2",

                                    input {
                                        r#type: "text",
                                        placeholder: "Custom map...",
                                        value: "{map_input}",

                                        class: "flex-1 min-w-0 bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-[10px] text-white outline-none focus:border-indigo-500",

                                        oninput: move |event| {
                                            map_input.set(event.value());
                                        },

                                        onkeydown: move |event| {
                                            if event.key() == Key::Enter {
                                                let map_name = map_input().trim().to_string();

                                                if !map_name.is_empty() {
                                                    on_command.call(
                                                        format!("changelevel {}", map_name)
                                                    );

                                                    map_input.set(String::new());
                                                    show_map_change.set(false);
                                                }
                                            }
                                        }
                                    }

                                    button {
                                        class: "px-2 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white rounded text-[9px] font-black",

                                        onclick: move |_| {
                                            let map_name = map_input().trim().to_string();

                                            if !map_name.is_empty() {
                                                on_command.call(
                                                    format!("changelevel {}", map_name)
                                                );

                                                map_input.set(String::new());
                                                show_map_change.set(false);
                                            }
                                        },

                                        "GO"
                                    }
                                }

                                // ----------------------------------------
                                // CLOSE
                                // ----------------------------------------

                                div {
                                    class: "flex justify-end mt-1",

                                    button {
                                        class: "px-1.5 py-1 text-zinc-500 hover:text-zinc-300 text-[10px]",

                                        onclick: move |_| {
                                            show_map_change.set(false);
                                            map_input.set(String::new());
                                        },

                                        "×"
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "text-zinc-700",
                        "—"
                    }

                    // ------------------------------------------------
                    // ROUND
                    // ------------------------------------------------

                    div {
                        class: "flex items-center gap-2",

                        span {
                            class: "text-zinc-400 text-[10px] font-black uppercase tracking-wider",
                            "ROUND {current_score.round}"
                        }

                        button {
                            class: "px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-[9px] font-black text-zinc-300 hover:border-indigo-500 hover:text-indigo-300",

                            onclick: move |_| {
                                on_command.call("mp_restartgame 1".to_string());
                            },

                            "RESTART"
                        }
                    }

                    div {
                        class: "text-zinc-700",
                        "—"
                    }

                    // ------------------------------------------------
                    // STATUS / PAUSE
                    // ------------------------------------------------

                    div {
                        class: "flex items-center gap-2",

                        div {
                            class: "flex items-center gap-2",

                            if is_paused {
                                span {
                                    class: "text-yellow-500 font-black text-[10px]",
                                    "● PAUSED"
                                }
                            } else {
                                span {
                                    class: "text-emerald-500 font-black text-[10px]",
                                    "● RUNNING"
                                }
                            }
                        }

                        {
                            let button_class = if is_paused {
                                "px-2 py-1 bg-emerald-900/40 border border-emerald-700 rounded text-[9px] font-black text-emerald-300 hover:border-emerald-500 hover:text-emerald-200"
                            } else {
                                "px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-[9px] font-black text-zinc-300 hover:border-red-500 hover:text-red-400"
                            };

                            let command = if is_paused {
                                "mp_unpause_match"
                            } else {
                                "mp_pause_match"
                            };

                            rsx! {
                                button {
                                    class: button_class,

                                    onclick: move |_| {
                                        on_command.call(command.to_string());
                                    },

                                    if is_paused {
                                        "UNPAUSE"
                                    } else {
                                        "PAUSE"
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "text-zinc-700",
                        "—"
                    }

                    // ------------------------------------------------
                    // CONFIG
                    // ------------------------------------------------

                    RconConfig {
                        on_command,
                    }
                }
            }

            // ========================================================
            // MAIN OVERVIEW
            // ========================================================

            div {
                class: "flex-1 min-h-0 flex",

                // ====================================================
                // TEAMS
                // ====================================================

                div {
                    class: "flex-1 min-w-0 p-6 overflow-y-auto",

                    div {
                        class: "grid grid-cols-2 gap-8",

                        // ==================================================
                        // CT
                        // ==================================================

                        div {
                            class: "flex flex-col min-w-0",

                            div {
                                class: "text-center mb-5",

                                div {
                                    class: "text-blue-400 font-black text-[11px] tracking-widest uppercase",
                                    "COUNTER-TERRORISTS"
                                }

                                div {
                                    class: "text-blue-400 text-5xl font-black mt-2",
                                    "{current_score.ct}"
                                }

                                div {
                                    class: "text-[9px] text-zinc-600 mt-1",
                                    "{ct_players.len()} PLAYERS"
                                }
                            }

                            div {
                                class: "space-y-2",

                                for player in ct_players.iter() {
                                    div {
                                        class: "px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded text-blue-300 font-bold",

                                        if player.name.is_empty() {
                                            "UNKNOWN"
                                        } else {
                                            "{player.name}"
                                        }
                                    }
                                }

                                if ct_players.is_empty() {
                                    div {
                                        class: "text-center text-zinc-700 text-[10px] py-4",
                                        "NO PLAYER DATA"
                                    }
                                }
                            }
                        }

                        // ==================================================
                        // T
                        // ==================================================

                        div {
                            class: "flex flex-col min-w-0",

                            div {
                                class: "text-center mb-5",

                                div {
                                    class: "text-red-400 font-black text-[11px] tracking-widest uppercase",
                                    "TERRORISTS"
                                }

                                div {
                                    class: "text-red-400 text-5xl font-black mt-2",
                                    "{current_score.t}"
                                }

                                div {
                                    class: "text-[9px] text-zinc-600 mt-1",
                                    "{t_players.len()} PLAYERS"
                                }
                            }

                            div {
                                class: "space-y-2",

                                for player in t_players.iter() {
                                    div {
                                        class: "px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded text-red-300 font-bold",

                                        if player.name.is_empty() {
                                            "UNKNOWN"
                                        } else {
                                            "{player.name}"
                                        }
                                    }
                                }

                                if t_players.is_empty() {
                                    div {
                                        class: "text-center text-zinc-700 text-[10px] py-4",
                                        "NO PLAYER DATA"
                                    }
                                }
                            }
                        }
                    }
                }

                // ====================================================
                // CHAT
                // ====================================================

                div {
                    class: "w-[360px] shrink-0 flex flex-col min-h-0 border-l border-zinc-800",

                    // ------------------------------------------------
                    // CHAT LOG
                    // ------------------------------------------------

                    div {
                        class: "flex-1 min-h-0",

                        RconChat {
                            logs: logs
                        }
                    }

                    // ------------------------------------------------
                    // CHAT INPUT
                    // ------------------------------------------------

                    div {
                        class: "shrink-0 border-t border-zinc-800 bg-zinc-900 p-2",

                        div {
                            class: "flex items-center gap-2",

                            input {
                                r#type: "text",
                                placeholder: "Send message...",
                                value: "{chat_input}",

                                class: "
                                    flex-1 min-w-0
                                    bg-zinc-950
                                    border border-zinc-700
                                    rounded
                                    px-2 py-1.5
                                    text-[10px] text-white
                                    placeholder-zinc-600
                                    outline-none
                                    focus:border-indigo-500
                                ",

                                oninput: move |event| {
                                    chat_input.set(event.value());
                                },

                                onkeydown: move |event| {
                                    if event.key() == Key::Enter {
                                        let message = chat_input().trim().to_string();

                                        if !message.is_empty() {
                                            on_command.call(format!("say {}", message));
                                            chat_input.set(String::new());
                                        }
                                    }
                                }
                            }

                            button {
                                class: "
                                    shrink-0
                                    px-3 py-1.5
                                    bg-indigo-600
                                    hover:bg-indigo-500
                                    text-white
                                    rounded
                                    text-[9px]
                                    font-black
                                ",

                                onclick: move |_| {
                                    let message = chat_input().trim().to_string();

                                    if !message.is_empty() {
                                        on_command.call(format!("say {}", message));
                                        chat_input.set(String::new());
                                    }
                                },

                                "SEND"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RconConfig(on_command: EventHandler<String>) -> Element {
    let mut show_config = use_signal(|| false);
    let mut exec_input = use_signal(String::new);

    rsx! {
        div {
            class: "relative",

            // ------------------------------------------------
            // CONFIG BUTTON
            // ------------------------------------------------

            button {
                class: "px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-[9px] font-black text-zinc-300 hover:border-indigo-500 hover:text-indigo-300",

                onclick: move |_| {
                    show_config.set(!show_config());
                },

                "CONFIG"
            }

            // ------------------------------------------------
            // CONFIG POPUP
            // ------------------------------------------------

            if show_config() {
                div {
                    class: "absolute top-8 left-0 z-50 bg-zinc-900 border border-zinc-700 rounded-lg p-3 shadow-xl w-[520px] max-w-[calc(100vw-2rem)]",

                    div {
                        class: "text-indigo-400 text-[10px] font-black tracking-widest pb-2 mb-3 border-b border-zinc-800",
                        "MATCH CONFIG"
                    }

                    div {
                        class: "grid grid-cols-2 gap-4",

                        // =================================================
                        // GAME MODE
                        // =================================================

                        div {
                            class: "min-w-0",

                            div {
                                class: "text-blue-400 text-[10px] font-black tracking-widest text-center pb-2",
                                "GAME MODE"
                            }

                            div {
                                class: "space-y-1",

                                button {
                                    class: "w-full text-left px-2 py-1.5 rounded text-[10px] font-bold text-blue-300/80 hover:bg-blue-500/10 hover:text-blue-200 truncate transition-colors",

                                    onclick: move |_| {
                                        on_command.call(
                                            "exec gamemode_armsrace.cfg".to_string()
                                        );

                                        show_config.set(false);
                                    },

                                    "ARMS RACE"
                                }

                                button {
                                    class: "w-full text-left px-2 py-1.5 rounded text-[10px] font-bold text-blue-300/80 hover:bg-blue-500/10 hover:text-blue-200 truncate transition-colors",

                                    onclick: move |_| {
                                        on_command.call(
                                            "exec gamemode_competitive.cfg".to_string()
                                        );

                                        show_config.set(false);
                                    },

                                    "COMPETITIVE"
                                }

                                button {
                                    class: "w-full text-left px-2 py-1.5 rounded text-[10px] font-bold text-blue-300/80 hover:bg-blue-500/10 hover:text-blue-200 truncate transition-colors",

                                    onclick: move |_| {
                                        on_command.call(
                                            "exec gamemode_casual.cfg".to_string()
                                        );

                                        show_config.set(false);
                                    },

                                    "CASUAL"
                                }

                                button {
                                    class: "w-full text-left px-2 py-1.5 rounded text-[10px] font-bold text-blue-300/80 hover:bg-blue-500/10 hover:text-blue-200 truncate transition-colors",

                                    onclick: move |_| {
                                        on_command.call(
                                            "exec gamemode_deathmatch.cfg".to_string()
                                        );

                                        show_config.set(false);
                                    },

                                    "DEATHMATCH"
                                }
                            }
                        }

                        // =================================================
                        // CUSTOM EXEC
                        // =================================================

                        div {
                            class: "min-w-0",

                            div {
                                class: "text-amber-400 text-[10px] font-black tracking-widest text-center pb-2",
                                "CUSTOM EXEC"
                            }

                            div {
                                class: "flex flex-col gap-2",

                                button {
                                    class: "w-full text-left px-2 py-1.5 rounded text-[10px] font-bold text-amber-300/80 hover:bg-amber-500/10 hover:text-amber-200 truncate transition-colors",

                                    onclick: move |_| {
                                        on_command.call(
                                            "exec turnier.cfg".to_string()
                                        );

                                        show_config.set(false);
                                    },

                                    "TURNIER.CFG"
                                }

                                div {
                                    class: "border-t border-zinc-800 pt-2",

                                    div {
                                        class: "flex gap-1",

                                        input {
                                            r#type: "text",
                                            placeholder: "config.cfg",
                                            value: "{exec_input}",

                                            class: "flex-1 min-w-0 bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-[10px] text-white outline-none focus:border-indigo-500",

                                            oninput: move |event| {
                                                exec_input.set(event.value());
                                            },

                                            onkeydown: move |event| {
                                                if event.key() == Key::Enter {
                                                    let config = exec_input().trim().to_string();

                                                    if !config.is_empty() {
                                                        on_command.call(
                                                            format!("exec {}", config)
                                                        );

                                                        exec_input.set(String::new());
                                                        show_config.set(false);
                                                    }
                                                }
                                            }
                                        }

                                        button {
                                            class: "px-2 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white rounded text-[9px] font-black",

                                            onclick: move |_| {
                                                let config = exec_input().trim().to_string();

                                                if !config.is_empty() {
                                                    on_command.call(
                                                        format!("exec {}", config)
                                                    );

                                                    exec_input.set(String::new());
                                                    show_config.set(false);
                                                }
                                            },

                                            "EXEC"
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
}
