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

fn steamid_to_steam64(steamid: &str) -> Option<String> {
    let steamid = steamid.trim();
    if let Ok(id) = steamid.parse::<u64>() {
        if id >= 76561197960265728 {
            return Some(id.to_string());
        }
        return Some((76561197960265728u64 + id).to_string());
    }
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
        if addr.ip.is_loopback() {
            continue;
        }
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
    let _ = (
        &map,
        &status,
        player_count,
        player_max,
        &logs,
        &paused,
        &maps,
        &get_maps,
    );

    let mut show_mobile_config = use_signal(|| false);
    let mut generated_config = use_signal(String::new);

    // Live Player data
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

    // Teams state
    let mut ct_team_name = use_signal(String::new);
    let mut t_team_name = use_signal(String::new);
    //let ct_captain = use_signal(|| None::<String>);
    //let captain = use_signal(|| None::<String>);

    // Manual Entry state
    let mut ct_extra_1_sid = use_signal(String::new);
    let mut ct_extra_1_name = use_signal(String::new);
    let mut ct_extra_2_sid = use_signal(String::new);
    let mut ct_extra_2_name = use_signal(String::new);
    let mut ct_extra_3_sid = use_signal(String::new);
    let mut ct_extra_3_name = use_signal(String::new);

    let mut t_extra_1_sid = use_signal(String::new);
    let mut t_extra_1_name = use_signal(String::new);
    let mut t_extra_2_sid = use_signal(String::new);
    let mut t_extra_2_name = use_signal(String::new);
    let mut t_extra_3_sid = use_signal(String::new);
    let mut t_extra_3_name = use_signal(String::new);

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

    let mut generate_config = {
        let ct_players = ct_players.clone();
        let t_players = t_players.clone();
        move || {
            let mut team1_players = HashMap::new();
            let mut team2_players = HashMap::new();

            for (sid, name) in &ct_players {
                if let Some(s64) = steamid_to_steam64(sid) {
                    team1_players.insert(s64, name.clone());
                }
            }
            let ct_extras = [
                (ct_extra_1_sid(), ct_extra_1_name()),
                (ct_extra_2_sid(), ct_extra_2_name()),
                (ct_extra_3_sid(), ct_extra_3_name()),
            ];
            for (sid, name) in ct_extras {
                if team1_players.len() >= players_per_team as usize {
                    break;
                }
                if let Some(s64) = steamid_to_steam64(&sid) {
                    team1_players.insert(
                        s64,
                        if name.trim().is_empty() {
                            "UNKNOWN".to_string()
                        } else {
                            name
                        },
                    );
                }
            }

            for (sid, name) in &t_players {
                if let Some(s64) = steamid_to_steam64(sid) {
                    team2_players.insert(s64, name.clone());
                }
            }
            let t_extras = [
                (t_extra_1_sid(), t_extra_1_name()),
                (t_extra_2_sid(), t_extra_2_name()),
                (t_extra_3_sid(), t_extra_3_name()),
            ];
            for (sid, name) in t_extras {
                if team2_players.len() >= players_per_team as usize {
                    break;
                }
                if let Some(s64) = steamid_to_steam64(&sid) {
                    team2_players.insert(
                        s64,
                        if name.trim().is_empty() {
                            "UNKNOWN".to_string()
                        } else {
                            name
                        },
                    );
                }
            }

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

            if let Ok(json) = serde_json::to_string_pretty(&config) {
                generated_config.set(json);
            }
        }
    };

    let ct_missing = players_per_team.saturating_sub(ct_players.len() as u8);
    let t_missing = players_per_team.saturating_sub(t_players.len() as u8);

    rsx! {
        div { class: "flex flex-col h-full min-h-0 bg-zinc-950 relative overflow-hidden",

            // Header
            div { class: "shrink-0 bg-zinc-900 border-b border-zinc-800 px-5 py-3",
                div { class: "text-white font-black text-sm truncate", "{hostname}" }
            }

            // Body
            div { class: "flex-1 min-h-0 flex",

                // Left: Teams
                div { class: "flex-1 min-w-0 p-6 overflow-y-auto",
                    div { class: "grid grid-cols-1 lg:grid-cols-2 gap-8",

                        // Team 1
                        div { class: "flex flex-col",
                            div { class: "text-center mb-5",
                                div { class: "text-blue-400 font-black text-[11px] tracking-widest uppercase", "TEAM 1" }
                                input {
                                    class: "mt-3 w-full bg-zinc-900 border border-zinc-800 focus:border-blue-500 rounded px-3 py-2 text-center text-blue-200 font-bold text-sm outline-none placeholder:text-zinc-700",
                                    placeholder: "ENTER TEAM NAME",
                                    value: "{ct_team_name}",
                                    oninput: move |e| ct_team_name.set(e.value())
                                }
                                div { class: "text-blue-400 text-5xl font-black mt-2", "{current_score.ct}" }
                                div { class: "text-[9px] text-zinc-600 mt-1", "{ct_players.len()} / 3 PLAYERS" }
                            }

                            div { class: "space-y-2",
                                for (sid , name) in ct_players.iter() {
                                    div { class: "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded",
                                        input { r#type: "checkbox", class: "w-3.5 h-3.5 accent-blue-500" }
                                        div { class: "flex-1 min-w-0",
                                            div { class: "text-blue-300 font-bold truncate text-sm", "{name}" }
                                            div { class: "text-[9px] text-zinc-600 font-mono truncate", "{sid}" }
                                        }
                                    }
                                }
                                if ct_missing >= 1 {
                                    div { class: "grid grid-cols-2 gap-2",
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-blue-200 outline-none", placeholder: "STEAMID", oninput: move |e| ct_extra_1_sid.set(e.value()) }
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-blue-200 outline-none", placeholder: "NAME", oninput: move |e| ct_extra_1_name.set(e.value()) }
                                    }
                                }
                                if ct_missing >= 2 {
                                    div { class: "grid grid-cols-2 gap-2",
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-blue-200 outline-none", placeholder: "STEAMID", oninput: move |e| ct_extra_2_sid.set(e.value()) }
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-blue-200 outline-none", placeholder: "NAME", oninput: move |e| ct_extra_2_name.set(e.value()) }
                                    }
                                }
                                if ct_missing >= 3 {
                                    div { class: "grid grid-cols-2 gap-2",
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-blue-200 outline-none", placeholder: "STEAMID", oninput: move |e| ct_extra_3_sid.set(e.value()) }
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-blue-200 outline-none", placeholder: "NAME", oninput: move |e| ct_extra_3_name.set(e.value()) }
                                    }
                                }
                            }
                        }

                        // Team 2
                        div { class: "flex flex-col",
                            div { class: "text-center mb-5",
                                div { class: "text-red-400 font-black text-[11px] tracking-widest uppercase", "TEAM 2" }
                                input {
                                    class: "mt-3 w-full bg-zinc-900 border border-zinc-800 focus:border-red-500 rounded px-3 py-2 text-center text-red-200 font-bold text-sm outline-none placeholder:text-zinc-700",
                                    placeholder: "ENTER TEAM NAME",
                                    value: "{t_team_name}",
                                    oninput: move |e| t_team_name.set(e.value())
                                }
                                div { class: "text-red-400 text-5xl font-black mt-2", "{current_score.t}" }
                                div { class: "text-[9px] text-zinc-600 mt-1", "{t_players.len()} / 3 PLAYERS" }
                            }

                            div { class: "space-y-2",
                                for (sid , name) in t_players.iter() {
                                    div { class: "flex items-center gap-3 px-4 py-2 bg-zinc-900/60 border border-zinc-800 rounded",
                                        input { r#type: "checkbox", class: "w-3.5 h-3.5 accent-red-500" }
                                        div { class: "flex-1 min-w-0",
                                            div { class: "text-red-300 font-bold truncate text-sm", "{name}" }
                                            div { class: "text-[9px] text-zinc-600 font-mono truncate", "{sid}" }
                                        }
                                    }
                                }
                                if t_missing >= 1 {
                                    div { class: "grid grid-cols-2 gap-2",
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-red-200 outline-none", placeholder: "STEAMID", oninput: move |e| t_extra_1_sid.set(e.value()) }
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-red-200 outline-none", placeholder: "NAME", oninput: move |e| t_extra_1_name.set(e.value()) }
                                    }
                                }
                                if t_missing >= 2 {
                                    div { class: "grid grid-cols-2 gap-2",
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-red-200 outline-none", placeholder: "STEAMID", oninput: move |e| t_extra_2_sid.set(e.value()) }
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-red-200 outline-none", placeholder: "NAME", oninput: move |e| t_extra_2_name.set(e.value()) }
                                    }
                                }
                                if t_missing >= 3 {
                                    div { class: "grid grid-cols-2 gap-2",
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-red-200 outline-none", placeholder: "STEAMID", oninput: move |e| t_extra_3_sid.set(e.value()) }
                                        input { class: "bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-red-200 outline-none", placeholder: "NAME", oninput: move |e| t_extra_3_name.set(e.value()) }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right: Config Panel
                div {
                    class: "
                        border-l border-zinc-800 bg-zinc-950 flex flex-col min-h-0
                        lg:relative lg:w-[380px] lg:shrink-0 lg:translate-x-0
                        fixed top-0 right-0 bottom-0 z-50 w-[90vw] max-w-[500px] shadow-2xl
                        transition-transform duration-200 ease-out
                    ",
                    class: if show_mobile_config() { "translate-x-0" } else { "translate-x-full lg:translate-x-0" },
                    onclick: move |e| e.stop_propagation(),

                    div { class: "shrink-0 flex items-center justify-between gap-2 px-4 py-4 border-b border-zinc-800 bg-zinc-900/50",
                        div { class: "text-zinc-400 text-[10px] font-black tracking-widest uppercase", "MATCHZY CONFIG" }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white text-[9px] font-black rounded transition-colors",
                                onclick: move |_| generate_config(),
                                "GENERATE"
                            }
                            button {
                                class: "px-3 py-1.5 bg-blue-600 hover:bg-blue-500 text-white text-[9px] font-black rounded transition-colors",
                                onclick: move |_| {
                                    let cfg = generated_config();
                                    let srv = addr;
                                    spawn(async move {
                                        if let Some(ip) = log_receiver_ip(srv) {
                                            let url = format!("http://{}:7131/MatchZyConfig", ip);
                                            let _ = reqwest::Client::new().post(&url).header("Content-Type", "application/json").body(cfg).send().await;
                                            on_command.call(format!(r#"matchzy_loadmatch_url "{}""#, url));
                                        }
                                    });
                                },
                                "PUBLISH"
                            }
                            button { class: "lg:hidden text-zinc-500 hover:text-white px-2 py-1", onclick: move |_| show_mobile_config.set(false), "×" }
                        }
                    }

                    textarea {
                        class: "flex-1 w-full bg-black p-4 text-emerald-400 font-mono text-xs outline-none resize-none",
                        value: "{generated_config}",
                        oninput: move |e| generated_config.set(e.value())
                    }
                }
            }

            // Mobile Backdrop
            if show_mobile_config() {
                div { class: "lg:hidden fixed inset-0 z-40 bg-black/60 backdrop-blur-sm", onclick: move |_| show_mobile_config.set(false) }
            }

            // Mobile FAB
            if !show_mobile_config() {
                button {
                    class: "lg:hidden fixed bottom-6 right-6 z-40 bg-indigo-600 text-white px-4 py-3 rounded-full shadow-2xl font-black text-[10px] tracking-widest",
                    onclick: move |_| show_mobile_config.set(true),
                    "CONFIG"
                }
            }
        }
    }
}
