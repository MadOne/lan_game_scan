use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::interval;

use crate::scanner::parser::{self, ParseResult, SplitBuffer};
use crate::server::ScannedServer;

/// Defines the type of query currently pending for a server endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingQuery {
    Info,
    Player,
    Rules,
}

/// Commands sent to the Scanner to initiate queries
#[derive(Debug)]
pub enum ScanCommand {
    /// Scan a single target endpoint
    ScanServer {
        addr: SocketAddr,
        query_type: PendingQuery,
    },
    /// Batch scan multiple target endpoints
    BatchScan {
        addrs: Vec<SocketAddr>,
        query_type: PendingQuery,
    },
    /// Cancel any active scans
    Cancel,
}

/// Internal signals for retrying queries after receiving challenge tokens
#[derive(Debug)]
pub enum RetrySignal {
    Info(SocketAddr),
    Player(SocketAddr),
}

/// Player details returned by server queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInfo {
    pub name: String,
    pub score: i32,
    pub ping: Option<u16>,
    pub duration_secs: Option<f32>,
    pub index: Option<u8>,
    pub team: Option<u8>,
    pub skin: Option<String>,
    pub is_bot: bool,
}

/// Dispatched back to the UI or coordinator layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerUpdate {
    FullServer(ScannedServer),
    PlayerList {
        addr: SocketAddr,
        players: Vec<PlayerInfo>,
    },
    Failed {
        addr: SocketAddr,
    },
}

pub struct Scanner {
    socket: Arc<UdpSocket>,
    cmd_rx: Receiver<ScanCommand>,
    ui_tx: Sender<ServerUpdate>,

    // State tracking
    pending_queries: HashMap<SocketAddr, (PendingQuery, Instant, u8)>, // (Type, StartTime, Retries)
    ping_tracker: HashMap<SocketAddr, Instant>,
    challenges: HashMap<SocketAddr, [u8; 4]>,
    split_cache: HashMap<(SocketAddr, u32), SplitBuffer>,

    // Configuration options
    timeout: Duration,
    max_retries: u8,
    broadcast: bool,
}

impl Scanner {
    pub async fn new(
        bind_addr: &str,
        cmd_rx: Receiver<ScanCommand>,
        ui_tx: Sender<ServerUpdate>,
    ) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(bind_addr).await?;
        if let Err(err) = socket.set_broadcast(true) {
            eprintln!("[SCANNER] Warning: Failed to set SO_BROADCAST: {}", err);
        }

        Ok(Scanner {
            socket: Arc::new(socket),
            cmd_rx,
            ui_tx,
            pending_queries: HashMap::new(),
            ping_tracker: HashMap::new(),
            challenges: HashMap::new(),
            split_cache: HashMap::new(),
            timeout: Duration::from_millis(1500),
            max_retries: 2,
            broadcast: true,
        })
    }

    /// Main event loop driven by tokio::select!
    pub async fn run(mut self) {
        let mut recv_buf = vec![0u8; 4096];
        let mut cleanup_ticker = interval(Duration::from_millis(250));
        let mut scan_ticker = interval(Duration::from_secs(10));
        loop {
            tokio::select! {
                // 1. Incoming command from the UI/Controller
                Some(cmd) = self.cmd_rx.recv() => {
                    //self.handle_command(cmd).await;
                }

                // 2. Incoming UDP packet response from a game server
                Ok((len, addr)) = self.socket.recv_from(&mut recv_buf) => {
                    self.handle_socket_data(&recv_buf[..len], addr).await;
                }

                // 3. Periodic timer to prune timed-out requests or retry missing challenges
                _ = cleanup_ticker.tick() => {
                    //self.handle_timeouts().await;
                }
                // 4. Automatic background scanning
                _ = scan_ticker.tick() => {
                    self.handle_auto_scan().await;
                }
            }
        }
    }

    // --- COMMAND HANDLING & OUTBOUND PACKETS ---

    async fn handle_command(&mut self, cmd: ScanCommand) {
        match cmd {
            ScanCommand::ScanServer { addr, query_type } => {
                self.send_query(addr, query_type).await;
            }
            ScanCommand::BatchScan { addrs, query_type } => {
                for addr in addrs {
                    self.send_query(addr, query_type).await;
                }
            }
            ScanCommand::Cancel => {
                self.pending_queries.clear();
                self.ping_tracker.clear();
                self.challenges.clear();
                self.split_cache.clear();
            }
        }
    }
    async fn handle_auto_scan(&self) {
        // 1. Broadcast to LAN query ports if enabled
        if self.broadcast {
            const BROADCAST_PORTS: &[u16] = &[27015, 27016, 27017, 27018, 27019, 27020];
            let payload = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";

            for &port in BROADCAST_PORTS {
                if let Ok(addr) = format!("255.255.255.255:{}", port).parse::<SocketAddr>() {
                    let _ = self.socket.send_to(payload, addr).await;
                }
            }
            // 2. Quake Engine Ports (Q3A default: 27960, Q2 default: 27910)
            const QUAKE_PORTS: &[u16] = &[
                27070, 27960, 27961, 27962, 27963, 27992, 28960, 28961, 28962, 28963,
            ];

            let quake_payload = b"\xFF\xFF\xFF\xFFgetstatus\x00";

            for &port in QUAKE_PORTS {
                if let Ok(addr) = format!("255.255.255.255:{}", port).parse::<SocketAddr>() {
                    let _ = self.socket.send_to(quake_payload, addr).await;
                }
            }
            // 3. GameSpy v1 Ports (UT2004 default query offset: 7778, Battlefield 1942: 23000, 28910)
            const GAMESPY_PORTS: &[u16] = &[7777, 7778, 7787, 7788, 23000, 12203, 12300];
            let gamespy_payload = b"\\status\\";

            for &port in GAMESPY_PORTS {
                if let Ok(addr) = format!("255.255.255.255:{}", port).parse::<SocketAddr>() {
                    let _ = self.socket.send_to(gamespy_payload, addr).await;
                }
            }
        }

        // 2. Optional: Re-query existing/known servers in state
        // self.refresh_known_servers().await;
    }

    async fn send_query(&mut self, addr: SocketAddr, query_type: PendingQuery) {
        let now = Instant::now();

        // Track ping timestamp and pending state
        self.ping_tracker.insert(addr, now);
        let retries = self
            .pending_queries
            .get(&addr)
            .map(|(_, _, r)| *r)
            .unwrap_or(0);
        self.pending_queries
            .insert(addr, (query_type, now, retries));

        let challenge = self.challenges.get(&addr).copied();

        // Construct raw payload depending on protocol / query requirements
        let payload = match query_type {
            PendingQuery::Info => {
                // Source / GoldSrc A2S_INFO query payload
                let mut pkt = vec![
                    0xFF, 0xFF, 0xFF, 0xFF, b'T', b'S', b'o', b'u', b'r', b'c', b'e', b' ', b'E',
                    b'n', b'g', b'i', b'n', b'e', b' ', b'Q', b'u', b'e', b'r', b'y', 0x00,
                ];
                if let Some(token) = challenge {
                    pkt.extend_from_slice(&token);
                }

                pkt
            }
            PendingQuery::Player => {
                // Source A2S_PLAYER query payload (Requires challenge token if server demands it)
                let mut pkt = vec![0xFF, 0xFF, 0xFF, 0xFF, b'U'];
                let token = challenge.unwrap_or([0xFF, 0xFF, 0xFF, 0xFF]);
                pkt.extend_from_slice(&token);
                pkt
            }
            PendingQuery::Rules => {
                // Source A2S_RULES query payload
                let mut pkt = vec![0xFF, 0xFF, 0xFF, 0xFF, b'V'];
                let token = challenge.unwrap_or([0xFF, 0xFF, 0xFF, 0xFF]);
                pkt.extend_from_slice(&token);
                pkt
            }
        };

        let _ = self.socket.send_to(&payload, addr).await;
    }

    // --- INBOUND RESPONSE PROCESSING ---

    async fn handle_socket_data(&mut self, data: &[u8], addr: SocketAddr) {
        let mut data_vec = data.to_vec();

        // Calculate RTT ping duration
        let mut ping_ms = self.calculate_ping(addr);
        if ping_ms == None {
            ping_ms = Some(999);
        }

        // Execute pure parser logic
        match parser::parse(&mut data_vec, addr, ping_ms, &mut self.split_cache) {
            ParseResult::Update(update) => {
                self.pending_queries.remove(&addr);
                match self.ui_tx.send(update).await {
                    Ok(_) => println!("[UI_TX SUCCESS] Dispatched server update for {}", addr),
                    Err(e) => {
                        eprintln!("[UI_TX ERROR] Failed to send update for {}: {:?}", addr, e)
                    }
                }
            }

            ParseResult::Challenge(token) => {
                self.challenges.insert(addr, token);
                self.send_query(addr, PendingQuery::Info).await;
            }

            ParseResult::PartialSplit => {
                // Packet split reassembly in progress
            }

            ParseResult::Ignored => {
                // Raw / Unrecognized response bytes
            }
        }
    }

    // --- TIMEOUT & RETRY MANAGEMENT ---

    async fn handle_timeouts(&mut self) {
        let now = Instant::now();
        let timeout = self.timeout;
        let max_retries = self.max_retries;

        let mut expired = Vec::new();
        let mut retries = Vec::new();

        for (&addr, &(query_type, start_time, retry_count)) in self.pending_queries.iter() {
            if now.duration_since(start_time) > timeout {
                if retry_count < max_retries {
                    retries.push((addr, query_type, retry_count + 1));
                } else {
                    expired.push(addr);
                }
            }
        }

        // Notify UI layer of dropped/timed-out servers
        for addr in expired {
            self.pending_queries.remove(&addr);
            self.ping_tracker.remove(&addr);
            let _ = self.ui_tx.send(ServerUpdate::Failed { addr }).await;
        }

        // Resend query for retried servers
        for (addr, query_type, count) in retries {
            self.pending_queries
                .insert(addr, (query_type, Instant::now(), count));
            self.send_query(addr, query_type).await;
        }
    }

    // 1. Helper function or method on your struct:
    fn calculate_ping(&mut self, addr: SocketAddr) -> Option<u16> {
        // Exact Unicast Match
        if let Some(start) = self.ping_tracker.remove(&addr) {
            return Some(start.elapsed().as_millis().max(1) as u16);
        }

        // Broadcast Fallback: Match if key IP is Broadcast AND Port matches
        let matching_key = self
            .ping_tracker
            .keys()
            .find(|k| k.port() == addr.port() && self.is_broadcast_ip(&k.ip()))
            .cloned();

        if let Some(key) = matching_key {
            if let Some(start) = self.ping_tracker.get(&key) {
                return Some(start.elapsed().as_millis().max(1) as u16);
            }
        }

        None
    }

    // Helper to identify global or subnet broadcast addresses
    fn is_broadcast_ip(&self, ip: &std::net::IpAddr) -> bool {
        match ip {
            std::net::IpAddr::V4(v4) => v4.is_broadcast() || v4.octets()[3] == 255,
            std::net::IpAddr::V6(_) => false,
        }
    }
}
