use std::net::SocketAddr;
use std::time::Duration;

use tokio::{net::UdpSocket, time::timeout};

use crate::RconError;

pub const QUAKE3_HEADER: [u8; 4] = [0xFF; 4];

pub struct Quake3RconClient {
    addr: SocketAddr,
    password: String,
    socket: Option<UdpSocket>,
    authenticated: bool,
}

impl Quake3RconClient {
    pub fn new(addr: SocketAddr, password: impl Into<String>) -> Self {
        Self {
            addr,
            password: password.into(),
            socket: None,
            authenticated: false,
        }
    }

    pub async fn connect(&mut self) -> Result<(), RconError> {
        log::debug!(
            target: "cbz_rcon::quake3",
            "Connecting to {}",
            self.addr
        );

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        self.socket = Some(socket);

        log::debug!(
            target: "cbz_rcon::quake3",
            "UDP socket bound successfully"
        );

        let response = self.command("status").await?;

        self.authenticated = self.is_valid_status_response(&response);

        if !self.authenticated {
            log::warn!(
                target: "cbz_rcon::quake3",
                "Quake3 authentication failed: invalid status response"
            );

            return Err(RconError::AuthenticationFailed);
        }

        log::debug!(
            target: "cbz_rcon::quake3",
            "Quake3 authentication successful"
        );

        Ok(())
    }

    /// Sends an RCON command without waiting for a response.
    pub async fn command_no_response(&self, command: &str) -> Result<(), RconError> {
        let socket = self.socket.as_ref().ok_or(RconError::NotConnected)?;

        log::debug!(
            target: "cbz_rcon::quake3",
            "Sending command: {:?}",
            command
        );

        let payload = format!("rcon \"{}\" {}\n", self.password, command);
        let mut packet = QUAKE3_HEADER.to_vec();
        packet.extend_from_slice(payload.as_bytes());

        socket.send_to(&packet, self.addr).await?;

        log::trace!(
            target: "cbz_rcon::quake3",
            "RCON command packet sent to {}",
            self.addr
        );

        Ok(())
    }

    pub async fn command(&self, command: &str) -> Result<String, RconError> {
        let socket = self.socket.as_ref().ok_or(RconError::NotConnected)?;

        self.command_no_response(command).await?;

        log::debug!(
            target: "cbz_rcon::quake3",
            "Waiting for RCON response"
        );

        let mut full_response = String::new();
        let mut buf = [0u8; 4096];

        let mut current_timeout = Duration::from_secs(3);

        loop {
            let read_result = timeout(current_timeout, socket.recv_from(&mut buf)).await;

            match read_result {
                Ok(Ok((amt, source))) => {
                    log::trace!(
                        target: "cbz_rcon::quake3",
                        "Received RCON response packet: {} bytes from {}",
                        amt,
                        source
                    );

                    let response_bytes = &buf[..amt];

                    if response_bytes.starts_with(&QUAKE3_HEADER) {
                        let text_bytes =
                            if response_bytes.len() >= 10 && &response_bytes[4..10] == b"print\n" {
                                &response_bytes[10..]
                            } else {
                                &response_bytes[4..]
                            };

                        let text = String::from_utf8_lossy(text_bytes);
                        full_response.push_str(&text);
                    } else {
                        log::trace!(
                            target: "cbz_rcon::quake3",
                            "Ignoring packet with invalid RCON header"
                        );
                    }

                    current_timeout = Duration::from_millis(100);
                }

                Ok(Err(error)) => {
                    log::warn!(
                        target: "cbz_rcon::quake3",
                        "Failed to receive RCON response: {}",
                        error
                    );

                    return Err(RconError::from(error));
                }

                Err(_) => {
                    if full_response.is_empty() {
                        log::warn!(
                            target: "cbz_rcon::quake3",
                            "Timed out waiting for RCON response"
                        );

                        return Err(RconError::Timeout);
                    }

                    log::trace!(
                        target: "cbz_rcon::quake3",
                        "RCON response stream ended after timeout"
                    );

                    break;
                }
            }
        }

        Ok(full_response)
    }

    pub fn disconnect(&mut self) {
        log::debug!(
            target: "cbz_rcon::quake3",
            "Disconnecting from {}",
            self.addr
        );

        self.socket = None;
        self.authenticated = false;
    }

    pub fn is_connected(&self) -> bool {
        self.authenticated && self.socket.is_some()
    }

    fn is_valid_status_response(&self, response: &str) -> bool {
        let response = response.trim();

        !response.is_empty() && !response.contains("Bad rconpassword")
    }
}
