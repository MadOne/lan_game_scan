use live_log::round_stats::RoundStats;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeamScore {
    pub ct: u8,
    pub t: u8,
    pub round: i32,
}

impl Default for TeamScore {
    fn default() -> Self {
        Self {
            ct: 0,
            t: 0,
            round: 1,
        }
    }
}

impl TeamScore {
    // -------------------------------------------------------------------------
    // SCORE UPDATE
    // -------------------------------------------------------------------------
    //
    // ScoreUpdate is authoritative for the current round number.
    //
    // The actual team score comes from RoundStats.
    // -------------------------------------------------------------------------

    pub fn update_with_score_update(&mut self, rounds: i32) {
        self.round = rounds;
    }

    pub fn update_with_roundstats(&mut self, roundstats: &RoundStats) {
        self.ct = roundstats.score_ct.min(u8::MAX as u16) as u8;
        self.t = roundstats.score_t.min(u8::MAX as u16) as u8;
    }
}
