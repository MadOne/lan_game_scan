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
        let stream = timeout(Duration::from_secs(3), TcpStream::connect(self.addr))
            .await
            .map_err(|_| RconError::Timeout)?
            .map_err(|e| RconError::Connection(e.to_string()))?;

        self.stream = Some(stream);

        // SERVERDATA_AUTH
        let packet = SourceRconPacket::new(99, 3, self.password.clone());

        self.send_packet(&packet).await?;

        let response = self.receive_packet().await?;

        /*println!(
            "[RCON] Auth response: id={}, type={}, body={:?}",
            response.id, response.packet_type, response.body
        );*/

        // Source RCON authentication failure is indicated by ID -1.
        if response.id == -1 {
            self.stream = None;
            return Err(RconError::AuthenticationFailed);
        }

        // We expect SERVERDATA_AUTH_RESPONSE (type 2).
        if response.packet_type != 2 {
            self.stream = None;

            return Err(RconError::AuthenticationFailed);
        }

        // We expect our authentication request ID back.
        if response.id != 99 {
            self.stream = None;

            return Err(RconError::AuthenticationFailed);
        }

        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.stream = None;
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub async fn command(&mut self, command: &str) -> Result<String, RconError> {
        let packet = SourceRconPacket::new(1, 2, command);
        self.send_packet(&packet).await?;
        let response = self.receive_packet().await?;
        Ok(response.body)
    }

    pub async fn command_no_response(&mut self, command: &str) -> Result<(), RconError> {
        let packet = SourceRconPacket::new(1, 2, command);
        self.send_packet(&packet).await
    }

    async fn send_packet(&mut self, packet: &SourceRconPacket) -> Result<(), RconError> {
        //println!("[RCON DEBUG] send_packet(): entered");

        let stream = self.stream.as_mut().ok_or(RconError::NotConnected)?;

        //println!("[RCON DEBUG] send_packet(): stream available");

        let bytes = packet.to_bytes();

        //println!("[RCON DEBUG] send_packet(): writing {} bytes", bytes.len());

        timeout(Duration::from_secs(3), stream.write_all(&bytes))
            .await
            .map_err(|_| RconError::Timeout)??;

        //println!("[RCON DEBUG] send_packet(): write completed");

        Ok(())
    }

    async fn receive_packet(&mut self) -> Result<SourceRconPacket, RconError> {
        let stream = self.stream.as_mut().ok_or(RconError::NotConnected)?;

        let mut size_buf = [0u8; 4];

        timeout(Duration::from_secs(3), stream.read_exact(&mut size_buf))
            .await
            .map_err(|_| RconError::Timeout)??;

        let size = i32::from_le_bytes(size_buf);

        if size < 10 {
            return Err(RconError::InvalidPacket);
        }

        let mut payload = vec![0u8; size as usize];

        timeout(Duration::from_secs(3), stream.read_exact(&mut payload))
            .await
            .map_err(|_| RconError::Timeout)??;

        let mut packet = Vec::with_capacity(4 + payload.len());

        packet.extend_from_slice(&size_buf);
        packet.extend_from_slice(&payload);

        SourceRconPacket::from_bytes(&packet)
    }
}
