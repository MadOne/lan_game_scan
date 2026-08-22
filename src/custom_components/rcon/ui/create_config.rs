use dioxus::prelude::*;
use if_addrs::{get_if_addrs, IfAddr};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use cbz_rcon::RconStatus;

use serde::Serialize;

use crate::custom_components::code::{RconLogEvent, RconPlayers, Team, TeamScore};

// =============================================================================
// Steam ID helpers
// =============================================================================

/// Converts a Steam2 ID into a Steam64 ID.
///
/// Steam2:
///     [U:1:55530433]
///
/// Steam64:
///     76561197960265728 + 55530433
///     = 76561198015796161
///
/// If the supplied value is already a Steam64 ID, it is returned unchanged.
///
/// If the value cannot be parsed, None is returned.
fn steamid_to_steam64(steamid: &str) -> Option<String> {
    let steamid = steamid.trim();

    // Already a Steam64 ID.
    if let Ok(id) = steamid.parse::<u64>() {
        // Steam64 IDs are much larger than Steam account IDs.
        if id >= 76561197960265728 {
            return Some(id.to_string());
        }

        // Allow a raw account ID as well.
        return Some((76561197960265728u64 + id).to_string());
    }

    // Steam2 format:
    //
    // [U:1:55530433]
    //
    if let Some(account_id) = steamid
        .strip_prefix("[U:1:")
        .and_then(|value| value.strip_suffix(']'))
        .and_then(|value| value.parse::<u64>().ok())
    {
        return Some((76561197960265728u64 + account_id).to_string());
    }

    None
}

// =============================================================================
// Network helpers
// =============================================================================

/// Finds the local IPv4 address that is on the same subnet as the
/// specified server address.
///
/// This is used for the MatchZy configuration URL.
///
/// Example:
///
///     server: 192.168.178.50:27015
///     local:  192.168.178.20
///
/// returns:
///
///     Some(192.168.178.20)
fn log_receiver_ip(server_addr: SocketAddr) -> Option<Ipv4Addr> {
    let server_ip = match server_addr.ip() {
        IpAddr::V4(ip) => ip,
        IpAddr::V6(_) => return None,
    };

    let interfaces = get_if_addrs().ok()?;

    for interface in interfaces {
        let IfAddr::V4(addr) = interface.addr else {
            continue;
        };

        // Ignore loopback.
        if addr.ip.is_loopback() {
            continue;
        }

        // Check whether the server belongs to this interface's subnet.
        if same_subnet(server_ip, addr.ip, addr.netmask) {
            return Some(addr.ip);
        }
    }

    None
}

fn same_subnet(a: Ipv4Addr, b: Ipv4Addr, netmask: Ipv4Addr) -> bool {
    let a = u32::from(a);
    let b = u32::from(b);
    let mask = u32::from(netmask);

    (a & mask) == (b & mask)
}

// =============================================================================
// Match configuration
// =============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct MatchConfig {
    pub team1: MatchTeam,
    pub team2: MatchTeam,

    pub num_maps: u8,

    pub maplist: Vec<String>,
    pub map_ban_order: Vec<String>,

    pub skip_veto: bool,

    pub players_per_team: u8,
    pub series_can_clinch: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchTeam {
    pub name: String,
    pub players: HashMap<String, String>,
}

// =============================================================================
// Component
// =============================================================================

#[component]
pub fn CreateConfig(
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
    // -------------------------------------------------------------------------
    // Player data
    //
    // Clone the live player information into owned values.
    // -------------------------------------------------------------------------

    let current_players = players();

    let ct_players: Vec<(String, String)> = current_players
        .players()
        .values()
        .filter(|player| player.team == Team::CT)
        .map(|player| {
            (
                player.steamid.to_string(),
                if player.name.is_empty() {
                    "UNKNOWN".to_string()
                } else {
                    player.name.clone()
                },
            )
        })
        .collect();

    let t_players: Vec<(String, String)> = current_players
        .players()
        .values()
        .filter(|player| player.team == Team::Terrorist)
        .map(|player| {
            (
                player.steamid.to_string(),
                if player.name.is_empty() {
                    "UNKNOWN".to_string()
                } else {
                    player.name.clone()
                },
            )
        })
        .collect();

    let current_score = score();

    // -------------------------------------------------------------------------
    // Team configuration
    // -------------------------------------------------------------------------

    let mut ct_team_name = use_signal(String::new);
    let mut t_team_name = use_signal(String::new);

    // -------------------------------------------------------------------------
    // Captain selection
    // -------------------------------------------------------------------------

    let mut ct_captain = use_signal(|| None::<String>);
    let mut t_captain = use_signal(|| None::<String>);

    // -------------------------------------------------------------------------
    // Missing player fields
    // -------------------------------------------------------------------------

    let mut ct_extra_1_steamid = use_signal(String::new);
    let mut ct_extra_1_name = use_signal(String::new);

    let mut ct_extra_2_steamid = use_signal(String::new);
    let mut ct_extra_2_name = use_signal(String::new);

    let mut ct_extra_3_steamid = use_signal(String::new);
    let mut ct_extra_3_name = use_signal(String::new);

    let mut t_extra_1_steamid = use_signal(String::new);
    let mut t_extra_1_name = use_signal(String::new);

    let mut t_extra_2_steamid = use_signal(String::new);
    let mut t_extra_2_name = use_signal(String::new);

    let mut t_extra_3_steamid = use_signal(String::new);
    let mut t_extra_3_name = use_signal(String::new);

    // -------------------------------------------------------------------------
    // Captain state for manually entered players
    // -------------------------------------------------------------------------

    let mut ct_extra_1_captain = use_signal(|| false);
    let mut ct_extra_2_captain = use_signal(|| false);
    let mut ct_extra_3_captain = use_signal(|| false);

    let mut t_extra_1_captain = use_signal(|| false);
    let mut t_extra_2_captain = use_signal(|| false);
    let mut t_extra_3_captain = use_signal(|| false);

    // -------------------------------------------------------------------------
    // Generated / editable JSON
    // -------------------------------------------------------------------------

    let mut generated_config = use_signal(String::new);

    // -------------------------------------------------------------------------
    // Match settings
    // -------------------------------------------------------------------------

    let players_per_team = 3u8;

    let maplist: Vec<String> = vec![
        "de_ancient".into(),
        "de_anubis".into(),
        "de_dust2".into(),
        "de_inferno".into(),
        "de_mirage".into(),
        "de_nuke".into(),
        "de_overpass".into(),
    ];

    // -------------------------------------------------------------------------
    // Generate config
    // -------------------------------------------------------------------------

    let mut generate_config = {
        let ct_players = ct_players.clone();
        let t_players = t_players.clone();

        move || {
            let mut team1_players = HashMap::<String, String>::new();
            let mut team2_players = HashMap::<String, String>::new();

            // ================================================================
            // CT / TEAM 1 - LIVE PLAYERS
            // ================================================================

            for (steamid, name) in &ct_players {
                if let Some(steam64) = steamid_to_steam64(steamid) {
                    team1_players.insert(steam64, name.clone());
                }
            }

            // ================================================================
            // CT / EXTRA PLAYERS
            // ================================================================

            let ct_extras = [
                (ct_extra_1_steamid(), ct_extra_1_name()),
                (ct_extra_2_steamid(), ct_extra_2_name()),
                (ct_extra_3_steamid(), ct_extra_3_name()),
            ];

            for (extra_steamid, extra_name) in ct_extras {
                if team1_players.len() >= players_per_team as usize {
                    break;
                }

                if let Some(steam64) = steamid_to_steam64(&extra_steamid) {
                    team1_players.insert(
                        steam64,
                        if extra_name.trim().is_empty() {
                            "UNKNOWN".to_string()
                        } else {
                            extra_name
                        },
                    );
                }
            }

            // ================================================================
            // T / TEAM 2 - LIVE PLAYERS
            // ================================================================

            for (steamid, name) in &t_players {
                if let Some(steam64) = steamid_to_steam64(steamid) {
                    team2_players.insert(steam64, name.clone());
                }
            }

            // ================================================================
            // T / EXTRA PLAYERS
            // ================================================================

            let t_extras = [
                (t_extra_1_steamid(), t_extra_1_name()),
                (t_extra_2_steamid(), t_extra_2_name()),
                (t_extra_3_steamid(), t_extra_3_name()),
            ];

            for (extra_steamid, extra_name) in t_extras {
                if team2_players.len() >= players_per_team as usize {
                    break;
                }

                if let Some(steam64) = steamid_to_steam64(&extra_steamid) {
                    team2_players.insert(
                        steam64,
                        if extra_name.trim().is_empty() {
                            "UNKNOWN".to_string()
                        } else {
                            extra_name
                        },
                    );
                }
            }

            // ================================================================
            // Build MatchZy config
            // ================================================================

            let config = MatchConfig {
                team1: MatchTeam {
                    name: ct_team_name().trim().to_string(),
                    players: team1_players,
                },

                team2: MatchTeam {
                    name: t_team_name().trim().to_string(),
                    players: team2_players,
                },

                num_maps: 3,

                maplist: maplist.clone(),

                map_ban_order: vec![
                    "team1_ban".into(),
                    "team2_ban".into(),
                    "team1_pick".into(),
                    "team2_pick".into(),
                ],

                skip_veto: false,

                players_per_team,

                series_can_clinch: true,
            };

            // Pretty JSON for the editable text window.
            match serde_json::to_string_pretty(&config) {
                Ok(json) => {
                    generated_config.set(json);
                }

                Err(error) => {
                    generated_config.set(format!("ERROR GENERATING CONFIG:\n\n{}", error));
                }
            }
        }
    };

    // -------------------------------------------------------------------------
    // Number of missing players
    // -------------------------------------------------------------------------

    let ct_missing = players_per_team.saturating_sub(ct_players.len() as u8);

    let t_missing = players_per_team.saturating_sub(t_players.len() as u8);

    // -------------------------------------------------------------------------
    // UI
    // -------------------------------------------------------------------------

    rsx! {
        div {
            class: "flex flex-col h-full min-h-0 bg-zinc-950",

            // =================================================================
            // SERVER CONTROL BAR
            // =================================================================

            div {
                class: "shrink-0 bg-zinc-900 border-b border-zinc-800 px-5 py-3",

                div {
                    class: "text-white font-black text-sm truncate",
                    "{hostname}"
                }
            }

            // =================================================================
            // MAIN CONFIGURATION
            // =================================================================

            div {
                class: "flex-1 min-h-0 flex",

                // =============================================================
                // TEAMS
                // =============================================================

                div {
                    class: "flex-1 min-w-0 p-6 overflow-y-auto",

                    div {
                        class: "grid grid-cols-2 gap-8",

                        // =====================================================
                        // TEAM 1 / CT
                        // =====================================================

                        div {
                            class: "flex flex-col min-w-0",

                            div {
                                class: "text-center mb-5",

                                div {
                                    class: "text-blue-400 font-black text-[11px] tracking-widest uppercase",
                                    "TEAM 1"
                                }

                                input {
                                    class: "mt-3 w-full bg-zinc-900 border border-zinc-800 focus:border-blue-500 rounded px-3 py-2 text-center text-blue-200 font-bold text-sm outline-none transition-colors placeholder:text-zinc-700",

                                    r#type: "text",
                                    placeholder: "ENTER TEAM NAME",
                                    value: "{ct_team_name}",

                                    oninput: move |event| {
                                        ct_team_name.set(event.value());
                                    }
                                }

                                div {
                                    class: "text-blue-400 text-5xl font-black mt-2",
                                    "{current_score.ct}"
                                }

                                div {
                                    class: "text-[9px] text-zinc-600 mt-1",
                                    "{ct_players.len()} / 3 PLAYERS"
                                }
                            }

                            div {
                                class: "space-y-2",

                                // -------------------------------------------------
                                // Player header
                                // -------------------------------------------------

                                div {
                                    class: "flex items-center gap-3 px-4 pb-1",

                                    div {
                                        class: "w-3.5 shrink-0"
                                    }

                                    div {
                                        class: "flex-1 text-[8px] font-black tracking-widest text-zinc-700",
                                        "PLAYER / STEAMID"
                                    }

                                    div {
                                        class: "w-16 text-center text-[8px] font-black tracking-widest text-zinc-700",
                                        "CAPTAIN"
                                    }
                                }

                                // -------------------------------------------------
                                // Live CT players
                                // -------------------------------------------------

                                for (steamid, name) in ct_players.iter() {
                                    {
                                        let player_id = steamid.clone();

                                        let is_captain = ct_captain()
                                            .as_deref()
                                            == Some(player_id.as_str());

                                        rsx! {
                                            div {
                                                class: if is_captain {
                                                    "flex items-center gap-3 px-4 py-2 bg-blue-950/30 border border-blue-800/70 rounded"
                                                } else {
                                                    "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded"
                                                },

                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-3.5 h-3.5 accent-blue-500 cursor-pointer shrink-0",
                                                    checked: is_captain,

                                                    onchange: move |_| {
                                                        if is_captain {
                                                            ct_captain.set(None);
                                                        } else {
                                                            ct_captain.set(Some(player_id.clone()));
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "flex-1 min-w-0",

                                                    div {
                                                        class: if is_captain {
                                                            "text-blue-200 font-bold truncate"
                                                        } else {
                                                            "text-blue-300 font-bold truncate"
                                                        },

                                                        "{name}"
                                                    }

                                                    div {
                                                        class: "text-[9px] text-zinc-600 font-mono truncate mt-0.5",

                                                        if let Some(steam64) =
                                                            steamid_to_steam64(steamid)
                                                        {
                                                            "{steam64}"
                                                        } else {
                                                            "{steamid}"
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "w-16 text-center",

                                                    if is_captain {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-blue-400",
                                                            "CAPTAIN"
                                                        }
                                                    } else {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-zinc-800",
                                                            "—"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // -------------------------------------------------
                                // Missing CT player 1
                                // -------------------------------------------------

                                if ct_missing >= 1 {
                                    {
                                        let is_captain = ct_extra_1_captain();

                                        rsx! {
                                            div {
                                                class: if is_captain {
                                                    "flex items-center gap-3 px-4 py-2 bg-blue-950/30 border border-blue-800/70 rounded"
                                                } else {
                                                    "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded"
                                                },

                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-3.5 h-3.5 accent-blue-500 cursor-pointer shrink-0",
                                                    checked: is_captain,

                                                    onchange: move |_| {
                                                        ct_extra_1_captain.set(!is_captain);
                                                    }
                                                }

                                                div {
                                                    class: "flex-1 min-w-0 grid grid-cols-2 gap-2",

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-blue-500 rounded px-3 py-1.5 text-blue-200 text-xs font-mono outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "STEAMID",
                                                        value: "{ct_extra_1_steamid}",

                                                        oninput: move |event| {
                                                            ct_extra_1_steamid.set(event.value());
                                                        }
                                                    }

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-blue-500 rounded px-3 py-1.5 text-blue-200 text-xs outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "PLAYER NAME",
                                                        value: "{ct_extra_1_name}",

                                                        oninput: move |event| {
                                                            ct_extra_1_name.set(event.value());
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "w-16 text-center",

                                                    if is_captain {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-blue-400",
                                                            "CAPTAIN"
                                                        }
                                                    } else {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-zinc-800",
                                                            "—"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // -------------------------------------------------
                                // Missing CT player 2
                                // -------------------------------------------------

                                if ct_missing >= 2 {
                                    {
                                        let is_captain = ct_extra_2_captain();

                                        rsx! {
                                            div {
                                                class: if is_captain {
                                                    "flex items-center gap-3 px-4 py-2 bg-blue-950/30 border border-blue-800/70 rounded"
                                                } else {
                                                    "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded"
                                                },

                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-3.5 h-3.5 accent-blue-500 cursor-pointer shrink-0",
                                                    checked: is_captain,

                                                    onchange: move |_| {
                                                        ct_extra_2_captain.set(!is_captain);
                                                    }
                                                }

                                                div {
                                                    class: "flex-1 min-w-0 grid grid-cols-2 gap-2",

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-blue-500 rounded px-3 py-1.5 text-blue-200 text-xs font-mono outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "STEAMID",
                                                        value: "{ct_extra_2_steamid}",

                                                        oninput: move |event| {
                                                            ct_extra_2_steamid.set(event.value());
                                                        }
                                                    }

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-blue-500 rounded px-3 py-1.5 text-blue-200 text-xs outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "PLAYER NAME",
                                                        value: "{ct_extra_2_name}",

                                                        oninput: move |event| {
                                                            ct_extra_2_name.set(event.value());
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "w-16 text-center",

                                                    if is_captain {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-blue-400",
                                                            "CAPTAIN"
                                                        }
                                                    } else {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-zinc-800",
                                                            "—"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // -------------------------------------------------
                                // Missing CT player 3
                                // -------------------------------------------------

                                if ct_missing >= 3 {
                                    {
                                        let is_captain = ct_extra_3_captain();

                                        rsx! {
                                            div {
                                                class: if is_captain {
                                                    "flex items-center gap-3 px-4 py-2 bg-blue-950/30 border border-blue-800/70 rounded"
                                                } else {
                                                    "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded"
                                                },

                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-3.5 h-3.5 accent-blue-500 cursor-pointer shrink-0",
                                                    checked: is_captain,

                                                    onchange: move |_| {
                                                        ct_extra_3_captain.set(!is_captain);
                                                    }
                                                }

                                                div {
                                                    class: "flex-1 min-w-0 grid grid-cols-2 gap-2",

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-blue-500 rounded px-3 py-1.5 text-blue-200 text-xs font-mono outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "STEAMID",
                                                        value: "{ct_extra_3_steamid}",

                                                        oninput: move |event| {
                                                            ct_extra_3_steamid.set(event.value());
                                                        }
                                                    }

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-blue-500 rounded px-3 py-1.5 text-blue-200 text-xs outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "PLAYER NAME",
                                                        value: "{ct_extra_3_name}",

                                                        oninput: move |event| {
                                                            ct_extra_3_name.set(event.value());
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "w-16 text-center",

                                                    if is_captain {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-blue-400",
                                                            "CAPTAIN"
                                                        }
                                                    } else {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-zinc-800",
                                                            "—"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if ct_players.is_empty() && ct_missing == 3 {
                                    div {
                                        class: "text-center text-zinc-700 text-[10px] py-4",
                                        "NO LIVE PLAYER DATA — ENTER PLAYERS BELOW"
                                    }
                                }
                            }
                        }

                        // =====================================================
                        // TEAM 2 / T
                        // =====================================================

                        div {
                            class: "flex flex-col min-w-0",

                            div {
                                class: "text-center mb-5",

                                div {
                                    class: "text-red-400 font-black text-[11px] tracking-widest uppercase",
                                    "TEAM 2"
                                }

                                input {
                                    class: "mt-3 w-full bg-zinc-900 border border-zinc-800 focus:border-red-500 rounded px-3 py-2 text-center text-red-200 font-bold text-sm outline-none transition-colors placeholder:text-zinc-700",

                                    r#type: "text",
                                    placeholder: "ENTER TEAM NAME",
                                    value: "{t_team_name}",

                                    oninput: move |event| {
                                        t_team_name.set(event.value());
                                    }
                                }

                                div {
                                    class: "text-red-400 text-5xl font-black mt-2",
                                    "{current_score.t}"
                                }

                                div {
                                    class: "text-[9px] text-zinc-600 mt-1",
                                    "{t_players.len()} / 3 PLAYERS"
                                }
                            }

                            div {
                                class: "space-y-2",

                                // -------------------------------------------------
                                // Player header
                                // -------------------------------------------------

                                div {
                                    class: "flex items-center gap-3 px-4 pb-1",

                                    div {
                                        class: "w-3.5 shrink-0"
                                    }

                                    div {
                                        class: "flex-1 text-[8px] font-black tracking-widest text-zinc-700",
                                        "PLAYER / STEAMID"
                                    }

                                    div {
                                        class: "w-16 text-center text-[8px] font-black tracking-widest text-zinc-700",
                                        "CAPTAIN"
                                    }
                                }

                                // -------------------------------------------------
                                // Live T players
                                // -------------------------------------------------

                                for (steamid, name) in t_players.iter() {
                                    {
                                        let player_id = steamid.clone();

                                        let is_captain = t_captain()
                                            .as_deref()
                                            == Some(player_id.as_str());

                                        rsx! {
                                            div {
                                                class: if is_captain {
                                                    "flex items-center gap-3 px-4 py-2 bg-red-950/30 border border-red-800/70 rounded"
                                                } else {
                                                    "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded"
                                                },

                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-3.5 h-3.5 accent-red-500 cursor-pointer shrink-0",
                                                    checked: is_captain,

                                                    onchange: move |_| {
                                                        if is_captain {
                                                            t_captain.set(None);
                                                        } else {
                                                            t_captain.set(Some(player_id.clone()));
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "flex-1 min-w-0",

                                                    div {
                                                        class: if is_captain {
                                                            "text-red-200 font-bold truncate"
                                                        } else {
                                                            "text-red-300 font-bold truncate"
                                                        },

                                                        "{name}"
                                                    }

                                                    div {
                                                        class: "text-[9px] text-zinc-600 font-mono truncate mt-0.5",

                                                        if let Some(steam64) =
                                                            steamid_to_steam64(steamid)
                                                        {
                                                            "{steam64}"
                                                        } else {
                                                            "{steamid}"
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "w-16 text-center",

                                                    if is_captain {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-red-400",
                                                            "CAPTAIN"
                                                        }
                                                    } else {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-zinc-800",
                                                            "—"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // -------------------------------------------------
                                // Missing T player 1
                                // -------------------------------------------------

                                if t_missing >= 1 {
                                    {
                                        let is_captain = t_extra_1_captain();

                                        rsx! {
                                            div {
                                                class: if is_captain {
                                                    "flex items-center gap-3 px-4 py-2 bg-red-950/30 border border-red-800/70 rounded"
                                                } else {
                                                    "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded"
                                                },

                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-3.5 h-3.5 accent-red-500 cursor-pointer shrink-0",
                                                    checked: is_captain,

                                                    onchange: move |_| {
                                                        t_extra_1_captain.set(!is_captain);
                                                    }
                                                }

                                                div {
                                                    class: "flex-1 min-w-0 grid grid-cols-2 gap-2",

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-red-500 rounded px-3 py-1.5 text-red-200 text-xs font-mono outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "STEAMID",
                                                        value: "{t_extra_1_steamid}",

                                                        oninput: move |event| {
                                                            t_extra_1_steamid.set(event.value());
                                                        }
                                                    }

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-red-500 rounded px-3 py-1.5 text-red-200 text-xs outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "PLAYER NAME",
                                                        value: "{t_extra_1_name}",

                                                        oninput: move |event| {
                                                            t_extra_1_name.set(event.value());
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "w-16 text-center",

                                                    if is_captain {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-red-400",
                                                            "CAPTAIN"
                                                        }
                                                    } else {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-zinc-800",
                                                            "—"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // -------------------------------------------------
                                // Missing T player 2
                                // -------------------------------------------------

                                if t_missing >= 2 {
                                    {
                                        let is_captain = t_extra_2_captain();

                                        rsx! {
                                            div {
                                                class: if is_captain {
                                                    "flex items-center gap-3 px-4 py-2 bg-red-950/30 border border-red-800/70 rounded"
                                                } else {
                                                    "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded"
                                                },

                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-3.5 h-3.5 accent-red-500 cursor-pointer shrink-0",
                                                    checked: is_captain,

                                                    onchange: move |_| {
                                                        t_extra_2_captain.set(!is_captain);
                                                    }
                                                }

                                                div {
                                                    class: "flex-1 min-w-0 grid grid-cols-2 gap-2",

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-red-500 rounded px-3 py-1.5 text-red-200 text-xs font-mono outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "STEAMID",
                                                        value: "{t_extra_2_steamid}",

                                                        oninput: move |event| {
                                                            t_extra_2_steamid.set(event.value());
                                                        }
                                                    }

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-red-500 rounded px-3 py-1.5 text-red-200 text-xs outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "PLAYER NAME",
                                                        value: "{t_extra_2_name}",

                                                        oninput: move |event| {
                                                            t_extra_2_name.set(event.value());
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "w-16 text-center",

                                                    if is_captain {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-red-400",
                                                            "CAPTAIN"
                                                        }
                                                    } else {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-zinc-800",
                                                            "—"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // -------------------------------------------------
                                // Missing T player 3
                                // -------------------------------------------------

                                if t_missing >= 3 {
                                    {
                                        let is_captain = t_extra_3_captain();

                                        rsx! {
                                            div {
                                                class: if is_captain {
                                                    "flex items-center gap-3 px-4 py-2 bg-red-950/30 border border-red-800/70 rounded"
                                                } else {
                                                    "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded"
                                                },

                                                input {
                                                    r#type: "checkbox",
                                                    class: "w-3.5 h-3.5 accent-red-500 cursor-pointer shrink-0",
                                                    checked: is_captain,

                                                    onchange: move |_| {
                                                        t_extra_3_captain.set(!is_captain);
                                                    }
                                                }

                                                div {
                                                    class: "flex-1 min-w-0 grid grid-cols-2 gap-2",

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-red-500 rounded px-3 py-1.5 text-red-200 text-xs font-mono outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "STEAMID",
                                                        value: "{t_extra_3_steamid}",

                                                        oninput: move |event| {
                                                            t_extra_3_steamid.set(event.value());
                                                        }
                                                    }

                                                    input {
                                                        class: "w-full bg-zinc-950 border border-zinc-800 focus:border-red-500 rounded px-3 py-1.5 text-red-200 text-xs outline-none placeholder:text-zinc-700",

                                                        r#type: "text",
                                                        placeholder: "PLAYER NAME",
                                                        value: "{t_extra_3_name}",

                                                        oninput: move |event| {
                                                            t_extra_3_name.set(event.value());
                                                        }
                                                    }
                                                }

                                                div {
                                                    class: "w-16 text-center",

                                                    if is_captain {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-red-400",
                                                            "CAPTAIN"
                                                        }
                                                    } else {
                                                        span {
                                                            class: "text-[8px] font-black tracking-widest text-zinc-800",
                                                            "—"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if t_players.is_empty() && t_missing == 3 {
                                    div {
                                        class: "text-center text-zinc-700 text-[10px] py-4",
                                        "NO LIVE PLAYER DATA — ENTER PLAYERS BELOW"
                                    }
                                }
                            }
                        }
                    }
                }

                // =============================================================
                // CONFIGURATION
                // =============================================================

                div {
                    class: "w-[42%] min-w-[420px] border-l border-zinc-800 bg-zinc-950 flex flex-col min-h-0",

                    // ---------------------------------------------------------
                    // Configuration header
                    // ---------------------------------------------------------

                    div {
                        class: "shrink-0 flex items-center justify-between px-5 py-4 border-b border-zinc-800",

                        div {
                            class: "text-zinc-400 text-[10px] font-black tracking-widest uppercase",
                            "MATCHZY CONFIGURATION"
                        }

                        div {
                            class: "flex items-center gap-2",

                            // Generate
                            button {
                                class: "px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-[10px] font-black tracking-widest rounded transition-colors",

                                onclick: move |_| {
                                    generate_config();
                                },

                                "GENERATE"
                            }

                            // Publish
                            button {
                                class: "px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white text-[10px] font-black tracking-widest rounded transition-colors",

                                onclick: move |_| {
                                    let config = generated_config();
                                    let server_addr = addr;

                                    // -------------------------------------------------
                                    // Find the local IP that can reach this server.
                                    // -------------------------------------------------

                                    let Some(receiver_ip) =
                                        log_receiver_ip(server_addr)
                                    else {
                                        eprintln!(
                                            "Could not determine local interface for server {}",
                                            server_addr
                                        );

                                        return;
                                    };

                                    let config_url = format!(
                                        "http://{}:7131/MatchZyConfig",
                                        receiver_ip
                                    );

                                    // -------------------------------------------------
                                    // Publish asynchronously.
                                    //
                                    // IMPORTANT:
                                    //
                                    // The RCON command is sent only after the POST
                                    // succeeded. This guarantees that MatchZy cannot
                                    // fetch the old/missing configuration.
                                    // -------------------------------------------------

                                    spawn(async move {
                                        match reqwest::Client::new()
                                            .post(&config_url)
                                            .header(
                                                "Content-Type",
                                                "application/json",
                                            )
                                            .body(config)
                                            .send()
                                            .await
                                        {
                                            Ok(response)
                                                if response.status().is_success() =>
                                            {
                                                // Tell MatchZy to fetch the
                                                // configuration we just published.
                                                on_command.call(format!(
                                                    r#"matchzy_loadmatch_url "{}""#,
                                                    config_url
                                                ));
                                            }

                                            Ok(response) => {
                                                eprintln!(
                                                    "Failed to publish MatchZy config: HTTP {}",
                                                    response.status()
                                                );
                                            }

                                            Err(error) => {
                                                eprintln!(
                                                    "Failed to publish MatchZy config: {}",
                                                    error
                                                );
                                            }
                                        }
                                    });
                                },

                                "PUBLISH"
                            }
                        }
                    }

                    // ---------------------------------------------------------
                    // Editable configuration
                    // ---------------------------------------------------------

                    textarea {
                        class: "flex-1 min-h-0 w-full bg-black p-4 text-emerald-400 font-mono text-xs outline-none resize-none",

                        value: "{generated_config}",

                        oninput: move |event| {
                            generated_config.set(event.value());
                        }
                    }
                }
            }
        }
    }
}
