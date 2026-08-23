// -----------------------------------------------------------------------------
// tests.rs
// -----------------------------------------------------------------------------

use std::net::SocketAddr;

use live_log::parser::{LogEvent, LogParser, LogType, Team};
use regex::Regex;

fn test_socketaddr() -> SocketAddr {
    "127.0.0.1:27015".parse().unwrap()
}

// -----------------------------------------------------------------------------
// STEAMID
// -----------------------------------------------------------------------------

mod steamid {
    use live_log::log_patterns::STEAMID_BLOCK;

    use super::*;

    #[test]
    fn test_standard() {
        let re = Regex::new(STEAMID_BLOCK).unwrap();
        assert!(re.is_match("[U:1:55530433]"));
    }

    #[test]
    fn test_short_id() {
        let re = Regex::new(STEAMID_BLOCK).unwrap();
        assert!(re.is_match("[G:1:123]"));
    }

    #[test]
    fn test_long_id() {
        let re = Regex::new(STEAMID_BLOCK).unwrap();
        assert!(re.is_match("[A:1:999999999]"));
    }

    #[test]
    fn test_bot() {
        let re = Regex::new(STEAMID_BLOCK).unwrap();
        assert!(re.is_match("BOT"));
    }

    #[test]
    fn test_pending() {
        let re = Regex::new(STEAMID_BLOCK).unwrap();
        assert!(re.is_match("STEAM_ID_PENDING"));
    }

    #[test]
    fn test_invalid_universe() {
        let re = Regex::new(STEAMID_BLOCK).unwrap();
        assert!(!re.is_match("[U:2:12345]"));
    }

    #[test]
    fn test_lowercase() {
        let re = Regex::new(STEAMID_BLOCK).unwrap();
        assert!(!re.is_match("[u:1:12345]"));
    }
}

// -----------------------------------------------------------------------------
// TEAM
// -----------------------------------------------------------------------------

mod team {
    use super::*;

    #[test]
    fn test_team_parsing() {
        assert_eq!(Team::from_str("CT"), Team::CT);
        assert_eq!(Team::from_str("ct"), Team::CT);
        assert_eq!(Team::from_str("COUNTER-TERRORISTS"), Team::CT);

        assert_eq!(Team::from_str("T"), Team::Terrorist);
        assert_eq!(Team::from_str("Terrorist"), Team::Terrorist);
        assert_eq!(Team::from_str("Terrorists"), Team::Terrorist);

        assert_eq!(Team::from_str("Spectator"), Team::Spectator);

        assert_eq!(Team::from_str(""), Team::Unassigned);
        assert_eq!(Team::from_str("<>"), Team::Unassigned);
        assert_eq!(Team::from_str("Unassigned"), Team::Unassigned);

        assert_eq!(Team::from_str("something_else"), Team::Unknown);
    }
}

// -----------------------------------------------------------------------------
// TIMESTAMP / PARSER PREFIX
// -----------------------------------------------------------------------------

mod timestamp {
    use live_log::log_patterns::TS_BLOCK;

    use super::*;

    #[test]
    fn test_block_logic() {
        let re = Regex::new(TS_BLOCK).unwrap();

        let caps = re
            .captures("08/11/2026 - 13:34:15.425")
            .expect("Regex failed standard timestamp");

        assert_eq!(&caps["ts"], "08/11/2026 - 13:34:15.425");
    }

    #[test]
    fn test_with_log_prefix() {
        let parser = LogParser::new();

        let raw_line =
            "[LOG] 08/11/2026 - 13:34:17.302 - \"Mad_One<0><[U:1:55530433]><CT>\" entered the game";

        let parsed = parser.parse(raw_line, test_socketaddr());

        assert_eq!(parsed.timestamp, "08/11/2026 - 13:34:17.302");
    }

    #[test]
    fn test_malformed_fallback() {
        let parser = LogParser::new();

        let raw_line = "2026-08-11 - 13:34:17.302 - some content";

        let parsed = parser.parse(raw_line, test_socketaddr());

        assert_eq!(parsed.timestamp, "");
    }

    #[test]
    fn test_socketaddr_is_preserved() {
        let parser = LogParser::new();

        let socketaddr: SocketAddr = "192.168.1.42:27015".parse().unwrap();

        let line =
            "08/11/2026 - 13:34:17.302 - \"Mad_One<0><[U:1:55530433]><CT>\" entered the game";

        let parsed = parser.parse(line, socketaddr);

        assert_eq!(parsed.socketaddr, socketaddr);
    }
}

// -----------------------------------------------------------------------------
// PLAYER
// -----------------------------------------------------------------------------

mod player {
    use live_log::log_patterns::get_player_re;

    use super::*;

    #[test]
    fn test_block_regex_logic() {
        let p_re_str = get_player_re("");
        let re = Regex::new(&p_re_str).unwrap();

        // Standard with Team
        let caps = re
            .captures("\"Mad_One<0><[U:1:55530433]><CT>\"")
            .expect("Player block failed standard");

        assert_eq!(caps.name("name").map(|m| m.as_str()), Some("Mad_One"));
        assert_eq!(caps.name("id").map(|m| m.as_str()), Some("0"));
        assert_eq!(
            caps.name("steamid").map(|m| m.as_str()),
            Some("[U:1:55530433]")
        );
        assert_eq!(caps.name("team").map(|m| m.as_str()), Some("CT"));

        // Empty team <>
        let caps = re
            .captures("\"Mad_One<0><[U:1:55530433]><>\"")
            .expect("Player block failed empty brackets");

        assert_eq!(caps.name("name").map(|m| m.as_str()), Some("Mad_One"));
        assert_eq!(caps.name("id").map(|m| m.as_str()), Some("0"));
        assert_eq!(
            caps.name("steamid").map(|m| m.as_str()),
            Some("[U:1:55530433]")
        );
        assert_eq!(caps.name("team").map(|m| m.as_str()), Some(""));

        // Missing team block entirely
        let caps = re
            .captures("\"Mad_One<0><[U:1:55530433]>\"")
            .expect("Player block failed missing brackets");

        assert_eq!(caps.name("name").map(|m| m.as_str()), Some("Mad_One"));
        assert_eq!(caps.name("id").map(|m| m.as_str()), Some("0"));
        assert_eq!(
            caps.name("steamid").map(|m| m.as_str()),
            Some("[U:1:55530433]")
        );
        assert!(caps.name("team").is_none());
    }

    #[test]
    fn test_bot_player_block() {
        let p_re_str = get_player_re("");
        let re = Regex::new(&p_re_str).unwrap();

        let caps = re
            .captures("\"Francis<13><BOT><TERRORIST>\"")
            .expect("BOT player block failed");

        assert_eq!(caps.name("name").map(|m| m.as_str()), Some("Francis"));
        assert_eq!(caps.name("id").map(|m| m.as_str()), Some("13"));
        assert_eq!(caps.name("steamid").map(|m| m.as_str()), Some("BOT"));
        assert_eq!(caps.name("team").map(|m| m.as_str()), Some("TERRORIST"));
    }

    #[test]
    fn test_block_team_greediness() {
        let p_re_str = get_player_re("");

        let re = Regex::new(&format!(r"^{} entered the game$", p_re_str))
            .expect("Failed greediness regex");

        let line = "\"Mad_One<0><[U:1:55530433]><CT>\" entered the game";

        let caps = re
            .captures(line)
            .expect("Greediness test failed to match line");

        assert_eq!(caps.name("name").map(|m| m.as_str()), Some("Mad_One"));
        assert_eq!(caps.name("id").map(|m| m.as_str()), Some("0"));
        assert_eq!(
            caps.name("steamid").map(|m| m.as_str()),
            Some("[U:1:55530433]")
        );
        assert_eq!(caps.name("team").map(|m| m.as_str()), Some("CT"));
    }

    #[test]
    fn test_no_team_integration() {
        let parser = LogParser::new();

        let line = "08/11/2026 - 13:34:17.302 - \"Mad_One<0><[U:1:55530433]><>\" entered the game";

        let res = parser.parse(line, test_socketaddr());

        if let LogEvent::Connection { player, .. } = res.event {
            assert_eq!(player.id, 0);
            assert_eq!(player.name, "Mad_One");
            assert_eq!(player.steamid, "[U:1:55530433]");
            assert_eq!(player.team, Team::Unassigned);
        } else {
            panic!("Failed to parse player with empty team brackets.");
        }
    }

    #[test]
    fn test_with_team_integration() {
        let parser = LogParser::new();

        let line =
            "08/11/2026 - 13:34:17.302 - \"Mad_One<0><[U:1:55530433]><CT>\" entered the game";

        let res = parser.parse(line, test_socketaddr());

        if let LogEvent::Connection { player, .. } = res.event {
            assert_eq!(player.id, 0);
            assert_eq!(player.name, "Mad_One");
            assert_eq!(player.steamid, "[U:1:55530433]");
            assert_eq!(player.team, Team::CT);
        } else {
            panic!("Failed to parse player with CT team");
        }
    }

    #[test]
    fn test_missing_team_brackets_integration() {
        let parser = LogParser::new();

        let line =
            "08/11/2026 - 13:34:17.302 - \"Mad_One<0><[U:1:55530433]>\" switched from team <CT> to <Unassigned>";

        let res = parser.parse(line, test_socketaddr());

        if let LogEvent::TeamSwitch { player, from } = res.event {
            assert_eq!(player.id, 0);
            assert_eq!(player.name, "Mad_One");
            assert_eq!(player.steamid, "[U:1:55530433]");
            assert_eq!(player.team, Team::Unassigned);
            assert_eq!(from, Team::CT);
        } else {
            panic!("Failed to parse player switch event");
        }
    }
}

// -----------------------------------------------------------------------------
// OTHER / PATTERN BUILDING BLOCKS
// -----------------------------------------------------------------------------

mod other {
    use live_log::log_patterns::{get_pos_re, STATS_BLOCK};

    use super::*;

    #[test]
    fn test_pos_block_regex_logic() {
        let pos_re_str = get_pos_re("test_");
        let re = Regex::new(&pos_re_str).unwrap();

        let caps = re.captures("[-1503 510 0]").expect("POS block failed");

        assert_eq!(&caps["test_pos"], "-1503 510 0");
    }

    #[test]
    fn test_stats_block_regex_logic() {
        let re = Regex::new(STATS_BLOCK).unwrap();

        let input =
            r#"(damage "1") (damage_armor "0") (health "99") (armor "100") (hitgroup "generic")"#;

        let caps = re.captures(input).expect("Stats block failed");

        assert_eq!(&caps["dmg"], "1");
    }

    #[test]
    fn test_full_parse_integration() {
        let parser = LogParser::new();

        let line =
            "08/11/2026 - 13:34:17.302 - \"Mad_One<0><[U:1:55530433]><CT>\" entered the game";

        let res = parser.parse(line, test_socketaddr());

        assert_eq!(res.timestamp, "08/11/2026 - 13:34:17.302");
        assert_eq!(res.log_type, LogType::Connection);

        if let LogEvent::Connection { player, .. } = res.event {
            assert_eq!(player.id, 0);
            assert_eq!(player.name, "Mad_One");
            assert_eq!(player.steamid, "[U:1:55530433]");
            assert_eq!(player.team, Team::CT);
        } else {
            panic!("Failed full parse integration. Event was: {:?}", res.event);
        }
    }

    #[test]
    fn test_log_prefix_purchase() {
        let parser = LogParser::new();

        let line =
            r#"[LOG] 08/11/2026 - 17:22:40.211 - "Mad_One<0><[U:1:55530433]><CT>" purchased "awp""#;

        let parsed = parser.parse(line, test_socketaddr());

        assert_eq!(parsed.log_type, LogType::Purchase);

        match parsed.event {
            LogEvent::Purchase { player, item } => {
                assert_eq!(player.id, 0);
                assert_eq!(player.name, "Mad_One");
                assert_eq!(player.steamid, "[U:1:55530433]");
                assert_eq!(player.team, Team::CT);
                assert_eq!(item, "awp");
            }

            event => panic!("Expected Purchase, got {:?}", event),
        }
    }

    #[test]
    fn test_purchase_integration() {
        let parser = LogParser::new();

        let line =
            r#"08/11/2026 - 17:22:40.211 - "Mad_One<0><[U:1:55530433]><CT>" purchased "awp""#;

        let res = parser.parse(line, test_socketaddr());

        assert_eq!(res.log_type, LogType::Purchase);

        match res.event {
            LogEvent::Purchase { player, item } => {
                assert_eq!(player.id, 0);
                assert_eq!(player.name, "Mad_One");
                assert_eq!(player.steamid, "[U:1:55530433]");
                assert_eq!(player.team, Team::CT);
                assert_eq!(item, "awp");
            }

            other => panic!("Expected Purchase, got {:?}", other),
        }
    }

    #[test]
    fn test_kill_headshot_and_penetration() {
        let parser = LogParser::new();

        let line = r#"08/11/2026 - 13:34:17.302 - "Mad_One<0><[U:1:55530433]><CT>" [-100 200 10] killed "Enemy<1><[U:1:123456]><T>" [50 300 10] with "ak47" (headshot) (penetrated)"#;

        let parsed = parser.parse(line, test_socketaddr());

        assert_eq!(parsed.log_type, LogType::Kill);

        match parsed.event {
            LogEvent::Kill {
                attacker,
                victim,
                headshot,
                penetrated,
                weapon,
                ..
            } => {
                assert_eq!(attacker.id, 0);
                assert_eq!(attacker.name, "Mad_One");
                assert_eq!(attacker.steamid, "[U:1:55530433]");
                assert_eq!(attacker.team, Team::CT);

                assert_eq!(victim.id, 1);
                assert_eq!(victim.name, "Enemy");
                assert_eq!(victim.steamid, "[U:1:123456]");
                assert_eq!(victim.team, Team::Terrorist);

                assert!(headshot);
                assert!(penetrated);
                assert_eq!(weapon, "ak47");
            }

            event => panic!("Expected Kill, got {:?}", event),
        }
    }
}

// -----------------------------------------------------------------------------
// UNKNOWN / FALLBACK
// -----------------------------------------------------------------------------

mod fallback {
    use super::*;

    #[test]
    fn test_unknown_line() {
        let parser = LogParser::new();

        let line = "08/11/2026 - 13:34:17.302 - This is definitely not a known CS2 event";

        let parsed = parser.parse(line, test_socketaddr());

        assert_eq!(parsed.log_type, LogType::Unknown);
        assert!(matches!(parsed.event, LogEvent::Unknown));
        assert!(parsed.pretty.is_empty());
    }
}

// -----------------------------------------------------------------------------
// ROUND STATS
// -----------------------------------------------------------------------------

mod round_stats {
    use live_log::round_stats::parse_round_stats;

    // -------------------------------------------------------------------------
    // Valid round stats
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_real_cs2_round_stats() {
        let json = r#"JSON_BEGIN
{
"name": "round_stats",
"round_number" : "3",
"score_t" : "0",
"score_ct" : "2",
"map" : "de_anubis",
"server" : "LinuxGSM",
"fields" : "             accountid,   team,  money,  kills, deaths,assists,    dmg,    hsp,    kdr,    adr,    mvp,     ef,     ud,     3k,     4k,     5k,clutchk, firstk,pistolk,sniperk, blindk,  bombk,firedmg,uniquek,  dinks,chickenk"
"players" : {
"player_0" : "            55530433,      2,   5800,      0,      2,      0,   0.00,   0.00,   0.00,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,0.000000,0.000000,      0,      0,      0"
"player_1" : "                   0,      3,   3850,      1,      1,      1, 151.00,   0.00,   1.00,     76,      0,      0,      0,      0,      0,      0,      1,      1,      0,      0,0.000000,0.000000,      1,      0,      0"
"player_2" : "                   0,      3,   5400,      3,      0,      0, 166.00,   0.00,   0.00,     83,      1,      0,      0,      0,      0,      0,      0,      0,      1,      0,      0,0.000000,0.000000,      1,      0,      0"
"player_3" : "                   0,      2,   2800,      0,      2,      1, 140.00,   0.00,   0.00,     70,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,0.000000,0.000000,      0,      0,      0"
"player_4" : "                   0,      3,   3250,      1,      0,      0, 100.00,   0.00,   0.00,     50,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,0.000000,0.000000,      0,      0,      0"
"player_5" : "                   0,      2,   4200,      1,      2,      0,  43.00,   0.00,   0.50,     22,      0,      0,      0,      0,      0,      0,      0,      0,      1,      0,      0,0.000000,0.000000,      0,      0,      0"
"player_6" : "                   0,      3,   6300,      4,      1,      1, 354.00,  25.00,   4.00,    177,      1,      0,      0,      1,      0,      0,      1,      4,      0,      0,0.000000,0.000000,      1,      0,      0"
"player_7" : "                   0,      2,   2500,      0,      2,      0,  39.00,   0.00,   0.00,     20,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,0.000000,0.000000,      0,      0,      0"
"player_8" : "                   0,      2,   3900,      1,      2,      1, 232.00,   0.00,   0.50,    116,      0,      0,      0,      0,      0,      0,      0,      0,      1,      0,      0,0.000000,0.000000,      0,      0,      0"
"player_9" : "                   0,      3,   5450,      1,      0,      2, 229.00,   0.00,   0.00,    114,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,      0,0.000000,0.000000,      0,      0,      0"
}}
JSON_END"#;

        let stats = parse_round_stats(json).expect("Failed to parse round stats");

        assert_eq!(stats.round_number, 3);
        assert_eq!(stats.score_t, 0);
        assert_eq!(stats.score_ct, 2);
        assert_eq!(stats.map, "de_anubis");
        assert_eq!(stats.server, "LinuxGSM");

        assert_eq!(stats.players.len(), 10);

        let player = stats.players.get(&0).expect("Missing player_0");

        assert_eq!(player.account_id, 55530433);
        assert_eq!(player.team, 2);
        assert_eq!(player.money, 5800);
        assert_eq!(player.kills, 0);
        assert_eq!(player.deaths, 2);
        assert_eq!(player.damage, 0);

        let player = stats.players.get(&6).expect("Missing player_6");

        assert_eq!(player.team, 3);
        assert_eq!(player.money, 6300);
        assert_eq!(player.kills, 4);
        assert_eq!(player.deaths, 1);
        assert_eq!(player.assists, 1);
        assert_eq!(player.damage, 354);
        assert_eq!(player.headshot_percent, 25);
        assert_eq!(player.kd_ratio, 4.0);
        assert_eq!(player.adr, 177);
        assert_eq!(player.mvp, 1);
        assert_eq!(player.kills_3k, 1);
        assert_eq!(player.first_kills, 1);
        assert_eq!(player.pistol_kills, 4);
        assert_eq!(player.unique_kills, 1);
    }

    #[test]
    fn test_parse_round_stats_with_non_contiguous_player_ids() {
        let json = r#"JSON_BEGIN
{
"name": "round_stats",
"round_number" : "1",
"score_t" : "0",
"score_ct" : "0",
"map" : "de_anubis",
"server" : "LinuxGSM",
"fields" : "accountid, team, money, kills, deaths, assists, dmg, hsp, kdr, adr, mvp, ef, ud, 3k, 4k, 5k, clutchk, firstk, pistolk, sniperk, blindk, bombk, firedmg, uniquek, dinks, chickenk"
"players" : {
"player_0" : "55530433, 2, 1000, 0, 0, 0, 0.00, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0, 0"
"player_1" : "0, 3, 1000, 0, 0, 0, 0.00, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0, 0"
"player_2" : "0, 2, 1000, 0, 0, 0, 0.00, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0, 0"
"player_3" : "0, 3, 1000, 0, 0, 0, 0.00, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0"
"player_10" : "0, 3, 1000, 0, 0, 0, 0.00, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0"
"player_11" : "0, 2, 1000, 0, 0, 0, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0"
"player_12" : "0, 3, 1000, 0, 0, 0, 0.00, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0"
"player_13" : "0, 2, 1000, 0, 0, 0, 0.00, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0"
"player_14" : "0, 3, 1000, 0, 0, 0, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0"
"player_15" : "0, 2, 1000, 0, 0, 0, 0.00, 0.00, 0.00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,0.000000,0.000000, 0, 0"
}}
JSON_END"#;

        let stats = parse_round_stats(json).expect("Failed to parse round stats");

        assert_eq!(stats.players.len(), 10);

        assert!(stats.players.contains_key(&0));
        assert!(stats.players.contains_key(&3));
        assert!(stats.players.contains_key(&10));
        assert!(stats.players.contains_key(&15));

        assert!(!stats.players.contains_key(&4));
        assert!(!stats.players.contains_key(&9));
    }

    // -------------------------------------------------------------------------
    // Invalid round stats
    // -------------------------------------------------------------------------

    #[test]
    fn test_invalid_json_returns_none() {
        let json = r#"JSON_BEGIN
{
    "name": "round_stats",
    "round_number": "1",
    "score_t": "0",
    "score_ct": "0",
    "map": "de_anubis",
    "server": "LinuxGSM",
    "fields": "accountid, team, money",
    "players": {
        "player_0": "55530433, 2, 1000"
    }
JSON_END"#;

        assert!(parse_round_stats(json).is_none());
    }

    #[test]
    fn test_missing_players_returns_none() {
        let json = r#"JSON_BEGIN
{
    "name": "round_stats",
    "round_number": "1",
    "score_t": "0",
    "score_ct": "0",
    "map": "de_anubis",
    "server": "LinuxGSM",
    "fields": "accountid, team, money"
}
JSON_END"#;

        assert!(parse_round_stats(json).is_none());
    }

    #[test]
    fn test_invalid_player_data_returns_none() {
        let json = r#"JSON_BEGIN
{
    "name": "round_stats",
    "round_number": "1",
    "score_t": "0",
    "score_ct": "0",
    "map": "de_anubis",
    "server": "LinuxGSM",
    "fields": "accountid, team, money, kills, deaths, assists, dmg, hsp, kdr, adr, mvp, ef, ud, 3k, 4k, 5k, clutchk, firstk, pistolk, sniperk, blindk, bombk, firedmg, uniquek, dinks, chickenk",
    "players": {
        "player_0": "55530433, 2, 1000"
    }
}
JSON_END"#;

        assert!(parse_round_stats(json).is_none());
    }
}

// -----------------------------------------------------------------------------
// REPRODUCTION TEST MODULE
// -----------------------------------------------------------------------------

#[cfg(test)]
mod repro_failure {
    use super::*;

    #[test]
    fn test_unmatched_purchase_line() {
        let parser = LogParser::new();

        let raw_line =
            "08/11/2026 - 17:00:15.180 - \"Mad_One<0><[U:1:55530433]><CT>\" purchased \"awp\"";

        let parsed = parser.parse(raw_line, test_socketaddr());

        assert_eq!(parsed.log_type, LogType::Purchase);

        match parsed.event {
            LogEvent::Purchase { player, item } => {
                assert_eq!(player.id, 0);
                assert_eq!(player.name, "Mad_One");
                assert_eq!(player.steamid, "[U:1:55530433]");
                assert_eq!(player.team, Team::CT);
                assert_eq!(item, "awp");
            }

            event => {
                panic!("Line was not parsed as Purchase. Parsed event: {:?}", event);
            }
        }
    }

    #[test]
    fn test_assist_integration() {
        let parser = LogParser::new();

        let line = r#"08/14/2026 - 16:20:07.596 - "Francis<13><BOT><TERRORIST>" assisted killing "Gustov<3><BOT><CT>""#;

        let parsed = parser.parse(line, test_socketaddr());

        assert_eq!(parsed.log_type, LogType::Assist);

        match parsed.event {
            LogEvent::Assist { assister, victim } => {
                assert_eq!(assister.id, 13);
                assert_eq!(assister.name, "Francis");
                assert_eq!(assister.steamid, "BOT");
                assert_eq!(assister.team, Team::Terrorist);

                assert_eq!(victim.id, 3);
                assert_eq!(victim.name, "Gustov");
                assert_eq!(victim.steamid, "BOT");
                assert_eq!(victim.team, Team::CT);
            }

            event => panic!("Expected Assist, got {:?}", event),
        }
    }

    #[test]
    fn test_molotov_projectile_spawn() {
        let parser = LogParser::new();

        let line = r#"08/14/2026 - 19:57:51.633 - Molotov projectile spawned at -844.486694 -747.298828 186.690460, velocity 549.273193 -377.051575 108.425064"#;

        let parsed = parser.parse(line, "127.0.0.1:27015".parse().unwrap());

        assert_eq!(parsed.log_type, LogType::Technical);

        match parsed.event {
            LogEvent::Technical { name, action } => {
                assert_eq!(name, "Molotov");
                assert_eq!(action, "Projectile Spawned");
            }

            other => panic!("Expected Technical event, got {:?}", other),
        }
    }

    #[test]
    fn test_bomb_death() {
        let parser = LogParser::new();

        let line = r#"08/14/2026 - 20:01:11.508 - "Mad_One<0><[U:1:55530433]><TERRORIST>" [866 2092 -32] was killed by the bomb."#;

        let parsed = parser.parse(line, "127.0.0.1:12345".parse().unwrap());

        assert_eq!(parsed.log_type, LogType::BombDeath);

        match parsed.event {
            LogEvent::BombDeath { player } => {
                assert_eq!(player.id, 0);
                assert_eq!(player.name, "Mad_One");
                assert_eq!(player.steamid, "[U:1:55530433]");
                assert_eq!(player.team, Team::Terrorist);
            }

            other => panic!("Expected BombDeath event, got {:?}", other),
        }
    }

    #[test]
    fn test_round_accolade() {
        let parser = LogParser::new();

        let line = r#"08/14/2026 - 20:08:44.838 - ACCOLADE, FINAL: {hsp},     Mad_One<0>,     VALUE: 87.500000,       POS: 1, SCORE: 60.277779"#;

        let parsed = parser.parse(line, "127.0.0.1:12345".parse().unwrap());

        assert_eq!(parsed.log_type, LogType::RoundAccolade);

        match parsed.event {
            LogEvent::RoundAccolade {
                category,
                player,
                value,
                position,
                score,
            } => {
                assert_eq!(category, "hsp");
                assert_eq!(player.name, "Mad_One");
                assert_eq!(value, 87.5);
                assert_eq!(position, 1);
                assert!((score - 60.277779).abs() < f32::EPSILON);
            }

            other => panic!("Expected RoundAccolade event, got {:?}", other),
        }
    }

    #[test]
    fn test_round_win_cts_win() {
        let parser = LogParser::new();

        let line = r#"08/14/2026 - 21:58:36.601 - Team "CT" triggered "SFUI_Notice_CTs_Win" (CT "3") (T "3")"#;

        let parsed = parser.parse(line, "127.0.0.1:12345".parse().unwrap());

        assert_eq!(parsed.log_type, LogType::RoundWin);

        match parsed.event {
            LogEvent::RoundWin {
                team,
                winner_side,
                reason,
                ct_score,
                t_score,
            } => {
                assert_eq!(team, "CT");
                assert_eq!(winner_side, Team::CT);
                assert_eq!(reason, "CTs_Win");
                assert_eq!(ct_score, 3);
                assert_eq!(t_score, 3);
            }

            other => panic!("Expected RoundWin, got {:?}", other),
        }
    }

    #[test]
    fn test_match_pause_patterns() {
        let parser = LogParser::new();

        let enabled = r#"08/15/2026 - 09:06:45.303 - Match pause is enabled - mp_pause_match"#;

        let disabled = r#"08/15/2026 - 09:06:47.115 - Match pause is disabled - mp_unpause_match"#;

        let enabled_parsed = parser.parse(enabled, "127.0.0.1:12345".parse().unwrap());

        let disabled_parsed = parser.parse(disabled, "127.0.0.1:12345".parse().unwrap());

        // Both patterns currently produce Technical events.
        assert_eq!(enabled_parsed.log_type, LogType::Technical);
        assert_eq!(disabled_parsed.log_type, LogType::Technical);

        match enabled_parsed.event {
            LogEvent::Technical { name, action } => {
                assert_eq!(name, "Match");
                assert_eq!(action, "Pause Enabled");
            }

            event => panic!("Expected Technical event, got {:?}", event),
        }

        match disabled_parsed.event {
            LogEvent::Technical { name, action } => {
                assert_eq!(name, "Match");
                assert_eq!(action, "Pause Disabled");
            }

            event => panic!("Expected Technical event, got {:?}", event),
        }
    }
}
