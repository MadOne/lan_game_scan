use std::collections::HashMap;
use std::net::SocketAddr;

use dioxus::prelude::*;
use lan_scan::{ScanCommand, ScannedServer, ServerProtocol};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::rcon_manager::RconManager;

#[derive(Clone, Copy)]
pub struct AppState {
    pub servers: Signal<HashMap<SocketAddr, GameServer>>,
    pub rcon_manager: RconManager,
    pub selected_rcon: Signal<Option<SocketAddr>>,
    pub query_tx: Signal<Option<Sender<ScanCommand>>>, // <--- Update type here
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameServer {
    pub scanned: ScannedServer,

    pub rcon_password: Option<String>,
    pub rcon_autologin: bool,
    pub is_favorite: bool,
    #[serde(skip)]
    pub last_update: Option<i64>,
}
