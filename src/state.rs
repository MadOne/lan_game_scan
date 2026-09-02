use std::collections::HashMap;
use std::net::SocketAddr;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

use crate::custom_components::code::RconSession;
//use crate::scanner::GameServer;
use crate::scanner::ScannedServer;

#[derive(Clone, Copy)]
pub struct AppState {
    pub servers: Signal<HashMap<SocketAddr, GameServer>>,
    pub rcon_sessions: Signal<HashMap<SocketAddr, RconSession>>,
    pub selected_rcon: Signal<Option<SocketAddr>>,
    pub query_tx: Signal<Option<Sender<SocketAddr>>>,
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
