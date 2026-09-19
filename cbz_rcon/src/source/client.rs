use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::RconError;
use crate::source::SourceRconPacket;

pub struct SourceRconClient {
    addr: SocketAddr,
    password: String,
    stream: Option<TcpStream>,
}

impl SourceRconClient {
    pub fn new(addr: SocketAddr, password: impl Into<String>) -> Self {
        Self {
            addr,
            password: password.into(),
            stream: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), RconError> {
        log::debug!(
            target: "cbz_rcon::source",
            "Connecting to {}",
            self.addr
        );

        let stream = timeout(Duration::from_secs(3), TcpStream::connect(self.addr))
            .await
            .map_err(|_| RconError::Timeout)?
            .map_err(|error| RconError::Connection(error.to_string()))?;

        self.stream = Some(stream);

        log::debug!(
            target: "cbz_rcon::source",
            "TCP connection established"
        );

        // SERVERDATA_AUTH (id = 99, type = 3)
        let packet = SourceRconPacket::new(99, 3, self.password.clone());
        self.send_packet(&packet).await?;

        loop {
            let response = self.receive_packet().await?;

            /*
            log::trace!(
                target: "cbz_rcon::source",
                "Received authentication response: id={}, type={}, body_len={}",
                response.id,
                response.packet_type,
                response.body.len()
            );
            */
            if response.id == -1 {
                log::warn!(
                    target: "cbz_rcon::source",
                    "Source RCON authentication failed: server rejected password"
                );

                self.stream = None;
                return Err(RconError::AuthenticationFailed);
            }

            if response.packet_type == 2 {
                if response.id != 99 {
                    log::warn!(
                        target: "cbz_rcon::source",
                        "Source RCON authentication failed: unexpected response id {}",
                        response.id
                    );

                    self.stream = None;
                    return Err(RconError::AuthenticationFailed);
                }

                log::debug!(
                    target: "cbz_rcon::source",
                    "Source RCON authentication successful"
                );

                return Ok(());
            }

            if response.packet_type == 0 && response.body.is_empty() {
                log::debug!(
                    target: "cbz_rcon::source",
                    "Received empty response-value packet during authentication"
                );

                continue;
            }

            log::warn!(
                target: "cbz_rcon::source",
                "Source RCON authentication failed: unexpected response type {}",
                response.packet_type
            );

            self.stream = None;
            return Err(RconError::AuthenticationFailed);
        }
    }

    pub fn disconnect(&mut self) {
        log::debug!(
            target: "cbz_rcon::source",
            "Disconnecting from {}",
            self.addr
        );

        self.stream = None;
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub async fn command(&mut self, command: &str) -> Result<String, RconError> {
        log::debug!(
            target: "cbz_rcon::source",
            "Sending command: {:?}",
            command
        );

        // 1. Send the command packet (id = 1, type 2)
        let cmd_packet = SourceRconPacket::new(1, 2, command);
        self.send_packet(&cmd_packet).await?;

        // 2. Send the sentinel packet (id = 2, type 0, empty body)
        let sentinel_packet = SourceRconPacket::new(2, 0, "");
        self.send_packet(&sentinel_packet).await?;

        let mut full_body = String::new();

        // 3. Loop until the sentinel echoes back
        loop {
            let response = self.receive_packet().await?;

            /*
            log::trace!(
                target: "cbz_rcon::source",
                "Received command packet: id={}, type={}, body_len={}",
                response.id,
                response.packet_type,
                response.body.len()
            );
            */
            if response.id == 1 {
                // Command chunk
                full_body.push_str(&response.body);
            } else if response.id == 2 {
                // Check if this is the TF2 junk packet (starts with \x00\x01)
                // If it is leftover junk, ignore it and continue reading!
                if response.body.as_bytes().starts_with(&[0x00, 0x01]) {
                    /*
                    log::trace!(
                        target: "cbz_rcon::source",
                        "Ignoring TF2 trailing junk packet"
                    );
                    */
                    continue;
                }

                // If this was the real empty sentinel (body.is_empty()),
                // on TF2 an extra junk packet may follow. We consume it if present.
                // We do a fast non-blocking or short 10ms peek/read:
                if let Ok(Ok(_junk)) =
                    timeout(Duration::from_millis(20), self.receive_packet()).await
                {
                    /*
                    log::trace!(
                        target: "cbz_rcon::source",
                        "Drained trailing TF2 packet: id={}, body_len={}",
                        junk.id,
                        junk.body.len()
                    );
                    */
                }

                break;
            }
        }

        Ok(full_body)
    }

    async fn send_packet(&mut self, packet: &SourceRconPacket) -> Result<(), RconError> {
        let stream = self.stream.as_mut().ok_or(RconError::NotConnected)?;

        let bytes = packet.to_bytes();
        /*
        log::trace!(
            target: "cbz_rcon::source",
            "Sending RCON packet: id={}, type={}, size={} bytes",
            packet.id,
            packet.packet_type,
            bytes.len()
        );
        */
        timeout(Duration::from_secs(3), stream.write_all(&bytes))
            .await
            .map_err(|_| RconError::Timeout)?
            .map_err(|error| RconError::Connection(error.to_string()))?;

        Ok(())
    }

    async fn receive_packet(&mut self) -> Result<SourceRconPacket, RconError> {
        let stream = self.stream.as_mut().ok_or(RconError::NotConnected)?;

        let mut size_buf = [0u8; 4];

        timeout(Duration::from_secs(3), stream.read_exact(&mut size_buf))
            .await
            .map_err(|_| RconError::Timeout)?
            .map_err(|error| RconError::Connection(error.to_string()))?;

        let size = i32::from_le_bytes(size_buf);

        /*
        log::trace!(
            target: "cbz_rcon::source",
            "Received RCON packet header: size={}",
            size
        );
        */
        if size < 10 {
            log::warn!(
                target: "cbz_rcon::source",
                "Invalid RCON packet size: {}",
                size
            );

            return Err(RconError::InvalidPacket);
        }

        let mut payload = vec![0u8; size as usize];

        timeout(Duration::from_secs(3), stream.read_exact(&mut payload))
            .await
            .map_err(|_| RconError::Timeout)?
            .map_err(|error| RconError::Connection(error.to_string()))?;

        let mut packet = Vec::with_capacity(4 + payload.len());

        packet.extend_from_slice(&size_buf);
        packet.extend_from_slice(&payload);

        let packet = SourceRconPacket::from_bytes(&packet)?;

        /*
        log::trace!(
            target: "cbz_rcon::source",
            "Parsed RCON packet: id={}, type={}, body_len={}",
            packet.id,
            packet.packet_type,
            packet.body.len()
        );
        */
        Ok(packet)
    }
}
