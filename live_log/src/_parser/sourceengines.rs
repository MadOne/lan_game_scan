// sourceengine.rs

pub const SOURCE2_TS_BLOCK: &str =
    r"(?:\[LOG\]\s+)?(?P<ts>\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}\.\d{3})\s+-\s+";

pub const GOLDSRC_TS_BLOCK: &str = r"(?:L\s+)?(?P<ts>\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}):\s*";

pub const SOURCE_TS_BLOCK: &str = r"(?:RL\s+)?(?P<ts>\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}):\s*";

pub const STEAMID3_BLOCK: &str = r"(?:\[[A-Z]:1:\d+\]|BOT|STEAM_ID_PENDING)";

pub const STEAMID2_BLOCK: &str = r"(?:STEAM_\d+:\d+:\d+|STEAM_ID_PENDING|STEAM_ID_LAN|VALVE_\d+:\d+:\d+|VALVE_ID_PENDING|VALVE_ID_LAN)";

pub const PLAYER_BLOCK_BASE: &str =
    r#""(?P<{0}name>[^<]+)<(?P<{0}id>\d+)><(?P<{0}steamid>{STEAMID})>(?:<(?P<{0}team>[^>]*)>)?""#;

pub const STATS_BLOCK: &str = r#"\(damage "(?P<dmg>\d+)"\) \(damage_armor "(?P<dmg_arm>\d+)"\) \(health "(?P<hp>\d+)"\) \(armor "(?P<arm>\d+)"\) \(hitgroup "(?P<hit>[^"]+)"\)"#;

pub const POS_BLOCK_BASE: &str = r#"\[(?P<{0}pos>-?\d+ -?\d+ -?\d+)\]"#;

pub mod cs2 {
    use regex::Regex;

    use crate::{
        _parser::{
            patterns::{parse_player, LogPattern, LogPatternBlocks},
            types::{LogEvent, Player, Team},
        },
        cvar_parser::ServerCvars,
        round_stats::parse_round_stats,
    };

    pub fn player_damaged(blocks: &LogPatternBlocks) -> LogPattern {
        let attacker = blocks.player("at_");
        let victim = blocks.player("vic_");
        let attacker_pos = blocks.position("at_");
        let victim_pos = blocks.position("vic_");

        LogPattern {
            id: "PLAYER_DAMAGED",
            regex: Regex::new(&format!(
                r#"^{} {} attacked {} {} with "(?P<weapon>[^"]+)" {}$"#,
                attacker,
                attacker_pos,
                victim,
                victim_pos,
                blocks.stats(),
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Attacked {
                    attacker: parse_player(c, "at_")?,
                    victim: parse_player(c, "vic_")?,
                    damage: c.name("dmg")?.as_str().parse().ok()?,
                    weapon: c.name("weapon")?.as_str().to_string(),
                    hitgroup: c.name("hit")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Attacked {
                    attacker,
                    victim,
                    damage,
                    weapon,
                    hitgroup,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "\x1b[90m{}{}\x1b[0m hit \
                 \x1b[90m{}{}\x1b[0m for {} ({}) in {}\x1b[0m",
                    attacker.team.color_code(),
                    attacker.name,
                    victim.team.color_code(),
                    victim.name,
                    damage,
                    weapon,
                    hitgroup
                )
            },
        }
    }

    pub fn player_killed(blocks: &LogPatternBlocks) -> LogPattern {
        let attacker = blocks.player("at_");
        let victim = blocks.player("vic_");
        let attacker_pos = blocks.position("at_");
        let victim_pos = blocks.position("vic_");

        LogPattern {
        id: "PLAYER_KILLED",
        regex: Regex::new(&format!(
            r#"^{}(?: {})? killed (?:other (?P<vic_other>".+?"))?{}(?: {})? with "(?P<weapon>[^"]+)"(?P<hs> \(headshot\))?(?P<pen> \(penetrated\))?(?P<smoke> \(throughsmoke\))?(?P<air> \(attackerinair\))?$"#,
            attacker,
            attacker_pos,
            victim,
            victim_pos,
        ))
        .unwrap(),

        parse_fn: |_line, c| {
            let attacker = parse_player(c, "at_")?;

            let victim = match parse_player(c, "vic_") {
                Some(player) => player,
                None => Player {
                    id: 0,
                    name: c
                        .name("vic_other")?
                        .as_str()
                        .trim_matches('"')
                        .to_string(),
                    steamid: String::new(),
                    team: Team::Unknown,
                },
            };

            Some(LogEvent::Kill {
                attacker,
                victim,
                weapon: c.name("weapon")?.as_str().to_string(),
                headshot: c.name("hs").is_some(),
                penetrated: c.name("pen").is_some(),
                through_smoke: c.name("smoke").is_some(),
                attacker_in_air: c.name("air").is_some(),
            })
        },

        pretty_fn: |event| {
            let LogEvent::Kill {
                attacker,
                victim,
                weapon,
                headshot,
                penetrated,
                through_smoke,
                attacker_in_air,
            } = event
            else {
                return String::new();
            };

            let hs = if *headshot { " 🎯" } else { "" };
            let wall = if *penetrated { " 🧱" } else { "" };
            let smoke = if *through_smoke { " 💨" } else { "" };
            let air = if *attacker_in_air { " 🪽" } else { "" };

            format!(
                "{}{}\x1b[0m killed \
                 {}{}\x1b[0m with {}{}{}{}{}",
                attacker.team.color_code(),
                attacker.name,
                victim.team.color_code(),
                victim.name,
                weapon,
                hs,
                wall,
                smoke,
                air
            )
        },
    }
    }

    pub fn chat(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "CHAT",
            regex: Regex::new(&format!(
                r#"^{} (?P<type>say|say_team) "(?P<msg>[^"]*)"$"#,
                player
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Chat {
                    player: parse_player(c, "")?,
                    msg: c.name("msg")?.as_str().to_string(),
                    is_team_chat: c.name("type")?.as_str() == "say_team",
                })
            },

            pretty_fn: |event| {
                let LogEvent::Chat {
                    player,
                    msg,
                    is_team_chat,
                } = event
                else {
                    return String::new();
                };

                let tag = if *is_team_chat { "[TEAM]" } else { "[ALL ]" };

                format!(
                    "\x1b[1m{} {}{}\x1b[0m: {}",
                    tag,
                    player.team.color_code(),
                    player.name,
                    msg
                )
            },
        }
    }

    pub fn player_team_switch(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_TEAM_SWITCH",
            regex: Regex::new(&format!(
                r#"^{} switched from team <(?P<old>[^>]*)> to <(?P<new>[^>]*)>$"#,
                player
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                let mut player = parse_player(c, "")?;

                let from = Team::from_str(c.name("old")?.as_str());
                player.team = Team::from_str(c.name("new")?.as_str());

                Some(LogEvent::TeamSwitch { player, from })
            },

            pretty_fn: |event| {
                let LogEvent::TeamSwitch { player, from } = event else {
                    return String::new();
                };

                format!(
                    "{} switched from {:?} to {:?}",
                    player.name, from, player.team
                )
            },
        }
    }

    pub fn bomb_event(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "BOMB_EVENT",
            regex: Regex::new(&format!(
                r#"^{} triggered "(?P<event>[^"]+)"(?: at bombsite (?P<site>[AB]))?$"#,
                player
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::BombEvent {
                    player: parse_player(c, "")?,
                    event: c.name("event")?.as_str().to_string(),
                    site: c.name("site").map(|m| m.as_str().to_string()),
                })
            },

            pretty_fn: |event| {
                let LogEvent::BombEvent {
                    player,
                    event,
                    site,
                } = event
                else {
                    return String::new();
                };

                let site = site
                    .as_ref()
                    .map(|s| format!(" at site {}", s))
                    .unwrap_or_default();

                format!(
                    "{}{} \x1b[0m{}{}",
                    player.team.color_code(),
                    player.name,
                    event,
                    site
                )
            },
        }
    }

    pub fn player_purchase(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_PURCHASE",
            regex: Regex::new(&format!(r#"^{} purchased "(?P<item>[^"]+)"$"#, player)).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Purchase {
                    player: parse_player(c, "")?,
                    item: c.name("item")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Purchase { player, item } = event else {
                    return String::new();
                };

                format!(
                    "{}{}\x1b[0mpurchased {}",
                    player.team.color_code(),
                    player.name,
                    item
                )
            },
        }
    }

    pub fn player_disconnected(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_DISCONNECTED",
            regex: Regex::new(&format!(
                r#"^{} disconnected \(reason "(?P<reason>.+)"\)$"#,
                player
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Connection {
                    player: parse_player(c, "")?,
                    action: "Disconnected".to_string(),
                    info: Some(c.name("reason")?.as_str().to_string()),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Connection { player, action, .. } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[36m{}\x1b[0m \
                 \x1b[1;35m{}\x1b[0m",
                    player.name, action
                )
            },
        }
    }

    pub fn player_connected(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_CONNECTED",
            regex: Regex::new(&format!(
                r#"^{} connected, address "(?P<addr>.+)"$"#,
                player
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Connection {
                    player: parse_player(c, "")?,
                    action: "Handshake".to_string(),
                    info: Some(c.name("addr")?.as_str().to_string()),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Connection { player, action, .. } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[36m{}\x1b[0m \
                 \x1b[1;35m{}\x1b[0m",
                    player.name, action
                )
            },
        }
    }

    pub fn player_entered_game(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_ENTERED_GAME",
            regex: Regex::new(&format!(r#"^{} entered the game$"#, player)).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Connection {
                    player: parse_player(c, "")?,
                    action: "Entered game".to_string(),
                    info: None,
                })
            },

            pretty_fn: |event| {
                let LogEvent::Connection { player, action, .. } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[36m{}\x1b[0m \
                 \x1b[1;35m{}\x1b[0m",
                    player.name, action
                )
            },
        }
    }

    pub fn player_suicide(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");
        let position = blocks.position("");

        LogPattern {
            id: "PLAYER_SUICIDE",
            regex: Regex::new(&format!(
                r#"^{}(?: {})? committed suicide with "(?P<weapon>.+)"$"#,
                player, position
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Suicide {
                    player: parse_player(c, "")?,
                    weapon: c.name("weapon")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Suicide { player, weapon } = event else {
                    return String::new();
                };

                format!(
                    "{}{}\x1b[0m committed suicide with {}",
                    player.team.color_code(),
                    player.name,
                    weapon
                )
            },
        }
    }

    pub fn player_validated(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_VALIDATED",
            regex: Regex::new(&format!(r#"^{} STEAM USERID validated$"#, player)).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Technical {
                    name: c.name("name")?.as_str().to_string(),
                    action: "Status Change".to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Technical { name, action } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[36m{}\x1b[0m \
                 \x1b[1;35m{}\x1b[0m",
                    name, action
                )
            },
        }
    }

    pub fn player_left_buyzone(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_LEFT_BUYZONE",
            regex: Regex::new(&format!(
                r#"^{} left buyzone with \[\s*(?P<items>.*?)\s*\]$"#,
                player
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::LeftBuyZone {
                    player: parse_player(c, "")?,
                    items: c
                        .name("items")?
                        .as_str()
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(str::to_string)
                        .collect(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::LeftBuyZone { player, items } = event else {
                    return String::new();
                };

                format!(
                    "{}\x1b[0m left buyzone with \x1b[33m{:?}\x1b[0m",
                    player.name, items
                )
            },
        }
    }

    pub fn player_assist(blocks: &LogPatternBlocks) -> LogPattern {
        let assister = blocks.player("assister_");
        let victim = blocks.player("victim_");

        LogPattern {
            id: "PLAYER_ASSIST",
            regex: Regex::new(&format!(r#"^{} assisted killing {}$"#, assister, victim)).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Assist {
                    assister: parse_player(c, "assister_")?,
                    victim: parse_player(c, "victim_")?,
                })
            },

            pretty_fn: |event| {
                let LogEvent::Assist { assister, victim } = event else {
                    return String::new();
                };

                format!(
                    "{}{}\x1b[0m assisted killing {}{}\x1b[0m",
                    assister.team.color_code(),
                    assister.name,
                    victim.team.color_code(),
                    victim.name
                )
            },
        }
    }

    pub fn player_grenade_throw(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_GRENADE_THROW",
            regex: Regex::new(&format!(
                r#"^{} threw (?P<grenade>[a-zA-Z0-9_]+) \[[-\d\s]+\].*$"#,
                player
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Grenade {
                    player: parse_player(c, "")?,
                    grenade: c.name("grenade")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Grenade { player, grenade } = event else {
                    return String::new();
                };

                format!(
                    "{}{}\x1b[0m threw {}",
                    player.team.color_code(),
                    player.name,
                    grenade
                )
            },
        }
    }

    pub fn server_grenade_throw(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
        id: "SERVER_GRENADE_THROW",
        regex: Regex::new(&format!(
            r#"^{} sv_throw_(?P<grenade>[a-z]+) (?P<x>-?\d+(?:\.\d+)?) (?P<y>-?\d+(?:\.\d+)?) (?P<z>-?\d+(?:\.\d+)?) .*$"#,
            player
        ))
        .unwrap(),

        parse_fn: |_line, c| {
            Some(LogEvent::SvGrenade {
                player: parse_player(c, "")?,
                grenade: c.name("grenade")?.as_str().to_string(),
            })
        },

        pretty_fn: |event| {
            let LogEvent::SvGrenade { player, grenade } = event else {
                return String::new();
            };

            format!(
                "\x1b[90m{}{}\x1b[0m sv_throw_{}",
                player.team.color_code(),
                player.name,
                grenade
            )
        },
    }
    }

    pub fn player_blinded(blocks: &LogPatternBlocks) -> LogPattern {
        let attacker = blocks.player("at_");
        let victim = blocks.player("vic_");

        LogPattern {
            id: "PLAYER_BLINDED",
            regex: Regex::new(&format!(
                r#"^{} blinded for (?P<duration>\d+(?:\.\d+)?) by {} from flashbang entindex \d+$"#,
                attacker, victim
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Blinded {
                    attacker: parse_player(c, "at_")?,
                    victim: parse_player(c, "vic_")?,
                    duration: c.name("duration")?.as_str().parse().ok()?,
                })
            },

            pretty_fn: |event| {
                let LogEvent::Blinded {
                    attacker,
                    victim,
                    duration,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "{}{}\x1b[0m blinded {}{}\x1b[0m for {:.2}s",
                    attacker.team.color_code(),
                    attacker.name,
                    victim.team.color_code(),
                    victim.name,
                    duration
                )
            },
        }
    }

    pub fn player_bomb_death(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");
        let position = blocks.position("");

        LogPattern {
            id: "PLAYER_BOMB_DEATH",
            regex: Regex::new(&format!(
                r#"^{} {} was killed by the bomb\.$"#,
                player, position
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::BombDeath {
                    player: parse_player(c, "")?,
                })
            },

            pretty_fn: |event| {
                let LogEvent::BombDeath { player } = event else {
                    return String::new();
                };

                format!(
                    "{}{}\x1b[0m was killed by the bomb",
                    player.team.color_code(),
                    player.name
                )
            },
        }
    }
    pub fn match_score() -> LogPattern {
        LogPattern {
        id: "MATCH_SCORE",
        regex: Regex::new(
            r#"^MatchStatus: Score: (?P<t1>\d+):(?P<t2>\d+) on map "(?P<map>.+)" RoundsPlayed: (?P<rounds>-?\d+)$"#,
        )
        .unwrap(),

        parse_fn: |_line, c| {
            Some(LogEvent::ScoreUpdate {
                t1: c.name("t1")?.as_str().parse().ok()?,
                t2: c.name("t2")?.as_str().parse().ok()?,
                map: c.name("map")?.as_str().to_string(),
                rounds: c.name("rounds")?.as_str().parse().ok()?,
            })
        },

        pretty_fn: |event| {
            let LogEvent::ScoreUpdate {
                t1,
                t2,
                map,
                ..
            } = event
            else {
                return String::new();
            };

            format!(
                "\x1b[1;33m[SCORE]\x1b[0m \
                 \x1b[31mT {}\x1b[0m - \
                 \x1b[34m{} CT\x1b[0m on {}",
                t1, t2, map
            )
        },
    }
    }

    pub fn round_win() -> LogPattern {
        LogPattern {
        id: "ROUND_WIN",
        regex: Regex::new(
            r#"^Team "(?P<team>[^"]+)" triggered "SFUI_Notice_(?P<reason>[^"]+)" \(CT "(?P<ct_score>\d+)"\) \(T "(?P<t_score>\d+)"\)$"#,
        )
        .unwrap(),

        parse_fn: |_line, c| {
            let reason = c.name("reason")?.as_str();

            let winner_side = match reason {
                "Target_Bombed" => Team::Terrorist,
                "Target_Saved" | "Bomb_Defused" | "CTs_Win" => Team::CT,
                "Terrorists_Win" => Team::Terrorist,
                _ => return None,
            };

            Some(LogEvent::RoundWin {
                team: c.name("team")?.as_str().to_string(),
                winner_side,
                reason: reason.to_string(),
                ct_score: c.name("ct_score")?.as_str().parse().ok()?,
                t_score: c.name("t_score")?.as_str().parse().ok()?,
            })
        },

        pretty_fn: |event| {
            let LogEvent::RoundWin {
                winner_side,
                reason,
                ct_score,
                t_score,
                ..
            } = event
            else {
                return String::new();
            };

            let reason = reason.replace('_', " ");

            format!(
                "{}{} WON\x1b[0m — {} | CT {} : T {}",
                winner_side.color_code(),
                match winner_side {
                    Team::CT => "CT",
                    Team::Terrorist => "TERRORIST",
                    Team::Spectator => "SPECTATOR",
                    Team::Unassigned => "UNASSIGNED",
                    Team::Unknown => "UNKNOWN",
                    Team::Allies => "ALLIES",
                    Team::Axis => "AXIS",
                    Team::Blue => "BLUE",
                    Team::Red => "RED",
                },
                reason,
                ct_score,
                t_score,
            )
        },
    }
    }

    pub fn world_trigger() -> LogPattern {
        LogPattern {
            id: "WORLD_TRIGGER",
            regex: Regex::new(r#"^World triggered "(?P<event>[^"]+)"$"#).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::WorldTrigger {
                    event: c.name("event")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::WorldTrigger { event } = event else {
                    return String::new();
                };

                format!("\x1b[35m[WORLD]\x1b[0m {}", event)
            },
        }
    }

    pub fn round_stats() -> LogPattern {
        LogPattern {
            id: "ROUND_STATS",
            regex: Regex::new(r#"(?s)^JSON_BEGIN.*JSON_END$"#).unwrap(),

            parse_fn: |line, _c| {
                parse_round_stats(line).map(|stats| LogEvent::RoundStats { roundstats: stats })
            },

            pretty_fn: |event| {
                let LogEvent::RoundStats { roundstats } = event else {
                    return String::new();
                };

                let mut output = format!(
                    "Round {} | {} {}:{} | {}\n",
                    roundstats.round_number,
                    roundstats.map,
                    roundstats.score_t,
                    roundstats.score_ct,
                    roundstats.server,
                );

                for (number, player) in &roundstats.players {
                    let team_color = match player.team {
                        2 => Team::Terrorist.color_code(),
                        3 => Team::CT.color_code(),
                        _ => "\x1b[37m",
                    };

                    output.push_str(&format!(
                        "{}  {:>2}: {:>8} | \
                     K/D/A {}/{}/{} | \
                     DMG {:>5} | \
                     HS {:>5}% | \
                     K/D {:>4.2} | \
                     ADR {:>3}\x1b[0m\n",
                        team_color,
                        number,
                        player.account_id,
                        player.kills,
                        player.deaths,
                        player.assists,
                        player.damage,
                        player.headshot_percent,
                        player.kd_ratio,
                        player.adr,
                    ));
                }

                output
            },
        }
    }

    pub fn match_team_playing() -> LogPattern {
        LogPattern {
            id: "MATCH_TEAM_PLAYING",
            regex: Regex::new(
                r#"^(?:MatchStatus: )?Team playing "(?P<team>CT|TERRORIST)": (?P<team_name>.+)$"#,
            )
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::MatchStatus {
                    team: Team::from_str(c.name("team")?.as_str()),
                    team_name: Some(c.name("team_name")?.as_str().to_string()),
                })
            },

            pretty_fn: |event| {
                let LogEvent::MatchStatus { team, team_name } = event else {
                    return String::new();
                };

                let team_label = match team {
                    Team::CT => "CT",
                    Team::Terrorist => "TERRORIST",
                    Team::Spectator => "SPECTATOR",
                    Team::Unassigned => "UNASSIGNED",
                    Team::Unknown => "UNKNOWN",
                    Team::Allies => "ALLIES",
                    Team::Axis => "AXIS",
                    Team::Blue => "BLUE",
                    Team::Red => "RED",
                };

                format!(
                    "{}{} \x1b[0mplaying {}",
                    team.color_code(),
                    team_label,
                    team_name.as_deref().unwrap_or("UNKNOWN")
                )
            },
        }
    }

    pub fn match_team_unset() -> LogPattern {
        LogPattern {
            id: "MATCH_TEAM_UNSET",
            regex: Regex::new(r#"^MatchStatus: Team "(?P<team>TERRORIST|CT)" is unset\.$"#)
                .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::MatchStatus {
                    team: Team::from_str(c.name("team")?.as_str()),
                    team_name: None,
                })
            },

            pretty_fn: |event| {
                let LogEvent::MatchStatus { team, .. } = event else {
                    return String::new();
                };

                let team_label = match team {
                    Team::CT => "CT",
                    Team::Terrorist => "TERRORIST",
                    Team::Spectator => "SPECTATOR",
                    Team::Unassigned => "UNASSIGNED",
                    Team::Unknown => "UNKNOWN",
                    Team::Allies => "ALLIES",
                    Team::Axis => "AXIS",
                    Team::Blue => "BLUE",
                    Team::Red => "RED",
                };

                format!("{}{} \x1b[0munset", team.color_code(), team_label)
            },
        }
    }

    pub fn round_freeze() -> LogPattern {
        LogPattern {
            id: "ROUND_FREEZE",
            regex: Regex::new(r#"^Starting Freeze period$"#).unwrap(),

            parse_fn: |_line, _c| Some(LogEvent::FreezePeriod),

            pretty_fn: |_event| "\x1b[90m[FREEZE]\x1b[0m Starting Freeze period".to_string(),
        }
    }

    pub fn match_start() -> LogPattern {
        LogPattern {
            id: "MATCH_START",
            regex: Regex::new(r#"^World triggered "Match_Start" on "(?P<map>[^"]+)"$"#).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::MatchStart {
                    map: c.name("map")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::MatchStart { map } = event else {
                    return String::new();
                };

                format!("\x1b[1;32m[MATCH]\x1b[0m started on {}", map)
            },
        }
    }

    pub fn round_team_score() -> LogPattern {
        LogPattern {
        id: "ROUND_TEAM_SCORE",
        regex: Regex::new(
            r#"^Team "(?P<team>CT|TERRORIST)" scored "(?P<score>\d+)" with "(?P<players>\d+)" players$"#,
        )
        .unwrap(),

        parse_fn: |_line, c| {
            Some(LogEvent::TeamScore {
                team: Team::from_str(c.name("team")?.as_str()),
                score: c.name("score")?.as_str().parse().ok()?,
                players: c.name("players")?.as_str().parse().ok()?,
            })
        },

        pretty_fn: |event| {
            let LogEvent::TeamScore {
                team,
                score,
                players,
            } = event
            else {
                return String::new();
            };

            let team_label = match team {
                Team::CT => "CT",
                Team::Terrorist => "TERRORIST",
                Team::Spectator => "SPECTATOR",
                Team::Unassigned => "UNASSIGNED",
                Team::Unknown => "UNKNOWN",
                Team::Allies => "ALLIES",
                Team::Axis => "AXIS",
                Team::Blue => "BLUE",
                Team::Red => "RED",
            };

            format!(
                "{}{} \x1b[0mscored {} with {} players",
                team.color_code(),
                team_label,
                score,
                players
            )
        },
    }
    }

    pub fn server_molotov_spawn() -> LogPattern {
        LogPattern {
            id: "SERVER_MOLOTOV_SPAWN",
            regex: Regex::new(r#"^Molotov projectile spawned at .*?, velocity .*$"#).unwrap(),

            parse_fn: |_line, _c| {
                Some(LogEvent::Technical {
                    name: "Molotov".to_string(),
                    action: "Projectile Spawned".to_string(),
                })
            },

            pretty_fn: |_event| "\x1b[90mMolotov projectile spawned\x1b[0m".to_string(),
        }
    }

    pub fn game_over() -> LogPattern {
        LogPattern {
        id: "GAME_OVER",
        regex: Regex::new(
            r#"^Game Over: (?P<mode>\S+)\s+(?P<map>\S+) score (?P<t_score>\d+):(?P<ct_score>\d+) after (?P<minutes>\d+) min$"#,
        )
        .unwrap(),

        parse_fn: |_line, c| {
            Some(LogEvent::GameOver {
                mode: c.name("mode")?.as_str().to_string(),
                map: c.name("map")?.as_str().to_string(),
                t_score: c.name("t_score")?.as_str().parse().ok()?,
                ct_score: c.name("ct_score")?.as_str().parse().ok()?,
                minutes: c.name("minutes")?.as_str().parse().ok()?,
            })
        },

        pretty_fn: |event| {
            let LogEvent::GameOver {
                mode,
                map,
                t_score,
                ct_score,
                minutes,
            } = event
            else {
                return String::new();
            };

            format!(
                "\x1b[1;33m[GAME OVER]\x1b[0m {} {} {}:{} after {} min",
                mode, map, t_score, ct_score, minutes
            )
        },
    }
    }

    pub fn server_cvar() -> LogPattern {
        LogPattern {
            id: "SERVER_CVAR",
            regex: Regex::new(r#"^(?i:server)_cvar: "(?P<name>[^"]+)" "(?P<value>[^"]*)"$"#)
                .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::ServerCvar {
                    name: c.name("name")?.as_str().to_string(),
                    value: c.name("value")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::ServerCvar { name, value } = event else {
                    return String::new();
                };

                format!("\x1b[90mserver_cvar\x1b[0m {} = {}", name, value)
            },
        }
    }

    pub fn round_accolade() -> LogPattern {
        LogPattern {
        id: "ROUND_ACCOLADE",
        regex: Regex::new(
            r#"^ACCOLADE, FINAL: \{(?P<category>[^}]+)\},\s*(?P<player>[^,]+),\s*VALUE:\s*(?P<value>-?\d+(?:\.\d+)?),\s*POS:\s*(?P<position>\d+),\s*SCORE:\s*(?P<score>-?\d+(?:\.\d+)?)$"#,
        )
        .unwrap(),

        parse_fn: |_line, c| {
            Some(LogEvent::RoundAccolade {
                category: c.name("category")?.as_str().to_string(),

                player: Player {
                    id: 0,
                    name: c.name("player")?.as_str().trim().to_string(),
                    steamid: String::new(),
                    team: Team::Unknown,
                },

                value: c.name("value")?.as_str().parse().ok()?,
                position: c.name("position")?.as_str().parse().ok()?,
                score: c.name("score")?.as_str().parse().ok()?,
            })
        },

        pretty_fn: |event| {
            let LogEvent::RoundAccolade {
                category,
                player,
                value,
                position,
                ..
            } = event
            else {
                return String::new();
            };

            format!(
                "\x1b[33m[ACCOLADE]\x1b[0m {}: {} ({:.2}) [#{}]",
                player.name, category, value, position
            )
        },
    }
    }

    pub fn chat_console() -> LogPattern {
        LogPattern {
            id: "CHAT_CONSOLE",
            regex: Regex::new(r#"^"Console<(?P<userid>\d+)>" say "(?P<msg>.*)"$"#).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Chat {
                    player: Player {
                        id: c.name("userid")?.as_str().parse().ok()?,
                        name: "Console".to_string(),
                        steamid: String::new(),
                        team: Team::Unknown,
                    },

                    msg: c.name("msg")?.as_str().to_string(),
                    is_team_chat: false,
                })
            },

            pretty_fn: |event| {
                let LogEvent::Chat {
                    player,
                    msg,
                    is_team_chat,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "\x1b[36m[CHAT]\x1b[0m {}{}: {}",
                    if *is_team_chat { "[TEAM] " } else { "" },
                    player.name,
                    msg
                )
            },
        }
    }

    pub fn match_pause_enabled() -> LogPattern {
        LogPattern {
            id: "MATCH_PAUSE_ENABLED",
            regex: Regex::new(r#"^Match pause is enabled - mp_pause_match$"#).unwrap(),

            parse_fn: |_line, _c| {
                Some(LogEvent::Technical {
                    name: "Match".to_string(),
                    action: "Pause Enabled".to_string(),
                })
            },

            pretty_fn: |_event| "Match pause enabled".to_string(),
        }
    }

    pub fn match_pause_disabled() -> LogPattern {
        LogPattern {
            id: "MATCH_PAUSE_DISABLED",
            regex: Regex::new(r#"^Match pause is disabled - mp_unpause_match$"#).unwrap(),

            parse_fn: |_line, _c| {
                Some(LogEvent::Technical {
                    name: "Match".to_string(),
                    action: "Pause Disabled".to_string(),
                })
            },

            pretty_fn: |_event| "Match pause disabled".to_string(),
        }
    }

    pub fn server_cvar_dump() -> LogPattern {
        LogPattern {
            id: "SERVER_CVAR_DUMP",
            regex: Regex::new(r#"(?s)^.*(?i:S)erver cvars start\n.*(?i:S)erver cvars end$"#)
                .unwrap(),

            parse_fn: |line, _c| {
                let dump = ServerCvars::parse(line)?;

                Some(LogEvent::ServerCvarDump { cvars: dump.values })
            },

            pretty_fn: |event| {
                let LogEvent::ServerCvarDump { cvars } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[90m[CVARS]\x1b[0m received {} server variables",
                    cvars.len()
                )
            },
        }
    }

    pub fn log_file() -> LogPattern {
        LogPattern {
        id: "LOG_FILE",
        regex: Regex::new(
            r#"^(?:Log file closed|Log file started \(file ".*"\) \(game ".*"\) \(version ".*"\))$"#,
        )
        .unwrap(),

        parse_fn: |line, _c| {
            Some(LogEvent::LogFile {
                started: line.starts_with("Log file started"),
            })
        },

        pretty_fn: |event| {
            let LogEvent::LogFile { started } = event else {
                return String::new();
            };

            if *started {
                "[LOG] file started".to_string()
            } else {
                "[LOG] file closed".to_string()
            }
        },
    }
    }

    pub fn map_loading() -> LogPattern {
        LogPattern {
            id: "MAP_LOADING",
            regex: Regex::new(r#"^Loading map "(?P<map>[^"]+)"$"#).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::MapLoading {
                    map: c.name("map")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::MapLoading { map } = event else {
                    return String::new();
                };

                format!("[MAP] loading {}", map)
            },
        }
    }

    pub fn server_started() -> LogPattern {
        LogPattern {
            id: "SERVER_STARTED",
            regex: Regex::new(r#"^Started:\s*".*"$"#).unwrap(),

            parse_fn: |_line, _c| Some(LogEvent::ServerStarted),

            pretty_fn: |_event| String::from("[SERVER] started"),
        }
    }

    pub fn rcon() -> LogPattern {
        LogPattern {
            id: "RCON",
            regex: Regex::new(r#"^rcon from "(?P<addr>[^"]+)": command "(?P<command>.*)"$"#)
                .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Rcon {
                    addr: c.name("addr")?.as_str().to_string(),
                    command: c.name("command")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Rcon { addr, command } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[38;5;244mrcon from\x1b[0m \"{}\"\x1b[38;5;244m: command\x1b[0m \"{}\"",
                    addr, command
                )
            },
        }
    }
}

pub mod cs16 {
    use regex::Regex;

    use crate::_parser::{
        patterns::{parse_player, LogPattern, LogPatternBlocks},
        types::{LogEvent, Player, Team},
    };

    pub fn chat_server() -> LogPattern {
        LogPattern {
            id: "CHAT_SERVER",
            regex: Regex::new(r#"^Server (?P<type>say|say_team) "(?P<msg>[^"]*)"$"#).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Chat {
                    player: Player {
                        id: 0,
                        name: "Server".to_string(),
                        steamid: String::new(),
                        team: Team::Unknown,
                    },
                    msg: c.name("msg")?.as_str().to_string(),
                    is_team_chat: c.name("type")?.as_str() == "say_team",
                })
            },

            pretty_fn: |event| {
                let LogEvent::Chat {
                    player,
                    msg,
                    is_team_chat,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "\x1b[36m[CHAT]\x1b[0m {}{}: {}",
                    if *is_team_chat { "[TEAM] " } else { "" },
                    player.name,
                    msg
                )
            },
        }
    }
    pub fn rcon16() -> LogPattern {
        LogPattern {
        id: "RCON16",
        regex: Regex::new(
            r#"^Rcon: "rcon (?P<challenge>\d+) (?P<password>[^ ]+) (?P<command>.*)" from "(?P<addr>[^"]+)"$"#
        )
        .unwrap(),

        parse_fn: |_line, c| {
            Some(LogEvent::Rcon {
                addr: c.name("addr")?.as_str().to_string(),
                command: c.name("command")?.as_str().to_string(),
            })
        },

        pretty_fn: |event| {
            let LogEvent::Rcon { addr, command } = event else {
                return String::new();
            };

            format!(
                "\x1b[38;5;244mrcon from\x1b[0m \"{}\"\x1b[38;5;244m: command\x1b[0m \"{}\"",
                addr, command
            )
        },
    }
    }

    pub fn player_team_switch16(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        LogPattern {
            id: "PLAYER_TEAM_SWITCH",
            regex: Regex::new(&format!(r#"^{} joined team "(?P<new>[^"]+)"$"#, player)).unwrap(),

            parse_fn: |_line, c| {
                let mut player = parse_player(c, "")?;

                let from = player.team;
                player.team = Team::from_str(c.name("new")?.as_str());

                Some(LogEvent::TeamSwitch { player, from })
            },

            pretty_fn: |event| {
                let LogEvent::TeamSwitch { player, from } = event else {
                    return String::new();
                };

                format!(
                    "{} switched from {:?} to {:?}",
                    player.name, from, player.team
                )
            },
        }
    }
    pub fn round_triggered16() -> LogPattern {
        LogPattern {
        id: "ROUND_TRIGGERED",
        regex: Regex::new(
            r#"^(?:(?:World)|Team "(?P<team>[^"]+)") triggered "(?P<event>[^"]+)" \(CT "(?P<ct_score>\d+)"\) \(T "(?P<t_score>\d+)"\)$"#
        )
        .unwrap(),

        parse_fn: |_line, c| {
            Some(LogEvent::RoundTrigger {
                team: c.name("team").map(|m| Team::from_str(m.as_str())),
                event: c.name("event")?.as_str().to_string(),
                ct_score: c.name("ct_score")?.as_str().parse().ok()?,
                t_score: c.name("t_score")?.as_str().parse().ok()?,
            })
        },

        pretty_fn: |event| {
            let LogEvent::RoundTrigger {
                team,
                event,
                ct_score,
                t_score,
            } = event
            else {
                return String::new();
            };

            let source = team
                .map(|team| format!("{:?}", team))
                .unwrap_or_else(|| "World".to_string());

            format!(
                "[ROUND] {} triggered {} — CT {} : T {}",
                source, event, ct_score, t_score
            )
        },
    }
    }
}

pub mod dods {
    use regex::Regex;

    use crate::_parser::{
        patterns::{parse_player, LogPattern, LogPatternBlocks},
        types::{LogEvent, Team},
    };

    pub fn player_role_change(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");
        let regex = format!(r##"^{} changed role to "(?P<role>[^"]+)"$"##, player);
        LogPattern {
            id: "PLAYER_ROLE_CHANGE",
            regex: Regex::new(&format!(
                r##"^{} changed role to "(?P<role>[^"]+)"$"##,
                player
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::PlayerRoleChange {
                    player: parse_player(c, "")?,
                    role: c.name("role")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::PlayerRoleChange { player, role } = event else {
                    return String::new();
                };

                format!("{} changed role to {}", player.name, role)
            },
        }
    }

    pub fn point_captured_dods(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("");

        let regex = format!(
            r##"^Team "(?P<capture_team>[^"]+)" triggered "captured_loc" \(flagindex "(?P<point_index>\d+)"\) \(flagname "(?P<point_name>[^"]+)"\) \(numplayers "(?P<numplayers>\d+)"\) \(player {}\)$"##,
            player
        );

        LogPattern {
            id: "POINT_CAPTURED",

            regex: Regex::new(&regex).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::PointCaptured {
                    team: Team::from_str(c.name("capture_team")?.as_str()),
                    point_index: c.name("point_index")?.as_str().parse().ok()?,
                    point_name: c.name("point_name")?.as_str().to_string(),
                    players: vec![parse_player(c, "")?],
                })
            },

            pretty_fn: |event| {
                let LogEvent::PointCaptured {
                    team,
                    point_name,
                    players,
                    ..
                } = event
                else {
                    return String::new();
                };

                let players = players
                    .iter()
                    .map(|player| player.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                if players.is_empty() {
                    format!("{team} captured {point_name}")
                } else {
                    format!("{team} captured {point_name} by {players}")
                }
            },
        }
    }
}

pub mod tf2 {
    use regex::Regex;

    use crate::_parser::{
        patterns::{parse_player, LogPattern, LogPatternBlocks},
        types::{LogEvent, Team},
    };

    pub fn point_captured_tf2(blocks: &LogPatternBlocks) -> LogPattern {
        let player = blocks.player("player1");

        let regex = format!(
            r##"^Team "(?P<team>[^"]+)" triggered "pointcaptured" \(cp "(?P<point_index>\d+)"\) \(cpname "(?P<point_name>[^"]+)"\) \(numcappers "(?P<num_cappers>\d+)"\) \(player1 {}\) \(position1 "(?P<position>[^"]+)"\)$"##,
            player
        );

        LogPattern {
            id: "POINT_CAPTURED",

            regex: Regex::new(&regex).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::PointCaptured {
                    team: Team::from_str(c.name("team")?.as_str()),
                    point_index: c.name("point_index")?.as_str().parse().ok()?,
                    point_name: c.name("point_name")?.as_str().to_string(),
                    players: vec![parse_player(c, "player1")?],
                })
            },

            pretty_fn: |event| {
                let LogEvent::PointCaptured {
                    team,
                    point_name,
                    players,
                    ..
                } = event
                else {
                    return String::new();
                };

                let players = players
                    .iter()
                    .map(|player| player.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                if players.is_empty() {
                    format!("{team} captured {point_name}")
                } else {
                    format!("{team} captured {point_name} by {players}")
                }
            },
        }
    }

    pub fn tick_score(_blocks: &LogPatternBlocks) -> LogPattern {
        let regex = concat!(
            r#"^Team "(?P<score_team>[^"]+)" triggered "tick_score" "#,
            r#"\(score "(?P<score>\d+)"\) "#,
            r#"\(totalscore "(?P<total_score>\d+)"\) "#,
            r#"\(numplayers "(?P<num_players>\d+)"\)$"#
        );

        LogPattern {
            id: "TICK_SCORE",
            regex: Regex::new(regex).unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::TickScore {
                    team: Team::from_str(c.name("score_team")?.as_str()),
                    score_delta: c.name("score")?.as_str().parse().ok()?,
                    total_score: c.name("total_score")?.as_str().parse().ok()?,
                    num_players: c.name("num_players")?.as_str().parse().ok()?,
                })
            },

            pretty_fn: |event| {
                let LogEvent::TickScore {
                    team,
                    score_delta,
                    total_score,
                    num_players,
                } = event
                else {
                    return String::new();
                };

                format!("{team} scored {score_delta} ({total_score} total, {num_players} players)")
            },
        }
    }
}
