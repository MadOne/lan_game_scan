use crate::custom_components::code::RconSession;
use dioxus::prelude::*;
use lan_scan::ServerProtocol;
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Clone, Copy)]
pub struct RconManager {
    pub sessions: Signal<HashMap<SocketAddr, RconSession>>,
}

impl RconManager {
    pub fn new() -> Self {
        Self {
            sessions: Signal::new(HashMap::new()),
        }
    }

    pub async fn connect(
        &mut self,
        addr: SocketAddr,
        password: String,
        protocol: ServerProtocol,
    ) -> bool {
        if let Some(session) = RconSession::connect(addr, password, protocol).await {
            self.insert(addr, session);
            true
        } else {
            false
        }
    }
    fn insert(&mut self, addr: SocketAddr, session: RconSession) {
        self.sessions.with_mut(|sessions| {
            sessions.insert(addr, session);
        });
    }

    pub fn with_session<R>(
        &self,
        addr: &SocketAddr,
        f: impl FnOnce(&RconSession) -> R,
    ) -> Option<R> {
        self.sessions.read().get(addr).map(f)
    }

    pub fn len(&self) -> usize {
        self.sessions.with(|sessions| sessions.len())
    }

    pub fn addresses(&self) -> Vec<SocketAddr> {
        self.sessions.read().keys().copied().collect()
    }

    pub fn attention_count(&self) -> usize {
        self.sessions
            .read()
            .values()
            .filter(|session| (session.need_attention)())
            .count()
    }

    pub async fn close_all(&mut self) {
        let sessions = self.sessions.with_mut(|sessions| std::mem::take(sessions));

        tracing::debug!("[RCON] Closing {} session(s)", sessions.len());

        for (addr, mut session) in sessions {
            tracing::debug!("[RCON] Closing RCON session {}", addr);

            let success = session.close().await;

            tracing::debug!("[RCON] RCON session {} closed: {}", addr, success);
        }

        tracing::debug!("[RCON] RCON cleanup complete");
    }

    pub async fn connect_multiple_servers(
        &mut self,
        targets: Vec<(SocketAddr, String, ServerProtocol)>,
    ) {
        if targets.is_empty() {
            return;
        }

        tracing::debug!(
            "[AUTO-CONNECT] Found {} server(s) for autologin",
            targets.len()
        );

        for (addr, password, protocol) in targets {
            tracing::debug!("[AUTO-CONNECT] Connecting to {}", addr);

            self.connect(addr, password, protocol).await;
        }
    }

    pub async fn disconnect(&mut self, addr: &SocketAddr) -> bool {
        let Some(mut session) = self.sessions.with_mut(|sessions| sessions.remove(addr)) else {
            return false;
        };

        session.close().await
    }
}
