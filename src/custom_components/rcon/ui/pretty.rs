use dioxus::prelude::*;

use live_log::{
    parser::{LogEvent, Team},
    round_stats::RSPlayer,
};

/// Render a parsed log event directly as Dioxus HTML.
///
/// This intentionally does NOT use LogPattern::pretty(), because that
/// produces ANSI terminal formatting. The web UI gets its own presentation.
pub fn pretty_log(event: &LogEvent) -> Element {
    match event {
        // ---------------------------------------------------------------------
        // Player damaged
        // ---------------------------------------------------------------------
        LogEvent::Attacked {
            attacker,
            victim,
            damage,
            weapon,
            hitgroup,
        } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&attacker.team),
                    "{attacker.name}"
                }

                " hit "

                span {
                    class: team_text_class(&victim.team),
                    "{victim.name}"
                }

                " for "

                span {
                    class: "text-red-400 font-semibold",
                    "{damage}"
                }

                " ({weapon}) in "

                span {
                    class: "text-zinc-400",
                    "{hitgroup}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Kill
        // ---------------------------------------------------------------------
        LogEvent::Kill {
            attacker,
            victim,
            weapon,
            headshot,
            penetrated,
            through_smoke,
            attacker_in_air,
        } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&attacker.team),
                    "{attacker.name}"
                }

                " killed "

                span {
                    class: team_text_class(&victim.team),
                    "{victim.name}"
                }

                " with "

                span {
                    class: "text-zinc-200",
                    "{weapon}"
                }

                if *headshot {
                    span {
                        class: "ml-1",
                        "🎯"
                    }
                }

                if *penetrated {
                    span {
                        class: "ml-1",
                        "🧱"
                    }
                }

                if *through_smoke {
                    span {
                        class: "ml-1",
                        "💨"
                    }
                }

                if *attacker_in_air {
                    span {
                        class: "ml-1",
                        "🪽"
                    }
                }
            }
        },

        // ---------------------------------------------------------------------
        // Chat
        // ---------------------------------------------------------------------
        LogEvent::Chat {
            player,
            msg,
            is_team_chat,
        } => rsx! {
            span {
                if *is_team_chat {
                    span {
                        class: "text-yellow-400 font-semibold",
                        "[TEAM]"
                    }
                } else {
                    span {
                        class: "text-zinc-500 font-semibold",
                        "[ALL]"
                    }
                }

                " "

                span {
                    class: team_text_class(&player.team),
                    "{player.name}"
                }

                ": "

                span {
                    class: "text-zinc-200",
                    "{msg}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Score update
        // ---------------------------------------------------------------------
        LogEvent::ScoreUpdate { t1, t2, map, .. } => rsx! {
            span {
                class: "font-semibold",

                span {
                    class: "text-yellow-400",
                    "[SCORE]"
                }

                " "

                span {
                    class: "text-red-400",
                    "T {t1}"
                }

                " - "

                span {
                    class: "text-blue-400",
                    "{t2} CT"
                }

                " on "

                span {
                    class: "text-zinc-200",
                    "{map}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Round win
        // ---------------------------------------------------------------------
        LogEvent::RoundWin {
            winner_side,
            reason,
            ct_score,
            t_score,
            ..
        } => {
            let winner = match winner_side {
                Team::CT => "CT",
                Team::Terrorist => "TERRORIST",
                _ => "UNKNOWN",
            };

            let reason = reason.replace('_', " ");

            rsx! {
                span {
                    class: "font-semibold",

                    span {
                        class: team_text_class(winner_side),
                        "{winner} WON"
                    }

                    " — "

                    span {
                        class: "text-zinc-300",
                        "{reason}"
                    }

                    " | CT "

                    span {
                        class: "text-blue-400",
                        "{ct_score}"
                    }

                    " : T "

                    span {
                        class: "text-red-400",
                        "{t_score}"
                    }
                }
            }
        }

        // ---------------------------------------------------------------------
        // Team switch
        // ---------------------------------------------------------------------
        LogEvent::TeamSwitch { player, from } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-zinc-100 font-semibold",
                    "{player.name}"
                }

                " switched from "

                span {
                    class: team_text_class(from),
                    "{team_label(from)}"
                }

                " to "

                span {
                    class: team_text_class(&player.team),
                    "{team_label(&player.team)}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Bomb event
        // ---------------------------------------------------------------------
        LogEvent::BombEvent {
            player,
            event,
            site,
        } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&player.team),
                    "{player.name}"
                }

                " "

                span {
                    class: "text-zinc-200",
                    "{event}"
                }

                if let Some(site) = site {
                    span {
                        class: "text-zinc-400",
                        " at site {site}"
                    }
                }
            }
        },

        // ---------------------------------------------------------------------
        // Purchase
        // ---------------------------------------------------------------------
        LogEvent::Purchase { player, item } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&player.team),
                    "{player.name}"
                }

                " purchased "

                span {
                    class: "text-zinc-200",
                    "{item}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Connection
        // ---------------------------------------------------------------------
        LogEvent::Connection {
            player,
            action,
            info,
        } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-cyan-400",
                    "{player.name}"
                }

                " "

                span {
                    class: "text-purple-400 font-semibold",
                    "{action}"
                }

                if let Some(info) = info {
                    " "

                    span {
                        class: "text-zinc-500",
                        "({info})"
                    }
                }
            }
        },

        // ---------------------------------------------------------------------
        // Suicide
        // ---------------------------------------------------------------------
        LogEvent::Suicide { player, weapon } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&player.team),
                    "{player.name}"
                }

                " committed suicide with "

                span {
                    class: "text-zinc-200",
                    "{weapon}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // World trigger
        // ---------------------------------------------------------------------
        LogEvent::WorldTrigger { event } => rsx! {
            span {
                class: "text-purple-400",
                "[WORLD]"
            }

            " "

            span {
                class: "text-zinc-300",
                "{event}"
            }
        },

        // ---------------------------------------------------------------------
        // Technical
        // ---------------------------------------------------------------------
        LogEvent::Technical { name, action } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-cyan-400",
                    "{name}"
                }

                " "

                span {
                    class: "text-purple-400",
                    "{action}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Left buyzone
        // ---------------------------------------------------------------------
        LogEvent::LeftBuyZone { player, items } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-zinc-100 font-semibold",
                    "{player.name}"
                }

                " left buyzone with "

                span {
                    class: "text-yellow-400",
                    "[{items.join(\", \")}]"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Round stats
        // ---------------------------------------------------------------------
        LogEvent::RoundStats { roundstats } => rsx! {
            RoundStatsView {
                roundstats: roundstats.clone()
            }
        },

        // ---------------------------------------------------------------------
        // Assist
        // ---------------------------------------------------------------------
        LogEvent::Assist { assister, victim } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&assister.team),
                    "{assister.name}"
                }

                " assisted killing "

                span {
                    class: team_text_class(&victim.team),
                    "{victim.name}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Grenade
        // ---------------------------------------------------------------------
        LogEvent::Grenade { player, grenade } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&player.team),
                    "{player.name}"
                }

                " threw "

                span {
                    class: "text-zinc-200",
                    "{grenade}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Server grenade
        // ---------------------------------------------------------------------
        LogEvent::SvGrenade { player, grenade } => rsx! {
            span {
                class: "text-zinc-500",

                span {
                    class: team_text_class(&player.team),
                    "{player.name}"
                }

                " sv_throw_{grenade}"
            }
        },

        // ---------------------------------------------------------------------
        // Blinded
        // ---------------------------------------------------------------------
        LogEvent::Blinded {
            attacker,
            victim,
            duration,
        } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&attacker.team),
                    "{attacker.name}"
                }

                " blinded "

                span {
                    class: team_text_class(&victim.team),
                    "{victim.name}"
                }

                " for "

                span {
                    class: "text-yellow-400",
                    "{duration:.2}s"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Match status
        // ---------------------------------------------------------------------
        LogEvent::MatchStatus { team, team_name } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(team),
                    "{team_label(team)}"
                }

                if let Some(team_name) = team_name {
                    " playing "

                    span {
                        class: "text-zinc-200",
                        "{team_name}"
                    }
                } else {
                    " unset"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Freeze period
        // ---------------------------------------------------------------------
        LogEvent::FreezePeriod => rsx! {
            span {
                class: "text-zinc-500",
                "[FREEZE] Starting Freeze period"
            }
        },

        // ---------------------------------------------------------------------
        // Match start
        // ---------------------------------------------------------------------
        LogEvent::MatchStart { map } => rsx! {
            span {
                class: "font-semibold",

                span {
                    class: "text-green-400",
                    "[MATCH]"
                }

                " started on "

                span {
                    class: "text-zinc-200",
                    "{map}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Team score
        // ---------------------------------------------------------------------
        LogEvent::TeamScore {
            team,
            score,
            players,
        } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(team),
                    "{team_label(team)}"
                }

                " scored "

                span {
                    class: "text-zinc-100 font-semibold",
                    "{score}"
                }

                " with {players} players"
            }
        },

        // ---------------------------------------------------------------------
        // Server CVar
        // ---------------------------------------------------------------------
        LogEvent::ServerCvar { name, value } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-zinc-500",
                    "server_cvar"
                }

                " "

                span {
                    class: "text-zinc-200",
                    "{name}"
                }

                " = "

                span {
                    class: "text-zinc-400",
                    "{value}"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Bomb death
        // ---------------------------------------------------------------------
        LogEvent::BombDeath { player } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: team_text_class(&player.team),
                    "{player.name}"
                }

                " was killed by the bomb"
            }
        },

        // ---------------------------------------------------------------------
        // Round accolade
        // ---------------------------------------------------------------------
        LogEvent::RoundAccolade {
            category,
            player,
            value,
            position,
            ..
        } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-yellow-400 font-semibold",
                    "[ACCOLADE]"
                }

                " "

                span {
                    class: team_text_class(&player.team),
                    "{player.name}"
                }

                ": "

                span {
                    class: "text-zinc-300",
                    "{category}"
                }

                " ({value:.2}) [#{position}]"
            }
        },

        // ---------------------------------------------------------------------
        // Server CVar dump
        // ---------------------------------------------------------------------
        LogEvent::ServerCvarDump { cvars } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-zinc-500",
                    "[CVARS]"
                }

                " received {cvars.len()} server variables"
            }
        },

        // ---------------------------------------------------------------------
        // Log file
        // ---------------------------------------------------------------------
        LogEvent::LogFile { started } => rsx! {
            span {
                class: "text-zinc-500",

                if *started {
                    "[LOG] file started"
                } else {
                    "[LOG] file closed"
                }
            }
        },

        // ---------------------------------------------------------------------
        // Map loading
        // ---------------------------------------------------------------------
        LogEvent::MapLoading { map } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-cyan-400",
                    "[MAP]"
                }

                " loading {map}"
            }
        },

        // ---------------------------------------------------------------------
        // Server started
        // ---------------------------------------------------------------------
        LogEvent::ServerStarted => rsx! {
            span {
                class: "text-green-400",
                "[SERVER] started"
            }
        },

        // ---------------------------------------------------------------------
        // Game over
        // ---------------------------------------------------------------------
        LogEvent::GameOver {
            mode,
            map,
            t_score,
            ct_score,
            minutes,
        } => rsx! {
            span {
                class: "font-semibold",

                span {
                    class: "text-yellow-400",
                    "[GAME OVER]"
                }

                " {mode} {map} "

                span {
                    class: "text-red-400",
                    "{t_score}"
                }

                ":"

                span {
                    class: "text-blue-400",
                    "{ct_score}"
                }

                " after {minutes} min"
            }
        },

        // ---------------------------------------------------------------------
        // Molotov spawn
        // ---------------------------------------------------------------------
        LogEvent::MolotovSpawn {
            x: _,
            y: _,
            z: _,
            vx: _,
            vy: _,
            vz: _,
        } => rsx! {
            span {
                class: "text-zinc-300",

                span {
                    class: "text-yellow-400 font-semibold",
                    "Molotov"
                }

                " spawned"
            }
        },

        // ---------------------------------------------------------------------
        // Unknown / ignored events
        // ---------------------------------------------------------------------
        LogEvent::Ignored => rsx! {
            span {
                class: "text-zinc-600",
                "[IGNORED]"
            }
        },

        LogEvent::Unknown => rsx! {
            span {
                class: "text-zinc-600",
                "[UNKNOWN]"
            }
        },

        // ---------------------------------------------------------------------
        // RCON
        // ---------------------------------------------------------------------
        LogEvent::Rcon { addr, command } => rsx! {
            span {
                class: "text-cyan-400",
                "[RCON] {addr}: {command}"
            }
        },
    }
}

// =============================================================================
// Round stats
// =============================================================================

#[component]
fn RoundStatsView(roundstats: live_log::round_stats::RoundStats) -> Element {
    rsx! {
        div {
            class: "my-2 rounded border border-zinc-700 bg-zinc-900/60 overflow-hidden",

            // Header
            div {
                class: "flex items-center justify-between px-3 py-2 border-b border-zinc-700 bg-zinc-800/70",

                div {
                    class: "flex items-center gap-3",

                    span {
                        class: "font-semibold text-zinc-100",
                        "Round {roundstats.round_number}"
                    }

                    span {
                        class: "text-zinc-500",
                        "|"
                    }

                    span {
                        class: "text-zinc-300",
                        "{roundstats.map}"
                    }
                }

                div {
                    class: "flex items-center gap-2 font-semibold",

                    span {
                        class: "text-red-400",
                        "{roundstats.score_t}"
                    }

                    span {
                        class: "text-zinc-500",
                        ":"
                    }

                    span {
                        class: "text-blue-400",
                        "{roundstats.score_ct}"
                    }
                }

                span {
                    class: "text-zinc-500 text-xs",
                    "{roundstats.server}"
                }
            }

            // Player table
            div {
                class: "overflow-x-auto",

                table {
                    class: "w-full text-xs",

                    thead {
                        tr {
                            class: "text-zinc-500 border-b border-zinc-800",

                            th {
                                class: "px-2 py-1.5 text-left font-normal",
                                "#"
                            }

                            th {
                                class: "px-2 py-1.5 text-left font-normal",
                                "Player"
                            }

                            th {
                                class: "px-2 py-1.5 text-right font-normal",
                                "K"
                            }

                            th {
                                class: "px-2 py-1.5 text-right font-normal",
                                "D"
                            }

                            th {
                                class: "px-2 py-1.5 text-right font-normal",
                                "A"
                            }

                            th {
                                class: "px-2 py-1.5 text-right font-normal",
                                "DMG"
                            }

                            th {
                                class: "px-2 py-1.5 text-right font-normal",
                                "HS"
                            }

                            th {
                                class: "px-2 py-1.5 text-right font-normal",
                                "K/D"
                            }

                            th {
                                class: "px-2 py-1.5 text-right font-normal",
                                "ADR"
                            }

                            th {
                                class: "px-2 py-1.5 text-right font-normal",
                                "MVP"
                            }
                        }
                    }

                    tbody {
                        for (number, player) in &roundstats.players {
                            RoundStatsPlayerRow {
                                number: *number,
                                player: player.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RoundStatsPlayerRow(number: u16, player: RSPlayer) -> Element {
    let team_class = match player.team {
        2 => "text-red-400",
        3 => "text-blue-400",
        _ => "text-zinc-400",
    };

    rsx! {
        tr {
            class: "border-b border-zinc-800/60 hover:bg-zinc-800/40",

            td {
                class: "px-2 py-1.5 text-zinc-500 text-right",
                "{number}"
            }

            td {
                class: "px-2 py-1.5",

                span {
                    class: "{team_class}",
                    "{player.account_id}"
                }
            }

            td {
                class: "px-2 py-1.5 text-right text-zinc-200",
                "{player.kills}"
            }

            td {
                class: "px-2 py-1.5 text-right text-zinc-400",
                "{player.deaths}"
            }

            td {
                class: "px-2 py-1.5 text-right text-zinc-400",
                "{player.assists}"
            }

            td {
                class: "px-2 py-1.5 text-right text-zinc-200",
                "{player.damage}"
            }

            td {
                class: "px-2 py-1.5 text-right text-zinc-300",
                "{player.headshot_percent}%"
            }

            td {
                class: "px-2 py-1.5 text-right text-zinc-300",
                "{player.kd_ratio:.2}"
            }

            td {
                class: "px-2 py-1.5 text-right text-zinc-300",
                "{player.adr}"
            }

            td {
                class: "px-2 py-1.5 text-right text-yellow-400",
                "{player.mvp}"
            }
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn team_text_class(team: &Team) -> &'static str {
    match team {
        Team::CT => "text-blue-400",
        Team::Terrorist => "text-red-400",
        Team::Spectator => "text-zinc-400",
        Team::Unassigned => "text-zinc-500",
        Team::Unknown => "text-zinc-500",
    }
}

fn team_label(team: &Team) -> &'static str {
    match team {
        Team::CT => "CT",
        Team::Terrorist => "TERRORIST",
        Team::Spectator => "SPECTATOR",
        Team::Unassigned => "UNASSIGNED",
        Team::Unknown => "UNKNOWN",
    }
}
