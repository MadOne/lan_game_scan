// -----------------------------------------------------------------------------
// round_stats.rs
// -----------------------------------------------------------------------------
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct RoundStatsRaw {
    pub name: String,
    pub round_number: String,
    pub score_t: String,
    pub score_ct: String,
    pub map: String,
    pub server: String,
    pub players: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoundStats {
    pub round_number: u16,
    pub score_t: u16,
    pub score_ct: u16,
    pub map: String,
    pub server: String,
    pub players: HashMap<u16, RSPlayer>,
}

pub fn parse_round_stats(json: &str) -> Option<RoundStats> {
    let ts_re =
        Regex::new(r#"(?m)^(?:\[LOG\]\s+)?\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}\.\d{3} - "#)
            .ok()?;

    let cleaned = ts_re
        .replace_all(json, "")
        .replace("JSON_BEGIN", "")
        .replace("JSON_END", "")
        .trim()
        .to_string();

    let mut lines: Vec<String> = cleaned.lines().map(str::to_string).collect();

    // -----------------------------------------------------------------
    // Fix CS2's invalid JSON.
    //
    // "fields" needs a comma.
    // Player entries need commas except the last one.
    // -----------------------------------------------------------------

    if let Some(line) = lines.get_mut(7) {
        let trimmed = line.trim_end();

        if !trimmed.ends_with(',') {
            *line = format!("{},", trimmed);
        }
    }

    // -----------------------------------------------------------------
    // Find players object
    // -----------------------------------------------------------------

    let players_start = lines
        .iter()
        .position(|line| line.trim() == "\"players\" : {")
        .or_else(|| {
            lines
                .iter()
                .position(|line| line.trim() == "\"players\": {")
        })?;

    let players_end = lines
        .iter()
        .enumerate()
        .skip(players_start + 1)
        .find(|(_, line)| line.trim() == "}}")
        .map(|(index, _)| index)?;

    // -----------------------------------------------------------------
    // Add commas between player entries.
    // -----------------------------------------------------------------

    if players_end > players_start + 2 {
        for i in (players_start + 1)..(players_end - 1) {
            let trimmed = lines[i].trim_end();

            if !trimmed.ends_with(',') {
                lines[i] = format!("{},", trimmed);
            }
        }
    }

    let cleaned = lines.join("\n");

    let stats: RoundStatsRaw = match serde_json::from_str(&cleaned) {
        Ok(stats) => stats,
        Err(e) => {
            eprintln!("RoundStats JSON parse error: {}", e);
            return None;
        }
    };

    let mut players = HashMap::new();

    for (key, data) in stats.players {
        let player_number = key.strip_prefix("player_")?.parse::<u16>().ok()?;

        match parse_player(&data) {
            Some(player) => {
                players.insert(player_number, player);
            }
            None => {
                eprintln!("FAILED TO PARSE PLAYER {}:\n{}", player_number, data);
            }
        }
    }

    Some(RoundStats {
        round_number: stats.round_number.parse().unwrap_or(999),
        score_t: stats.score_t.parse().unwrap_or(999),
        score_ct: stats.score_ct.parse().unwrap_or(999),
        map: stats.map,
        server: stats.server,
        players,
    })
}

// -----------------------------------------------------------------------------
// PLAYER
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RSPlayer {
    pub account_id: u64,
    pub team: u32,
    pub money: u32,

    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,

    pub damage: u32,
    pub headshot_percent: u32,
    pub kd_ratio: f32,
    pub adr: u32,

    pub mvp: u32,
    pub enemies_flashed: u32,
    pub utility_damage: u32,

    pub kills_3k: u32,
    pub kills_4k: u32,
    pub kills_5k: u32,

    pub clutch_kills: u32,
    pub first_kills: u32,
    pub pistol_kills: u32,
    pub sniper_kills: u32,
    pub blind_kills: u32,

    pub bomb_kills: u32,
    pub fire_damage: u32,
    pub unique_kills: u32,
    pub dink_kills: u32,
    pub chicken_kills: u32,
}

// -----------------------------------------------------------------------------
// PLAYER PARSER
// -----------------------------------------------------------------------------

pub fn parse_player(data: &str) -> Option<RSPlayer> {
    let values: Vec<&str> = data.split(',').map(str::trim).collect();

    if values.len() != 26 {
        eprintln!(
            "Invalid player data: expected 26 fields, got {}: {:?}",
            values.len(),
            data
        );
        return None;
    }

    Some(RSPlayer {
        account_id: values[0].parse().ok()?,
        team: values[1].parse().ok()?,
        money: values[2].parse().ok()?,
        kills: values[3].parse().ok()?,
        deaths: values[4].parse().ok()?,
        assists: values[5].parse().ok()?,
        // CS2 prints these as floating point values.
        // We only use f32 as an intermediate representation.
        damage: values[6].parse::<f32>().ok()? as u32,
        headshot_percent: values[7].parse::<f32>().ok()? as u32,
        kd_ratio: values[8].parse().ok()?,
        adr: values[9].parse().ok()?,
        mvp: values[10].parse().ok()?,
        enemies_flashed: values[11].parse().ok()?,
        utility_damage: values[12].parse().ok()?,
        kills_3k: values[13].parse().ok()?,
        kills_4k: values[14].parse().ok()?,
        kills_5k: values[15].parse().ok()?,
        clutch_kills: values[16].parse().ok()?,
        first_kills: values[17].parse().ok()?,
        pistol_kills: values[18].parse().ok()?,
        sniper_kills: values[19].parse().ok()?,
        blind_kills: values[20].parse().ok()?,
        // These are printed by CS2 as 0.000000 even though they are
        // semantically integer counters/values in our Player struct.
        bomb_kills: values[21].parse::<f32>().ok()? as u32,
        fire_damage: values[22].parse::<f32>().ok()? as u32,
        unique_kills: values[23].parse().ok()?,
        dink_kills: values[24].parse().ok()?,
        chicken_kills: values[25].parse().ok()?,
    })
}
