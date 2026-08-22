use live_log::{
    parser::{Player as LiveLogPlayer, Team as LiveLogTeam},
    round_stats::RoundStats,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub name: String,
    pub team: Team,
    pub id: u16,
    pub steamid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    CT,
    Terrorist,
    Spectator,
    Unassigned,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct RconPlayers {
    players: HashMap<u16, Player>,
}

impl RconPlayers {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }

    // =========================================================================
    // TEAM SWITCH
    // =========================================================================

    pub fn update_with_team_switch(&mut self, player: &LiveLogPlayer) {
        let team = match player.team {
            LiveLogTeam::CT => Team::CT,
            LiveLogTeam::Terrorist => Team::Terrorist,
            LiveLogTeam::Spectator => Team::Spectator,
            LiveLogTeam::Unassigned => Team::Unassigned,
            LiveLogTeam::Unknown => Team::Unknown,
        };

        match self.players.get_mut(&player.id) {
            Some(existing) => {
                existing.name = player.name.clone();
                existing.team = team;
                existing.id = player.id;
                existing.steamid = player.steamid.clone();
            }

            None => {
                self.players.insert(
                    player.id,
                    Player {
                        name: player.name.clone(),
                        team,
                        id: player.id,
                        steamid: player.steamid.clone(),
                    },
                );
            }
        }
    }

    // =========================================================================
    // ROUND STATS
    // =========================================================================
    //
    // RoundStats is authoritative for:
    //   - current player roster
    //   - player ID
    //   - team
    //
    // It does not contain the player name, so existing names are preserved.
    // =========================================================================

    pub fn update_with_roundstats(&mut self, roundstats: &RoundStats) {
        // RoundStats is authoritative for the current roster.
        self.players
            .retain(|player_id, _| roundstats.players.contains_key(player_id));

        for (&player_id, stats_player) in &roundstats.players {
            let team = match stats_player.team {
                2 => Team::Terrorist,
                3 => Team::CT,

                other => {
                    eprintln!(
                        "[RCON_PLAYERS] Unknown team {} for player {}",
                        other, player_id
                    );

                    continue;
                }
            };

            match self.players.get_mut(&player_id) {
                Some(player) => {
                    // RoundStats does not contain the name.
                    // Keep the existing name.
                    player.team = team;
                }

                None => {
                    self.players.insert(
                        player_id,
                        Player {
                            name: String::new(),
                            team,
                            id: 999,
                            steamid: "ABDC".to_string(),
                        },
                    );
                }
            }
        }
    }

    // =========================================================================
    // ACCESS
    // =========================================================================

    pub fn players(&self) -> &HashMap<u16, Player> {
        &self.players
    }
}
