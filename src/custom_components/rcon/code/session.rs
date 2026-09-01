use std::{net::SocketAddr, sync::Arc};

use cbz_rcon::{RconClient, RconStatus};
use dioxus::{core::Task, prelude::*};
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
    pub live_log: LiveLog,
    pub log_url: Option<String>,

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
    pub async fn new(addr: SocketAddr, password: String) -> Self {
        let client = Arc::new(tokio::sync::Mutex::new(RconClient::new(addr, password)));

        let live_log = LiveLog::new().await.expect("Error creating LiveLog");

        Self {
            addr,
            client,
            live_log,

            logs: Signal::new(Vec::new()),
            status: Signal::new(RconStatus::Disconnected),
            players: Signal::new(RconPlayers::new()),

            match_paused: Signal::new(false),

            score: Signal::new(TeamScore {
                ct: 0,
                t: 0,
                round: 0,
            }),

            maps: Signal::new(Vec::new()),

            team_name_ct: Signal::new("TeamA".to_string()),
            team_name_t: Signal::new(String::new()),

            max_rounds: Signal::new(0),
            need_attention: Signal::new(false),
            log_url: None,
            live_log_task: None,
            cvar_db: Signal::new(None),
            command_history: Signal::new(Vec::new()),
        }
    }

    // =========================================================================
    // CONNECTION
    // =========================================================================

    async fn connect_rcon(&self) -> bool {
        match self.client.lock().await.connect().await {
            Ok(()) => {
                println!("RCON authentication successful.");
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
        let port = self.live_log.port();

        self.push_log(RconLogEvent::Info(format!(
            "[LIVE_LOG] Listening on port {}.",
            port
        )));

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
            return false;
        }
        /*
        if !self
            .send_rcon_command(
                "logaddress_delall_http",
                "[LIVE_LOG] Cleared old HTTP log addresses: ",
                "[LIVE_LOG] Failed to clear old HTTP log addresses: ",
            )
            .await
        {
            return false;
        }
        */
        let command = format!("logaddress_add_http \"{}\"", log_url);
        self.log_url = Some(log_url.clone());

        if !self
            .send_rcon_command(
                &command,
                &format!("[LIVE_LOG] Registered {}: ", log_url),
                &format!("[LIVE_LOG] Failed to register {}: ", log_url),
            )
            .await
        {
            return false;
        }

        if !self
            .send_rcon_command(
                "logaddress_list_http",
                "[LIVE_LOG] HTTP log addresses:\n",
                "[LIVE_LOG] Failed to list HTTP log addresses: ",
            )
            .await
        {
            return false;
        }

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
        println!("[RCON DEBUG] Waiting for client lock: {}", command);

        let mut client = self.client.lock().await;

        println!("[RCON DEBUG] Client lock acquired: {}", command);

        match client.command(command).await {
            Ok(response) => {
                println!("[RCON DEBUG] Command returned successfully");

                self.push_log(RconLogEvent::RconResponse(format!(
                    "{}{}",
                    success_prefix, response
                )));

                true
            }

            Err(error) => {
                println!("[RCON DEBUG] Command returned error: {}", error);

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
                            let _ = client.command("mp_pause_match").await;
                        });
                    }

                    Some(AdminCommand::UnPause) => {
                        let client = client.clone();

                        spawn(async move {
                            let mut client = client.lock().await;
                            let _ = client.command("mp_unpause_match").await;
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

    pub async fn connect(addr: SocketAddr, password: String) -> Option<Self> {
        let mut session = Self::new(addr, password).await;

        session.push_log(RconLogEvent::Info(format!(
            "[RCON] Connecting to {}...",
            addr
        )));

        if !session.connect_rcon().await {
            return None;
        }

        session.push_log(RconLogEvent::Info("[RCON] Authenticated.".to_string()));

        if !session.start_live_log().await {
            session.push_log(RconLogEvent::Info(
                "[RCON] Failed to configure live log.".to_string(),
            ));

            return None;
        }

        let cvarlist = session
            .client
            .lock()
            .await
            .command("cvarlist")
            .await
            .expect("Failed to get cvarlist via rcon");
        let db = CvarDatabase::new(&cvarlist);
        session.cvar_db = Signal::new(Some(db));

        let receiver = session.live_log.take_receiver();
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

        session.push_log(RconLogEvent::Info("[RCON] Session created.".to_string()));

        session.status.set(RconStatus::Authenticated);

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
        println!("[SHUTDOWN] RconSession::close() entered");

        if let Some(task) = self.live_log_task.take() {
            println!("[SHUTDOWN] Stopping LiveLog task");

            task.cancel();

            println!("[SHUTDOWN] LiveLog task stopped");
        }

        let Some(log_url) = self.log_url.clone() else {
            println!("[SHUTDOWN] No log URL");
            return true;
        };

        let command = format!("logaddress_del_http \"{}\"", log_url);

        println!("[SHUTDOWN] Sending cleanup command: {}", command);

        let mut client = self.client.lock().await;
        match client.command_no_response(&command).await {
            Ok(()) => {
                println!("[SHUTDOWN] Cleanup command sent successfully");
                true
            }

            Err(error) => {
                println!("[SHUTDOWN] Cleanup command failed: {}", error);
                false
            }
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
