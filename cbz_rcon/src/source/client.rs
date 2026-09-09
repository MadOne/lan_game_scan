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

        // SERVERDATA_AUTH
        let packet = SourceRconPacket::new(99, 3, self.password.clone());

        self.send_packet(&packet).await?;

        loop {
            let response = self.receive_packet().await?;

            log::trace!(
                target: "cbz_rcon::source",
                "Received authentication response: id={}, type={}, body_len={}",
                response.id,
                response.packet_type,
                response.body.len()
            );

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

        let packet = SourceRconPacket::new(1, 2, command);

        self.send_packet(&packet).await?;

        let response = self.receive_packet().await?;

        log::trace!(
            target: "cbz_rcon::source",
            "Received command response: id={}, type={}, body_len={}",
            response.id,
            response.packet_type,
            response.body.len()
        );

        Ok(response.body)
    }

    pub async fn command_no_response(&mut self, command: &str) -> Result<(), RconError> {
        log::debug!(
            target: "cbz_rcon::source",
            "Sending command without waiting for response: {:?}",
            command
        );

        let packet = SourceRconPacket::new(1, 2, command);

        self.send_packet(&packet).await
    }

    async fn send_packet(&mut self, packet: &SourceRconPacket) -> Result<(), RconError> {
        let stream = self.stream.as_mut().ok_or(RconError::NotConnected)?;

        let bytes = packet.to_bytes();

        log::trace!(
            target: "cbz_rcon::source",
            "Sending RCON packet: id={}, type={}, size={} bytes",
            packet.id,
            packet.packet_type,
            bytes.len()
        );

        timeout(Duration::from_secs(3), stream.write_all(&bytes))
            .await
            .map_err(|_| RconError::Timeout)??;

        Ok(())
    }

    async fn receive_packet(&mut self) -> Result<SourceRconPacket, RconError> {
        let stream = self.stream.as_mut().ok_or(RconError::NotConnected)?;

        let mut size_buf = [0u8; 4];

        timeout(Duration::from_secs(3), stream.read_exact(&mut size_buf))
            .await
            .map_err(|_| RconError::Timeout)??;

        let size = i32::from_le_bytes(size_buf);

        log::trace!(
            target: "cbz_rcon::source",
            "Received RCON packet header: size={}",
            size
        );

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
            .map_err(|_| RconError::Timeout)??;

        let mut packet = Vec::with_capacity(4 + payload.len());

        packet.extend_from_slice(&size_buf);
        packet.extend_from_slice(&payload);

        let packet = SourceRconPacket::from_bytes(&packet)?;

        log::trace!(
            target: "cbz_rcon::source",
            "Parsed RCON packet: id={}, type={}, body_len={}",
            packet.id,
            packet.packet_type,
            packet.body.len()
        );

        Ok(packet)
    }
}
