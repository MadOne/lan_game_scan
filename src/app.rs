use crate::custom_components::code::RconSession;
use crate::custom_components::matchzy::matchzy::start_matchzy_server;
use crate::custom_components::server::Favourites;
use crate::custom_components::server::LAN;
use crate::custom_components::ui::RconTab;
use crate::custom_components::Navbar;
use crate::misc::load_from_disk;
use crate::scanner::scanner::ScanCommand;
use crate::scanner::{PendingQuery, Scanner, ServerUpdate};
use crate::state::AppState;
use crate::state::GameServer;
use dioxus::prelude::*;
use std::net::SocketAddr;
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

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

    // cleanup on Linux
    let shutdown = use_context::<ShutdownSignal>();
    use_future(move || {
        let shutdown = shutdown.clone();

        async move {
            println!("[SHUTDOWN] Shutdown worker started");
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

    // cleanup on Android
    use_drop(move || {
        println!("[UI] App scope is dropping. Starting RCON cleanup...");

        let sessions = state.rcon_sessions.with_mut(|sessions| {
            sessions
                .drain()
                .filter_map(|(addr, session)| {
                    let log_url = session.log_url.clone()?;

                    Some((addr, session.client.clone(), log_url))
                })
                .collect::<Vec<_>>()
        });

        tokio::spawn(async move {
            println!("[SHUTDOWN] Closing {} RCON connection(s)", sessions.len());

            for (addr, client, log_url) in sessions {
                let command = format!("logaddress_del_http \"{}\"", log_url);

                println!(
                    "[SHUTDOWN] Sending cleanup command to {}: {}",
                    addr, command
                );

                let mut client = client.lock().await;

                match client.command_no_response(&command).await {
                    Ok(()) => {
                        println!("[SHUTDOWN] RCON cleanup for {} successful", addr);
                    }

                    Err(error) => {
                        println!("[SHUTDOWN] RCON cleanup for {} failed: {}", addr, error);
                    }
                }
            }

            println!("[SHUTDOWN] RCON cleanup complete");
        });
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
    // AUTO-CONNECT ON STARTUP
    // ------------------------------------------------------------
    use_future(move || {
        let connect_fn = connect_rcon;

        async move {
            let autoconnect_targets: Vec<(SocketAddr, String)> = state.servers.with(|map| {
                map.iter()
                    .filter(|(_, srv)| srv.rcon_autologin && srv.rcon_password.is_some())
                    .map(|(addr, srv)| (*addr, srv.rcon_password.clone().unwrap()))
                    .collect()
            });

            if !autoconnect_targets.is_empty() {
                println!(
                    "[AUTO-CONNECT] Found {} server(s) for autologin",
                    autoconnect_targets.len()
                );

                for (addr, password) in autoconnect_targets {
                    println!("[AUTO-CONNECT] Triggering autologin for {}", addr);
                    connect_fn.call((addr, password));
                }
            }
        }
    });

    // ------------------------------------------------------------
    // 1. DISCOVERY SCANNER
    // ------------------------------------------------------------

    use_future(move || async move {
        let (cmd_tx, cmd_rx) = mpsc::channel(100);
        let (ui_tx, mut ui_rx) = mpsc::channel(100);

        state.query_tx.set(Some(cmd_tx));

        if let Ok(scanner) = Scanner::new("0.0.0.0:0", cmd_rx, ui_tx).await {
            tokio::spawn(async move {
                scanner.run().await;
            });
        } else {
            eprintln!("[SCANNER] Failed to bind UDP socket for scanner");
            return;
        }

        while let Some(update) = ui_rx.recv().await {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            //println!("ui received update: {:?}", update);
            state.servers.with_mut(|map| match update {
                ServerUpdate::FullServer(mut incoming) => {
                    let addr = incoming.socket_addr;

                    if let Some(existing) = map.get_mut(&addr) {
                        if incoming.players_list.is_empty()
                            && !existing.scanned.players_list.is_empty()
                        {
                            incoming.players_list = existing.scanned.players_list.clone();
                        }
                        existing.scanned = incoming;
                        existing.last_update = Some(now);
                    } else {
                        map.insert(
                            addr,
                            GameServer {
                                scanned: incoming,
                                rcon_password: None,
                                rcon_autologin: false,
                                is_favorite: false,
                                last_update: Some(now),
                            },
                        );
                    }
                }

                ServerUpdate::PlayerList { addr, players } => {
                    if let Some(existing) = map.get_mut(&addr) {
                        existing.scanned.players = Some(players.len() as u8);
                        existing.scanned.players_list = players;
                        existing.last_update = Some(now);
                    }
                }

                ServerUpdate::Failed { addr } => {
                    if let Some(existing) = map.get_mut(&addr) {
                        existing.scanned.ping = None;
                    }
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
                            srv.scanned.ping = None;
                        }

                        if elapsed >= 6 {
                            to_ping.push(srv.scanned.socket_addr);
                        }
                    }
                }
            });

            for addr in to_ping {
                let _ = qry
                    .send(ScanCommand::ScanServer {
                        addr,
                        query_type: PendingQuery::Info,
                    })
                    .await;
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
