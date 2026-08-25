use crate::custom_components::code::RconSession;
use crate::custom_components::matchzy::matchzy::start_matchzy_server;
use crate::custom_components::server::Favourites;
use crate::custom_components::server::LAN;
use crate::custom_components::ui::RconTab;
use crate::custom_components::Navbar;
use crate::misc::load_from_disk;
use crate::scanner::create_scaner;
use crate::state::AppState;
use dioxus::prelude::*;
use std::net::SocketAddr;
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use std::sync::Arc;
use tokio::sync::Notify;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
// app.rs

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Navbar)]
    #[route("/")]
    LAN {},

    #[route("/favourites")]
    Favourites {},

    #[route("/rcon")]
    RconTab {},
}

#[derive(Clone)]
pub struct ShutdownSignal(pub Arc<Notify>);

#[component]
pub fn App() -> Element {
    let mut state = use_context_provider(|| AppState {
        servers: Signal::new(load_from_disk()),
        rcon_sessions: Signal::new(HashMap::new()),
        selected_rcon: Signal::new(None),
        query_tx: Signal::new(None),
    });
    let shutdown = use_context::<ShutdownSignal>();

    use_future(move || {
        let shutdown = shutdown.clone();

        async move {
            shutdown.0.notified().await;

            println!("[SHUTDOWN] Shutdown requested");

            let sessions = state
                .rcon_sessions
                .with_mut(|sessions| std::mem::take(sessions));

            println!("[SHUTDOWN] Closing {} RCON session(s)", sessions.len());

            for (addr, mut session) in sessions {
                println!("[SHUTDOWN] Closing RCON session {}", addr);

                let success = session.close().await;

                println!("[SHUTDOWN] RCON session {} closed: {}", addr, success);
            }

            println!("[SHUTDOWN] RCON cleanup complete");
        }
    });
    let connect_rcon = Callback::new(move |(addr, password): (SocketAddr, String)| {
        let mut state = state;

        spawn(async move {
            if let Some(session) = RconSession::connect(addr, password).await {
                state.rcon_sessions.with_mut(|sessions| {
                    sessions.insert(addr, session);
                });

                state.selected_rcon.set(Some(addr));
            }
        });
    });

    use_future(|| async {
        let addr: SocketAddr = "0.0.0.0:7131"
            .parse()
            .expect("Invalid MatchZy HTTP address");

        start_matchzy_server(addr).await;
    });

    use_context_provider(|| connect_rcon);
    // ------------------------------------------------------------
    // 1. DISCOVERY SCANNER
    // ------------------------------------------------------------

    use_future(move || async move {
        let (rx, query_sender) = create_scaner().await;

        state.query_tx.set(Some(query_sender));

        let mut rx = rx.lock().await;

        while let Some(mut incoming) = rx.recv().await {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            incoming.last_update = Some(now);

            state.servers.with_mut(|map| {
                if let Some(existing) = map.get_mut(&incoming.socket_addr) {
                    existing.hostname = incoming.hostname;
                    existing.game = incoming.game;
                    existing.players = incoming.players;
                    existing.players_max = incoming.players_max;
                    existing.map = incoming.map;
                    existing.last_update = Some(now);
                    existing.ping = incoming.ping;
                    existing.bots = incoming.bots;
                    existing.has_password = incoming.has_password
                } else {
                    incoming.is_favorite = false;
                    map.insert(incoming.socket_addr, incoming);
                }
            });
        }
    });

    // ------------------------------------------------------------
    // 2. SERVER LIFECYCLE
    // ------------------------------------------------------------

    use_future(move || async move {
        loop {
            let qry = match state.query_tx.cloned() {
                Some(q) => q,

                None => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            tokio::time::sleep(Duration::from_secs(6)).await;

            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            let timeout = 15;
            let mut to_ping = vec![];

            state.servers.with_mut(|map| {
                map.retain(|_addr, srv| {
                    let elapsed = now - srv.last_update.unwrap_or(0);

                    elapsed < timeout || srv.is_favorite
                });

                for srv in map.values_mut() {
                    if srv.is_favorite {
                        let elapsed = now - srv.last_update.unwrap_or(0);

                        if elapsed >= timeout {
                            srv.ping = None;
                        }

                        if elapsed >= 6 {
                            to_ping.push(srv.socket_addr);
                        }
                    }
                }
            });

            for addr in to_ping {
                let _ = qry.send(addr).await;
            }
        }
    });

    rsx! {
        document::Link {
            rel: "icon",
            href: FAVICON
        }

        document::Link {
            rel: "stylesheet",
            href: MAIN_CSS
        }

        document::Link {
            rel: "stylesheet",
            href: TAILWIND_CSS
        }

        div {
            class: "h-screen w-screen bg-zinc-950 text-zinc-300 font-sans overflow-hidden",

            Router::<Route> {}
        }
    }
}
