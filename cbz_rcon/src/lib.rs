use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RconStatus {
    Disconnected,
    Connecting,
    Authenticated,
    Error,
}

#[derive(Debug)]
pub struct RconPacket {
    pub id: i32,
    pub packet_type: i32,
    pub body: String,
}

impl RconPacket {
    pub fn new(id: i32, packet_type: i32, body: impl Into<String>) -> Self {
        Self {
            id,
            packet_type,
            body: body.into(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let body = self.body.as_bytes();

        let size = (body.len() + 10) as i32;

        let mut bytes = Vec::with_capacity((size + 4) as usize);

        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&self.id.to_le_bytes());
        bytes.extend_from_slice(&self.packet_type.to_le_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(&[0, 0]);

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RconError> {
        if bytes.len() < 12 {
            return Err(RconError::InvalidPacket);
        }

        let size = i32::from_le_bytes(
            bytes[0..4]
                .try_into()
                .map_err(|_| RconError::InvalidPacket)?,
        );

        if size < 10 {
            return Err(RconError::InvalidPacket);
        }

        let id = i32::from_le_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| RconError::InvalidPacket)?,
        );

        let packet_type = i32::from_le_bytes(
            bytes[8..12]
                .try_into()
                .map_err(|_| RconError::InvalidPacket)?,
        );

        let body_end = bytes.len().saturating_sub(2);

        let body =
            String::from_utf8(bytes[12..body_end].to_vec()).map_err(|_| RconError::InvalidUtf8)?;

        Ok(Self {
            id,
            packet_type,
            body,
        })
    }
}

#[derive(Debug)]
pub enum RconError {
    Connection(String),
    Timeout,
    AuthenticationFailed,
    NotConnected,
    InvalidPacket,
    InvalidUtf8,
    Io(String),
}

impl std::fmt::Display for RconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(e) => write!(f, "Connection error: {}", e),
            Self::Timeout => write!(f, "Connection timed out"),
            Self::AuthenticationFailed => write!(f, "Authentication failed"),
            Self::NotConnected => write!(f, "Not connected"),
            Self::InvalidPacket => write!(f, "Invalid RCON packet"),
            Self::InvalidUtf8 => write!(f, "Invalid UTF-8 in RCON response"),
            Self::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for RconError {}

impl From<std::io::Error> for RconError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

pub struct RconClient {
    addr: SocketAddr,
    password: String,
    stream: Option<TcpStream>,
}

impl RconClient {
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
        let packet = RconPacket::new(99, 3, self.password.clone());

        self.send_packet(&packet).await?;

        let response = self.receive_packet().await?;

        println!(
            "[RCON] Auth response: id={}, type={}, body={:?}",
            response.id, response.packet_type, response.body
        );

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
        println!("[RCON DEBUG] command(): sending '{}'", command);

        let packet = RconPacket::new(1, 2, command);

        println!("[RCON DEBUG] command(): calling send_packet()");

        self.send_packet(&packet).await?;

        println!("[RCON DEBUG] command(): send_packet() returned");

        println!("[RCON DEBUG] command(): calling receive_packet()");

        let response = self.receive_packet().await?;

        println!("[RCON DEBUG] command(): receive_packet() returned");

        Ok(response.body)
    }

    pub async fn command_no_response(&mut self, command: &str) -> Result<(), RconError> {
        let packet = RconPacket::new(1, 2, command);

        self.send_packet(&packet).await
    }

    async fn send_packet(&mut self, packet: &RconPacket) -> Result<(), RconError> {
        println!("[RCON DEBUG] send_packet(): entered");

        let stream = self.stream.as_mut().ok_or(RconError::NotConnected)?;

        println!("[RCON DEBUG] send_packet(): stream available");

        let bytes = packet.to_bytes();

        println!("[RCON DEBUG] send_packet(): writing {} bytes", bytes.len());

        timeout(Duration::from_secs(3), stream.write_all(&bytes))
            .await
            .map_err(|_| RconError::Timeout)??;

        println!("[RCON DEBUG] send_packet(): write completed");

        Ok(())
    }

    async fn receive_packet(&mut self) -> Result<RconPacket, RconError> {
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

        RconPacket::from_bytes(&packet)
    }
}
