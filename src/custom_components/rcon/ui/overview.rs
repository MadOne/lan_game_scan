use dioxus::prelude::*;
use std::net::SocketAddr;

use crate::custom_components::{
    code::{Player, RconLogEvent, RconPlayers, Team, TeamScore},
    ui::RconChat,
};

use cbz_rcon::RconStatus;

// =============================================================================
// RCON OVERVIEW
// =============================================================================

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

    let ct_players: Vec<Player> = current_players
        .players()
        .values()
        .filter(|player| player.team == Team::CT)
        .cloned()
        .collect();

    let t_players: Vec<Player> = current_players
        .players()
        .values()
        .filter(|player| player.team == Team::Terrorist)
        .cloned()
        .collect();

    let current_score = score();

    rsx! {
        div {
            class: "flex flex-col h-full min-h-0 bg-zinc-950",

            // ========================================================
            // SERVER CONTROL BAR
            // ========================================================

            RconControlBar {
                hostname,
                map,
                score: current_score,
                paused,
                maps,
                on_command,
                get_maps,
            }

            // ========================================================
            // MAIN OVERVIEW
            // ========================================================

            div {
                class: "flex-1 min-h-0 flex relative",

                // ====================================================
                // TEAMS
                // ====================================================

                RconTeams {
                    ct_players,
                    t_players,
                    score: current_score,
                    on_command,
                }

                // ====================================================
                // CHAT
                // ====================================================

                RconChatPanel {
                    logs,
                    on_command,
                }
            }
        }
    }
}

// =============================================================================
// CONTROL BAR
// =============================================================================

#[component]
fn RconControlBar(
    hostname: String,
    map: String,
    score: TeamScore,
    paused: Signal<bool>,
    maps: Signal<Vec<String>>,
    on_command: EventHandler<String>,
    get_maps: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "shrink-0 bg-zinc-900 border-b border-zinc-800 px-5 py-3",

            div {
                class: "text-white font-black text-sm truncate",
                "{hostname}"
            }

            div {
                class: "flex items-center gap-4 mt-2 flex-wrap",

                // ------------------------------------------------
                // MAP
                // ------------------------------------------------

                RconMapSelector {
                    map,
                    maps,
                    on_command,
                    get_maps,
                }

                div {
                    class: "text-zinc-700",
                    "—"
                }

                // ------------------------------------------------
                // ROUND
                // ------------------------------------------------

                RconRoundControls {
                    round: score.round,
                    on_command,
                }

                div {
                    class: "text-zinc-700",
                    "—"
                }

                // ------------------------------------------------
                // STATUS / PAUSE
                // ------------------------------------------------

                RconPauseControls {
                    paused,
                    on_command,
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
    }
}

// =============================================================================
// MAP SELECTOR
// =============================================================================

#[component]
fn RconMapSelector(
    map: String,
    maps: Signal<Vec<String>>,
    on_command: EventHandler<String>,
    get_maps: EventHandler<()>,
) -> Element {
    let mut show_map_change = use_signal(|| false);
    let mut map_input = use_signal(String::new);

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
            // MAP SELECTOR POPUP
            // ------------------------------------------------

            if show_map_change() {
                div {
                    class: "absolute top-8 left-0 z-50 bg-zinc-900 border border-zinc-700 rounded-lg p-3 shadow-xl w-[720px] max-w-[calc(100vw-2rem)]",

                    if current_maps.is_empty() {
                        div {
                            class: "text-zinc-600 text-[10px] px-2 py-2 animate-pulse text-center",
                            "LOADING MAPS..."
                        }
                    } else {
                        div {
                            class: "grid grid-cols-4 gap-3",

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
    }
}

// =============================================================================
// ROUND CONTROLS
// =============================================================================

#[component]
fn RconRoundControls(round: i32, on_command: EventHandler<String>) -> Element {
    rsx! {
        div {
            class: "flex items-center gap-2",

            span {
                class: "text-zinc-400 text-[10px] font-black uppercase tracking-wider",
                "ROUND {round}"
            }

            button {
                class: "px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-[9px] font-black text-zinc-300 hover:border-indigo-500 hover:text-indigo-300",

                onclick: move |_| {
                    on_command.call("mp_restartgame 1".to_string());
                },

                "RESTART"
            }
        }
    }
}

// =============================================================================
// PAUSE CONTROLS
// =============================================================================

#[component]
fn RconPauseControls(paused: Signal<bool>, on_command: EventHandler<String>) -> Element {
    let is_paused = paused();

    rsx! {
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
    }
}

// =============================================================================
// TEAMS
// =============================================================================

#[component]
fn RconTeams(
    ct_players: Vec<Player>,
    t_players: Vec<Player>,
    score: TeamScore,
    on_command: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: "flex-1 min-w-0 p-6 overflow-y-auto",

            div {
                class: "grid grid-cols-2 gap-8",

                RconTeamColumn {
                    team: Team::CT,
                    players: ct_players,
                    score: score.ct,
                    on_command,
                }

                RconTeamColumn {
                    team: Team::Terrorist,
                    players: t_players,
                    score: score.t,
                    on_command,
                }
            }
        }
    }
}

// =============================================================================
// TEAM COLUMN
// =============================================================================

#[component]
fn RconTeamColumn(
    team: Team,
    players: Vec<Player>,
    score: u8,
    on_command: EventHandler<String>,
) -> Element {
    let (team_name, team_color) = match team {
        Team::CT => ("COUNTER-TERRORISTS", "blue"),
        Team::Terrorist => ("TERRORISTS", "red"),
        _ => ("UNKNOWN", "zinc"),
    };

    let title_class = match team_color {
        "blue" => "text-blue-400 font-black text-[11px] tracking-widest uppercase",
        "red" => "text-red-400 font-black text-[11px] tracking-widest uppercase",
        _ => "text-zinc-400 font-black text-[11px] tracking-widest uppercase",
    };

    let score_class = match team_color {
        "blue" => "text-blue-400 text-5xl font-black mt-2",
        "red" => "text-red-400 text-5xl font-black mt-2",
        _ => "text-zinc-400 text-5xl font-black mt-2",
    };

    let player_class = match team_color {
        "blue" => "px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded text-blue-300 font-bold cursor-context-menu",
        "red" => "px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded text-red-300 font-bold cursor-context-menu",
        _ => "px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded text-zinc-300 font-bold cursor-context-menu",
    };

    // Player ID of the player whose context menu is open.
    let mut context_player = use_signal(|| None::<u16>);

    // Ban duration in minutes.
    // 0 means permanent.
    let mut ban_duration = use_signal(|| 0u32);

    rsx! {
        div {
            class: "flex flex-col min-w-0",

            div {
                class: "text-center mb-5",

                div {
                    class: title_class,
                    "{team_name}"
                }

                div {
                    class: score_class,
                    "{score}"
                }

                div {
                    class: "text-[9px] text-zinc-600 mt-1",
                    "{players.len()} PLAYERS"
                }
            }

            div {
                class: "space-y-2",

                for player in players.iter() {
                    {
                        let player_id = player.id;
                        let player_name = if player.name.is_empty() {
                            "UNKNOWN"
                        } else {
                            &player.name
                        };

                        let steamid = player.steamid.clone();
                        let is_context_player = context_player() == Some(player_id);

                        rsx! {
                            div {
                                key: "{player_id}",
                                class: "relative",

                                // ============================================
                                // PLAYER
                                // ============================================

                                div {
                                    class: player_class,

                                    oncontextmenu: move |event| {
                                        event.prevent_default();
                                        context_player.set(Some(player_id));
                                        ban_duration.set(0);
                                    },

                                    "{player_name}"

                                    // ========================================
                                    // CONTEXT MENU
                                    // ========================================

                                    if is_context_player {
                                        // ========================================
                                        // OUTSIDE CLICK
                                        // ========================================

                                        div {
                                            class: "
                                                fixed
                                                inset-0
                                                z-40
                                            ",

                                            onclick: move |_| {
                                                context_player.set(None);
                                            },
                                        }

                                        // ========================================
                                        // CONTEXT MENU
                                        // ========================================

                                        div {
                                            class: "
                                                absolute
                                                left-0
                                                top-full
                                                mt-1
                                                z-50
                                                w-64
                                                bg-zinc-900
                                                border
                                                border-zinc-700
                                                rounded-lg
                                                shadow-2xl
                                                overflow-hidden
                                            ",

                                            // --------------------------------
                                            // PLAYER INFO
                                            // --------------------------------

                                            div {
                                                class: "px-3 py-2 border-b border-zinc-800",

                                                div {
                                                    class: "text-white text-[10px] font-black truncate",
                                                    "{player_name}"
                                                }

                                                div {
                                                    class: "flex items-center gap-1 mt-1",

                                                    span {
                                                        class: "text-zinc-600 text-[8px] font-bold",
                                                        "STEAMID"
                                                    }

                                                    span {
                                                        class: "text-zinc-400 text-[9px] font-mono truncate flex-1",
                                                        "{steamid}"
                                                    }
                                                }
                                            }

                                            // --------------------------------
                                            // ACTIONS
                                            // --------------------------------

                                            div {
                                                class: "p-1",

                                                button {
                                                    class: "
                                                        w-full
                                                        text-left
                                                        px-3
                                                        py-2
                                                        rounded
                                                        text-[10px]
                                                        font-bold
                                                        text-zinc-300
                                                        hover:bg-zinc-800
                                                        hover:text-white
                                                    ",

                                                    onclick: move |_| {
                                                        on_command.call(
                                                            format!("kickid {}", player_id)
                                                        );

                                                        context_player.set(None);
                                                    },

                                                    "KICK"
                                                }

                                                button {
                                                    class: "
                                                        w-full
                                                        text-left
                                                        px-3
                                                        py-2
                                                        rounded
                                                        text-[10px]
                                                        font-bold
                                                        text-zinc-300
                                                        hover:bg-zinc-800
                                                        hover:text-white
                                                    ",

                                                    onclick: move |_| {
                                                        on_command.call(
                                                            format!("kill {}", player_id)
                                                        );

                                                        context_player.set(None);
                                                    },

                                                    "KILL"
                                                }

                                                // ----------------------------
                                                // BAN
                                                // ----------------------------

                                                div {
                                                    class: "mt-1 pt-1 border-t border-zinc-800",

                                                    div {
                                                        class: "px-3 pt-2 pb-1 text-[8px] font-black tracking-widest text-red-400",
                                                        "BAN"
                                                    }

                                                    select {
                                                        class: "
                                                            w-full
                                                            appearance-none
                                                            bg-zinc-800
                                                            text-white
                                                            border
                                                            border-zinc-700
                                                            rounded
                                                            px-2
                                                            py-1
                                                            text-sm
                                                        ",
                                                        style: "color-scheme: dark;",

                                                        value: "{ban_duration()}",

                                                        onchange: move |evt| {
                                                            if let Ok(value) = evt.value().parse::<u32>() {
                                                                ban_duration.set(value);
                                                            }
                                                        },

                                                        option {
                                                            value: "0",
                                                            "Permanent"
                                                        }

                                                        option {
                                                            value: "5",
                                                            "5 minutes"
                                                        }

                                                        option {
                                                            value: "30",
                                                            "30 minutes"
                                                        }

                                                        option {
                                                            value: "60",
                                                            "1 hour"
                                                        }

                                                        option {
                                                            value: "1440",
                                                            "1 day"
                                                        }

                                                        option {
                                                            value: "10080",
                                                            "1 week"
                                                        }
                                                    }

                                                    button {
                                                        class: "
                                                            w-full
                                                            mt-2
                                                            px-3
                                                            py-2
                                                            bg-red-900/70
                                                            hover:bg-red-800
                                                            text-white
                                                            rounded
                                                            text-sm
                                                            font-medium
                                                        ",

                                                        onclick: move |_| {
                                                            let duration = ban_duration();

                                                            on_command.call(
                                                                format!(
                                                                    "banid {} {}",
                                                                    duration,
                                                                    steamid
                                                                )
                                                            );

                                                            context_player.set(None);
                                                        },

                                                        "BAN PLAYER"
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

                if players.is_empty() {
                    div {
                        class: "text-center text-zinc-700 text-[10px] py-4",
                        "NO PLAYER DATA"
                    }
                }
            }
        }
    }
}

// =============================================================================
// CHAT PANEL
// =============================================================================

#[component]
fn RconChatPanel(logs: Signal<Vec<RconLogEvent>>, on_command: EventHandler<String>) -> Element {
    let chat_input = use_signal(String::new);

    // ------------------------------------------------------------
    // Mobile chat state
    // ------------------------------------------------------------

    let show_mobile_chat = use_signal(|| false);

    // Number of log entries that were already seen while the chat
    // was open. We use the log length so this does not depend on
    // the internal structure of RconLogEvent.
    let mut seen_log_count = use_signal(|| logs().len());

    let current_log_count = logs().len();

    let has_unread_chat = !show_mobile_chat() && current_log_count > seen_log_count();

    // Once the mobile chat is opened, everything currently in the
    // log becomes read.
    if show_mobile_chat() && current_log_count > seen_log_count() {
        seen_log_count.set(current_log_count);
    }

    rsx! {
        RconDesktopChat {
            logs,
            chat_input,
            on_command,
        }

        RconMobileChat {
            logs,
            chat_input,
            show_mobile_chat,
            seen_log_count,
            has_unread_chat,
            on_command,
        }
    }
}

// =============================================================================
// DESKTOP CHAT
// =============================================================================

#[component]
fn RconDesktopChat(
    logs: Signal<Vec<RconLogEvent>>,
    chat_input: Signal<String>,
    on_command: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: "hidden md:flex w-[360px] shrink-0 flex-col min-h-0 border-l border-zinc-800",

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

            RconChatInput {
                chat_input,
                on_command,
            }
        }
    }
}

// =============================================================================
// MOBILE CHAT
// =============================================================================

#[component]
fn RconMobileChat(
    logs: Signal<Vec<RconLogEvent>>,
    chat_input: Signal<String>,
    show_mobile_chat: Signal<bool>,
    seen_log_count: Signal<usize>,
    has_unread_chat: bool,
    on_command: EventHandler<String>,
) -> Element {
    rsx! {
        // ====================================================
        // MOBILE CHAT BUTTON
        // ====================================================

        button {
            class: "
                md:hidden
                absolute
                right-4
                bottom-4
                z-30
                px-4
                py-2.5
                bg-zinc-900
                border
                border-zinc-700
                rounded-lg
                shadow-xl
                text-zinc-300
                text-[10px]
                font-black
                tracking-widest
                hover:border-indigo-500
                hover:text-indigo-300
                transition-all
            ",

            onclick: move |_| {
                seen_log_count.set(logs().len());
                show_mobile_chat.set(true);
            },

            "CHAT"

            if has_unread_chat {
                span {
                    class: "
                        absolute
                        -top-1.5
                        -right-1.5
                        min-w-[10px]
                        h-[10px]
                        rounded-full
                        bg-red-500
                        border-2
                        border-zinc-950
                        shadow
                    ",
                }

                span {
                    class: "
                        absolute
                        -top-7
                        right-0
                        px-1.5
                        py-0.5
                        bg-red-600
                        rounded
                        text-[7px]
                        text-white
                        font-black
                        tracking-wider
                        shadow
                    ",
                    "NEW"
                }
            }
        }

        // ====================================================
        // MOBILE CHAT SLIDE-IN
        // ====================================================

        if show_mobile_chat() {
            div {
                class: "
                    md:hidden
                    absolute
                    inset-y-0
                    right-0
                    z-40
                    w-[min(360px,100%)]
                    flex
                    flex-col
                    min-h-0
                    bg-zinc-950
                    border-l
                    border-zinc-700
                    shadow-2xl
                ",

                // ------------------------------------------------
                // MOBILE CHAT HEADER
                // ------------------------------------------------

                div {
                    class: "
                        shrink-0
                        h-12
                        flex
                        items-center
                        justify-between
                        px-4
                        bg-zinc-900
                        border-b
                        border-zinc-800
                    ",

                    div {
                        class: "text-indigo-400 text-[10px] font-black tracking-widest",
                        "CHAT"
                    }

                    button {
                        class: "
                            w-8
                            h-8
                            flex
                            items-center
                            justify-center
                            rounded
                            text-zinc-500
                            hover:text-white
                            hover:bg-zinc-800
                            text-lg
                        ",

                        onclick: move |_| {
                            show_mobile_chat.set(false);
                            seen_log_count.set(logs().len());
                        },

                        "×"
                    }
                }

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

                RconChatInput {
                    chat_input,
                    on_command,
                    mobile: true,
                }
            }
        }
    }
}

// =============================================================================
// CHAT INPUT
// =============================================================================

#[component]
fn RconChatInput(
    chat_input: Signal<String>,
    on_command: EventHandler<String>,
    #[props(default = false)] mobile: bool,
) -> Element {
    let container_class = "shrink-0 border-t border-zinc-800 bg-zinc-900 p-2";

    let input_class = if mobile {
        "
            flex-1 min-w-0
            bg-zinc-950
            border border-zinc-700
            rounded
            px-2 py-2
            text-[10px] text-white
            placeholder-zinc-600
            outline-none
            focus:border-indigo-500
        "
    } else {
        "
            flex-1 min-w-0
            bg-zinc-950
            border border-zinc-700
            rounded
            px-2 py-1.5
            text-[10px] text-white
            placeholder-zinc-600
            outline-none
            focus:border-indigo-500
        "
    };

    let button_class = if mobile {
        "
            shrink-0
            px-3
            py-2
            bg-indigo-600
            hover:bg-indigo-500
            text-white
            rounded
            text-[9px]
            font-black
        "
    } else {
        "
            shrink-0
            px-3 py-1.5
            bg-indigo-600
            hover:bg-indigo-500
            text-white
            rounded
            text-[9px]
            font-black
        "
    };

    rsx! {
        div {
            class: container_class,

            div {
                class: "flex items-center gap-2",

                input {
                    r#type: "text",
                    placeholder: "Send message...",
                    value: "{chat_input}",

                    class: input_class,

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
                    class: button_class,

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

// =============================================================================
// CONFIG
// =============================================================================

// =============================================================================
// CONFIG
// =============================================================================

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
                    class: "absolute top-8 left-1/2 -translate-x-1/2 z-50 bg-zinc-900 border border-zinc-700 rounded-lg p-3 shadow-xl w-[520px] max-w-[calc(100vw-2rem)]",

                    div {
                        class: "text-indigo-400 text-[10px] font-black tracking-widest pb-2 mb-3 border-b border-zinc-800",
                        "MATCH CONFIG"
                    }

                    // =================================================
                    // GAME MODE + CUSTOM EXEC
                    // =================================================

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

                    // =================================================
                    // BOTS
                    // =================================================

                    div {
                        class: "border-t border-zinc-800 mt-4 pt-3",

                        div {
                            class: "text-emerald-400 text-[10px] font-black tracking-widest text-center pb-2",
                            "BOTS"
                        }

                        div {
                            class: "grid grid-cols-2 gap-4",

                            // =============================================
                            // ADD BOT
                            // =============================================

                            div {
                                class: "min-w-0",

                                div {
                                    class: "text-zinc-500 text-[9px] font-black tracking-widest pb-1.5",
                                    "ADD BOT"
                                }

                                div {
                                    class: "grid grid-cols-3 gap-1",

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-emerald-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-emerald-500/10
                                            hover:border-emerald-700
                                            hover:text-emerald-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_add".to_string());
                                        },

                                        "ANY"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-red-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-red-500/10
                                            hover:border-red-700
                                            hover:text-red-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_add T".to_string());
                                        },

                                        "T"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-blue-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-blue-500/10
                                            hover:border-blue-700
                                            hover:text-blue-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_add CT".to_string());
                                        },

                                        "CT"
                                    }
                                }
                            }

                            // =============================================
                            // KICK BOT
                            // =============================================

                            div {
                                class: "min-w-0",

                                div {
                                    class: "text-zinc-500 text-[9px] font-black tracking-widest pb-1.5",
                                    "KICK BOT"
                                }

                                div {
                                    class: "grid grid-cols-3 gap-1",

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-red-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-red-500/10
                                            hover:border-red-700
                                            hover:text-red-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_kick".to_string());
                                        },

                                        "ALL"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-red-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-red-500/10
                                            hover:border-red-700
                                            hover:text-red-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_kick T".to_string());
                                        },

                                        "T"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-blue-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-blue-500/10
                                            hover:border-blue-700
                                            hover:text-blue-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_kick CT".to_string());
                                        },

                                        "CT"
                                    }
                                }
                            }

                            // =============================================
                            // KILL BOT
                            // =============================================

                            div {
                                class: "min-w-0",

                                div {
                                    class: "text-zinc-500 text-[9px] font-black tracking-widest pb-1.5",
                                    "KILL BOT"
                                }

                                div {
                                    class: "grid grid-cols-3 gap-1",

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-orange-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-orange-500/10
                                            hover:border-orange-700
                                            hover:text-orange-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_kill".to_string());
                                        },

                                        "ALL"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-red-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-red-500/10
                                            hover:border-red-700
                                            hover:text-red-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_kill T".to_string());
                                        },

                                        "T"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-blue-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:bg-blue-500/10
                                            hover:border-blue-700
                                            hover:text-blue-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_kill CT".to_string());
                                        },

                                        "CT"
                                    }
                                }
                            }

                            // =============================================
                            // DIFFICULTY
                            // =============================================

                            div {
                                class: "min-w-0",

                                div {
                                    class: "text-zinc-500 text-[9px] font-black tracking-widest pb-1.5",
                                    "DIFFICULTY"
                                }

                                div {
                                    class: "grid grid-cols-4 gap-1",

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-zinc-400
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:border-zinc-600
                                            hover:text-zinc-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_difficulty 0".to_string());
                                        },

                                        "EASY"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-zinc-300
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:border-indigo-700
                                            hover:text-indigo-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_difficulty 1".to_string());
                                        },

                                        "NORMAL"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-amber-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:border-amber-700
                                            hover:text-amber-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_difficulty 2".to_string());
                                        },

                                        "HARD"
                                    }

                                    button {
                                        class: "
                                            px-2 py-1.5
                                            rounded
                                            text-[9px]
                                            font-bold
                                            text-red-300/80
                                            bg-zinc-950
                                            border border-zinc-800
                                            hover:border-red-700
                                            hover:text-red-200
                                            transition-colors
                                        ",

                                        onclick: move |_| {
                                            on_command.call("bot_difficulty 3".to_string());
                                        },

                                        "EXPERT"
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
