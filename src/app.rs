use crate::app_log;

use crate::app_log::app_log_store;
use crate::custom_components::applicatiton_logs::ApplicationLogs;
use crate::custom_components::matchzy::matchzy::start_matchzy_server;
use crate::custom_components::server::Favourites;
use crate::custom_components::server::LAN;
use crate::custom_components::ui::RconTab;
use crate::custom_components::Navbar;
use crate::misc::load_from_disk;
use crate::rcon_manager::RconManager;
use crate::state::AppState;
use crate::state::GameServer;
use dioxus::prelude::*;
use lan_scan::PendingQuery;
use lan_scan::ScanCommand;
use lan_scan::ServerUpdate;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

use lan_scan::Scanner;
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

    #[route("/logs")]
ApplicationLogs {},
}

#[derive(Clone)]
pub struct ShutdownSignal(pub Arc<Notify>);

#[component]
pub fn App() -> Element {
    let mut state = use_context_provider(|| AppState {
        servers: Signal::new(load_from_disk()),
        rcon_manager: RconManager::new(),
        selected_rcon: Signal::new(None),
        query_tx: Signal::new(None),
    });

    // ------------------------------------------------------------
    // RCON CLEANUP ON SHUTDOWN
    // ------------------------------------------------------------

    let shutdown = use_context::<ShutdownSignal>();
    use_future(move || {
        let shutdown = shutdown.clone();

        async move {
            tracing::debug!("[SHUTDOWN] Shutdown worker started");
            shutdown.0.notified().await;

            tracing::debug!("[SHUTDOWN] Shutdown requested");

            state.rcon_manager.close_all().await;

            tracing::debug!("[SHUTDOWN] RCON cleanup complete");
        }
    });
    let shutdown = use_context::<ShutdownSignal>();
    use_drop(move || {
        tracing::debug!("[UI] App scope is dropping. Signaling shutdown...");
        shutdown.0.notify_one();
    });

    // ------------------------------------------------------------
    // MATCHZY HTTP SERVER
    // ------------------------------------------------------------

    use_future(|| async {
        let addr: SocketAddr = "0.0.0.0:7131"
            .parse()
            .expect("Invalid MatchZy HTTP address");

        start_matchzy_server(addr).await;
    });

    // ------------------------------------------------------------
    // AUTO-CONNECT ON STARTUP
    // ------------------------------------------------------------

    use_future(move || async move {
        let targets = state.servers.with(|servers| {
            servers
                .iter()
                .filter_map(|(addr, server)| {
                    if !server.rcon_autologin {
                        return None;
                    }

                    Some((
                        *addr,
                        server.rcon_password.clone()?,
                        server.scanned.protocol,
                    ))
                })
                .collect::<Vec<_>>()
        });

        state.rcon_manager.connect_multiple_servers(targets).await;
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
            tracing::error!("[SCANNER] Failed to bind UDP socket for scanner");
            return;
        }

        while let Some(update) = ui_rx.recv().await {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

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
    let mut app_log_entries = use_signal(|| app_log_store().entries());

    use_context_provider(|| app_log_entries);

    use_future(move || async move {
        loop {
            app_log_entries.set(app_log_store().entries());

            tokio::time::sleep(Duration::from_millis(100)).await;
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
