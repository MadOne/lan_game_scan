use crate::log_patterns::*; // Assuming helpers are here
use crate::parser::{LogEvent, Player, Team};
use crate::round_stats::parse_player;
use regex::Regex;

// --- Pattern Factory Suite ---

pub fn attacked_pattern() -> LogPattern {
    let p_at = get_player_re("at_");
    let p_vic = get_player_re("vic_");
    let pos_at = get_pos_re("at_");
    let pos_vic = get_pos_re("vic_");

    LogPattern {
        id: "PLAYER_DAMAGED",
        regex: Regex::new(&format!(
            r#"^{} {} attacked {} {} with "(?P<weapon>[^"]+)" {}$"#,
            p_at, pos_at, p_vic, pos_vic, STATS_BLOCK
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
        pretty_fn: |e| {
            if let LogEvent::Attacked {
                attacker,
                victim,
                damage,
                weapon,
                hitgroup,
            } = e
            {
                format!(
                    "\x1b[90m{}{}\x1b[0m hit \x1b[90m{}{}\x1b[0m for {} ({}) in {}",
                    attacker.team.color_code(),
                    attacker.name,
                    victim.team.color_code(),
                    victim.name,
                    damage,
                    weapon,
                    hitgroup
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn kill_pattern() -> LogPattern {
    let p_at = get_player_re("at_");
    let p_vic = get_player_re("vic_");
    let pos_at = get_pos_re("at_");
    let pos_vic = get_pos_re("vic_");

    LogPattern {
        id: "PLAYER_KILLED",
        regex: Regex::new(&format!(r#"^{} {} killed (?:other (?P<vic_other>".+?"))?{} {} with "(?P<weapon>[^"]+)"(?P<hs> \(headshot\))?(?P<pen> \(penetrated\))?(?P<smoke> \(throughsmoke\))?(?P<air> \(attackerinair\))?$"#, p_at, pos_at, p_vic, pos_vic)).unwrap(),
        parse_fn: |_line, c| {
            let attacker = parse_player(c, "at_")?;
            let victim = parse_player(c, "vic_").unwrap_or_else(|| Player { id: 0, name: c.name("vic_other")?.as_str().trim_matches('"').to_string(), steamid: String::new(), team: Team::Unknown });
            Some(LogEvent::Kill { attacker, victim, weapon: c.name("weapon")?.as_str().to_string(), headshot: c.name("hs").is_some(), penetrated: c.name("pen").is_some(), through_smoke: c.name("smoke").is_some(), attacker_in_air: c.name("air").is_some() })
        },
        pretty_fn: |e| if let LogEvent::Kill { attacker, victim, weapon, headshot, penetrated, through_smoke, attacker_in_air } = e {
            format!("{}{}\x1b[0m killed {}{}\x1b[0m with {}{}{}{}{}", attacker.team.color_code(), attacker.name, victim.team.color_code(), victim.name, weapon, if *headshot{" 🎯"}else{""}, if *penetrated{" 🧱"}else{""}, if *through_smoke{" 💨"}else{""}, if *attacker_in_air{" 🪽"}else{""})
        } else { String::new() },
    }
}

pub fn chat_pattern() -> LogPattern {
    let p_none = get_player_re("");
    LogPattern {
        id: "CHAT",
        regex: Regex::new(&format!(
            r#"^{} (?P<type>say|say_team) "(?P<msg>[^"]*)"$"#,
            p_none
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Chat {
                player: parse_player(c, "")?,
                msg: c.name("msg")?.as_str().to_string(),
                is_team_chat: c.name("type")?.as_str() == "say_team",
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Chat {
                player,
                msg,
                is_team_chat,
            } = e
            {
                format!(
                    "\x1b[1m{} {}{}\x1b[0m: {}",
                    if *is_team_chat { "[TEAM]" } else { "[ALL ]" },
                    player.team.color_code(),
                    player.name,
                    msg
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn score_pattern() -> LogPattern {
    LogPattern {
        id: "MATCH_SCORE",
        regex: Regex::new(r#"^MatchStatus: Score: (?P<t1>\d+):(?P<t2>\d+) on map "(?P<map>.+)" RoundsPlayed: (?P<rounds>-?\d+)$"#).unwrap(),
        parse_fn: |_line, c| Some(LogEvent::ScoreUpdate { t1: c.name("t1")?.as_str().parse().ok()?, t2: c.name("t2")?.as_str().parse().ok()?, map: c.name("map")?.as_str().to_string(), rounds: c.name("rounds")?.as_str().parse().ok()? }),
        pretty_fn: |e| if let LogEvent::ScoreUpdate { t1, t2, map, .. } = e { format!("\x1b[1;33m[SCORE]\x1b[0m \x1b[31mT {}\x1b[0m - \x1b[34m{} CT\x1b[0m on {}", t1, t2, map) } else { String::new() },
    }
}

pub fn round_win_pattern() -> LogPattern {
    LogPattern {
        id: "ROUND_WIN",
        regex: Regex::new(r#"^Team "(?P<team>[^"]+)" triggered "SFUI_Notice_(?P<reason>[^"]+)" \(CT "(?P<ct_score>\d+)"\) \(T "(?P<t_score>\d+)"\)$"#).unwrap(),
        parse_fn: |_line, c| {
            let reason = c.name("reason")?.as_str();
            let winner_side = match reason { "Target_Bombed" => Team::Terrorist, "Target_Saved" | "Bomb_Defused" | "CTs_Win" => Team::CT, "Terrorists_Win" => Team::Terrorist, _ => return None };
            Some(LogEvent::RoundWin { team: c.name("team")?.as_str().to_string(), winner_side, reason: reason.to_string(), ct_score: c.name("ct_score")?.as_str().parse().ok()?, t_score: c.name("t_score")?.as_str().parse().ok()? })
        },
        pretty_fn: |e| if let LogEvent::RoundWin { winner_side, reason, ct_score, t_score, .. } = e {
            format!("{}{} WON\x1b[0m — {} | CT {} : T {}", winner_side.color_code(), match winner_side { Team::CT => "CT", Team::Terrorist => "TERRORIST", _ => "UNKNOWN" }, reason.replace('_', " "), ct_score, t_score)
        } else { String::new() },
    }
}

pub fn team_switch_pattern() -> LogPattern {
    let p_none = get_player_re("");
    LogPattern {
        id: "PLAYER_TEAM_SWITCH",
        regex: Regex::new(&format!(
            r#"^{} switched from team <(?P<old>[^>]*)> to <(?P<new>[^>]*)>$"#,
            p_none
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            let mut player = parse_player(c, "")?;
            let from = Team::from_str(c.name("old")?.as_str());
            player.team = Team::from_str(c.name("new")?.as_str());
            Some(LogEvent::TeamSwitch { player, from })
        },
        pretty_fn: |e| {
            if let LogEvent::TeamSwitch { player, from } = e {
                format!(
                    "{} switched from {:?} to {:?}",
                    player.name, from, player.team
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn bomb_event_pattern() -> LogPattern {
    let p_none = get_player_re("");
    LogPattern {
        id: "BOMB_EVENT",
        regex: Regex::new(&format!(
            r#"^{} triggered "(?P<event>[^"]+)"(?: at bombsite (?P<site>[AB]))?$"#,
            p_none
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::BombEvent {
                player: parse_player(c, "")?,
                event: c.name("event")?.as_str().to_string(),
                site: c.name("site").map(|m| m.as_str().to_string()),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::BombEvent {
                player,
                event,
                site,
            } = e
            {
                format!(
                    "{}{} \x1b[0m{}{}",
                    player.team.color_code(),
                    player.name,
                    event,
                    site.as_ref()
                        .map(|s| format!(" at site {}", s))
                        .unwrap_or_default()
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn purchase_pattern() -> LogPattern {
    let p_none = get_player_re("");
    LogPattern {
        id: "PLAYER_PURCHASE",
        regex: Regex::new(&format!(r#"^{} purchased "(?P<item>[^"]+)"$"#, p_none)).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Purchase {
                player: parse_player(c, "")?,
                item: c.name("item")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Purchase { player, item } = e {
                format!(
                    "{}{} \x1b[0mpurchased {}",
                    player.team.color_code(),
                    player.name,
                    item
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn connection_patterns() -> Vec<LogPattern> {
    let p_none = get_player_re("");
    vec![
        LogPattern {
            id: "PLAYER_DISCONNECTED",
            regex: Regex::new(&format!(
                r#"^{} disconnected \(reason "(?P<reason>.+)"\)$"#,
                p_none
            ))
            .unwrap(),
            parse_fn: |_line, c| {
                Some(LogEvent::Connection {
                    player: parse_player(c, "")?,
                    action: "Disconnected".to_string(),
                    info: Some(c.name("reason")?.as_str().to_string()),
                })
            },
            pretty_fn: |e| {
                if let LogEvent::Connection { player, action, .. } = e {
                    format!("\x1b[36m{}\x1b[0m \x1b[1;35m{}\x1b[0m", player.name, action)
                } else {
                    String::new()
                }
            },
        },
        LogPattern {
            id: "PLAYER_CONNECTED",
            regex: Regex::new(&format!(
                r#"^{} connected, address "(?P<addr>.+)"$"#,
                p_none
            ))
            .unwrap(),
            parse_fn: |_line, c| {
                Some(LogEvent::Connection {
                    player: parse_player(c, "")?,
                    action: "Handshake".to_string(),
                    info: Some(c.name("addr")?.as_str().to_string()),
                })
            },
            pretty_fn: |e| {
                if let LogEvent::Connection { player, action, .. } = e {
                    format!("\x1b[36m{}\x1b[0m \x1b[1;35m{}\x1b[0m", player.name, action)
                } else {
                    String::new()
                }
            },
        },
        LogPattern {
            id: "PLAYER_ENTERED_GAME",
            regex: Regex::new(&format!(r#"^{} entered the game$"#, p_none)).unwrap(),
            parse_fn: |_line, c| {
                Some(LogEvent::Connection {
                    player: parse_player(c, "")?,
                    action: "Entered game".to_string(),
                    info: None,
                })
            },
            pretty_fn: |e| {
                if let LogEvent::Connection { player, action, .. } = e {
                    format!("\x1b[36m{}\x1b[0m \x1b[1;35m{}\x1b[0m", player.name, action)
                } else {
                    String::new()
                }
            },
        },
    ]
}

pub fn suicide_pattern() -> LogPattern {
    let p_none = get_player_re("");
    let pos_none = get_pos_re("");
    LogPattern {
        id: "PLAYER_SUICIDE",
        regex: Regex::new(&format!(
            r#"^{} {} committed suicide with "(?P<weapon>.+)"$"#,
            p_none, pos_none
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Suicide {
                player: parse_player(c, "")?,
                weapon: c.name("weapon")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Suicide { player, weapon } = e {
                format!(
                    "{}{} committed suicide with {}",
                    player.team.color_code(),
                    player.name,
                    weapon
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn world_trigger_pattern() -> LogPattern {
    LogPattern {
        id: "WORLD_TRIGGER",
        regex: Regex::new(r#"^World triggered "(?P<event>[^"]+)"$"#).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::WorldTrigger {
                event: c.name("event")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::WorldTrigger { event } = e {
                format!("\x1b[35m[WORLD]\x1b[0m {}", event)
            } else {
                String::new()
            }
        },
    }
}

pub fn validated_pattern() -> LogPattern {
    let p_none = get_player_re("");
    LogPattern {
        id: "PLAYER_VALIDATED",
        regex: Regex::new(&format!(r#"^{} STEAM USERID validated$"#, p_none)).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Technical {
                name: c.name("name")?.as_str().to_string(),
                action: "Status Change".to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Technical { name, action } = e {
                format!("\x1b[36m{}\x1b[0m \x1b[1;35m{}\x1b[0m", name, action)
            } else {
                String::new()
            }
        },
    }
}

pub fn left_buyzone_pattern() -> LogPattern {
    let p_none = get_player_re("");
    LogPattern {
        id: "PLAYER_LEFT_BUYZONE",
        regex: Regex::new(&format!(
            r#"^{} left buyzone with \[\s*(?P<items>.*?)\s*\]$"#,
            p_none
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::LeftBuyZone {
                player: parse_player(c, "")?,
                items: c
                    .name("items")?
                    .as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::LeftBuyZone { player, items } = e {
                format!(
                    "{}\x1b[0m left buyzone with \x1b[33m{:?}\x1b[0m",
                    player.name, items
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn round_stats_pattern() -> LogPattern {
    LogPattern {
        id: "ROUND_STATS",
        regex: Regex::new(r#"(?s)^JSON_BEGIN.*JSON_END$"#).unwrap(),
        parse_fn: |line, _| parse_round_stats(line).map(|s| LogEvent::RoundStats { roundstats: s }),
        pretty_fn: |e| {
            if let LogEvent::RoundStats { roundstats } = e {
                let mut output = format!(
                    "Round {} | {} {}:{} | {}\n",
                    roundstats.round_number,
                    roundstats.map,
                    roundstats.score_t,
                    roundstats.score_ct,
                    roundstats.server
                );
                for (n, p) in &roundstats.players {
                    let color = if p.team == 2 {
                        "\x1b[31m"
                    } else if p.team == 3 {
                        "\x1b[34m"
                    } else {
                        "\x1b[37m"
                    };
                    output.push_str(&format!(
                        "{}{} {:>2}: {:>8} | K/D/A {}/{}/{} | DMG {:>5}\x1b[0m\n",
                        color, "", n, p.account_id, p.kills, p.deaths, p.assists, p.damage
                    ));
                }
                output
            } else {
                String::new()
            }
        },
    }
}

pub fn assist_pattern() -> LogPattern {
    LogPattern {
        id: "PLAYER_ASSIST",
        regex: Regex::new(&format!(
            r#"^{} assisted killing {}$"#,
            get_player_re("assister_"),
            get_player_re("victim_")
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Assist {
                assister: parse_player(c, "assister_")?,
                victim: parse_player(c, "victim_")?,
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Assist { assister, victim } = e {
                format!(
                    "{}{}\x1b[0m assisted killing {}{}\x1b[0m",
                    assister.team.color_code(),
                    assister.name,
                    victim.team.color_code(),
                    victim.name
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn grenade_pattern() -> LogPattern {
    let p_none = get_player_re("");
    LogPattern {
        id: "PLAYER_GRENADE_THROW",
        regex: Regex::new(&format!(
            r#"^{} threw (?P<grenade>[a-zA-Z0-9_]+) \[[-\d\s]+\].*$"#,
            p_none
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Grenade {
                player: parse_player(c, "")?,
                grenade: c.name("grenade")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Grenade { player, grenade } = e {
                format!(
                    "{}{} threw {}",
                    player.team.color_code(),
                    player.name,
                    grenade
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn sv_grenade_pattern() -> LogPattern {
    let p_none = get_player_re("");
    LogPattern {
        id: "SERVER_GRENADE_THROW",
        regex: Regex::new(&format!(r#"^{} sv_throw_(?P<grenade>[a-z]+) .*$"#, p_none)).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::SvGrenade {
                player: parse_player(c, "")?,
                grenade: c.name("grenade")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::SvGrenade { player, grenade } = e {
                format!(
                    "\x1b[90m{}{}\x1b[0m sv_throw_{}",
                    player.team.color_code(),
                    player.name,
                    grenade
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn blinded_pattern() -> LogPattern {
    LogPattern {
        id: "PLAYER_BLINDED",
        regex: Regex::new(&format!(
            r#"^{} blinded for (?P<duration>\d+(?:\.\d+)?) by {} from flashbang entindex \d+$"#,
            get_player_re("at_"),
            get_player_re("vic_")
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Blinded {
                attacker: parse_player(c, "at_")?,
                victim: parse_player(c, "vic_")?,
                duration: c.name("duration")?.as_str().parse().ok()?,
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Blinded {
                attacker,
                victim,
                duration,
            } = e
            {
                format!(
                    "{}{} blinded {}{} for {:.2}s",
                    attacker.team.color_code(),
                    attacker.name,
                    victim.team.color_code(),
                    victim.name,
                    duration
                )
            } else {
                String::new()
            }
        },
    }
}

pub fn match_status_patterns() -> Vec<LogPattern> {
    vec![
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
            pretty_fn: |e| {
                if let LogEvent::MatchStatus { team, team_name } = e {
                    format!(
                        "{}{} playing {}",
                        team.color_code(),
                        "TEAM",
                        team_name.as_deref().unwrap_or("...")
                    )
                } else {
                    String::new()
                }
            },
        },
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
            pretty_fn: |e| {
                if let LogEvent::MatchStatus { team, .. } = e {
                    format!("{}{} unset", team.color_code(), "TEAM")
                } else {
                    String::new()
                }
            },
        },
    ]
}

pub fn freeze_period_pattern() -> LogPattern {
    LogPattern {
        id: "ROUND_FREEZE",
        regex: Regex::new(r#"^Starting Freeze period$"#).unwrap(),
        parse_fn: |_line, _| Some(LogEvent::FreezePeriod),
        pretty_fn: |_| "\x1b[90m[FREEZE]\x1b[0m Starting".to_string(),
    }
}

pub fn match_start_pattern() -> LogPattern {
    LogPattern {
        id: "MATCH_START",
        regex: Regex::new(r#"^World triggered "Match_Start" on "(?P<map>[^"]+)"$"#).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::MatchStart {
                map: c.name("map")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::MatchStart { map } = e {
                format!("\x1b[1;32m[MATCH]\x1b[0m started on {}", map)
            } else {
                String::new()
            }
        },
    }
}

pub fn team_score_pattern() -> LogPattern {
    LogPattern {
        id: "ROUND_TEAM_SCORE",
        regex: Regex::new(r#"^Team "(?P<team>CT|TERRORIST)" scored "(?P<score>\d+)" with "(?P<players>\d+)" players$"#).unwrap(),
        parse_fn: |_line, c| Some(LogEvent::TeamScore { team: Team::from_str(c.name("team")?.as_str()), score: c.name("score")?.as_str().parse().ok()?, players: c.name("players")?.as_str().parse().ok()? }),
        pretty_fn: |e| if let LogEvent::TeamScore { team, score, .. } = e { format!("{}{} scored {}", team.color_code(), "TEAM", score) } else { String::new() },
    }
}

pub fn molotov_spawn_pattern() -> LogPattern {
    LogPattern {
        id: "SERVER_MOLOTOV_SPAWN",
        regex: Regex::new(r#"^Molotov projectile spawned at .*$"#).unwrap(),
        parse_fn: |_line, _| {
            Some(LogEvent::Technical {
                name: "Molotov".to_string(),
                action: "Projectile Spawned".to_string(),
            })
        },
        pretty_fn: |_| "\x1b[90mMolotov spawned\x1b[0m".to_string(),
    }
}

pub fn game_over_pattern() -> LogPattern {
    LogPattern {
        id: "GAME_OVER",
        regex: Regex::new(r#"^Game Over: (?P<mode>\S+) (?P<map>\S+) score (?P<t_score>\d+):(?P<ct_score>\d+) after (?P<minutes>\d+) min$"#).unwrap(),
        parse_fn: |_line, c| Some(LogEvent::GameOver { mode: c.name("mode")?.as_str().to_string(), map: c.name("map")?.as_str().to_string(), t_score: c.name("t_score")?.as_str().parse().ok()?, ct_score: c.name("ct_score")?.as_str().parse().ok()?, minutes: c.name("minutes")?.as_str().parse().ok()? }),
        pretty_fn: |e| if let LogEvent::GameOver { mode, map, .. } = e { format!("\x1b[1;33m[GAME OVER]\x1b[0m {} {}", mode, map) } else { String::new() },
    }
}

pub fn server_cvar_pattern() -> LogPattern {
    LogPattern {
        id: "SERVER_CVAR",
        regex: Regex::new(r#"^server_cvar: "(?P<name>[^"]+)" "(?P<value>[^"]*)"$"#).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::ServerCvar {
                name: c.name("name")?.as_str().to_string(),
                value: c.name("value")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::ServerCvar { name, value } = e {
                format!("\x1b[90mserver_cvar\x1b[0m {} = {}", name, value)
            } else {
                String::new()
            }
        },
    }
}

pub fn bomb_death_pattern() -> LogPattern {
    LogPattern {
        id: "PLAYER_BOMB_DEATH",
        regex: Regex::new(&format!(
            r#"^{} {} was killed by the bomb\.$"#,
            get_player_re(""),
            get_pos_re("")
        ))
        .unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::BombDeath {
                player: parse_player(c, "")?,
            })
        },
        pretty_fn: |e| {
            if let LogEvent::BombDeath { player } = e {
                format!("{}{} killed by bomb", player.team.color_code(), player.name)
            } else {
                String::new()
            }
        },
    }
}

pub fn accolade_pattern() -> LogPattern {
    LogPattern {
        id: "ROUND_ACCOLADE",
        regex: Regex::new(r#"^ACCOLADE, FINAL: \{(?P<category>[^}]+)\},\s*(?P<player>[^,]+),\s*VALUE:\s*(?P<value>-?\d+(?:\.\d+)?),\s*POS:\s*(?P<position>\d+),\s*SCORE:\s*(?P<score>-?\d+(?:\.\d+)?)$"#).unwrap(),
        parse_fn: |_line, c| Some(LogEvent::RoundAccolade { category: c.name("category")?.as_str().to_string(), player: Player { id: 0, name: c.name("player")?.as_str().trim().to_string(), steamid: "".into(), team: Team::Unknown }, value: c.name("value")?.as_str().parse().ok()?, position: c.name("position")?.as_str().parse().ok()?, score: c.name("score")?.as_str().parse().ok()? }),
        pretty_fn: |e| if let LogEvent::RoundAccolade { category, player, .. } = e { format!("\x1b[33m[ACCOLADE]\x1b[0m {}: {}", player.name, category) } else { String::new() },
    }
}

pub fn console_say_pattern() -> LogPattern {
    LogPattern {
        id: "CHAT_CONSOLE",
        regex: Regex::new(r#"^"Console<(?P<userid>\d+)>" say "(?P<msg>.*)"$"#).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Chat {
                player: Player {
                    id: c.name("userid")?.as_str().parse().ok()?,
                    name: "Console".into(),
                    steamid: "".into(),
                    team: Team::Unknown,
                },
                msg: c.name("msg")?.as_str().to_string(),
                is_team_chat: false,
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Chat { player, msg, .. } = e {
                format!("\x1b[36m[CHAT]\x1b[0m {}: {}", player.name, msg)
            } else {
                String::new()
            }
        },
    }
}

pub fn pause_patterns() -> Vec<LogPattern> {
    vec![
        LogPattern {
            id: "MATCH_PAUSE_ENABLED",
            regex: Regex::new(r#"^Match pause is enabled - mp_pause_match$"#).unwrap(),
            parse_fn: |_line, _| {
                Some(LogEvent::Technical {
                    name: "Match".into(),
                    action: "Pause Enabled".into(),
                })
            },
            pretty_fn: |_| "Match pause enabled".into(),
        },
        LogPattern {
            id: "MATCH_PAUSE_DISABLED",
            regex: Regex::new(r#"^Match pause is disabled - mp_unpause_match$"#).unwrap(),
            parse_fn: |_line, _| {
                Some(LogEvent::Technical {
                    name: "Match".into(),
                    action: "Pause Disabled".into(),
                })
            },
            pretty_fn: |_| "Match pause disabled".into(),
        },
    ]
}

pub fn cvar_dump_pattern() -> LogPattern {
    LogPattern {
        id: "SERVER_CVAR_DUMP",
        regex: Regex::new(r#"(?s)^.*server cvars start\n.*server cvars end$"#).unwrap(),
        parse_fn: |line, _| {
            ServerCvars::parse(line).map(|d| LogEvent::ServerCvarDump { cvars: d.values })
        },
        pretty_fn: |e| {
            if let LogEvent::ServerCvarDump { cvars } = e {
                format!("[CVARS] {}", cvars.len())
            } else {
                String::new()
            }
        },
    }
}

pub fn log_file_pattern() -> LogPattern {
    LogPattern {
        id: "LOG_FILE",
        regex: Regex::new(r#"^Log file (?:closed|started)"#).unwrap(),
        parse_fn: |line, _| {
            Some(LogEvent::LogFile {
                started: line.starts_with("Log file started"),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::LogFile { started } = e {
                format!("[LOG] {}", if *started { "started" } else { "closed" })
            } else {
                String::new()
            }
        },
    }
}

pub fn map_loading_pattern() -> LogPattern {
    LogPattern {
        id: "MAP_LOADING",
        regex: Regex::new(r#"^Loading map "(?P<map>[^"]+)"$"#).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::MapLoading {
                map: c.name("map")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::MapLoading { map } = e {
                format!("[MAP] {}", map)
            } else {
                String::new()
            }
        },
    }
}

pub fn server_started_pattern() -> LogPattern {
    LogPattern {
        id: "SERVER_STARTED",
        regex: Regex::new(r#"^Started:\s*".*"$"#).unwrap(),
        parse_fn: |_line, _| Some(LogEvent::ServerStarted),
        pretty_fn: |_| "[SERVER] started".to_string(),
    }
}

pub fn rcon_pattern() -> LogPattern {
    LogPattern {
        id: "RCON",
        regex: Regex::new(r#"^rcon from "(?P<addr>[^"]+)": command "(?P<command>.*)"$"#).unwrap(),
        parse_fn: |_line, c| {
            Some(LogEvent::Rcon {
                addr: c.name("addr")?.as_str().to_string(),
                command: c.name("command")?.as_str().to_string(),
            })
        },
        pretty_fn: |e| {
            if let LogEvent::Rcon { addr, command } = e {
                format!(
                    "\x1b[38;5;244mrcon from\x1b[0m \"{}\"\x1b[38;5;244m: command\x1b[0m \"{}\"",
                    addr, command
                )
            } else {
                String::new()
            }
        },
    }
}
