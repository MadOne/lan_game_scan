// server.rs

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::SocketAddr};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] // Added Serialize/Deserialize
pub struct ScannedServer {
    pub socket_addr: SocketAddr,
    pub hostname: Option<String>,
    pub game: Game,
    pub map: Option<String>,
    pub players: Option<u8>,
    pub players_max: Option<u8>,
    pub players_list: Vec<PlayerInfo>,
    pub query_port: Option<u16>,
    #[serde(skip)] // Don't save live ping to disk
    pub ping: Option<u16>,
    #[serde(skip)] // Don't save live timestamp to disk
    pub bots: Option<u8>,
    pub has_password: bool,
    pub password: Option<String>,
    pub protocol: ServerProtocol,
}

/// Defines the type of query currently pending for a server endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingQuery {
    Info,
    Player,
    Rules,
}

/// Commands sent to the Scanner to initiate queries
#[derive(Debug)]
pub enum ScanCommand {
    /// Scan a single target endpoint
    ScanServer {
        addr: SocketAddr,
        query_type: PendingQuery,
    },
    /// Batch scan multiple target endpoints
    BatchScan {
        addrs: Vec<SocketAddr>,
        query_type: PendingQuery,
    },
    /// Cancel any active scans
    Cancel,
}

/// Internal signals for retrying queries after receiving challenge tokens
#[derive(Debug)]
pub enum RetrySignal {
    Info(SocketAddr),
    Player(SocketAddr),
}

/// Player details returned by server queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInfo {
    pub name: String,
    pub score: i32,
    pub ping: Option<u16>,
    pub duration_secs: Option<f32>,
    pub index: Option<u8>,
    pub team: Option<u8>,
    pub skin: Option<String>,
    pub is_bot: bool,
}

/// Dispatched back to the UI or coordinator layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerUpdate {
    FullServer(ScannedServer),
    PlayerList {
        addr: SocketAddr,
        players: Vec<PlayerInfo>,
    },
    Failed {
        addr: SocketAddr,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerProtocol {
    GoldSrc,
    Source,
    Source2,
    Quake3,
    GameSpy,
    Unknown,
}

#[derive(Debug)]
pub enum ParseResult {
    /// Server payload parsed successfully.
    ///
    /// The query type identifies which pending request produced this response,
    /// allowing the scanner to clear only that request.
    Update {
        query_type: PendingQuery,
        update: ServerUpdate,
    },

    /// Challenge token received from server (4 bytes).
    Challenge([u8; 4]),

    /// Waiting for remaining split fragments to complete reassembly.
    PartialSplit,

    /// Unrecognized packet format or corrupted data.
    Ignored,
}

/// Buffer for reassembling multi-packet UDP responses
#[derive(Default, Debug)]
pub struct SplitBuffer {
    pub total: u8,
    pub packets: BTreeMap<u8, Vec<u8>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Game {
    Cs2,
    Css,
    Cs16,
    DoDS,
    TF2,
    HLDM2,
    CoD4,
    UT2k4,
    GenericSource(String),
    GenericSource2(String),
    GenericGoldSrc(String),
    GenericQuake3(String),
    GenericGameSpy(String),
    UNKNOWN,
}
use std::fmt;

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Game::Cs2 => write!(f, "CS2"),
            Game::Css => write!(f, "CS:S"),
            Game::Cs16 => write!(f, "Cs1.6"),
            Game::DoDS => write!(f, "DoD:S"),
            Game::TF2 => write!(f, "TF2"),
            Game::HLDM2 => write!(f, "HLDM2"),
            Game::CoD4 => write!(f, "CoD4"),
            Game::UT2k4 => write!(f, "UT2k4"),

            Game::GenericSource(name) => write!(f, "{name}"),
            Game::GenericSource2(name) => write!(f, "{name}"),
            Game::GenericGoldSrc(name) => write!(f, "{name}"),
            Game::GenericQuake3(name) => write!(f, "{name}"),
            Game::GenericGameSpy(name) => write!(f, "{name}"),

            Game::UNKNOWN => write!(f, "Unknown"),
        }
    }
}
