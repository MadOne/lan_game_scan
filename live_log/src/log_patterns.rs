// -----------------------------------------------------------------------------
// log_patterns.rs
// -----------------------------------------------------------------------------

use regex::{Captures, Regex};

use crate::{
    cvar_parser::ServerCvars,
    parser::{LogEvent, Player, Team},
    round_stats::parse_round_stats,
};

// -----------------------------------------------------------------------------
// EXPOSED BUILDING BLOCKS
// -----------------------------------------------------------------------------

pub const TS_BLOCK: &str = r"(?:\[LOG\]\s+)?(?P<ts>\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}\.\d{3})";

// -----------------------------------------------------------------------------
// PLAYER BLOCK
// -----------------------------------------------------------------------------
//
// CS2 player format:
//
//     "PlayerName<userid><steamid><team>"
//
// Examples:
//
//     "Player<5><[U:1:123456]><CT>"
//     "Bot<7><BOT><TERRORIST>"
//     "Player<4><STEAM_ID_PENDING><CT>"
//
// IMPORTANT:
//
// `id` is the CS2 userid.
//
// This is the identifier used when matching players against RoundStats.
//
// `steamid` is kept separately and may also contain:
//
//     BOT
//     STEAM_ID_PENDING
//
// -----------------------------------------------------------------------------

pub const STEAMID_BLOCK: &str = r"(?:\[[A-Z]:1:\d+\]|BOT|STEAM_ID_PENDING)";

pub const PLAYER_BLOCK_BASE: &str =
    r#""(?P<{0}name>[^<]+)<(?P<{0}id>\d+)><(?P<{0}steamid>{STEAMID})>(?:<(?P<{0}team>[^>]*)>)?""#;

pub const POS_BLOCK_BASE: &str = r#"\[(?P<{0}pos>-?\d+ -?\d+ -?\d+)\]"#;

pub const STATS_BLOCK: &str = r#"\(damage "(?P<dmg>\d+)"\) \(damage_armor "(?P<dmg_arm>\d+)"\) \(health "(?P<hp>\d+)"\) \(armor "(?P<arm>\d+)"\) \(hitgroup "(?P<hit>[^"]+)"\)"#;

pub fn get_player_re(prefix: &str) -> String {
    PLAYER_BLOCK_BASE
        .replace("{0}", prefix)
        .replace("{STEAMID}", STEAMID_BLOCK)
}

pub fn get_pos_re(prefix: &str) -> String {
    POS_BLOCK_BASE.replace("{0}", prefix)
}

// -----------------------------------------------------------------------------
// PLAYER PARSER
// -----------------------------------------------------------------------------
//
// Converts a matched player block into the new Player architecture.
//
// `id` is deliberately parsed from the CS2 userid field.
//
// `steamid` remains a String because the source may contain:
//     [U:1:...]
//     BOT
//     STEAM_ID_PENDING
//
// -----------------------------------------------------------------------------

fn parse_player(c: &Captures, prefix: &str) -> Option<Player> {
    let name = c.name(&format!("{prefix}name"))?.as_str().to_string();

    let id = c
        .name(&format!("{prefix}id"))?
        .as_str()
        .parse::<u16>()
        .ok()?;

    let steamid = c.name(&format!("{prefix}steamid"))?.as_str().to_string();

    let team = c
        .name(&format!("{prefix}team"))
        .map(|m| Team::from_str(m.as_str()))
        .unwrap_or(Team::Unknown);

    Some(Player {
        id,
        name,
        steamid,
        team,
    })
}

// -----------------------------------------------------------------------------
// LOG PATTERN
// -----------------------------------------------------------------------------

pub struct LogPattern {
    pub id: &'static str,
    pub regex: Regex,
    parse_fn: fn(&str, &Captures) -> Option<LogEvent>,
    pretty_fn: fn(&LogEvent) -> String,
}

impl LogPattern {
    pub fn parse(&self, line: &str, caps: &Captures) -> Option<LogEvent> {
        (self.parse_fn)(line, caps)
    }

    pub fn pretty(&self, event: &LogEvent) -> String {
        (self.pretty_fn)(event)
    }
}

// -----------------------------------------------------------------------------
// BUILD PATTERNS
// -----------------------------------------------------------------------------

pub fn build_patterns() -> Vec<LogPattern> {
    let p_at = get_player_re("at_");
    let p_vic = get_player_re("vic_");
    let p_none = get_player_re("");

    let pos_at = get_pos_re("at_");
    let pos_vic = get_pos_re("vic_");
    let pos_none = get_pos_re("");

    vec![
        // ---------------------------------------------------------------------
        // 0: Attacked
        // ---------------------------------------------------------------------
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
        },

        // ---------------------------------------------------------------------
        // 1: Kill
        // ---------------------------------------------------------------------
        LogPattern {
            id: "PLAYER_KILLED",
            regex: Regex::new(&format!(
                r#"^{} {} killed (?:other (?P<vic_other>".+?"))?{} {} with "(?P<weapon>[^"]+)"(?P<hs> \(headshot\))?(?P<pen> \(penetrated\))?(?P<smoke> \(throughsmoke\))?(?P<air> \(attackerinair\))?$"#,
                p_at, pos_at, p_vic, pos_vic
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
        },

        // ---------------------------------------------------------------------
        // 2: Chat
        // ---------------------------------------------------------------------
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

            pretty_fn: |event| {
                let LogEvent::Chat {
                    player,
                    msg,
                    is_team_chat,
                } = event
                else {
                    return String::new();
                };

                let tag = if *is_team_chat {
                    "[TEAM]"
                } else {
                    "[ALL ]"
                };

                format!(
                    "\x1b[1m{} {}{}\x1b[0m: {}",
                    tag,
                    player.team.color_code(),
                    player.name,
                    msg
                )
            },
        },

        // ---------------------------------------------------------------------
        // 3: Score Update
        // ---------------------------------------------------------------------
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
        },

        // ---------------------------------------------------------------------
        // 4: Round Win
        // ---------------------------------------------------------------------
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
                        Team::Spectator
                        | Team::Unassigned
                        | Team::Unknown => "UNKNOWN",
                    },
                    reason,
                    ct_score,
                    t_score,
                )
            },
        },

        // ---------------------------------------------------------------------
        // 5: Team Switch
        // ---------------------------------------------------------------------
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

                Some(LogEvent::TeamSwitch {
                    player,
                    from,
                })
            },

            pretty_fn: |event| {
                let LogEvent::TeamSwitch {
                    player,
                    from,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "{} switched from {:?} to {:?}",
                    player.name,
                    from,
                    player.team
                )
            },
        },

        // ---------------------------------------------------------------------
        // 6: Bomb Event
        // ---------------------------------------------------------------------
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
        },

        // ---------------------------------------------------------------------
        // 7: Purchase
        // ---------------------------------------------------------------------
        LogPattern {
            id: "PLAYER_PURCHASE",
            regex: Regex::new(&format!(
                r#"^{} purchased "(?P<item>[^"]+)"$"#,
                p_none
            ))
            .unwrap(),

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
                    "{}{} \x1b[0mpurchased {}",
                    player.team.color_code(),
                    player.name,
                    item
                )
            },
        },

        // ---------------------------------------------------------------------
        // 8: Disconnect
        // ---------------------------------------------------------------------
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

            pretty_fn: |event| {
                let LogEvent::Connection { player, action, .. } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[36m{}\x1b[0m \
                     \x1b[1;35m{}\x1b[0m",
                    player.name,
                    action
                )
            },
        },

        // ---------------------------------------------------------------------
        // 9: Handshake
        // ---------------------------------------------------------------------
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

            pretty_fn: |event| {
                let LogEvent::Connection { player, action, .. } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[36m{}\x1b[0m \
                     \x1b[1;35m{}\x1b[0m",
                    player.name,
                    action
                )
            },
        },

        // ---------------------------------------------------------------------
        // 10: Entered Game
        // ---------------------------------------------------------------------
        LogPattern {
            id: "PLAYER_ENTERED_GAME",
            regex: Regex::new(&format!(
                r#"^{} entered the game$"#,
                p_none
            ))
            .unwrap(),

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
                    player.name,
                    action
                )
            },
        },

        // ---------------------------------------------------------------------
        // 11: Suicide
        // ---------------------------------------------------------------------
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

            pretty_fn: |event| {
                let LogEvent::Suicide {
                    player,
                    weapon,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "{}{}\x1b[0m committed suicide with {}",
                    player.team.color_code(),
                    player.name,
                    weapon
                )
            },
        },

        // ---------------------------------------------------------------------
        // 12: World Trigger
        // ---------------------------------------------------------------------
        LogPattern {
            id: "WORLD_TRIGGER",
            regex: Regex::new(
                r#"^World triggered "(?P<event>[^"]+)"$"#
            )
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::WorldTrigger {
                    event: c.name("event")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::WorldTrigger { event } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[35m[WORLD]\x1b[0m {}",
                    event
                )
            },
        },

        // ---------------------------------------------------------------------
        // 13: Validated
        // ---------------------------------------------------------------------
        LogPattern {
            id: "PLAYER_VALIDATED",
            regex: Regex::new(&format!(
                r#"^{} STEAM USERID validated$"#,
                p_none
            ))
            .unwrap(),

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
                    name,
                    action
                )
            },
        },

        // ---------------------------------------------------------------------
        // 14: Left Buyzone
        // ---------------------------------------------------------------------
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
                    player.name,
                    items
                )
            },
        },

        // ---------------------------------------------------------------------
        // 15: JSON / Round Stats
        // ---------------------------------------------------------------------
        LogPattern {
            id: "ROUND_STATS",
            regex: Regex::new(
                r#"(?s)^JSON_BEGIN.*JSON_END$"#
            )
            .unwrap(),

            parse_fn: |line, _c| {
                parse_round_stats(line).map(|stats| LogEvent::RoundStats {
                    roundstats: stats,
                })
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
        },

        // ---------------------------------------------------------------------
        // 16: Assist
        // ---------------------------------------------------------------------
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

            pretty_fn: |event| {
                let LogEvent::Assist {
                    assister,
                    victim,
                } = event
                else {
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
        },

        // ---------------------------------------------------------------------
        // 17: Grenade
        // ---------------------------------------------------------------------
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

            pretty_fn: |event| {
                let LogEvent::Grenade {
                    player,
                    grenade,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "{}{}\x1b[0m threw {}",
                    player.team.color_code(),
                    player.name,
                    grenade
                )
            },
        },

        // ---------------------------------------------------------------------
        // 18: SV Grenade
        // ---------------------------------------------------------------------
        LogPattern {
            id: "SERVER_GRENADE_THROW",
            regex: Regex::new(&format!(
                r#"^{} sv_throw_(?P<grenade>[a-z]+) (?P<x>-?\d+(?:\.\d+)?) (?P<y>-?\d+(?:\.\d+)?) (?P<z>-?\d+(?:\.\d+)?) .*$"#,
                p_none
            ))
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::SvGrenade {
                    player: parse_player(c, "")?,
                    grenade: c.name("grenade")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::SvGrenade {
                    player,
                    grenade,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "\x1b[90m{}{}\x1b[0m sv_throw_{}",
                    player.team.color_code(),
                    player.name,
                    grenade
                )
            },
        },

        // ---------------------------------------------------------------------
        // 19: Blinded
        // ---------------------------------------------------------------------
        LogPattern {
            id: "PLAYER_BLINDED",
            regex: Regex::new(&format!(
                r#"^{} blinded for (?P<duration>\d+(?:\.\d+)?) by {} from flashbang entindex \d+$"#,
                p_at, p_vic
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
        },

        // ---------------------------------------------------------------------
        // 20: Match Status - Team Playing
        // ---------------------------------------------------------------------
        LogPattern {
            id: "MATCH_TEAM_PLAYING",
            regex: Regex::new(
                r#"^(?:MatchStatus: )?Team playing "(?P<team>CT|TERRORIST)": (?P<team_name>.+)$"#
            )
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::MatchStatus {
                    team: Team::from_str(c.name("team")?.as_str()),
                    team_name: Some(c.name("team_name")?.as_str().to_string()),
                })
            },

            pretty_fn: |event| {
                let LogEvent::MatchStatus {
                    team,
                    team_name,
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
                };

                format!(
                    "{}{} \x1b[0mplaying {}",
                    team.color_code(),
                    team_label,
                    team_name.as_deref().unwrap_or("UNKNOWN")
                )
            },
        },

        // ---------------------------------------------------------------------
        // 21: Match Status - Team Unset
        // ---------------------------------------------------------------------
        LogPattern {
            id: "MATCH_TEAM_UNSET",
            regex: Regex::new(
                r#"^MatchStatus: Team "(?P<team>TERRORIST|CT)" is unset\.$"#
            )
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::MatchStatus {
                    team: Team::from_str(c.name("team")?.as_str()),
                    team_name: None,
                })
            },

            pretty_fn: |event| {
                let LogEvent::MatchStatus {
                    team,
                    ..
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
                };

                format!(
                    "{}{} \x1b[0munset",
                    team.color_code(),
                    team_label
                )
            },
        },

        // ---------------------------------------------------------------------
        // 22: Freezetime
        // ---------------------------------------------------------------------
        LogPattern {
            id: "ROUND_FREEZE",
            regex: Regex::new(
                r#"^Starting Freeze period$"#
            )
            .unwrap(),

            parse_fn: |_line, _c| {
                Some(LogEvent::FreezePeriod)
            },

            pretty_fn: |_event| {
                "\x1b[90m[FREEZE]\x1b[0m Starting Freeze period".to_string()
            },
        },

        // ---------------------------------------------------------------------
        // 23: Match Start
        // ---------------------------------------------------------------------
        LogPattern {
            id: "MATCH_START",
            regex: Regex::new(
                r#"^World triggered "Match_Start" on "(?P<map>[^"]+)"$"#
            )
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::MatchStart {
                    map: c.name("map")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::MatchStart { map } = event else {
                    return String::new();
                };

                format!(
                    "\x1b[1;32m[MATCH]\x1b[0m started on {}",
                    map
                )
            },
        },

        // ---------------------------------------------------------------------
        // 24: Team Score
        // ---------------------------------------------------------------------
        LogPattern {
            id: "ROUND_TEAM_SCORE",
            regex: Regex::new(
                r#"^Team "(?P<team>CT|TERRORIST)" scored "(?P<score>\d+)" with "(?P<players>\d+)" players$"#
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
                };

                format!(
                    "{}{} \x1b[0mscored {} with {} players",
                    team.color_code(),
                    team_label,
                    score,
                    players
                )
            },
        },

        // ---------------------------------------------------------------------
        // 25: Molotov Projectile Spawn
        // ---------------------------------------------------------------------
        LogPattern {
            id: "SERVER_MOLOTOV_SPAWN",
            regex: Regex::new(
                r#"^Molotov projectile spawned at .*?, velocity .*$"#
            )
            .unwrap(),

            parse_fn: |_line, _c| {
                Some(LogEvent::Technical {
                    name: "Molotov".to_string(),
                    action: "Projectile Spawned".to_string(),
                })
            },

            pretty_fn: |_event| {
                "\x1b[90mMolotov projectile spawned\x1b[0m".to_string()
            },
        },

        // ---------------------------------------------------------------------
        // 26: Game Over
        // ---------------------------------------------------------------------
        LogPattern {
            id: "GAME_OVER",
            regex: Regex::new(
                r#"^Game Over: (?P<mode>\S+)\s+(?P<map>\S+) score (?P<t_score>\d+):(?P<ct_score>\d+) after (?P<minutes>\d+) min$"#
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
                    mode,
                    map,
                    t_score,
                    ct_score,
                    minutes
                )
            },
        },

        // ---------------------------------------------------------------------
        // 27: Server CVar
        // ---------------------------------------------------------------------
        LogPattern {
            id: "SERVER_CVAR",
            regex: Regex::new(
                r#"^server_cvar: "(?P<name>[^"]+)" "(?P<value>[^"]*)"$"#
            )
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

                format!(
                    "\x1b[90mserver_cvar\x1b[0m {} = {}",
                    name,
                    value
                )
            },
        },

        // ---------------------------------------------------------------------
        // 28: Bomb Death
        // ---------------------------------------------------------------------
        LogPattern {
            id: "PLAYER_BOMB_DEATH",
            regex: Regex::new(&format!(
                r#"^{} {} was killed by the bomb\.$"#,
                p_none, pos_none
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
        },

        // ---------------------------------------------------------------------
        // 29: Round Accolade
        // ---------------------------------------------------------------------
        LogPattern {
            id: "ROUND_ACCOLADE",
            regex: Regex::new(
                r#"^ACCOLADE, FINAL: \{(?P<category>[^}]+)\},\s*(?P<player>[^,]+),\s*VALUE:\s*(?P<value>-?\d+(?:\.\d+)?),\s*POS:\s*(?P<position>\d+),\s*SCORE:\s*(?P<score>-?\d+(?:\.\d+)?)$"#
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
                    player.name,
                    category,
                    value,
                    position
                )
            },
        },

        // ---------------------------------------------------------------------
        // 30: Console Say
        // ---------------------------------------------------------------------
        LogPattern {
            id: "CHAT_CONSOLE",
            regex: Regex::new(
                r#"^"Console<(?P<userid>\d+)>" say "(?P<msg>.*)"$"#
            )
            .unwrap(),

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
        },

        // ---------------------------------------------------------------------
        // 31: Match Pause Enabled
        // ---------------------------------------------------------------------
        LogPattern {
            id: "MATCH_PAUSE_ENABLED",
            regex: Regex::new(
                r#"^Match pause is enabled - mp_pause_match$"#
            )
            .unwrap(),

            parse_fn: |_line, _c| {
                Some(LogEvent::Technical {
                    name: "Match".to_string(),
                    action: "Pause Enabled".to_string(),
                })
            },

            pretty_fn: |_event| {
                "Match pause enabled".to_string()
            },
        },

        // ---------------------------------------------------------------------
        // 32: Match Pause Disabled
        // ---------------------------------------------------------------------
        LogPattern {
            id: "MATCH_PAUSE_DISABLED",
            regex: Regex::new(
                r#"^Match pause is disabled - mp_unpause_match$"#
            )
            .unwrap(),

            parse_fn: |_line, _c| {
                Some(LogEvent::Technical {
                    name: "Match".to_string(),
                    action: "Pause Disabled".to_string(),
                })
            },

            pretty_fn: |_event| {
                "Match pause disabled".to_string()
            },
        },

        // ---------------------------------------------------------------------
        // 33: CVar Dump
        // ---------------------------------------------------------------------
        LogPattern {
            id: "SERVER_CVAR_DUMP",
            regex: Regex::new(
                r#"(?s)^.*server cvars start\n.*server cvars end$"#
            )
            .unwrap(),

            parse_fn: |line, _c| {
                let dump = ServerCvars::parse(line)?;

                Some(LogEvent::ServerCvarDump {
                    cvars: dump.values,
                })
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
        },

        // ---------------------------------------------------------------------
        // 34: Log File
        // ---------------------------------------------------------------------
        LogPattern {
            id: "LOG_FILE",
            regex: Regex::new(
                r#"^(?:Log file closed|Log file started \(file ".*"\) \(game ".*"\) \(version ".*"\))$"#
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
        },

        // ---------------------------------------------------------------------
        // 35: Map Loading
        // ---------------------------------------------------------------------
        LogPattern {
            id: "MAP_LOADING",
            regex: Regex::new(
                r#"^Loading map "(?P<map>[^"]+)"$"#
            )
            .unwrap(),

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
        },

        // ---------------------------------------------------------------------
        // 36: Server Started
        // ---------------------------------------------------------------------
        LogPattern {
            id: "SERVER_STARTED",
            regex: Regex::new(
                r#"^Started:\s*".*"$"#
            )
            .unwrap(),

            parse_fn: |_line, _c| {
                Some(LogEvent::ServerStarted)
            },

            pretty_fn: |_event| {
                String::from("[SERVER] started")
            },
        },

        // ---------------------------------------------------------------------
        // 37: Rcon
        // ---------------------------------------------------------------------
        LogPattern {
            id: "RCON",
            regex: Regex::new(
                r#"^rcon from "(?P<addr>[^"]+)": command "(?P<command>.*)"$"#
            )
            .unwrap(),

            parse_fn: |_line, c| {
                Some(LogEvent::Rcon {
                    addr: c.name("addr")?.as_str().to_string(),
                    command: c.name("command")?.as_str().to_string(),
                })
            },

            pretty_fn: |event| {
                let LogEvent::Rcon {
                    addr,
                    command,
                } = event
                else {
                    return String::new();
                };

                format!(
                    "\x1b[38;5;244mrcon from\x1b[0m \"{}\"\x1b[38;5;244m: command\x1b[0m \"{}\"",
                    addr,
                    command
                )
            },
        },
    ]
}
