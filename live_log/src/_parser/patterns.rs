// -----------------------------------------------------------------------------
// log_patterns.rs
// -----------------------------------------------------------------------------

use regex::{Captures, Regex};

use crate::_parser::{
    sourceengines::{PLAYER_BLOCK_BASE, POS_BLOCK_BASE, STATS_BLOCK},
    types::{LogEvent, Player, Team},
};

pub struct LogPatternBlocks {
    pub timestamp: &'static str,
    pub steamid: &'static str,
}
impl LogPatternBlocks {
    pub fn new(timestamp: &'static str, steamid: &'static str) -> Self {
        Self { timestamp, steamid }
    }

    pub fn player(&self, prefix: &str) -> String {
        PLAYER_BLOCK_BASE
            .replace("{0}", prefix)
            .replace("{STEAMID}", self.steamid)
    }

    pub fn position(&self, prefix: &str) -> String {
        POS_BLOCK_BASE.replace("{0}", prefix)
    }

    pub fn stats(&self) -> &'static str {
        STATS_BLOCK
    }
}

// -----------------------------------------------------------------------------
// PLAYER PARSER
// -----------------------------------------------------------------------------

pub fn parse_player(c: &Captures, prefix: &str) -> Option<Player> {
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
    pub(crate) parse_fn: fn(&str, &Captures) -> Option<LogEvent>,
    pub pretty_fn: fn(&LogEvent) -> String,
}

impl LogPattern {
    pub fn parse(&self, line: &str, caps: &Captures) -> Option<LogEvent> {
        (self.parse_fn)(line, caps)
    }

    pub fn pretty(&self, event: &LogEvent) -> String {
        (self.pretty_fn)(event)
    }
}
