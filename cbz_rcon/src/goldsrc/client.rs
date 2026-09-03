use std::str;
use std::{net::SocketAddr, time::Duration};
use tokio::{net::UdpSocket, time::timeout};

use crate::RconError;
use crate::goldsrc::packet::{
    GoldSrcPacketResponse, challenge_request, parse_response, rcon_request,
};

pub const GOLD_SRC_HEADER: [u8; 4] = [0xFF; 4];

pub struct GoldSrcRconClient {
    addr: SocketAddr,
    password: String,
    socket: Option<UdpSocket>,
    challenge: Option<String>,
    authenticated: bool,
}

impl GoldSrcRconClient {
    pub fn new(addr: SocketAddr, password: impl Into<String>) -> Self {
        println!("GoldSource Rcon Client created");
        Self {
            addr,
            password: password.into(),
            socket: None,
            challenge: None,
            authenticated: false,
        }
    }

    pub async fn connect(&mut self) -> Result<(), RconError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        self.socket = Some(socket);
        println!("GoldSource c");
        let response = self.command("status").await?;

        self.authenticated = self.is_valid_status_response(&response);

        Ok(())
    }

    pub async fn command(&mut self, command: &str) -> Result<String, RconError> {
        println!("[GoldSrc RCON] command: {command}");

        self.command_no_response(command).await?;
        println!("[GoldSrc RCON] command packet sent");

        let socket = self.socket.as_ref().ok_or(RconError::NotConnected)?;
        let mut buf = [0u8; 8192];

        println!("[GoldSrc RCON] waiting for command response");

        let mut collected_chunks: Vec<(u8, Vec<u8>)> = Vec::new();
        let mut seq_counter: u8 = 0;

        // Read the first datagram
        let (len, _) = timeout(Duration::from_secs(3), socket.recv_from(&mut buf))
            .await
            .map_err(|_| RconError::Timeout)??;

        println!("[GoldSrc RCON] received {len} bytes");

        let mut is_multi_packet = false;
        let mut current_is_last = false;

        match parse_response(&buf[..len])? {
            GoldSrcPacketResponse::Single(response) => {
                let chunk = response
                    .strip_prefix(&GOLD_SRC_HEADER)
                    .unwrap_or(&response)
                    .to_vec();
                collected_chunks.push((seq_counter, chunk));
                seq_counter += 1;
            }
            GoldSrcPacketResponse::Multi {
                packet_number,
                is_last,
                payload,
                ..
            } => {
                is_multi_packet = true;
                current_is_last = is_last;
                collected_chunks.push((packet_number, payload));
            }
        }

        // Keep collecting trailing datagrams (both single-header chunks and split-multi chunks)
        loop {
            if is_multi_packet && current_is_last {
                break;
            }

            // Use a short timeout (500ms) to drain remaining UDP buffer packets
            let recv_result = timeout(Duration::from_millis(500), socket.recv_from(&mut buf)).await;

            let len = match recv_result {
                Ok(Ok((len, _))) => len,
                _ => break, // Timeout means no more chunks are pending on the socket
            };

            println!("[GoldSrc RCON] Received next packet {} bytes", len);

            if let Ok(parsed) = parse_response(&buf[..len]) {
                match parsed {
                    GoldSrcPacketResponse::Multi {
                        packet_number,
                        is_last,
                        payload,
                        ..
                    } => {
                        collected_chunks.push((packet_number, payload));
                        current_is_last = is_last;
                    }
                    GoldSrcPacketResponse::Single(payload) => {
                        let chunk = payload
                            .strip_prefix(&GOLD_SRC_HEADER)
                            .unwrap_or(&payload)
                            .to_vec();
                        collected_chunks.push((seq_counter, chunk));
                        seq_counter += 1;
                    }
                }
            }
        }

        // Sort fragments by index and remove duplicate UDP packets
        collected_chunks.sort_by_key(|(num, _)| *num);
        collected_chunks.dedup_by_key(|(num, _)| *num);

        // Flatten and strip headers
        let mut full_payload = Vec::new();
        for (_, chunk) in collected_chunks {
            let clean_chunk = chunk.strip_prefix(&GOLD_SRC_HEADER).unwrap_or(&chunk);
            full_payload.extend_from_slice(clean_chunk);
        }

        // Convert to string and clean up trailing null bytes and "l\n" prefix
        let raw_str = str::from_utf8(&full_payload)
            .map_err(|_| RconError::InvalidUtf8)?
            .trim_matches('\0')
            .trim();

        let response = raw_str.strip_prefix('l').unwrap_or(raw_str).trim();

        Ok(response.to_string())
    }

    pub async fn command_no_response(&mut self, command: &str) -> Result<(), RconError> {
        let socket = self.socket.as_ref().ok_or(RconError::NotConnected)?;
        println!("[GoldSrc RCON] sending challenge request");
        socket.send_to(&challenge_request(), self.addr).await?;
        println!("[GoldSrc RCON] waiting for challenge");
        let mut buf = [0u8; 1024];
        let (len, source) = timeout(Duration::from_secs(3), socket.recv_from(&mut buf))
            .await
            .map_err(|_| RconError::Timeout)??;
        println!("[GoldSrc RCON] received {} bytes from {}", len, source);

        let response = &buf[..len];

        let response = response
            .strip_prefix(&GOLD_SRC_HEADER)
            .ok_or(RconError::InvalidPacket)?;

        let response = str::from_utf8(response)
            .map_err(|_| RconError::InvalidUtf8)?
            .trim_matches(char::from(0))
            .trim();

        let mut parts = response.split_whitespace();

        if parts.next() != Some("challenge") || parts.next() != Some("rcon") {
            return Err(RconError::InvalidPacket);
        }

        let challenge = parts.next().ok_or(RconError::InvalidPacket)?;
        let packet = rcon_request(challenge, &self.password, command);
        socket.send_to(&packet, self.addr).await?;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.socket = None;
        self.authenticated = false;
    }

    pub fn is_connected(&self) -> bool {
        self.authenticated && self.socket.is_some()
    }

    fn is_valid_status_response(&self, response: &str) -> bool {
        let response = response.trim();

        response.starts_with("Server:") && response.ends_with("#end")
    }
}
