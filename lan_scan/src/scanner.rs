use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::interval;

use crate::{parser, ParseResult, PendingQuery, ScanCommand, ServerUpdate, SplitBuffer};

const MAX_PING: Duration = Duration::from_secs(1);

pub struct Scanner {
    socket: Arc<UdpSocket>,
    cmd_rx: Receiver<ScanCommand>,
    ui_tx: Sender<ServerUpdate>,

    // State tracking
    pending_requests: HashMap<SocketAddr, Vec<PendingRequest>>,
    ping_tracker: HashMap<SocketAddr, Instant>,
    challenges: HashMap<SocketAddr, [u8; 4]>,
    split_cache: HashMap<(SocketAddr, u32), SplitBuffer>,

    // Configuration options
    timeout: Duration,
    max_retries: u8,
    broadcast: bool,
}

struct PendingRequest {
    query_type: PendingQuery,
    started: Instant,
    retries: u8,
}

impl Scanner {
    pub async fn new(
        bind_addr: &str,
        cmd_rx: Receiver<ScanCommand>,
        ui_tx: Sender<ServerUpdate>,
    ) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind(bind_addr).await?;

        if let Err(error) = socket.set_broadcast(true) {
            log::warn!(
                target: "lan_scan::scanner",
                "Failed to set SO_BROADCAST: {}",
                error
            );
        }

        Ok(Self {
            socket: Arc::new(socket),
            cmd_rx,
            ui_tx,
            pending_requests: HashMap::new(),
            ping_tracker: HashMap::new(),
            challenges: HashMap::new(),
            split_cache: HashMap::new(),
            timeout: Duration::from_millis(1000),
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
                // 1. Incoming command from the UI/controller
                command = self.cmd_rx.recv() => {
                    match command {
                        Some(command) => {
                            self.handle_command(command).await;
                        }

                        None => {
                            log::debug!(
                                target: "lan_scan::scanner",
                                "Scanner command channel closed"
                            );
                            break;
                        }
                    }
                }

                // 2. Incoming UDP packet response from a game server
                result = self.socket.recv_from(&mut recv_buf) => {
                    match result {
                        Ok((len, addr)) => {
                            self.handle_socket_data(&recv_buf[..len], addr).await;
                        }

                        Err(error) => {
                            log::error!(
                                target: "lan_scan::scanner",
                                "UDP receive failed: {}",
                                error
                            );
                        }
                    }
                }

                // 3. Periodic timeout/retry handling
                _ = cleanup_ticker.tick() => {
                    self.handle_timeouts().await;
                }

                // 4. Automatic background scanning
                _ = scan_ticker.tick() => {
                    self.handle_auto_scan().await;
                }
            }
        }
    }

    // =========================================================================
    // COMMAND HANDLING & OUTBOUND PACKETS
    // =========================================================================

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
                self.pending_requests.clear();
                self.ping_tracker.clear();
                self.challenges.clear();
                self.split_cache.clear();
            }
        }
    }

    async fn handle_auto_scan(&mut self) {
        if !self.broadcast {
            return;
        }

        // 1. Source / GoldSrc broadcast ports
        const BROADCAST_PORTS: &[u16] = &[27015, 27016, 27017, 27018, 27019, 27020];

        let payload = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";

        for &port in BROADCAST_PORTS {
            if let Ok(addr) = format!("255.255.255.255:{}", port).parse::<SocketAddr>() {
                let now = Instant::now();
                self.ping_tracker.insert(addr, now);

                if let Err(error) = self.socket.send_to(payload, addr).await {
                    log::warn!(
                        target: "lan_scan::scanner",
                        "Failed to send broadcast query to {}: {}",
                        addr,
                        error
                    );
                }
            }
        }

        // 2. Quake Engine ports
        const QUAKE_PORTS: &[u16] = &[
            27070, 27960, 27961, 27962, 27963, 27992, 28960, 28961, 28962, 28963,
        ];

        let quake_payload = b"\xFF\xFF\xFF\xFFgetstatus\x00";

        for &port in QUAKE_PORTS {
            if let Ok(addr) = format!("255.255.255.255:{}", port).parse::<SocketAddr>() {
                let now = Instant::now();
                self.ping_tracker.insert(addr, now);

                if let Err(error) = self.socket.send_to(quake_payload, addr).await {
                    log::warn!(
                        target: "lan_scan::scanner",
                        "Failed to send Quake broadcast query to {}: {}",
                        addr,
                        error
                    );
                }
            }
        }

        // 3. GameSpy v1 ports
        const GAMESPY_PORTS: &[u16] = &[7777, 7778, 7787, 7788, 23000, 12203, 12300];

        let gamespy_payload = b"\\status\\";

        for &port in GAMESPY_PORTS {
            if let Ok(addr) = format!("255.255.255.255:{}", port).parse::<SocketAddr>() {
                let now = Instant::now();
                self.ping_tracker.insert(addr, now);

                if let Err(error) = self.socket.send_to(gamespy_payload, addr).await {
                    log::warn!(
                        target: "lan_scan::scanner",
                        "Failed to send GameSpy broadcast query to {}: {}",
                        addr,
                        error
                    );
                }
            }
        }

        // Optional: re-query existing/known servers.
        // self.refresh_known_servers().await;
    }

    async fn send_query(&mut self, addr: SocketAddr, query_type: PendingQuery) {
        let now = Instant::now();

        // Track ping timestamp.
        self.ping_tracker.insert(addr, now);

        // Preserve the retry count if this query already exists.
        let requests = self.pending_requests.entry(addr).or_default();

        if let Some(request) = requests
            .iter_mut()
            .find(|request| request.query_type == query_type)
        {
            request.started = now;
        } else {
            requests.push(PendingRequest {
                query_type,
                started: now,
                retries: 0,
            });
        }

        let challenge = self.challenges.get(&addr).copied();

        let payload = match query_type {
            PendingQuery::Info => {
                // Source / GoldSrc A2S_INFO query.
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
                // Source A2S_PLAYER query.
                let mut pkt = vec![0xFF, 0xFF, 0xFF, 0xFF, b'U'];

                let token = challenge.unwrap_or([0xFF, 0xFF, 0xFF, 0xFF]);

                pkt.extend_from_slice(&token);
                pkt
            }

            PendingQuery::Rules => {
                // Source A2S_RULES query.
                let mut pkt = vec![0xFF, 0xFF, 0xFF, 0xFF, b'V'];

                let token = challenge.unwrap_or([0xFF, 0xFF, 0xFF, 0xFF]);

                pkt.extend_from_slice(&token);
                pkt
            }
        };

        if let Err(error) = self.socket.send_to(&payload, addr).await {
            log::warn!(
                target: "lan_scan::scanner",
                "Failed to send {:?} query to {}: {}",
                query_type,
                addr,
                error
            );
        }
    }

    // =========================================================================
    // INBOUND RESPONSE PROCESSING
    // =========================================================================

    async fn handle_socket_data(&mut self, data: &[u8], addr: SocketAddr) {
        let mut data_vec = data.to_vec();

        let ping_ms = self.calculate_ping(addr);

        match parser::parse(&mut data_vec, addr, ping_ms, &mut self.split_cache) {
            ParseResult::Update { query_type, update } => {
                if let Some(requests) = self.pending_requests.get_mut(&addr) {
                    requests.retain(|request| request.query_type != query_type);

                    if requests.is_empty() {
                        self.pending_requests.remove(&addr);
                    }
                }

                if let Err(error) = self.ui_tx.send(update).await {
                    log::error!(
                        target: "lan_scan::scanner",
                        "Failed to send update for {}: {}",
                        addr,
                        error
                    );
                }
            }

            ParseResult::Challenge(token) => {
                // Keep the challenge until it is replaced by a newer one.
                self.challenges.insert(addr, token);

                // We do not know which pending query caused the challenge,
                // so request Info again using the new token.
                self.send_query(addr, PendingQuery::Info).await;
            }

            ParseResult::PartialSplit => {
                // Packet split reassembly is in progress.
            }

            ParseResult::Ignored => {
                // Raw / unrecognized response bytes.
            }
        }
    }

    // =========================================================================
    // TIMEOUT & RETRY MANAGEMENT
    // =========================================================================

    async fn handle_timeouts(&mut self) {
        let now = Instant::now();
        let timeout = self.timeout;
        let max_retries = self.max_retries;

        let mut expired = Vec::new();
        let mut retries = Vec::new();

        for (&addr, requests) in &self.pending_requests {
            for request in requests {
                if now.duration_since(request.started) > timeout {
                    if request.retries < max_retries {
                        retries.push((addr, request.query_type, request.retries + 1));
                    } else {
                        expired.push((addr, request.query_type));
                    }
                }
            }
        }

        // Remove only the query that actually expired.
        for (addr, query_type) in expired {
            let mut all_requests_expired = false;

            if let Some(requests) = self.pending_requests.get_mut(&addr) {
                requests.retain(|request| request.query_type != query_type);
                all_requests_expired = requests.is_empty();
            }

            if !all_requests_expired {
                log::debug!(
                    target: "lan_scan::scanner",
                    "Query {:?} timed out for {} after {} retries",
                    query_type,
                    addr,
                    max_retries
                );

                continue;
            }

            self.pending_requests.remove(&addr);
            self.ping_tracker.remove(&addr);

            self.split_cache
                .retain(|(split_addr, _), _| *split_addr != addr);

            log::debug!(
                target: "lan_scan::scanner",
                "All queries for {} timed out after {} retries",
                addr,
                max_retries
            );

            if let Err(error) = self.ui_tx.send(ServerUpdate::Failed { addr }).await {
                log::error!(
                    target: "lan_scan::scanner",
                    "Failed to send timeout update for {}: {}",
                    addr,
                    error
                );
            }
        }

        // Retry each individual query.
        for (addr, query_type, retry_count) in retries {
            if let Some(requests) = self.pending_requests.get_mut(&addr) {
                if let Some(request) = requests
                    .iter_mut()
                    .find(|request| request.query_type == query_type)
                {
                    request.retries = retry_count;
                }
            }

            self.send_query(addr, query_type).await;
        }
    }

    // =========================================================================
    // PING TRACKING
    // =========================================================================

    fn calculate_ping(&mut self, addr: SocketAddr) -> Option<u16> {
        // Exact unicast match.
        if let Some(start) = self.ping_tracker.remove(&addr) {
            let elapsed = start.elapsed();

            if elapsed <= MAX_PING {
                return Some(elapsed.as_millis().clamp(1, u16::MAX as u128) as u16);
            }

            return None;
        }

        // Broadcast fallback.
        let matching_key = self
            .ping_tracker
            .keys()
            .find(|key| key.port() == addr.port() && self.is_broadcast_ip(&key.ip()))
            .cloned();

        if let Some(key) = matching_key {
            if let Some(start) = self.ping_tracker.get(&key) {
                let elapsed = start.elapsed();

                if elapsed <= MAX_PING {
                    return Some(elapsed.as_millis().clamp(1, u16::MAX as u128) as u16);
                }
            }
        }

        None
    }

    fn is_broadcast_ip(&self, ip: &std::net::IpAddr) -> bool {
        match ip {
            std::net::IpAddr::V4(v4) => v4.is_broadcast() || v4.octets()[3] == 255,

            std::net::IpAddr::V6(_) => false,
        }
    }
}
