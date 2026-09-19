use crate::_parser::{
    patterns::{LogPattern, LogPatternBlocks},
    sourceengines::{
        cs16::{chat_server, player_team_switch16, rcon16, round_triggered16},
        cs2::*,
        GOLDSRC_TS_BLOCK, STEAMID2_BLOCK,
    },
};

pub const CS16_TS_BLOCK: &str = GOLDSRC_TS_BLOCK;
pub fn cs16_build_patterns() -> Vec<LogPattern> {
    let blocks = LogPatternBlocks::new(CS16_TS_BLOCK, STEAMID2_BLOCK);
    vec![
        player_damaged(&blocks),
        player_killed(&blocks),
        chat(&blocks),
        match_score(),
        round_win(),
        player_team_switch16(&blocks),
        bomb_event(&blocks),
        player_purchase(&blocks),
        player_disconnected(&blocks),
        player_connected(&blocks),
        player_entered_game(&blocks),
        player_suicide(&blocks),
        world_trigger(),
        player_validated(&blocks),
        player_left_buyzone(&blocks),
        round_stats(),
        player_assist(&blocks),
        player_grenade_throw(&blocks),
        server_grenade_throw(&blocks),
        player_blinded(&blocks),
        match_team_playing(),
        match_team_unset(),
        round_freeze(),
        match_start(),
        round_team_score(),
        server_molotov_spawn(),
        game_over(),
        server_cvar(),
        player_bomb_death(&blocks),
        round_accolade(),
        chat_server(),
        match_pause_enabled(),
        match_pause_disabled(),
        server_cvar_dump(),
        log_file(),
        map_loading(),
        server_started(),
        rcon16(),
        round_triggered16(),
    ]
}
