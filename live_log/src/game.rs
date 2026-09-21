#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game {
    Cs2,
    Css,
    Cs16,
    DoDS,
    TF2,
    CoD4,
    UT2k4,
    GenericGoldSrc,
    GenericSource,
    GenericSource2,
    GenericQuake3,
    GenericGameSpy,
}
use std::fmt;

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Game::Cs2 => "CS2",
            Game::Css => "CSS",
            Game::Cs16 => "CS 1.6",
            Game::DoDS => "Day of Defeat: Source",
            Game::TF2 => "Team Fortress 2",
            Game::CoD4 => "Call of Duty 4",
            Game::UT2k4 => "Unreal Tournament 2004",
            Game::GenericGoldSrc => "Generic GoldSrc",
            Game::GenericSource => "Generic Source",
            Game::GenericSource2 => "Generic Source 2",
            Game::GenericQuake3 => "Generic Quake 3",
            Game::GenericGameSpy => "Generic GameSpy",
        };

        f.write_str(name)
    }
}
