use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use cbz_rcon::{RconClient, RconProtocol, RconStatus};
use dioxus::{core::Task, prelude::*};
use lan_scan::ServerProtocol;
use live_log::{
    http_catcher::LiveLog,
    parser::{LogEvent, ParsedLine, Team},
};

use crate::{
    custom_components::{
        code::{RconPlayers, TeamScore},
        cvar::CvarDatabase,
    },
    network::log_receiver_ip,
};

#[derive(Debug, Clone)]
pub enum RconLogEvent {
    LiveLog(ParsedLine),
    RconResponse(String),
    Info(String),
}

pub struct RconSession {
    pub addr: SocketAddr,

    // -------------------------------------------------------------------------
    // RCON connection
    // -------------------------------------------------------------------------
    pub client: Arc<tokio::sync::Mutex<RconClient>>,

    // -------------------------------------------------------------------------
    // Live log processing for this server
    // -------------------------------------------------------------------------
    pub live_log: Option<LiveLog>,
    pub live_log_url: Option<String>,
    pub matchzy_log_url: Option<String>,

    // -------------------------------------------------------------------------
    // Reactive session state
    // -------------------------------------------------------------------------
    pub logs: Signal<Vec<RconLogEvent>>,
    pub status: Signal<RconStatus>,
    pub players: Signal<RconPlayers>,
    pub match_paused: Signal<bool>,
    pub score: Signal<TeamScore>,
    pub maps: Signal<Vec<String>>,
    pub team_name_ct: Signal<String>,
    pub team_name_t: Signal<String>,
    pub max_rounds: Signal<u8>,
    pub need_attention: Signal<bool>,
    live_log_task: Option<Task>,
    pub cvar_db: Signal<Option<CvarDatabase>>,
    pub command_history: Signal<Vec<String>>,
}

impl RconSession {
    pub fn new(addr: SocketAddr, password: String, protocol: RconProtocol) -> Self {
        let client = Arc::new(tokio::sync::Mutex::new(RconClient::new(
            addr, password, protocol,
        )));

        Self {
            addr,
            client,
            live_log: None,

            logs: Signal::new_in_scope(Vec::new(), ScopeId::APP),
            status: Signal::new_in_scope(RconStatus::Disconnected, ScopeId::APP),
            players: Signal::new_in_scope(RconPlayers::new(), ScopeId::APP),

            match_paused: Signal::new_in_scope(false, ScopeId::APP),

            score: Signal::new_in_scope(
                TeamScore {
                    ct: 0,
                    t: 0,
                    round: 0,
                },
                ScopeId::APP,
            ),

            maps: Signal::new_in_scope(Vec::new(), ScopeId::APP),

            team_name_ct: Signal::new_in_scope("TeamA".to_string(), ScopeId::APP),
            team_name_t: Signal::new_in_scope(String::new(), ScopeId::APP),

            max_rounds: Signal::new_in_scope(0, ScopeId::APP),
            need_attention: Signal::new_in_scope(false, ScopeId::APP),

            live_log_url: None,
            matchzy_log_url: None,
            live_log_task: None,

            cvar_db: Signal::new_in_scope(None, ScopeId::APP),
            command_history: Signal::new_in_scope(Vec::new(), ScopeId::APP),
        }
    }

    // =========================================================================
    // CONNECTION
    // =========================================================================

    async fn connect_rcon(&self) -> bool {
        match self.client.lock().await.connect().await {
            Ok(()) => {
                tracing::debug!("RCON authentication successful for {}", self.addr);
                true
            }

            Err(error) => {
                self.push_log(RconLogEvent::Info(format!(
                    "[RCON] Connection failed: {}",
                    error
                )));

                false
            }
        }
    }

    // =========================================================================
    // LIVE LOG
    // =========================================================================

    async fn start_live_log(&mut self) -> bool {
        let receiver_ip = match log_receiver_ip(self.addr) {
            Some(ip) => ip,
            None => {
                self.push_log(RconLogEvent::Info(format!(
                    "[LIVE_LOG] Could not determine a local IP for server {}.",
                    self.addr
                )));
                return false;
            }
        };

        let live_log = match LiveLog::new().await {
            Ok(log) => log,
            Err(err) => {
                self.push_log(RconLogEvent::Info(format!(
                    "[LIVE_LOG] Failed to bind live log listener: {}",
                    err
                )));
                return false;
            }
        };

        let port = live_log.port();

        self.push_log(RconLogEvent::Info(format!(
            "[LIVE_LOG] Listening on port {}.",
            port
        )));

        let log_url = format!("http://{}:{}", receiver_ip, port);
        self.push_log(RconLogEvent::Info(format!(
            "[LIVE_LOG] Receiver URL: {}",
            log_url
        )));

        if !self
            .send_rcon_command(
                "sv_logfile 1",
                "[LIVE_LOG] Enabled server logging: ",
                "[LIVE_LOG] Failed to enable server logging: ",
            )
            .await
        {
            live_log.stop().await;
            return false;
        }

        if !self
            .send_rcon_command(
                "log on",
                "[LIVE_LOG] Enabled log output: ",
                "[LIVE_LOG] Failed to enable log output: ",
            )
            .await
        {
            live_log.stop().await;
            return false;
        }

        let command = format!("logaddress_add_http \"{}\"", log_url);

        if !self
            .send_rcon_command(
                &command,
                &format!("[LIVE_LOG] Registered {}: ", log_url),
                &format!("[LIVE_LOG] Failed to register {}: ", log_url),
            )
            .await
        {
            live_log.stop().await;
            return false;
        }

        let _ = self
            .send_rcon_command(
                "logaddress_list_http",
                "[LIVE_LOG] HTTP log addresses:\n",
                "[LIVE_LOG] Failed to list HTTP log addresses: ",
            )
            .await;
        self.live_log = Some(live_log);
        self.live_log_url = Some(log_url);
        true
    }

    // =========================================================================
    // LOGS
    // =========================================================================

    fn push_log(&self, event: RconLogEvent) {
        let mut logs = self.logs;
        logs.write().push(event);
    }

    // =========================================================================
    // RCON COMMAND
    // =========================================================================

    async fn send_rcon_command(
        &self,
        command: &str,
        success_prefix: &str,
        error_prefix: &str,
    ) -> bool {
        let mut client = self.client.lock().await;

        match client.command(command).await {
            Ok(response) => {
                self.push_log(RconLogEvent::RconResponse(format!(
                    "{}{}",
                    success_prefix, response
                )));

                true
            }

            Err(error) => {
                self.push_log(RconLogEvent::Info(format!("{}{}", error_prefix, error)));

                false
            }
        }
    }

    // =========================================================================
    // LIVE LOG PROCESSING
    // =========================================================================

    async fn process_live_log(
        mut receiver: tokio::sync::mpsc::Receiver<ParsedLine>,
        client: Arc<tokio::sync::Mutex<RconClient>>,
        mut logs: Signal<Vec<RconLogEvent>>,
        mut players: Signal<RconPlayers>,
        mut match_paused: Signal<bool>,
        mut score: Signal<TeamScore>,
        mut team_name_ct: Signal<String>,
        mut team_name_t: Signal<String>,
        mut max_rounds: Signal<u8>,
        mut need_attention: Signal<bool>,
        mut cvar_db: Signal<Option<CvarDatabase>>,
    ) {
        while let Some(parsed) = receiver.recv().await {
            logs.write().push(RconLogEvent::LiveLog(parsed.clone()));

            match &parsed.event {
                // -----------------------------------------------------------------
                // Player changed team
                //
                // NEW:
                // TeamSwitch {
                //     player: Player,
                //     from: Team,
                // }
                // -----------------------------------------------------------------
                LogEvent::TeamSwitch { player, .. } => {
                    players.write().update_with_team_switch(player);
                }

                // -----------------------------------------------------------------
                // Round stats
                //
                // RSPlayer belongs ONLY to RoundStats.
                // RconPlayers can consume the roundstats structure itself.
                // -----------------------------------------------------------------
                LogEvent::RoundStats { roundstats } => {
                    players.write().update_with_roundstats(roundstats);

                    score.write().update_with_roundstats(roundstats);
                }

                // -----------------------------------------------------------------
                // Score update
                // -----------------------------------------------------------------
                LogEvent::ScoreUpdate { rounds, .. } => {
                    score.write().update_with_score_update(*rounds);
                }

                // -----------------------------------------------------------------
                // Match status
                // -----------------------------------------------------------------
                LogEvent::MatchStatus {
                    team,
                    team_name: Some(team_name),
                } => match team {
                    Team::CT => {
                        team_name_ct.set(team_name.clone());
                    }

                    Team::Terrorist => {
                        team_name_t.set(team_name.clone());
                    }

                    _ => {}
                },

                // -----------------------------------------------------------------
                // Match pause
                // -----------------------------------------------------------------
                LogEvent::Technical { name, action } if name == "Match" => match action.as_str() {
                    "Pause Enabled" => {
                        match_paused.set(true);
                    }

                    "Pause Disabled" => {
                        match_paused.set(false);
                    }

                    _ => {}
                },

                // -----------------------------------------------------------------
                // Maximum rounds
                // -----------------------------------------------------------------
                LogEvent::ServerCvar { name, value } => {
                    if name == "mp_maxrounds" {
                        max_rounds.set(value.parse().unwrap_or(0));
                    }
                    if let Some(db) = cvar_db.write().as_mut() {
                        db.update(&name, &value);
                    }
                }

                // -----------------------------------------------------------------
                // Chat / admin commands
                //
                // Chat now contains:
                //
                // Chat {
                //     player: Player,
                //     msg,
                //     is_team_chat,
                // }
                //
                // is_command() handles the Player internally.
                // -----------------------------------------------------------------
                LogEvent::Chat { .. } => match is_command(&parsed.event) {
                    Some(AdminCommand::Admin) => {
                        need_attention.set(true);
                    }

                    Some(AdminCommand::Clear) => {
                        need_attention.set(false);
                    }

                    Some(AdminCommand::Pause) => {
                        let client = client.clone();

                        spawn(async move {
                            let mut client = client.lock().await;

                            if let Err(error) = client.command("mp_pause_match").await {
                                tracing::error!("Failed to pause match: {}", error);
                            }
                        });
                    }

                    Some(AdminCommand::UnPause) => {
                        let client = client.clone();

                        spawn(async move {
                            let mut client = client.lock().await;

                            if let Err(error) = client.command("mp_unpause_match").await {
                                tracing::error!("Failed to pause match: {}", error);
                            }
                        });
                    }

                    None => {}
                },

                _ => {}
            }
        }
    }

    // =========================================================================
    // CONNECTION / SESSION CREATION
    // =========================================================================

    pub async fn connect(
        addr: SocketAddr,
        password: String,
        protocol: ServerProtocol,
    ) -> Option<Self> {
        let is_cs2 = matches!(protocol, ServerProtocol::Source2);
        let rcon_protocol = Self::rcon_protocol(protocol)?;

        let mut session = Self::new(addr, password, rcon_protocol);

        session.push_log(RconLogEvent::Info(format!(
            "[RCON] Connecting to {}...",
            addr
        )));

        if !session.connect_rcon().await {
            return None;
        }

        session.push_log(RconLogEvent::Info("[RCON] Authenticated.".to_string()));

        if is_cs2 {
            if !session.start_live_log().await {
                session.push_log(RconLogEvent::Info(
                    "[RCON] Failed to configure live log.".to_string(),
                ));
                return None;
            }

            let cvarlist = match session.client.lock().await.command("cvarlist").await {
                Ok(response) => response,
                Err(error) => {
                    session.push_log(RconLogEvent::Info(format!(
                        "[RCON] Failed to get cvarlist: {}",
                        error
                    )));

                    String::new()
                }
            };

            let db = CvarDatabase::new(&cvarlist);
            session.cvar_db.set(Some(db));

            if let Some(live_log) = session.live_log.as_mut() {
                let receiver = match live_log.take_receiver() {
                    Some(receiver) => receiver,
                    None => {
                        tracing::error!("Failed to obtain live log receiver for {}", session.addr);

                        if let Some(live_log) = session.live_log.take() {
                            live_log.stop().await;
                        }

                        return None;
                    }
                };
                let logs = session.logs;
                let players = session.players;
                let match_paused = session.match_paused;
                let score = session.score;
                let team_name_ct = session.team_name_ct;
                let team_name_t = session.team_name_t;
                let max_rounds = session.max_rounds;
                let need_attention = session.need_attention;
                let client = session.client.clone();
                let cvar_db = session.cvar_db;

                let live_log_task = spawn(async move {
                    Self::process_live_log(
                        receiver,
                        client,
                        logs,
                        players,
                        match_paused,
                        score,
                        team_name_ct,
                        team_name_t,
                        max_rounds,
                        need_attention,
                        cvar_db,
                    )
                    .await;
                });
                session.live_log_task = Some(live_log_task);
            }
        } else {
            session.push_log(RconLogEvent::Info(
                "[LIVE_LOG] Skipped live log setup for non-CS2 server.".to_string(),
            ));
        }

        session.push_log(RconLogEvent::Info("[RCON] Session created.".to_string()));
        session.status.set(RconStatus::Authenticated);

        let local_ip = log_receiver_ip(addr).unwrap_or_else(|| Ipv4Addr::new(127, 0, 0, 1));

        let port = 7131;
        let matchzy_log_url = format!("http://{}:{}/MatchZyLogs", local_ip, port);

        // Store the log_url in the session for cleanup later.
        session.matchzy_log_url = Some(matchzy_log_url.clone());

        // Tell MatchZy/CS2 where to send remote logs.
        let log_command = format!("matchzy_remote_log_url \"{}\"", matchzy_log_url);

        let mut client_lock = session.client.lock().await;

        match client_lock.command(&log_command).await {
            Ok(resp) => {
                tracing::debug!(
                    "Successfully registered log address for {}, response: {}",
                    addr,
                    resp
                );
            }

            Err(e) => {
                tracing::error!("Failed to register log address for {}: {}", addr, e);
            }
        }

        // Drop lock before mutating state.
        drop(client_lock);

        Some(session)
    }

    // =========================================================================
    // MAPS
    // =========================================================================

    fn parse_maps(response: &str) -> Vec<String> {
        let mut maps = response
            .lines()
            .map(str::trim)
            .filter(|line| {
                (line.starts_with("de_") || line.starts_with("cs_") || line.starts_with("ar_"))
                    && !line.contains("_vanity")
            })
            .map(String::from)
            .collect::<Vec<_>>();

        maps.sort_by(|a, b| {
            let group = |map: &str| {
                if map.starts_with("de_") {
                    0
                } else if map.starts_with("cs_") {
                    1
                } else {
                    2
                }
            };

            group(a).cmp(&group(b)).then_with(|| a.cmp(b))
        });
        maps.dedup();
        maps
    }

    pub fn get_maps(&self) {
        let client = self.client.clone();
        let mut maps = self.maps;
        let mut logs = self.logs;

        spawn(async move {
            let mut client = client.lock().await;

            match client.command("maps *").await {
                Ok(response) => {
                    let parsed_maps = RconSession::parse_maps(&response);
                    maps.set(parsed_maps);
                }

                Err(error) => {
                    logs.write().push(RconLogEvent::Info(format!(
                        "[RCON] Failed to get maps: {}",
                        error
                    )));
                }
            }
        });
    }

    pub async fn close(&mut self) -> bool {
        if let Some(task) = self.live_log_task.take() {
            task.cancel();
        }

        if let Some(live_log) = self.live_log.take() {
            live_log.stop().await;
        }

        let mut success = true;
        let mut client = self.client.lock().await;

        if let Some(live_log_url) = self.live_log_url.take() {
            let command_live_log = format!("logaddress_del_http \"{}\"", live_log_url);

            let cleanup_live_log = match client.command_no_response(&command_live_log).await {
                Ok(()) => true,
                Err(error) => {
                    tracing::error!("Cleanup live log failed for {}: {}", self.addr, error);
                    false
                }
            };

            success &= cleanup_live_log;
        }

        if self.matchzy_log_url.take().is_some() {
            let cleanup_matchzy = match client
                .command_no_response("matchzy_remote_log_url \"\"")
                .await
            {
                Ok(()) => true,
                Err(error) => {
                    tracing::error!("Cleanup MatchZy failed for {}: {}", self.addr, error);
                    false
                }
            };

            success &= cleanup_matchzy;
        }

        success
    }

    fn rcon_protocol(protocol: ServerProtocol) -> Option<RconProtocol> {
        match protocol {
            ServerProtocol::Source | ServerProtocol::Source2 => Some(RconProtocol::Source),
            ServerProtocol::GoldSrc => Some(RconProtocol::GoldSrc),
            ServerProtocol::Quake3 => Some(RconProtocol::Quake3),
            _ => None,
        }
    }
}

// =============================================================================
// ADMIN COMMANDS
// =============================================================================

fn is_command(event: &LogEvent) -> Option<AdminCommand> {
    match event {
        LogEvent::Chat { player, msg, .. } => {
            let command = msg.trim_start().split_whitespace().next().unwrap_or("");

            match command.to_ascii_lowercase().as_str() {
                "!admin" => Some(AdminCommand::Admin),

                "!pause" => Some(AdminCommand::Pause),

                "!unpause" => Some(AdminCommand::UnPause),

                "!clear" | "!solved" if player.name == "Console" => Some(AdminCommand::Clear),

                _ => None,
            }
        }

        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdminCommand {
    Admin,
    Clear,
    Pause,
    UnPause,
}
