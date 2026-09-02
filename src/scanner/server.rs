// server.rs

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] // Added Serialize/Deserialize
pub struct ScannedServer {
    pub socket_addr: SocketAddr,
    pub hostname: Option<String>,
    pub game: Option<String>,
    pub map: Option<String>,
    pub players: Option<u8>,
    pub players_max: Option<u8>,
    pub query_port: Option<u16>,
    #[serde(skip)] // Don't save live ping to disk
    pub ping: Option<u16>,
    #[serde(skip)] // Don't save live timestamp to disk
    pub bots: Option<u8>,
    pub has_password: bool,
    pub password: Option<String>,
}
