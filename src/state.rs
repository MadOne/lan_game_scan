use std::collections::HashMap;
use std::net::SocketAddr;

use dioxus::prelude::*;
use tokio::sync::mpsc::Sender;

use crate::custom_components::code::RconSession;
use crate::scanner::GameServer;

#[derive(Clone, Copy)]
pub struct AppState {
    pub servers: Signal<HashMap<SocketAddr, GameServer>>,
    pub rcon_sessions: Signal<HashMap<SocketAddr, RconSession>>,
    pub selected_rcon: Signal<Option<SocketAddr>>,
    pub query_tx: Signal<Option<Sender<SocketAddr>>>,
}
