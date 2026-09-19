use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use cbz_rcon::{RconClient, RconProtocol, RconStatus};
use dioxus::{
    core::{spawn_forever, Task},
    prelude::*,
};
use lan_scan::ServerProtocol;
use live_log::{
    _parser::types::{LogEvent, ParsedLine, Team},
    game::Game,
    live_log::LiveLog,
};
use tokio::sync::mpsc::{Receiver, Sender};

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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RconState {
    logs: Signal<Vec<RconLogEvent>>,
    status: Signal<RconStatus>,
    players: Signal<RconPlayers>,
    match_paused: Signal<bool>,
    score: Signal<TeamScore>,
    maps: Signal<Vec<String>>,
    team_name_ct: Signal<String>,
    team_name_t: Signal<String>,
    max_rounds: Signal<u8>,
    need_attention: Signal<bool>,
    cvar_db: Signal<Option<CvarDatabase>>,
    command_history: Signal<Vec<String>>,
    pub sender: Signal<Sender<StateUpdate>>,
    receiver: Signal<Receiver<StateUpdate>>,
}

impl RconState {
    pub fn new() -> RconState {
        let (sender, receiver) = tokio::sync::mpsc::channel::<StateUpdate>(256);

        RconState {
            logs: Signal::new_in_scope(Vec::new(), ScopeId::ROOT),
            status: Signal::new_in_scope(RconStatus::Disconnected, ScopeId::ROOT),
            players: Signal::new_in_scope(RconPlayers::new(), ScopeId::ROOT),
            match_paused: Signal::new_in_scope(false, ScopeId::ROOT),
            score: Signal::new_in_scope(
                TeamScore {
                    ct: 0,
                    t: 0,
                    round: 0,
                },
                ScopeId::ROOT,
            ),
            maps: Signal::new_in_scope(Vec::new(), ScopeId::ROOT),
            team_name_ct: Signal::new_in_scope("TeamA".to_string(), ScopeId::ROOT),
            team_name_t: Signal::new_in_scope(String::new(), ScopeId::ROOT),
            max_rounds: Signal::new_in_scope(0, ScopeId::ROOT),
            need_attention: Signal::new_in_scope(false, ScopeId::ROOT),
            cvar_db: Signal::new_in_scope(None, ScopeId::ROOT),
            command_history: Signal::new_in_scope(Vec::new(), ScopeId::ROOT),
            sender: Signal::new_in_scope(sender, ScopeId::ROOT),
            receiver: Signal::new_in_scope(receiver, ScopeId::ROOT),
        }
        /*
        RconState {
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
            cvar_db: Signal::new(None),
            command_history: Signal::new(Vec::new()),
            sender: Signal::new(sender),
            receiver: Signal::new(receiver),
        }
        */
    }
    pub async fn run(&mut self) {
        loop {
            let update = {
                let mut receiver = self.receiver.write();
                receiver.recv().await
            };

            let Some(update) = update else {
                tracing::debug!("Break in RconState::run ");
                break;
            };

            self.update(update);
        }
    }

    pub fn update(&self, update: StateUpdate) {
        match update {
            StateUpdate::Log(rcon_log_event) => {
                let mut logs = self.logs;
                logs.write().push(rcon_log_event);
            }

            StateUpdate::LogEvent(parsed) => {
                let mut logs = self.logs;
                logs.write().push(RconLogEvent::LiveLog(parsed.clone()));

                match &parsed.event {
                    LogEvent::Connection {
                        player,
                        action,
                        info: _,
                    } => {
                        let mut players = self.players;

                        if action.eq_ignore_ascii_case("disconnected") {
                            players.write().remove_player(player.id);
                        }
                    }

                    LogEvent::TeamSwitch { player, .. } => {
                        let mut players = self.players;
                        players.write().update_with_team_switch(player);
                    }

                    LogEvent::RoundStats { roundstats } => {
                        let mut players = self.players;
                        players.write().update_with_roundstats(roundstats);

                        let mut score = self.score;
                        score.write().update_with_roundstats(roundstats);
                    }

                    LogEvent::ScoreUpdate { rounds, .. } => {
                        let mut score = self.score;
                        score.write().update_with_score_update(*rounds);
                    }

                    LogEvent::MatchStatus {
                        team,
                        team_name: Some(team_name),
                    } => match team {
                        Team::CT => {
                            let mut team_name_ct = self.team_name_ct;
                            team_name_ct.set(team_name.clone());
                        }

                        Team::Terrorist => {
                            let mut team_name_t = self.team_name_t;
                            team_name_t.set(team_name.clone());
                        }

                        _ => {}
                    },

                    LogEvent::Technical { name, action } if name == "Match" => {
                        match action.as_str() {
                            "Pause Enabled" => {
                                let mut match_paused = self.match_paused;
                                match_paused.set(true);
                            }

                            "Pause Disabled" => {
                                let mut match_paused = self.match_paused;
                                match_paused.set(false);
                            }

                            _ => {}
                        }
                    }

                    LogEvent::ServerCvar { name, value } => {
                        if name == "mp_maxrounds" {
                            let mut max_rounds = self.max_rounds;
                            max_rounds.set(value.parse().unwrap_or(0));
                        }

                        let mut cvar_db = self.cvar_db;

                        {
                            let mut cvar_db_guard = cvar_db.write();

                            if let Some(db) = cvar_db_guard.as_mut() {
                                db.update(name, value);
                            }
                        }
                    }

                    LogEvent::Chat { .. } => match is_command(&parsed.event) {
                        Some(AdminCommand::Admin) => {
                            let mut attention = self.need_attention;
                            attention.set(true);
                        }

                        Some(AdminCommand::Clear) => {
                            let mut attention = self.need_attention;
                            attention.set(false);
                        }

                        _ => {}
                    },

                    _ => {}
                }
            }

            StateUpdate::CommandHistory(command) => {
                let mut history = self.command_history;
                history.write().push(command);
            }
        }
    }

    pub fn push_log(&self, event: RconLogEvent) {
        let mut logs = self.logs;
        logs.write().push(event);
    }
    pub fn add_command_to_history(&self, command: String) {
        let sender = self.sender.read().clone();

        spawn(async move {
            if sender
                .send(StateUpdate::CommandHistory(command))
                .await
                .is_err()
            {
                tracing::error!("Failed to update command history");
            }
        });
    }
    pub fn logs(&self) -> ReadSignal<Vec<RconLogEvent>> {
        self.logs.into()
    }
    pub fn status(&self) -> ReadSignal<RconStatus> {
        self.status.into()
    }
    pub fn players(&self) -> ReadSignal<RconPlayers> {
        self.players.into()
    }
    pub fn score(&self) -> ReadSignal<TeamScore> {
        self.score.into()
    }
    pub fn maps(&self) -> ReadSignal<Vec<String>> {
        self.maps.into()
    }

    pub fn team_name_ct(&self) -> ReadSignal<String> {
        self.team_name_ct.into()
    }

    pub fn team_name_t(&self) -> ReadSignal<String> {
        self.team_name_t.into()
    }

    pub fn need_attention(&self) -> ReadSignal<bool> {
        self.need_attention.into()
    }

    pub fn cvar_db(&self) -> ReadSignal<Option<CvarDatabase>> {
        self.cvar_db.into()
    }

    pub fn command_history(&self) -> ReadSignal<Vec<String>> {
        self.command_history.into()
    }

    pub fn match_paused(&self) -> ReadSignal<bool> {
        self.match_paused.into()
    }
    fn close(&mut self) {
        tracing::debug!("RconState: Dropping signals to prevent leaks...");

        // Manually drop each signal
        self.logs.manually_drop();
        self.status.manually_drop();
        self.players.manually_drop();
        self.match_paused.manually_drop();
        self.score.manually_drop();
        self.maps.manually_drop();
        self.team_name_ct.manually_drop();
        self.team_name_t.manually_drop();
        self.max_rounds.manually_drop();
        self.need_attention.manually_drop();
        self.cvar_db.manually_drop();
        self.command_history.manually_drop();
        self.sender.manually_drop();
        self.receiver.manually_drop();
    }
}

#[derive(Debug, Clone)]
pub enum StateUpdate {
    Log(RconLogEvent),
    CommandHistory(String),
    LogEvent(ParsedLine),
}
pub struct RconSession {
    pub addr: SocketAddr,

    // -------------------------------------------------------------------------
    // RCON connection
    // -------------------------------------------------------------------------
    pub client: Arc<tokio::sync::Mutex<RconClient>>,
    pub live_log: Option<LiveLog>,
    pub live_log_task: Option<Task>,
    pub live_log_url: Option<String>,
    pub matchzy_log_url: Option<String>,
    pub state: RconState,
}

impl RconSession {
    pub fn new(addr: SocketAddr, password: String, protocol: RconProtocol) -> Self {
        let client = Arc::new(tokio::sync::Mutex::new(RconClient::new(
            addr, password, protocol,
        )));
        let state = RconState::new();

        spawn_forever({
            let mut state = state;
            async move {
                state.run().await;
            }
        });
        Self {
            addr,
            client,
            live_log: None,
            live_log_url: None,
            matchzy_log_url: None,
            live_log_task: None,
            state,
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

    async fn start_live_log(&mut self, game: Game) -> bool {
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

        let live_log = match LiveLog::new(game).await {
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

        let address = format!("{}:{}", receiver_ip, port);

        let command = match game {
            Game::Cs2 => format!("logaddress_add_http \"{}\"", log_url),
            Game::Cs16 => format!("logaddress_add {} {}", receiver_ip, port),
            Game::Css => format!("logaddress_add {}:{}", receiver_ip, port),
            Game::DoDS => format!("logaddress_add {}:{}", receiver_ip, port),
            Game::GenericGoldSrc => format!("logaddress_add {} {}", receiver_ip, port),
            Game::GenericSource => format!("logaddress_add {}:{}", receiver_ip, port),
            Game::GenericSource2 => format!("logaddress_add_http \"{}\"", log_url),
        };

        if !self
            .send_rcon_command(
                &command,
                &format!("[LIVE_LOG] Registered {}: ", address),
                &format!("[LIVE_LOG] Failed to register {}: ", address),
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
        self.state.push_log(event);
    }

    // =========================================================================
    // RCON COMMAND
    // =========================================================================

    async fn send_rcon_command(
        &mut self,
        command: &str,
        success_prefix: &str,
        error_prefix: &str,
    ) -> bool {
        let mut client = self.client.lock().await;

        match client.command(command).await {
            Ok(response) => {
                self.state.push_log(RconLogEvent::RconResponse(format!(
                    "{}{}",
                    success_prefix, response
                )));

                true
            }

            Err(error) => {
                self.state
                    .push_log(RconLogEvent::Info(format!("{}{}", error_prefix, error)));

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
        rcon_state: RconState,
    ) {
        let sender = rcon_state.sender.read().clone();

        while let Some(parsed) = receiver.recv().await {
            if sender
                .send(StateUpdate::LogEvent(parsed.clone()))
                .await
                .is_err()
            {
                break;
            }
            match is_command(&parsed.event) {
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
                            tracing::error!("Failed to unpause match: {}", error);
                        }
                    });
                }

                _ => {}
            }
        }
    }

    // =========================================================================
    // CONNECTION / SESSION CREATION
    // =========================================================================

    pub async fn connect(addr: SocketAddr, password: String, game: Game) -> Option<Self> {
        let protocol = match game {
            Game::Cs2 => ServerProtocol::Source2,
            Game::Css => ServerProtocol::Source,
            Game::Cs16 => ServerProtocol::GoldSrc,
            Game::DoDS => ServerProtocol::Source,
            Game::GenericGoldSrc => ServerProtocol::GoldSrc,
            Game::GenericSource => ServerProtocol::Source,
            Game::GenericSource2 => ServerProtocol::Source2,
        };

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

        let has_live_log = matches!(game, Game::Cs2 | Game::Css | Game::Cs16 | Game::DoDS);

        if has_live_log {
            if !session.start_live_log(game).await {
                session.push_log(RconLogEvent::Info(
                    "[RCON] Failed to configure live log.".to_string(),
                ));
                return None;
            }

            if matches!(game, Game::Cs2) {
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
                session.state.cvar_db.set(Some(db));
            }

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

                let client = session.client.clone();
                let rcon_state = session.state.clone();

                //let live_log_task =
                spawn_forever(async move {
                    tracing::debug!("========== LIVE LOG TASK START ==========");
                    Self::process_live_log(receiver, client, rcon_state).await;
                    tracing::debug!("========== LIVE LOG TASK END ==========");
                });
                tracing::debug!("RconSession::connect: storing live_log_task for {}", addr);
                //session.live_log_task = Some(live_log_task);
                tracing::debug!("process_live_log task stored in RconSession");
            }
        } else {
            session.push_log(RconLogEvent::Info(
                "[LIVE_LOG] Skipped live log setup for unsupported game.".to_string(),
            ));
        }

        session.push_log(RconLogEvent::Info("[RCON] Session created.".to_string()));

        *session.state.status.write() = RconStatus::Authenticated;

        if matches!(game, Game::Cs2) {
            let local_ip = log_receiver_ip(addr).unwrap_or_else(|| Ipv4Addr::new(127, 0, 0, 1));

            let port = 7131;
            let matchzy_log_url = format!("http://{}:{}/MatchZyLogs", local_ip, port);

            session.matchzy_log_url = Some(matchzy_log_url.clone());

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

                Err(error) => {
                    tracing::error!("Failed to register log address for {}: {}", addr, error);
                }
            }

            drop(client_lock);
        }

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
        let mut maps = self.state.maps;
        let mut logs = self.state.logs;

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
            tracing::debug!("Cancelling process_live_log task");
            task.cancel();
        }

        if let Some(live_log) = self.live_log.take() {
            live_log.stop().await;
        }

        let mut success = true;
        let mut client = self.client.lock().await;

        if let Some(live_log_url) = self.live_log_url.take() {
            let command_live_log = format!("logaddress_del_http \"{}\"", live_log_url);

            let cleanup_live_log = match client.command(&command_live_log).await {
                Ok(_) => true,
                Err(error) => {
                    tracing::error!("Cleanup live log failed for {}: {}", self.addr, error);
                    false
                }
            };

            success &= cleanup_live_log;
        }

        if self.matchzy_log_url.take().is_some() {
            let cleanup_matchzy = match client.command("matchzy_remote_log_url \"\"").await {
                Ok(_) => true,
                Err(error) => {
                    tracing::error!("Cleanup MatchZy failed for {}: {}", self.addr, error);
                    false
                }
            };

            success &= cleanup_matchzy;
        }
        self.state.close();

        success
    }

    fn rcon_protocol(protocol: ServerProtocol) -> Option<RconProtocol> {
        match protocol {
            ServerProtocol::Source => Some(RconProtocol::Source),
            ServerProtocol::Source2 => Some(RconProtocol::Source2),
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
