use dioxus::prelude::*;
use std::net::SocketAddr;

use crate::state::AppState;
use cbz_rcon::RconStatus;

#[component]
pub fn RconDashboard() -> Element {
    let state = use_context::<AppState>();

    let sessions = state.rcon_sessions.read();

    let mut session_list: Vec<SocketAddr> = sessions.keys().copied().collect();

    session_list.sort_by_key(|addr| addr.to_string());

    rsx! {
        div {
            class: "h-full overflow-y-auto bg-zinc-950 p-6",

            // =========================================================
            // HEADER
            // =========================================================

            div {
                class: "flex items-center justify-between mb-6",

                div {
                    div {
                        class: "text-white text-sm font-black uppercase tracking-wider",
                        "RCON SERVERS"
                    }

                    div {
                        class: "text-zinc-600 text-[10px] font-bold mt-1",
                        "{session_list.len()} CONNECTED"
                    }
                }
            }

            // =========================================================
            // SERVER CARDS
            // =========================================================

            div {
                class: "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4",

                for addr in session_list {
                    RconServerCard {
                        key: "{addr}",
                        addr: addr,
                    }
                }
            }
        }
    }
}

// =============================================================================
// SERVER CARD
// =============================================================================

#[component]
fn RconServerCard(addr: SocketAddr) -> Element {
    let state = use_context::<AppState>();

    // This is the Signal itself, not a borrow into AppState.
    // It can therefore safely be captured by the onclick handler.
    let mut selected_rcon = state.selected_rcon;

    let sessions = state.rcon_sessions.read();

    let session = match sessions.get(&addr) {
        Some(session) => session,
        None => {
            return rsx! {};
        }
    };

    // -------------------------------------------------------------------------
    // Copy the Signals out of the borrowed session.
    // -------------------------------------------------------------------------

    let status = session.status;
    let match_paused = session.match_paused;
    let score = session.score;
    let players = session.players;
    let team_name_ct = session.team_name_ct;
    let team_name_t = session.team_name_t;
    let need_attention = session.need_attention;

    // -------------------------------------------------------------------------
    // Session state
    // -------------------------------------------------------------------------

    let current_status = status();
    let is_paused = match_paused();
    let current_score = score();
    let current_players = players();

    let current_need_attention = need_attention();

    let current_team_name_ct = team_name_ct();
    let current_team_name_t = team_name_t();

    let player_count = current_players.players().len();

    // -------------------------------------------------------------------------
    // Server information
    // -------------------------------------------------------------------------

    let server = {
        let servers = state.servers.read();

        servers.get(&addr).cloned()
    };

    let hostname = server
        .as_ref()
        .and_then(|server| server.hostname.clone())
        .unwrap_or_else(|| addr.to_string());

    let current_map = server
        .as_ref()
        .and_then(|server| server.map.clone())
        .unwrap_or_else(|| "UNKNOWN".to_string());

    // -------------------------------------------------------------------------
    // Team names
    // -------------------------------------------------------------------------

    let display_team_name_ct = if current_team_name_ct.is_empty() {
        "COUNTER-TERRORISTS"
    } else {
        current_team_name_ct.as_str()
    };

    let display_team_name_t = if current_team_name_t.is_empty() {
        "TERRORISTS"
    } else {
        current_team_name_t.as_str()
    };

    // -------------------------------------------------------------------------
    // Status
    // -------------------------------------------------------------------------

    let (status_text, status_class) = match current_status {
        RconStatus::Authenticated if is_paused => ("● PAUSED", "text-orange-400"),

        RconStatus::Authenticated => ("● RUNNING", "text-emerald-500"),

        RconStatus::Connecting => ("● CONNECTING", "text-blue-500"),

        RconStatus::Disconnected => ("● OFFLINE", "text-yellow-600"),

        RconStatus::Error => ("● ERROR", "text-red-500"),
    };

    // -------------------------------------------------------------------------
    // Player names
    // -------------------------------------------------------------------------

    let player_names: Vec<String> = current_players
        .players()
        .values()
        .filter(|player| !player.name.is_empty())
        .map(|player| player.name.clone())
        .collect();

    // -------------------------------------------------------------------------
    // Card styling
    // -------------------------------------------------------------------------

    let card_class = if current_need_attention {
        "
            bg-zinc-900/70
            border
            border-red-500
            rounded-lg
            p-5
            hover:border-red-400
            transition-colors
            cursor-pointer
        "
    } else {
        "
            bg-zinc-900/70
            border
            border-zinc-800
            rounded-lg
            p-5
            hover:border-indigo-500
            transition-colors
            cursor-pointer
        "
    };

    rsx! {
        div {
            class: card_class,

            // ================================================================
            // SELECT SERVER / CLEAR ATTENTION
            // ================================================================

            onclick: move |_| {
                selected_rcon.set(Some(addr));
            },

            // ================================================================
            // HEADER
            // ================================================================

            div {
                class: "flex items-start justify-between gap-3",

                div {
                    class: "min-w-0 flex items-center gap-2",

                    // --------------------------------------------------------
                    // ATTENTION INDICATOR
                    // --------------------------------------------------------

                    if current_need_attention {
                        span {
                            class: "shrink-0 text-red-500 text-sm font-black animate-pulse",
                            title: "ADMIN MESSAGE",
                            "⚠"
                        }
                    }

                    div {
                        class: "min-w-0",

                        div {
                            class: "text-white text-sm font-black truncate",
                            "{hostname}"
                        }

                        div {
                            class: "text-zinc-600 text-[9px] font-mono mt-1",
                            "{addr}"
                        }
                    }
                }

                span {
                    class: "shrink-0 text-[9px] font-black {status_class}",
                    "{status_text}"
                }
            }

            // ================================================================
            // MAP
            // ================================================================

            div {
                class: "mt-5",

                div {
                    class: "text-zinc-600 text-[8px] font-black tracking-widest",
                    "MAP"
                }

                div {
                    class: "text-zinc-200 text-sm font-black mt-1 truncate",
                    "{current_map}"
                }
            }

            // ================================================================
            // TEAMS / SCORE
            // ================================================================

            div {
                class: "grid grid-cols-2 gap-3 mt-5",

                // ------------------------------------------------------------
                // CT
                // ------------------------------------------------------------

                div {
                    class: "bg-zinc-950/70 rounded p-3 text-center min-w-0",

                    div {
                        class: "text-blue-400 text-[9px] font-black uppercase tracking-wider truncate",
                        "{display_team_name_ct}"
                    }

                    div {
                        class: "text-blue-300 text-2xl font-black mt-1",
                        "{current_score.ct}"
                    }
                }

                // ------------------------------------------------------------
                // T
                // ------------------------------------------------------------

                div {
                    class: "bg-zinc-950/70 rounded p-3 text-center min-w-0",

                    div {
                        class: "text-red-400 text-[9px] font-black uppercase tracking-wider truncate",
                        "{display_team_name_t}"
                    }

                    div {
                        class: "text-red-300 text-2xl font-black mt-1",
                        "{current_score.t}"
                    }
                }
            }

            // ================================================================
            // PLAYERS
            // ================================================================

            div {
                class: "mt-4 pt-3 border-t border-zinc-800",

                div {
                    class: "flex items-center justify-between mb-2",

                    span {
                        class: "text-zinc-500 text-[9px] font-bold",
                        "PLAYERS"
                    }

                    span {
                        class: "text-zinc-300 text-[10px] font-black",
                        "{player_count}"
                    }
                }

                div {
                    class: "space-y-1",

                    if player_names.is_empty() {
                        div {
                            class: "text-zinc-700 text-[9px] py-1",
                            "NO PLAYER DATA"
                        }
                    } else {
                        for player_name in player_names {
                            div {
                                class: "text-zinc-300 text-[10px] font-bold truncate",
                                "{player_name}"
                            }
                        }
                    }
                }
            }
        }
    }
}
